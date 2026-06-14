use std::path::PathBuf;
use std::process::ExitCode;

use _native::board::Board;
use _native::generator::{
    generate_fungster_board, generate_random_board, generate_rejection_solvable_board,
    FungsterConfig, FungsterPartitionStrategy, RandomConfig, RejectionConfig, Rng64,
    TupleDifficulty, TupleTargetSampling,
};
use _native::instrument::{
    instrumentation_available, profile_solver_call, FlamegraphEvent, FlamegraphSettings,
};
use _native::solver::{
    candidate_profile_snapshot, reset_candidate_profile, set_candidate_profile_enabled,
    solve_exhaustive, solve_first_empty, MoveOrdering, SearchError, SolverLimits,
};
use clap::{Args, Parser, ValueEnum};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
/// Generation modes are deliberately selected from the CLI so the same solver
/// binaries can compare constructed, random, and rejection-sampled populations.
enum GeneratorKind {
    Fungster,
    Random,
    Rejection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum FungsterPartitionArg {
    StraightStrips,
    RandomBacktracking,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum TupleTargetSamplingArg {
    Max,
    Uniform,
    Easy,
    Normal,
    Hard,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum SolverPresetArg {
    Iteration,
    Benchmark,
}

#[derive(Clone, Debug, Parser)]
#[command(about = "Benchmark static Fruitbox solvers on generated boards")]
/// Clap-owned benchmark configuration. Keeping defaults here makes the binary
/// the reproducible entry point for timing and state-count comparisons.
struct Config {
    #[command(flatten)]
    generation: GenerationArgs,
    #[command(flatten)]
    board: BoardArgs,
    #[command(flatten, next_help_heading = "Fungster")]
    fungster: FungsterArgs,
    #[command(flatten, next_help_heading = "Solver")]
    solver: SolverArgs,
    #[command(flatten, next_help_heading = "Output")]
    output: OutputArgs,
}

#[derive(Clone, Debug, Args)]
struct GenerationArgs {
    #[arg(long, value_enum, default_value = "fungster")]
    generator: GeneratorKind,
    #[arg(long, default_value_t = 3)]
    samples: usize,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Generation retry budget used by modes that can retry candidate boards.
    #[arg(long, alias = "groups", default_value_t = 32)]
    max_attempts: usize,
}

#[derive(Clone, Debug, Args)]
struct BoardArgs {
    #[arg(long, default_value_t = 17)]
    width: usize,
    #[arg(long, default_value_t = 10)]
    height: usize,
}

#[derive(Clone, Debug, Args)]
struct FungsterArgs {
    #[arg(long, value_enum, default_value = "straight-strips")]
    fungster_partition: FungsterPartitionArg,
    #[arg(long, default_value_t = 2)]
    min_tuple: usize,
    #[arg(long, default_value_t = 4)]
    max_tuple: usize,
    /// Lower target tuple tried after a sampled target fails. Defaults to min-tuple.
    #[arg(long)]
    fallback_min_tuple: Option<usize>,
    /// How to sample the target upper tuple for each generated board.
    #[arg(long, value_enum, default_value = "max")]
    tuple_target_sampling: TupleTargetSamplingArg,
    /// Comma-separated weights for min-tuple..=max-tuple, overriding tuple-target-sampling.
    #[arg(long)]
    tuple_weights: Option<String>,
}

#[derive(Clone, Debug, Args)]
struct SolverArgs {
    /// Preset state budgets: iteration=140, benchmark=2000.
    #[arg(long, value_enum, default_value = "benchmark")]
    solver_preset: SolverPresetArg,
    /// Override the selected solver preset state budget.
    #[arg(long)]
    max_states: Option<usize>,
    /// Stop exhaustive DP after this many empty-board solutions are encountered.
    #[arg(long)]
    max_empty_solutions: Option<u128>,
}

#[derive(Clone, Debug, Args)]
struct OutputArgs {
    /// Print sampled boards as text grids. By default this is sampling-only and
    /// does not run solvers unless `--run-solvers` is also set.
    #[arg(long)]
    print_board: bool,
    /// Run solver benchmarks even when `--print-board` is set.
    #[arg(long)]
    run_solvers: bool,
    /// Emit candidate vector histograms and collect/sort split per DFS run.
    #[arg(long)]
    candidate_profile: bool,
    /// Directory where per-approach flamegraph SVGs are written.
    #[arg(long)]
    flamegraph_dir: Option<PathBuf>,
    /// Sampling frequency for profiler-based flamegraph captures.
    #[arg(long, default_value_t = 199)]
    flamegraph_frequency: i32,
}

fn main() -> ExitCode {
    let config = Config::parse();
    set_candidate_profile_enabled(config.output.candidate_profile);
    if !instrumentation_available() && wants_instrumentation(&config) {
        eprintln!(
            "[fruitbox_bench] event=instrumentation_disabled reason=compiled_with_no_instrument"
        );
    }

    let mut rng = Rng64::new(config.generation.seed);
    if should_run_solvers(&config) {
        println!(
            "sample,generator,approach,width,height,total_sum,solvable,max_score,empty_steps,states,terminal_paths,empty_solutions,solution_limit_reached,elapsed_us,status"
        );
    }

    for sample in 0..config.generation.samples {
        let board = match build_board(&config, &mut rng) {
            Ok(board) => board,
            Err(error) => {
                eprintln!("failed to generate sample {sample}: {error:?}");
                return ExitCode::FAILURE;
            }
        };
        if config.output.print_board {
            print_board(sample, &board);
        }
        if should_run_solvers(&config) {
            run_approaches(sample, &config, &board);
        }
    }

    ExitCode::SUCCESS
}

fn should_run_solvers(config: &Config) -> bool {
    !config.output.print_board || config.output.run_solvers
}

fn wants_instrumentation(config: &Config) -> bool {
    config.output.candidate_profile || config.output.flamegraph_dir.is_some()
}

fn flamegraph_settings(config: &Config) -> FlamegraphSettings {
    FlamegraphSettings::from_parts(
        config.output.flamegraph_dir.clone(),
        config.output.flamegraph_frequency,
    )
}

fn effective_max_states(config: &Config) -> usize {
    config
        .solver
        .max_states
        .unwrap_or(match config.solver.solver_preset {
            SolverPresetArg::Iteration => 140,
            SolverPresetArg::Benchmark => 2_000,
        })
}

fn build_board(config: &Config, rng: &mut Rng64) -> Result<Board, String> {
    match config.generation.generator {
        GeneratorKind::Fungster => generate_fungster_board(
            &FungsterConfig {
                width: config.board.width,
                height: config.board.height,
                attempts: config.generation.max_attempts,
                min_tuple: config.fungster.min_tuple,
                max_tuple: config.fungster.max_tuple,
                fallback_min_tuple: config
                    .fungster
                    .fallback_min_tuple
                    .unwrap_or(config.fungster.min_tuple),
                target_tuple_sampling: fungster_tuple_sampling(&config.fungster)?,
                partition_strategy: config.fungster.fungster_partition.into(),
            },
            rng,
        )
        .map_err(|error| format!("{error:?}")),
        GeneratorKind::Random => generate_random_board(
            &RandomConfig {
                width: config.board.width,
                height: config.board.height,
            },
            rng,
        )
        .map_err(|error| format!("{error:?}")),
        GeneratorKind::Rejection => generate_rejection_solvable_board(
            &RejectionConfig {
                width: config.board.width,
                height: config.board.height,
                max_attempts: config.generation.max_attempts,
                solver_limits: SolverLimits {
                    max_states: effective_max_states(config),
                    max_empty_solutions: config.solver.max_empty_solutions,
                },
            },
            rng,
        )
        .map_err(|error| format!("{error:?}")),
    }
}

fn fungster_tuple_sampling(args: &FungsterArgs) -> Result<TupleTargetSampling, String> {
    if let Some(raw_weights) = &args.tuple_weights {
        return parse_tuple_weights(raw_weights).map(TupleTargetSampling::Weights);
    }

    Ok(match args.tuple_target_sampling {
        TupleTargetSamplingArg::Max => TupleTargetSampling::Max,
        TupleTargetSamplingArg::Uniform => TupleTargetSampling::Uniform,
        TupleTargetSamplingArg::Easy => TupleTargetSampling::Difficulty(TupleDifficulty::Easy),
        TupleTargetSamplingArg::Normal => TupleTargetSampling::Difficulty(TupleDifficulty::Normal),
        TupleTargetSamplingArg::Hard => TupleTargetSampling::Difficulty(TupleDifficulty::Hard),
    })
}

fn parse_tuple_weights(raw_weights: &str) -> Result<Vec<u32>, String> {
    if raw_weights.trim().is_empty() {
        return Err("--tuple-weights must not be empty".to_string());
    }

    raw_weights
        .split(',')
        .map(|part| {
            let value = part.trim();
            value
                .parse::<u32>()
                .map_err(|_| format!("invalid tuple weight: {value}"))
        })
        .collect()
}

impl From<FungsterPartitionArg> for FungsterPartitionStrategy {
    fn from(strategy: FungsterPartitionArg) -> Self {
        match strategy {
            FungsterPartitionArg::StraightStrips => Self::StraightStrips,
            FungsterPartitionArg::RandomBacktracking => Self::RandomBacktracking,
        }
    }
}

fn print_board(sample: usize, board: &Board) {
    println!(
        "# sample={sample} board={}x{} sum={}",
        board.width(),
        board.height(),
        board.cells().iter().map(|cell| *cell as u16).sum::<u16>()
    );
    for row in board.cells().chunks(board.width()) {
        let line = row
            .iter()
            .map(|cell| cell.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        println!("# {line}");
    }
}

fn run_approaches(sample: usize, config: &Config, board: &Board) {
    let generator = generator_name(config.generation.generator);
    let total_sum: u16 = board.cells().iter().map(|cell| *cell as u16).sum();
    let limits = SolverLimits {
        max_states: effective_max_states(config),
        max_empty_solutions: config.solver.max_empty_solutions,
    };
    let flamegraph = flamegraph_settings(config);
    trace_solver_batch_start(sample, generator, board, total_sum, limits);

    for (name, ordering) in [
        ("dfs_first_largest", MoveOrdering::LargestScoreFirst),
        ("dfs_first_smallest", MoveOrdering::SmallestScoreFirst),
    ] {
        if config.output.candidate_profile {
            reset_candidate_profile();
        }
        eprintln!("[fruitbox_bench] sample={sample} approach={name} event=start");
        let (result, flamegraph_event) = profile_solver_call(&flamegraph, sample, name, || {
            solve_first_empty(board, ordering, limits)
        });
        trace_flamegraph_event(sample, name, flamegraph_event.as_ref());
        match result {
            Ok(result) => {
                let elapsed_us = result.elapsed.as_micros();
                eprintln!(
                    "[fruitbox_bench] sample={sample} approach={name} event=finish status=ok solvable={} score={} steps={} states={} elapsed_us={elapsed_us}",
                    result.empty_solvable,
                    result.score,
                    option_u16(result.steps),
                    result.states_evaluated,
                );
                println!(
                    "{sample},{generator},{name},{},{},{total_sum},{},{},{},{},,,,{elapsed_us},ok",
                    board.width(),
                    board.height(),
                    result.empty_solvable,
                    result.score,
                    option_u16(result.steps),
                    result.states_evaluated,
                )
            }
            Err(error) => {
                trace_search_error(sample, name, &error);
                print_search_error(sample, generator, name, board, total_sum, error);
            }
        }
        if config.output.candidate_profile {
            trace_candidate_profile(sample, name);
        }
    }

    eprintln!("[fruitbox_bench] sample={sample} approach=dp_exhaustive event=start");
    let (result, flamegraph_event) =
        profile_solver_call(&flamegraph, sample, "dp_exhaustive", || {
            solve_exhaustive(board, limits)
        });
    trace_flamegraph_event(sample, "dp_exhaustive", flamegraph_event.as_ref());
    match result {
        Ok(result) => {
            let elapsed_us = result.elapsed.as_micros();
            eprintln!(
                "[fruitbox_bench] sample={sample} approach=dp_exhaustive event=finish status=ok solvable={} max_score={} min_empty_steps={} states={} terminal_paths={} empty_solutions={} solution_limit_reached={} elapsed_us={elapsed_us}",
                result.empty_solvable,
                result.max_score,
                option_u16(result.min_empty_steps),
                result.states_evaluated,
                result.terminal_paths,
                result.empty_solution_count,
                result.solution_limit_reached,
            );
            println!(
                "{sample},{generator},dp_exhaustive,{},{},{total_sum},{},{},{},{},{},{},{},{elapsed_us},ok",
                board.width(),
                board.height(),
                result.empty_solvable,
                result.max_score,
                option_u16(result.min_empty_steps),
                result.states_evaluated,
                result.terminal_paths,
                result.empty_solution_count,
                result.solution_limit_reached,
            )
        }
        Err(error) => {
            trace_search_error(sample, "dp_exhaustive", &error);
            print_search_error(sample, generator, "dp_exhaustive", board, total_sum, error)
        }
    }
}

fn trace_solver_batch_start(
    sample: usize,
    generator: &str,
    board: &Board,
    total_sum: u16,
    limits: SolverLimits,
) {
    eprintln!(
        "[fruitbox_bench] sample={sample} event=solver_batch_start generator={generator} board={}x{} total_sum={total_sum} max_states={} max_empty_solutions={}",
        board.width(),
        board.height(),
        limits.max_states,
        option_u128(limits.max_empty_solutions),
    );
}

fn trace_search_error(sample: usize, approach: &str, error: &SearchError) {
    match error {
        SearchError::StateLimitExceeded { max_states } => eprintln!(
            "[fruitbox_bench] sample={sample} approach={approach} event=finish status=state_limit max_states={max_states}"
        ),
    }
}

fn trace_candidate_profile(sample: usize, approach: &str) {
    if let Some(profile) = candidate_profile_snapshot() {
        eprintln!(
            "[fruitbox_bench] sample={sample} approach={approach} event=candidate_profile calls={} total_candidates={} avg_candidates={:.2} p50={} p90={} p99={} max={} collect_time_us={} sort_time_us={}",
            profile.calls,
            profile.total_candidates,
            profile.avg_candidates,
            profile.p50_candidates,
            profile.p90_candidates,
            profile.p99_candidates,
            profile.max_candidates,
            profile.collect_time_us,
            profile.sort_time_us,
        );
    }
}

fn trace_flamegraph_event(sample: usize, approach: &str, event: Option<&FlamegraphEvent>) {
    match event {
        Some(FlamegraphEvent::Written { path, elapsed_us }) => eprintln!(
            "[fruitbox_bench] sample={sample} approach={approach} event=flamegraph_written path={} elapsed_us={elapsed_us}",
            path.display()
        ),
        Some(FlamegraphEvent::Failed { reason }) => eprintln!(
            "[fruitbox_bench] sample={sample} approach={approach} event=flamegraph_failed reason={reason}"
        ),
        Some(FlamegraphEvent::Skipped { reason }) => eprintln!(
            "[fruitbox_bench] sample={sample} approach={approach} event=flamegraph_skipped reason={reason}"
        ),
        None => {}
    }
}

fn print_search_error(
    sample: usize,
    generator: &str,
    approach: &str,
    board: &Board,
    total_sum: u16,
    error: SearchError,
) {
    let status = match error {
        SearchError::StateLimitExceeded { max_states } => format!("state_limit_{max_states}"),
    };
    println!(
        "{sample},{generator},{approach},{},{},{total_sum},false,0,,0,,,,,{status}",
        board.width(),
        board.height(),
    );
}

fn option_u16(value: Option<u16>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

fn option_u128(value: Option<u128>) -> String {
    value.map_or_else(|| "none".to_string(), |value| value.to_string())
}

fn generator_name(generator: GeneratorKind) -> &'static str {
    match generator {
        GeneratorKind::Fungster => "fungster",
        GeneratorKind::Random => "random",
        GeneratorKind::Rejection => "rejection",
    }
}
