//! Static Fruitbox search over the official 17x10 sum-10 rectangle game.
//!
//! The user-facing questions are intentionally query-shaped rather than
//! sequence-shaped: whether a board can be emptied, the highest score reachable
//! over all legal play, and the minimum number of moves needed to empty the
//! board. A naive DFS over rectangles is useful as a witness search, but the
//! "all solutions" solver should not materialize every move sequence. Instead,
//! this module treats a board position as a live-cell bitmask and memoizes a
//! compact summary per reachable state.
//!
//! Initial generated boards contain positive apple scores, while recursive
//! search represents cleared apples by removing bits from the live mask. That
//! lets rectangle sums stay deterministic and non-negative without mutating the
//! original board values. The exhaustive DP answers aggregate questions about
//! the reachable state graph; the first-empty DFS remains separate because
//! rejection sampling and solver benchmarks often only need one empty-board
//! witness under a fixed state budget.

use std::collections::{HashMap, HashSet};
#[cfg(not(feature = "no_instrument"))]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(feature = "no_instrument"))]
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::board::{Board, Mask, Rectangle, TARGET_SUM};

#[derive(Clone, Copy, Debug)]
/// Bounds intentionally live outside each solver so benchmark runs can compare
/// algorithms under the same search budget without changing solver internals.
/// `max_empty_solutions` is a negotiable exhaustive-search hint: unset means
/// exact/all solutions, while `Some(n)` allows stopping after n empty paths.
pub struct SolverLimits {
    pub max_states: usize,
    pub max_empty_solutions: Option<u128>,
}

impl Default for SolverLimits {
    fn default() -> Self {
        Self {
            max_states: 1_000_000,
            max_empty_solutions: None,
        }
    }
}

pub fn set_candidate_profile_enabled(enabled: bool) {
    #[cfg(not(feature = "no_instrument"))]
    {
        CANDIDATE_PROFILE_ENABLED.store(enabled, Ordering::Relaxed);
    }
    #[cfg(feature = "no_instrument")]
    {
        let _ = enabled;
    }
}

pub fn reset_candidate_profile() {
    #[cfg(not(feature = "no_instrument"))]
    {
        if !candidate_profile_enabled() {
            return;
        }
        let mutex = CANDIDATE_PROFILE.get_or_init(|| Mutex::new(CandidateProfile::default()));
        if let Ok(mut profile) = mutex.lock() {
            *profile = CandidateProfile::default();
        }
    }
}

pub fn candidate_profile_snapshot() -> Option<CandidateProfileSnapshot> {
    #[cfg(not(feature = "no_instrument"))]
    {
        if !candidate_profile_enabled() {
            return None;
        }
        let mutex = CANDIDATE_PROFILE.get_or_init(|| Mutex::new(CandidateProfile::default()));
        let profile = mutex.lock().ok()?.clone();
        if profile.calls == 0 {
            return Some(CandidateProfileSnapshot {
                calls: 0,
                total_candidates: 0,
                avg_candidates: 0.0,
                p50_candidates: 0,
                p90_candidates: 0,
                p99_candidates: 0,
                max_candidates: 0,
                collect_time_us: 0,
                sort_time_us: 0,
            });
        }
        return Some(CandidateProfileSnapshot {
            calls: profile.calls,
            total_candidates: profile.total_candidates,
            avg_candidates: profile.total_candidates as f64 / profile.calls as f64,
            p50_candidates: histogram_quantile(&profile.histogram, profile.calls, 0.50),
            p90_candidates: histogram_quantile(&profile.histogram, profile.calls, 0.90),
            p99_candidates: histogram_quantile(&profile.histogram, profile.calls, 0.99),
            max_candidates: profile.max_candidates,
            collect_time_us: profile.collect_ns / 1_000,
            sort_time_us: profile.sort_ns / 1_000,
        });
    }
    #[cfg(feature = "no_instrument")]
    {
        None
    }
}

#[cfg(not(feature = "no_instrument"))]
fn candidate_profile_enabled() -> bool {
    CANDIDATE_PROFILE_ENABLED.load(Ordering::Relaxed)
}

#[cfg(feature = "no_instrument")]
fn candidate_profile_enabled() -> bool {
    false
}

