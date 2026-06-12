use std::env;
use std::process::ExitCode;

use _native::board::Board;
use _native::generator::{
    generate_fungster_board, generate_random_board, generate_rejection_solvable_board,
    FungsterConfig, RandomConfig, RejectionConfig, Rng64,
};
use _native::solver::{
    solve_exhaustive, solve_first_empty, MoveOrdering, SearchError, SolverLimits,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GeneratorKind {
    Fungster,
    Random,
    Rejection,
}

#[derive(Clone, Debug)]
struct Config {
    generator: GeneratorKind,
    width: usize,
    height: usize,
    samples: usize,
    seed: u64,
    groups: usize,
    min_tuple: usize,
    max_tuple: usize,
    max_attempts: usize,
    max_states: usize,
    print_board: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            generator: GeneratorKind::Fungster,
            width: 17,
            height: 10,
            samples: 3,
            seed: 1,
            groups: 32,
            min_tuple: 2,
            max_tuple: 4,
            max_attempts: 100,
            max_states: 1_000_000,
            print_board: false,
        }
    }
}

fn main() -> ExitCode {
    let config = match parse_args(env::args().skip(1)) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            print_usage();
            return ExitCode::FAILURE;
        }
    };

    let mut rng = Rng64::new(config.seed);
    println!(
        "sample,generator,approach,width,height,total_sum,solvable,max_score,empty_steps,states,terminal_paths,empty_solutions,elapsed_us,status"
    );

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
        run_approaches(sample, &config, &board);
    }

    ExitCode::SUCCESS
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
    let generator = match config.generator {
        GeneratorKind::Fungster => "fungster",
        GeneratorKind::Random => "random",
        GeneratorKind::Rejection => "rejection",
    };
    let total_sum: u16 = board.cells().iter().map(|cell| *cell as u16).sum();
    let limits = SolverLimits {
        max_states: config.max_states,
    };

    for (name, ordering) in [
        ("dfs_first_largest", MoveOrdering::LargestScoreFirst),
        ("dfs_first_smallest", MoveOrdering::SmallestScoreFirst),
    ] {
        match solve_first_empty(board, ordering, limits) {
            Ok(result) => println!(
                "{sample},{generator},{name},{},{},{total_sum},{},{},{},{},,,{},ok",
                board.width(),
                board.height(),
                result.empty_solvable,
                result.score,
                option_u16(result.steps),
                result.states_evaluated,
                result.elapsed.as_micros(),
            ),
            Err(error) => print_search_error(sample, generator, name, board, total_sum, error),
        }
    }

    match solve_exhaustive(board, limits) {
        Ok(result) => println!(
            "{sample},{generator},dp_exhaustive,{},{},{total_sum},{},{},{},{},{},{},{},ok",
            board.width(),
            board.height(),
            result.empty_solvable,
            result.max_score,
            option_u16(result.min_empty_steps),
            result.states_evaluated,
            result.terminal_paths,
            result.empty_solution_count,
            result.elapsed.as_micros(),
        ),
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
        "{sample},{generator},{approach},{},{},{total_sum},false,0,,0,,,,{status}",
        board.width(),
        board.height(),
    );
}

fn option_u16(value: Option<u16>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Config, String> {
    let mut config = Config::default();
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--generator" => {
                config.generator = match take_value(&mut args, "--generator")?.as_str() {
                    "fungster" => GeneratorKind::Fungster,
                    "random" => GeneratorKind::Random,
                    "rejection" => GeneratorKind::Rejection,
                    value => return Err(format!("unknown generator {value:?}")),
                };
            }
            "--width" => config.width = parse_usize(&mut args, "--width")?,
            "--height" => config.height = parse_usize(&mut args, "--height")?,
            "--samples" => config.samples = parse_usize(&mut args, "--samples")?,
            "--seed" => config.seed = parse_u64(&mut args, "--seed")?,
            "--groups" => config.groups = parse_usize(&mut args, "--groups")?,
            "--min-tuple" => config.min_tuple = parse_usize(&mut args, "--min-tuple")?,
            "--max-tuple" => config.max_tuple = parse_usize(&mut args, "--max-tuple")?,
            "--max-attempts" => config.max_attempts = parse_usize(&mut args, "--max-attempts")?,
            "--max-states" => config.max_states = parse_usize(&mut args, "--max-states")?,
            "--print-board" => config.print_board = true,
            "--help" | "-h" => return Err(String::new()),
            value => return Err(format!("unknown argument {value:?}")),
        }
    }

    Ok(config)
}

fn take_value(
    args: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    flag: &str,
) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_usize(
    args: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    flag: &str,
) -> Result<usize, String> {
    take_value(args, flag)?
        .parse()
        .map_err(|_| format!("{flag} requires a positive integer"))
}

fn parse_u64(
    args: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    flag: &str,
) -> Result<u64, String> {
    take_value(args, flag)?
        .parse()
        .map_err(|_| format!("{flag} requires a positive integer"))
}

fn print_usage() {
    eprintln!(
        "usage: cargo run --release --bin fruitbox_bench -- [--generator fungster|random|rejection] [--width N] [--height N] [--samples N] [--seed N] [--groups N] [--min-tuple N] [--max-tuple N] [--max-attempts N] [--max-states N] [--print-board]"
    );
}
