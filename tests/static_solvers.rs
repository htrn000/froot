use _native::board::{Board, TARGET_SUM};
use _native::generator::{
    generate_fungster_board, generate_random_board, generate_rejection_solvable_board,
    FungsterConfig, GeneratorError, RandomConfig, RejectionConfig, Rng64,
};
use _native::solver::{
    has_empty_solution, solve_exhaustive, solve_first_empty, MoveOrdering, SearchError,
    SolverLimits,
};
use goldenfile::Mint;
use std::io::Write;

const OFFICIAL_WIDTH: usize = 17;
const OFFICIAL_HEIGHT: usize = 10;

fn board_total(board: &Board) -> u16 {
    board.cells().iter().map(|cell| *cell as u16).sum()
}

fn assert_official_positive_board(board: &Board) {
    assert_eq!(board.width(), OFFICIAL_WIDTH);
    assert_eq!(board.height(), OFFICIAL_HEIGHT);
    assert_eq!(board.cells().len(), OFFICIAL_WIDTH * OFFICIAL_HEIGHT);
    assert!(board.cells().iter().all(|cell| (1..=9).contains(cell)));
    assert_eq!(board_total(board) % TARGET_SUM, 0);
}

fn write_board_case(mint: &mut Mint, file_name: &str, generator: &str, seed: u64, board: &Board) {
    let mut file = mint.new_goldenfile(file_name).unwrap();
    writeln!(file, "generator={generator}").unwrap();
    writeln!(file, "seed={seed}").unwrap();
    writeln!(file, "width={}", board.width()).unwrap();
    writeln!(file, "height={}", board.height()).unwrap();
    writeln!(file, "total={}", board_total(board)).unwrap();
    writeln!(file, "cells=").unwrap();
    for row in board.cells().chunks(board.width()) {
        let line = row
            .iter()
            .map(|cell| cell.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        writeln!(file, "{line}").unwrap();
    }
}

fn write_rejection_case(
    mint: &mut Mint,
    file_name: &str,
    seed: u64,
    result: Result<Board, GeneratorError>,
) {
    let mut file = mint.new_goldenfile(file_name).unwrap();
    writeln!(file, "generator=rejection").unwrap();
    writeln!(file, "seed={seed}").unwrap();
    match result {
        Ok(board) => {
            writeln!(file, "result=ok").unwrap();
            writeln!(file, "width={}", board.width()).unwrap();
            writeln!(file, "height={}", board.height()).unwrap();
            writeln!(file, "total={}", board_total(&board)).unwrap();
        }
        Err(GeneratorError::ExhaustedAttempts { attempts }) => {
            writeln!(file, "result=exhausted").unwrap();
            writeln!(file, "attempts={attempts}").unwrap();
        }
        Err(error) => {
            writeln!(file, "result=error").unwrap();
            writeln!(file, "error={error:?}").unwrap();
        }
    }
}

#[test]
fn rng_is_deterministic_and_uses_full_seeded_state() {
    let mut left = Rng64::new(42);
    let mut right = Rng64::new(42);
    let mut different = Rng64::new(43);

    let left_values = (0..8).map(|_| left.next_u64()).collect::<Vec<_>>();
    let right_values = (0..8).map(|_| right.next_u64()).collect::<Vec<_>>();
    let different_values = (0..8).map(|_| different.next_u64()).collect::<Vec<_>>();

    assert_eq!(left_values, right_values);
    assert_ne!(left_values, different_values);
    assert!(left_values.iter().any(|value| *value != 0));
}

#[test]
fn golden_fungster_generation_for_official_seeds() {
    let mut mint = Mint::new("tests/gold_test/generator");
    let cases = [(17, "golden_test_case_0"), (23, "golden_test_case_1")];

    for (seed, file_name) in cases {
        let mut rng = Rng64::new(seed);
        let board = generate_fungster_board(&FungsterConfig::default(), &mut rng).unwrap();

        assert_official_positive_board(&board);
        write_board_case(&mut mint, file_name, "fungster", seed, &board);
    }
}

#[test]
fn golden_random_generation_for_official_seeds() {
    let mut mint = Mint::new("tests/gold_test/generator");
    let cases = [(17, "golden_test_case_2"), (23, "golden_test_case_3")];

    for (seed, file_name) in cases {
        let mut rng = Rng64::new(seed);
        let board = generate_random_board(&RandomConfig::default(), &mut rng).unwrap();

        assert_official_positive_board(&board);
        write_board_case(&mut mint, file_name, "random", seed, &board);
    }
}

#[test]
fn golden_rejection_generation_for_bounded_official_seeds() {
    let mut mint = Mint::new("tests/gold_test/generator");
    for (seed, file_name) in [(13, "golden_test_case_4"), (23, "golden_test_case_5")] {
        let mut rng = Rng64::new(seed);
        let result = generate_rejection_solvable_board(
            &RejectionConfig {
                width: OFFICIAL_WIDTH,
                height: OFFICIAL_HEIGHT,
                max_attempts: 2,
                solver_limits: SolverLimits {
                    max_states: 1,
                    max_empty_solutions: None,
                },
            },
            &mut rng,
        );

        assert!(matches!(
            &result,
            Err(GeneratorError::ExhaustedAttempts { attempts: 2 })
        ));
        write_rejection_case(&mut mint, file_name, seed, result);
    }
}

#[test]
fn soft_fuzz_generation_parameters_on_official_size() {
    for (seed, min_tuple, max_tuple, attempts) in [(31, 2, 4, 4), (37, 2, 5, 8), (41, 3, 5, 8)] {
        let mut rng = Rng64::new(seed);
        let board = generate_fungster_board(
            &FungsterConfig {
                width: OFFICIAL_WIDTH,
                height: OFFICIAL_HEIGHT,
                attempts,
                min_tuple,
                max_tuple,
            },
            &mut rng,
        )
        .unwrap();

        assert_official_positive_board(&board);
    }

    for seed in [31, 37, 41] {
        let mut rng = Rng64::new(seed);
        let board = generate_random_board(&RandomConfig::default(), &mut rng).unwrap();

        assert_official_positive_board(&board);
    }
}

#[test]
fn exhaustive_solver_answers_empty_board_queries() {
    let board = Board::new(vec![1, 9, 4, 6], 2).unwrap();
    let result = solve_exhaustive(&board, SolverLimits::default()).unwrap();

    assert!(result.empty_solvable);
    assert_eq!(result.max_score, 4);
    assert_eq!(result.min_empty_steps, Some(2));
    assert!(result.empty_solution_count >= 2);
    assert!(!result.solution_limit_reached);
}

#[test]
fn exhaustive_solver_scores_best_terminal_path_when_not_empty_solvable() {
    let board = Board::new(vec![1, 9, 5], 3).unwrap();
    let result = solve_exhaustive(&board, SolverLimits::default()).unwrap();

    assert!(!result.empty_solvable);
    assert_eq!(result.max_score, 2);
    assert_eq!(result.min_empty_steps, None);
}

#[test]
fn exhaustive_solver_can_stop_after_empty_solution_cap() {
    let board = Board::new(vec![1, 9, 4, 6], 2).unwrap();
    let result = solve_exhaustive(
        &board,
        SolverLimits {
            max_states: 1_000,
            max_empty_solutions: Some(1),
        },
    )
    .unwrap();

    assert!(result.empty_solvable);
    assert_eq!(result.empty_solution_count, 1);
    assert!(result.solution_limit_reached);
}

#[test]
fn dfs_solver_finds_fungster_board_solution() {
    let mut rng = Rng64::new(7);
    let board = generate_fungster_board(
        &FungsterConfig {
            width: 6,
            height: 4,
            attempts: 4,
            min_tuple: 2,
            max_tuple: 4,
        },
        &mut rng,
    )
    .unwrap();
    let total_sum: u16 = board.cells().iter().map(|cell| *cell as u16).sum();
    let solution = solve_first_empty(
        &board,
        MoveOrdering::LargestScoreFirst,
        SolverLimits::default(),
    )
    .unwrap();

    assert_eq!(total_sum % TARGET_SUM, 0);
    assert!(board.cells().iter().all(|cell| (1..=9).contains(cell)));
    assert!(solution.empty_solvable);
}

#[test]
fn early_empty_search_finds_solution_without_retaining_path() {
    let board = Board::new(vec![1, 9, 4, 6], 2).unwrap();
    let result = has_empty_solution(
        &board,
        MoveOrdering::LargestScoreFirst,
        SolverLimits::default(),
    )
    .unwrap();

    assert!(result.empty_solvable);
    assert!(result.states_evaluated > 0);
}

#[test]
fn early_empty_search_respects_state_limits() {
    let board = Board::new(vec![1, 9, 4, 6], 2).unwrap();
    let result = has_empty_solution(
        &board,
        MoveOrdering::LargestScoreFirst,
        SolverLimits {
            max_states: 0,
            max_empty_solutions: None,
        },
    );

    assert_eq!(
        result,
        Err(SearchError::StateLimitExceeded { max_states: 0 })
    );
}

#[test]
fn dfs_solver_finds_another_fungster_board_solution() {
    let mut rng = Rng64::new(17);
    let board = generate_fungster_board(
        &FungsterConfig {
            width: 6,
            height: 4,
            attempts: 4,
            min_tuple: 2,
            max_tuple: 4,
        },
        &mut rng,
    )
    .unwrap();
    let total_sum: u16 = board.cells().iter().map(|cell| *cell as u16).sum();
    let solution = solve_first_empty(
        &board,
        MoveOrdering::SmallestScoreFirst,
        SolverLimits::default(),
    )
    .unwrap();

    assert_eq!(total_sum % TARGET_SUM, 0);
    assert!(board.cells().iter().all(|cell| (1..=9).contains(cell)));
    assert!(solution.empty_solvable);
}

#[test]
fn rejection_generator_continues_after_state_limited_attempts() {
    let mut rng = Rng64::new(13);
    let result = generate_rejection_solvable_board(
        &RejectionConfig {
            width: OFFICIAL_WIDTH,
            height: OFFICIAL_HEIGHT,
            max_attempts: 2,
            solver_limits: SolverLimits {
                max_states: 1,
                max_empty_solutions: None,
            },
        },
        &mut rng,
    );

    assert!(matches!(
        result,
        Err(GeneratorError::ExhaustedAttempts { attempts: 2 })
    ));
}

#[test]
fn random_generator_uses_official_size_and_total_sum_rule_by_default() {
    let mut rng = Rng64::new(19);
    let board = generate_random_board(&RandomConfig::default(), &mut rng).unwrap();
    let total_sum: u16 = board.cells().iter().map(|cell| *cell as u16).sum();

    assert_official_positive_board(&board);
    assert_eq!(total_sum % TARGET_SUM, 0);
}
