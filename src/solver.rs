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
use std::time::{Duration, Instant};

use crate::board::{Board, Mask, Rectangle, TARGET_SUM};

#[derive(Clone, Copy, Debug)]
/// Bounds intentionally live outside each solver so benchmark runs can compare
/// algorithms under the same search budget without changing solver internals.
pub struct SolverLimits {
    pub max_states: usize,
}

impl Default for SolverLimits {
    fn default() -> Self {
        Self {
            max_states: 1_000_000,
        }
    }
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
pub struct ExhaustiveSummary {
    pub empty_solvable: bool,
    pub max_score: u16,
    pub min_empty_steps: Option<u16>,
    pub empty_solution_count: u128,
    pub states_evaluated: usize,
    pub terminal_paths: u128,
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

pub fn solve_exhaustive(
    board: &Board,
    limits: SolverLimits,
) -> Result<ExhaustiveSummary, SearchError> {
    let started = Instant::now();
    let mut memo = HashMap::new();
    let summary = solve_state(board, board.initial_state(), &mut memo, limits)?;

    Ok(ExhaustiveSummary {
        empty_solvable: summary.min_empty_steps.is_some(),
        max_score: summary.max_score,
        min_empty_steps: summary.min_empty_steps,
        empty_solution_count: summary.empty_solution_count,
        states_evaluated: memo.len(),
        terminal_paths: summary.terminal_paths,
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
    limits: SolverLimits,
) -> Result<StateSummary, SearchError> {
    if let Some(summary) = memo.get(&state) {
        return Ok(*summary);
    }
    if memo.len() >= limits.max_states {
        return Err(SearchError::StateLimitExceeded {
            max_states: limits.max_states,
        });
    }
    if state.is_empty() {
        let summary = StateSummary {
            max_score: 0,
            min_empty_steps: Some(0),
            empty_solution_count: 1,
            terminal_paths: 1,
        };
        memo.insert(state, summary);
        return Ok(summary);
    }

    let mut max_score = 0;
    let mut min_empty_steps = None;
    let mut empty_solution_count = 0_u128;
    let mut terminal_paths = 0_u128;
    let mut found_move = false;

    for rectangle in board.valid_moves(state, TARGET_SUM) {
        found_move = true;
        let next = board.apply(state, rectangle);
        let child = solve_state(board, next, memo, limits)?;
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
    memo.insert(state, summary);
    Ok(summary)
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
    let mut candidates: Vec<Rectangle> = board.valid_moves(state, TARGET_SUM).collect();
    candidates.sort_by_key(|rectangle| board.live_score(state, *rectangle));
    if ordering == MoveOrdering::LargestScoreFirst {
        candidates.reverse();
    }
    candidates
}
