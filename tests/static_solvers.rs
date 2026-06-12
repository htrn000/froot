use _native::board::{Board, TARGET_SUM};
use _native::generator::{
    generate_fungster_board, generate_random_board, generate_rejection_solvable_board,
    FungsterAxis, FungsterConfig, GeneratorError, RandomConfig, RejectionConfig, Rng64,
};
use _native::solver::{
    has_empty_solution, solve_exhaustive, solve_first_empty, MoveOrdering, SearchError,
    SolverLimits,
};

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
            groups: 4,
            min_tuple: 2,
            max_tuple: 4,
            axis: FungsterAxis::Row,
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
fn dfs_solver_finds_column_fungster_board_solution() {
    let mut rng = Rng64::new(17);
    let board = generate_fungster_board(
        &FungsterConfig {
            width: 6,
            height: 4,
            groups: 4,
            min_tuple: 2,
            max_tuple: 4,
            axis: FungsterAxis::Column,
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
            width: 17,
            height: 10,
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

    assert_eq!(board.width(), 17);
    assert_eq!(board.height(), 10);
    assert!(board.cells().iter().all(|cell| (1..=9).contains(cell)));
    assert_eq!(total_sum % TARGET_SUM, 0);
}
