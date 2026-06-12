use _native::board::{Board, TARGET_SUM};
use _native::generator::{
    generate_fungster_board, generate_random_board, generate_rejection_solvable_board,
    FungsterConfig, GeneratorError, RandomConfig, RejectionConfig, Rng64,
};
use _native::solver::{solve_exhaustive, solve_first_empty, MoveOrdering, SolverLimits};

#[test]
fn exhaustive_solver_answers_empty_board_queries() {
    let board = Board::new(vec![1, 9, 4, 6], 2).unwrap();
    let result = solve_exhaustive(&board, SolverLimits::default()).unwrap();

    assert!(result.empty_solvable);
    assert_eq!(result.max_score, 4);
    assert_eq!(result.min_empty_steps, Some(2));
    assert!(result.empty_solution_count >= 2);
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
fn dfs_solver_finds_fungster_board_solution() {
    let mut rng = Rng64::new(7);
    let board = generate_fungster_board(
        &FungsterConfig {
            width: 6,
            height: 4,
            groups: 4,
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
fn rejection_generator_continues_after_state_limited_attempts() {
    let mut rng = Rng64::new(13);
    let result = generate_rejection_solvable_board(
        &RejectionConfig {
            width: 17,
            height: 10,
            max_attempts: 2,
            solver_limits: SolverLimits { max_states: 1 },
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
