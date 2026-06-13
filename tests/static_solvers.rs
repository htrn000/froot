use _native::board::{Board, TARGET_SUM};
use _native::generator::{
    generate_fungster_board, generate_random_board, generate_rejection_solvable_board,
    FungsterConfig, GeneratorError, RandomConfig, RejectionConfig, Rng64,
};
use _native::solver::{
    has_empty_solution, solve_exhaustive, solve_first_empty, MoveOrdering, SearchError,
    SolverLimits,
};

const OFFICIAL_WIDTH: usize = 17;
const OFFICIAL_HEIGHT: usize = 10;

fn board_total(board: &Board) -> u16 {
    board.cells().iter().map(|cell| *cell as u16).sum()
}

fn board_signature(board: &Board) -> u64 {
    board
        .cells()
        .iter()
        .fold(1_469_598_103_934_665_603, |signature, cell| {
            (signature ^ *cell as u64).wrapping_mul(1_099_511_628_211)
        })
}

fn assert_official_positive_board(board: &Board) {
    assert_eq!(board.width(), OFFICIAL_WIDTH);
    assert_eq!(board.height(), OFFICIAL_HEIGHT);
    assert_eq!(board.cells().len(), OFFICIAL_WIDTH * OFFICIAL_HEIGHT);
    assert!(board.cells().iter().all(|cell| (1..=9).contains(cell)));
    assert_eq!(board_total(board) % TARGET_SUM, 0);
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
    let cases = [
        (17, 600, 2_664_057_104_356_375_053),
        (23, 590, 12_247_289_152_284_244_121),
    ];

    for (seed, expected_total, expected_signature) in cases {
        let mut rng = Rng64::new(seed);
        let board = generate_fungster_board(&FungsterConfig::default(), &mut rng).unwrap();

        assert_official_positive_board(&board);
        assert_eq!(board_total(&board), expected_total);
        assert_eq!(board_signature(&board), expected_signature);
    }
}

#[test]
fn golden_random_generation_for_official_seeds() {
    let cases = [
        (17, 840, 8_093_049_011_389_795_167),
        (23, 900, 5_487_380_702_641_069_113),
    ];

    for (seed, expected_total, expected_signature) in cases {
        let mut rng = Rng64::new(seed);
        let board = generate_random_board(&RandomConfig::default(), &mut rng).unwrap();

        assert_official_positive_board(&board);
        assert_eq!(board_total(&board), expected_total);
        assert_eq!(board_signature(&board), expected_signature);
    }
}

#[test]
fn golden_rejection_generation_for_bounded_official_seeds() {
    for seed in [13, 23] {
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
            result,
            Err(GeneratorError::ExhaustedAttempts { attempts: 2 })
        ));
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
