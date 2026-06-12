use std::process::ExitCode;

use _native::board::Board;
use _native::generator::{
    generate_fungster_board, generate_random_board, generate_rejection_solvable_board,
    FungsterAxis, FungsterConfig, RandomConfig, RejectionConfig, Rng64,
};
use _native::solver::{
    solve_exhaustive, solve_first_empty, MoveOrdering, SearchError, SolverLimits,
};
use clap::{Parser, ValueEnum};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
/// Generation modes are deliberately selected from the CLI so the same solver
/// binaries can compare constructed, random, and rejection-sampled populations.
enum GeneratorKind {
    Fungster,
    Random,
    Rejection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
/// Fungster's simplified construction can recurse through horizontal or
/// vertical strips; exposing this keeps both hack variants benchmarkable.
enum FungsterAxisArg {
    Row,
    Column,
}

#[derive(Clone, Debug, Parser)]
#[command(about = "Benchmark static Fruitbox solvers on generated boards")]
/// Clap-owned benchmark configuration. Keeping defaults here makes the binary
/// the reproducible entry point for timing and state-count comparisons.
struct Config {
    #[arg(long, value_enum, default_value = "fungster")]
    generator: GeneratorKind,
    #[arg(long, default_value_t = 17)]
    width: usize,
    #[arg(long, default_value_t = 10)]
    height: usize,
    #[arg(long, default_value_t = 3)]
    samples: usize,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    #[arg(long, default_value_t = 32)]
    groups: usize,
    #[arg(long, default_value_t = 2)]
    min_tuple: usize,
    #[arg(long, default_value_t = 4)]
    max_tuple: usize,
    #[arg(long, value_enum, default_value = "row")]
    fungster_axis: FungsterAxisArg,
    #[arg(long, default_value_t = 100)]
    max_attempts: usize,
    #[arg(long, default_value_t = 1_000_000)]
    max_states: usize,
    /// Stop exhaustive DP after this many empty-board solutions are encountered.
    #[arg(long)]
    max_empty_solutions: Option<u128>,
    /// Print sampled boards as text grids. By default this is sampling-only and
    /// does not run solvers unless `--run-solvers` is also set.
    #[arg(long)]
    print_board: bool,
    /// Run solver benchmarks even when `--print-board` is set.
    #[arg(long)]
    run_solvers: bool,
}

fn main() -> ExitCode {
    let config = Config::parse();

    let mut rng = Rng64::new(config.seed);
    if should_run_solvers(&config) {
        println!(
            "sample,generator,approach,width,height,total_sum,solvable,max_score,empty_steps,states,terminal_paths,empty_solutions,solution_limit_reached,elapsed_us,status"
        );
    }

    for sample in 0..config.samples {
        let board = match build_board(&config, &mut rng) {
            Ok(board) => board,
            Err(error) => {
                eprintln!("failed to generate sample {sample}: {error:?}");
                return ExitCode::FAILURE;
            }
        };
        if config.print_board {
            print_board(sample, &board);
        }
        if should_run_solvers(&config) {
            run_approaches(sample, &config, &board);
        }
    }

    ExitCode::SUCCESS
}

fn should_run_solvers(config: &Config) -> bool {
    !config.print_board || config.run_solvers
}

fn build_board(config: &Config, rng: &mut Rng64) -> Result<Board, String> {
    match config.generator {
        GeneratorKind::Fungster => generate_fungster_board(
            &FungsterConfig {
                width: config.width,
                height: config.height,
                groups: config.groups,
                min_tuple: config.min_tuple,
                max_tuple: config.max_tuple,
                axis: config.fungster_axis.into(),
            },
            rng,
        )
        .map_err(|error| format!("{error:?}")),
        GeneratorKind::Random => generate_random_board(
            &RandomConfig {
                width: config.width,
                height: config.height,
            },
            rng,
        )
        .map_err(|error| format!("{error:?}")),
        GeneratorKind::Rejection => generate_rejection_solvable_board(
            &RejectionConfig {
                width: config.width,
                height: config.height,
                max_attempts: config.max_attempts,
                solver_limits: SolverLimits {
                    max_states: config.max_states,
                    max_empty_solutions: config.max_empty_solutions,
                },
            },
            rng,
        )
        .map_err(|error| format!("{error:?}")),
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
    let generator = generator_name(config.generator);
    let total_sum: u16 = board.cells().iter().map(|cell| *cell as u16).sum();
    let limits = SolverLimits {
        max_states: config.max_states,
        max_empty_solutions: config.max_empty_solutions,
    };

    for (name, ordering) in [
        ("dfs_first_largest", MoveOrdering::LargestScoreFirst),
        ("dfs_first_smallest", MoveOrdering::SmallestScoreFirst),
    ] {
        match solve_first_empty(board, ordering, limits) {
            Ok(result) => {
                let elapsed_us = result.elapsed.as_micros();
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
            Err(error) => print_search_error(sample, generator, name, board, total_sum, error),
        }
    }

    match solve_exhaustive(board, limits) {
        Ok(result) => {
            let elapsed_us = result.elapsed.as_micros();
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
            print_search_error(sample, generator, "dp_exhaustive", board, total_sum, error)
        }
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

fn generator_name(generator: GeneratorKind) -> &'static str {
    match generator {
        GeneratorKind::Fungster => "fungster",
        GeneratorKind::Random => "random",
        GeneratorKind::Rejection => "rejection",
    }
}

impl From<FungsterAxisArg> for FungsterAxis {
    fn from(axis: FungsterAxisArg) -> Self {
        match axis {
            FungsterAxisArg::Row => Self::Row,
            FungsterAxisArg::Column => Self::Column,
        }
    }
}