#[cfg(not(feature = "no_instrument"))]
fn record_candidate_profile(candidate_count: usize, collect_ns: u128, sort_ns: u128) {
    if !candidate_profile_enabled() {
        return;
    }
    let mutex = CANDIDATE_PROFILE.get_or_init(|| Mutex::new(CandidateProfile::default()));
    let Ok(mut profile) = mutex.lock() else {
        return;
    };
    profile.calls = profile.calls.saturating_add(1);
    profile.total_candidates = profile
        .total_candidates
        .saturating_add(candidate_count as u64);
    profile.max_candidates = profile.max_candidates.max(candidate_count);
    profile.collect_ns = profile.collect_ns.saturating_add(collect_ns);
    profile.sort_ns = profile.sort_ns.saturating_add(sort_ns);
    if candidate_count >= profile.histogram.len() {
        profile.histogram.resize(candidate_count + 1, 0);
    }
    profile.histogram[candidate_count] = profile.histogram[candidate_count].saturating_add(1);
}

#[cfg(feature = "no_instrument")]
fn record_candidate_profile(_candidate_count: usize, _collect_ns: u128, _sort_ns: u128) {}

#[cfg(not(feature = "no_instrument"))]
fn histogram_quantile(histogram: &[u64], total_count: u64, quantile: f64) -> usize {
    if total_count == 0 {
        return 0;
    }
    let target = ((total_count as f64 * quantile).ceil() as u64).max(1);
    let mut seen = 0_u64;
    for (value, count) in histogram.iter().enumerate() {
        seen = seen.saturating_add(*count);
        if seen >= target {
            return value;
        }
    }
    histogram.len().saturating_sub(1)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// First-solution DFS is heuristic-sensitive, so ordering is explicit and
/// benchmarkable instead of being hidden in the recursive search.
pub enum MoveOrdering {
    LargestScoreFirst,
    SmallestScoreFirst,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Search failures are separate from "unsolvable" results: a board can exceed a
/// benchmark budget before the solver proves anything about the state graph.
pub enum SearchError {
    StateLimitExceeded { max_states: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Compressed answer for the "all solutions" static solver. The DP stores this
/// summary per reachable board mask so callers can query solvability, best
/// terminal score, and shortest empty-board path without materializing paths.
/// If `solution_limit_reached` is true, score/step fields describe the explored
/// prefix rather than a complete proof over the whole state graph.
pub struct ExhaustiveSummary {
    pub empty_solvable: bool,
    pub max_score: u16,
    pub min_empty_steps: Option<u16>,
    pub empty_solution_count: u128,
    pub states_evaluated: usize,
    pub terminal_paths: u128,
    pub solution_limit_reached: bool,
    pub elapsed: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A concrete witness from the fast DFS candidate. This exists separately from
/// `ExhaustiveSummary` because rejection sampling only needs one empty-board
/// proof, while exhaustive search needs aggregate facts about every branch.
pub struct SingleSolution {
    pub empty_solvable: bool,
    pub score: u16,
    pub steps: Option<u16>,
    pub moves: Vec<(usize, usize, usize, usize)>,
    pub states_evaluated: usize,
    pub elapsed: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Minimal first-hit result for callers that only need to know whether an
/// empty-board solution exists under a budget. Rejection sampling uses this
/// instead of `SingleSolution` to avoid retaining a path it will discard.
pub struct EmptySearchResult {
    pub empty_solvable: bool,
    pub states_evaluated: usize,
    pub elapsed: Duration,
}

#[derive(Clone, Debug, PartialEq)]
/// Optional per-call candidate telemetry for the DFS ordering stage. These
/// statistics are intended for local tuning and only collect data when the
/// benchmark explicitly enables candidate profiling.
pub struct CandidateProfileSnapshot {
    pub calls: u64,
    pub total_candidates: u64,
    pub avg_candidates: f64,
    pub p50_candidates: usize,
    pub p90_candidates: usize,
    pub p99_candidates: usize,
    pub max_candidates: usize,
    pub collect_time_us: u128,
    pub sort_time_us: u128,
}

#[derive(Clone, Debug, Default)]
#[cfg(not(feature = "no_instrument"))]
struct CandidateProfile {
    calls: u64,
    total_candidates: u64,
    max_candidates: usize,
    collect_ns: u128,
    sort_ns: u128,
    histogram: Vec<u64>,
}

#[cfg(not(feature = "no_instrument"))]
static CANDIDATE_PROFILE_ENABLED: AtomicBool = AtomicBool::new(false);
#[cfg(not(feature = "no_instrument"))]
static CANDIDATE_PROFILE: OnceLock<Mutex<CandidateProfile>> = OnceLock::new();

#[derive(Clone, Copy, Debug)]
/// Internal memo payload for one board state. Keeping this smaller than the
/// public summary avoids recording elapsed time and evaluated-state counts per
/// node, which belong to the whole run rather than each state.
struct StateSummary {
    max_score: u16,
    min_empty_steps: Option<u16>,
    empty_solution_count: u128,
    terminal_paths: u128,
}

#[derive(Clone, Copy, Debug, Default)]
struct ExhaustiveProgress {
    states_evaluated: usize,
    empty_solutions_seen: u128,
    solution_limit_reached: bool,
}

pub fn solve_exhaustive(
    board: &Board,
    limits: SolverLimits,
) -> Result<ExhaustiveSummary, SearchError> {
    let started = Instant::now();
    let mut memo = HashMap::new();
    let mut progress = ExhaustiveProgress::default();
    let use_memo = limits.max_empty_solutions.is_none();

    if solution_limit_reached(0, limits) {
        return Ok(ExhaustiveSummary {
            empty_solvable: false,
            max_score: 0,
            min_empty_steps: None,
            empty_solution_count: 0,
            states_evaluated: 0,
            terminal_paths: 0,
            solution_limit_reached: true,
            elapsed: started.elapsed(),
        });
    }

    let summary = solve_state(
        board,
        board.initial_state(),
        &mut memo,
        &mut progress,
        limits,
        use_memo,
    )?;

    Ok(ExhaustiveSummary {
        empty_solvable: summary.min_empty_steps.is_some(),
        max_score: summary.max_score,
        min_empty_steps: summary.min_empty_steps,
        empty_solution_count: summary.empty_solution_count,
        states_evaluated: progress.states_evaluated,
        terminal_paths: summary.terminal_paths,
        solution_limit_reached: progress.solution_limit_reached,
        elapsed: started.elapsed(),
    })
}

pub fn has_empty_solution(
    board: &Board,
    ordering: MoveOrdering,
    limits: SolverLimits,
) -> Result<EmptySearchResult, SearchError> {
    let started = Instant::now();
    let mut dead_states = HashSet::new();
    let mut states_evaluated = 0;
    let empty_solvable = dfs_has_empty(
        board,
        board.initial_state(),
        ordering,
        limits,
        &mut dead_states,
        &mut states_evaluated,
    )?;

    Ok(EmptySearchResult {
        empty_solvable,
        states_evaluated,
        elapsed: started.elapsed(),
    })
}

pub fn solve_first_empty(
    board: &Board,
    ordering: MoveOrdering,
    limits: SolverLimits,
) -> Result<SingleSolution, SearchError> {
    let started = Instant::now();
    let mut dead_states = HashSet::new();
    let mut moves = Vec::new();
    let mut states_evaluated = 0;
    let solved = dfs_first_empty(
        board,
        board.initial_state(),
        ordering,
        limits,
        &mut dead_states,
        &mut moves,
        &mut states_evaluated,
    )?;
    let score = moves.iter().map(|(_, _, _, _, score)| *score).sum();
    let move_coords: Vec<(usize, usize, usize, usize)> = moves
        .into_iter()
        .map(|(left, top, right, bottom, _)| (left, top, right, bottom))
        .collect();

    Ok(SingleSolution {
        empty_solvable: solved,
        score,
        steps: solved.then_some(move_coords_len(&move_coords)),
        moves: move_coords,
        states_evaluated,
        elapsed: started.elapsed(),
    })
}

fn move_coords_len(moves: &[(usize, usize, usize, usize)]) -> u16 {
    moves.len().min(u16::MAX as usize) as u16
}

fn solve_state(
    board: &Board,
    state: Mask,
    memo: &mut HashMap<Mask, StateSummary>,
    progress: &mut ExhaustiveProgress,
    limits: SolverLimits,
    use_memo: bool,
) -> Result<StateSummary, SearchError> {
    if use_memo {
        if let Some(summary) = memo.get(&state) {
            return Ok(*summary);
        }
    }
    if progress.states_evaluated >= limits.max_states {
        return Err(SearchError::StateLimitExceeded {
            max_states: limits.max_states,
        });
    }
    progress.states_evaluated += 1;
    if state.is_empty() {
        progress.empty_solutions_seen = progress.empty_solutions_seen.saturating_add(1);
        if solution_limit_reached(progress.empty_solutions_seen, limits) {
            progress.solution_limit_reached = true;
        }
        let summary = StateSummary {
            max_score: 0,
            min_empty_steps: Some(0),
            empty_solution_count: 1,
            terminal_paths: 1,
        };
        if use_memo {
            memo.insert(state, summary);
        }
        return Ok(summary);
    }

    let mut max_score = 0;
    let mut min_empty_steps = None;
    let mut empty_solution_count = 0_u128;
    let mut terminal_paths = 0_u128;
    let mut found_move = false;

    for rectangle in board.valid_moves(state, TARGET_SUM) {
        if progress.solution_limit_reached {
            break;
        }
        found_move = true;
        let next = board.apply(state, rectangle);
        let child = solve_state(board, next, memo, progress, limits, use_memo)?;
        let move_score = board.live_score(state, rectangle);
        max_score = max_score.max(move_score.saturating_add(child.max_score));

        if let Some(child_steps) = child.min_empty_steps {
            let candidate = child_steps.saturating_add(1);
            min_empty_steps =
                Some(min_empty_steps.map_or(candidate, |steps: u16| steps.min(candidate)));
            empty_solution_count = empty_solution_count.saturating_add(child.empty_solution_count);
        }

        terminal_paths = terminal_paths.saturating_add(child.terminal_paths);
    }

    if !found_move {
        terminal_paths = 1;
    }

    let summary = StateSummary {
        max_score,
        min_empty_steps,
        empty_solution_count,
        terminal_paths,
    };
    if use_memo {
        memo.insert(state, summary);
    }
    Ok(summary)
}

fn solution_limit_reached(empty_solutions_seen: u128, limits: SolverLimits) -> bool {
    limits
        .max_empty_solutions
        .is_some_and(|limit| empty_solutions_seen >= limit)
}

fn dfs_has_empty(
    board: &Board,
    state: Mask,
    ordering: MoveOrdering,
    limits: SolverLimits,
    dead_states: &mut HashSet<Mask>,
    states_evaluated: &mut usize,
) -> Result<bool, SearchError> {
    if state.is_empty() {
        return Ok(true);
    }
    if dead_states.contains(&state) {
        return Ok(false);
    }
    if *states_evaluated >= limits.max_states {
        return Err(SearchError::StateLimitExceeded {
            max_states: limits.max_states,
        });
    }
    *states_evaluated += 1;

    for rectangle in ordered_candidates(board, state, ordering) {
        if dfs_has_empty(
            board,
            board.apply(state, rectangle),
            ordering,
            limits,
            dead_states,
            states_evaluated,
        )? {
            return Ok(true);
        }
    }

    dead_states.insert(state);
    Ok(false)
}

fn dfs_first_empty(
    board: &Board,
    state: Mask,
    ordering: MoveOrdering,
    limits: SolverLimits,
    dead_states: &mut HashSet<Mask>,
    moves: &mut Vec<(usize, usize, usize, usize, u16)>,
    states_evaluated: &mut usize,
) -> Result<bool, SearchError> {
    if state.is_empty() {
        return Ok(true);
    }
    if dead_states.contains(&state) {
        return Ok(false);
    }
    if *states_evaluated >= limits.max_states {
        return Err(SearchError::StateLimitExceeded {
            max_states: limits.max_states,
        });
    }
    *states_evaluated += 1;

    for rectangle in ordered_candidates(board, state, ordering) {
        let score = board.live_score(state, rectangle);
        moves.push((
            rectangle.left,
            rectangle.top,
            rectangle.right,
            rectangle.bottom,
            score,
        ));
        if dfs_first_empty(
            board,
            board.apply(state, rectangle),
            ordering,
            limits,
            dead_states,
            moves,
            states_evaluated,
        )? {
            return Ok(true);
        }
        moves.pop();
    }

    dead_states.insert(state);
    Ok(false)
}

fn ordered_candidates(board: &Board, state: Mask, ordering: MoveOrdering) -> Vec<Rectangle> {
    let profiling_enabled = candidate_profile_enabled();
    let collect_started = profiling_enabled.then(Instant::now);
    let mut candidates: Vec<Rectangle> = board.valid_moves(state, TARGET_SUM).collect();
    let collect_ns = collect_started.map_or(0, |started| started.elapsed().as_nanos());
    let sort_started = profiling_enabled.then(Instant::now);
    candidates.sort_by_key(|rectangle| board.live_score(state, *rectangle));
    if ordering == MoveOrdering::LargestScoreFirst {
        candidates.reverse();
    }
    let sort_ns = sort_started.map_or(0, |started| started.elapsed().as_nanos());
    record_candidate_profile(candidates.len(), collect_ns, sort_ns);
    candidates
}
