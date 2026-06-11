use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
#[cfg(feature = "static-solver-timeout")]
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
struct Rectangle {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

struct GameState {
    cells: Vec<u8>,
    score: u32,
    steps: usize,
    terminated: bool,
    truncated: bool,
    rng: SplitMix64,
}

#[pyclass]
struct StaticSolverResult {
    #[pyo3(get)]
    actions: Vec<usize>,
    #[pyo3(get)]
    rectangles: Vec<(usize, usize, usize, usize)>,
    #[pyo3(get)]
    score: u32,
    #[pyo3(get)]
    solutions_seen: u64,
    #[pyo3(get)]
    timed_out: bool,
    #[pyo3(get)]
    exhausted: bool,
}

struct SearchNode {
    cells: Vec<u8>,
    score: u32,
    actions: Vec<usize>,
    next_action: usize,
}

impl Rectangle {
    fn as_tuple(self) -> (usize, usize, usize, usize) {
        (self.left, self.top, self.right, self.bottom)
    }
}

#[derive(Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    fn fruit_value(&mut self) -> u8 {
        ((self.next_u64() % 9) + 1) as u8
    }
}

impl GameState {
    fn new(cell_count: usize, seed: u64) -> Self {
        let mut state = Self {
            cells: vec![0; cell_count],
            score: 0,
            steps: 0,
            terminated: false,
            truncated: false,
            rng: SplitMix64::new(seed),
        };
        state.reset(None);
        state
    }

    fn reset(&mut self, seed: Option<u64>) {
        if let Some(seed) = seed {
            self.rng = SplitMix64::new(seed);
        }

        for cell in &mut self.cells {
            *cell = self.rng.fruit_value();
        }

        self.score = 0;
        self.steps = 0;
        self.terminated = false;
        self.truncated = false;
    }

    fn done(&self) -> bool {
        self.terminated || self.truncated
    }
}

/// Find all axis-aligned rectangles whose cell values sum to the target.
///
/// The fruitbox board uses small non-negative values, so this deterministic
/// primitive is a good fit for Rust and can later be reused from Wasm.
#[pyfunction]
fn find_sum_rectangles(
    cells: Vec<u8>,
    width: usize,
    target: u16,
) -> PyResult<Vec<(usize, usize, usize, usize)>> {
    if width == 0 {
        return Err(PyValueError::new_err("width must be greater than zero"));
    }
    if target == 0 {
        return Err(PyValueError::new_err("target must be greater than zero"));
    }
    if cells.is_empty() || cells.len() % width != 0 {
        return Err(PyValueError::new_err(
            "cells length must be a non-empty multiple of width",
        ));
    }

    let height = cells.len() / width;
    Ok(find_rectangles_for_sum(&cells, width, height, target)
        .into_iter()
        .map(|rectangle| {
            (
                rectangle.left,
                rectangle.top,
                rectangle.right,
                rectangle.bottom,
            )
        })
        .collect())
}

#[pyfunction]
fn static_solver_supports_timeout() -> bool {
    cfg!(feature = "static-solver-timeout")
}

#[pyfunction]
#[pyo3(signature = (cells, width, target=10, max_solutions=None, timeout_ms=None))]
fn solve_static_board_native(
    cells: Vec<u8>,
    width: usize,
    target: u16,
    max_solutions: Option<u64>,
    timeout_ms: Option<u64>,
) -> PyResult<StaticSolverResult> {
    if width == 0 {
        return Err(PyValueError::new_err("width must be greater than zero"));
    }
    if target == 0 {
        return Err(PyValueError::new_err("target must be greater than zero"));
    }
    if cells.is_empty() || cells.len() % width != 0 {
        return Err(PyValueError::new_err(
            "cells length must be a non-empty multiple of width",
        ));
    }
    if cells.iter().any(|cell| *cell > 9) {
        return Err(PyValueError::new_err("cells must be in the range 0..=9"));
    }

    let height = cells.len() / width;
    let actions = enumerate_rectangles(width, height);
    Ok(solve_static_cells(
        cells,
        width,
        target,
        &actions,
        normalize_max_solutions(max_solutions),
        timeout_ms,
    ))
}

#[pyclass]
struct BatchedFruitboxSimulator {
    width: usize,
    height: usize,
    target: u16,
    max_steps: usize,
    actions: Vec<Rectangle>,
    games: Vec<GameState>,
}

#[pymethods]
impl BatchedFruitboxSimulator {
    #[new]
    #[pyo3(signature = (width, height, batch_size, target=10, max_steps=None, seed=0))]
    fn new(
        width: usize,
        height: usize,
        batch_size: usize,
        target: u16,
        max_steps: Option<usize>,
        seed: u64,
    ) -> PyResult<Self> {
        if width == 0 || height == 0 {
            return Err(PyValueError::new_err(
                "width and height must be greater than zero",
            ));
        }
        if batch_size == 0 {
            return Err(PyValueError::new_err(
                "batch_size must be greater than zero",
            ));
        }
        if target == 0 {
            return Err(PyValueError::new_err("target must be greater than zero"));
        }

        let cell_count = width
            .checked_mul(height)
            .ok_or_else(|| PyValueError::new_err("board dimensions are too large"))?;
        let actions = enumerate_rectangles(width, height);
        let games = (0..batch_size)
            .map(|index| GameState::new(cell_count, seed.wrapping_add(index as u64)))
            .collect();

        Ok(Self {
            width,
            height,
            target,
            max_steps: max_steps.unwrap_or(actions.len()),
            actions,
            games,
        })
    }

    #[getter]
    fn width(&self) -> usize {
        self.width
    }

    #[getter]
    fn height(&self) -> usize {
        self.height
    }

    #[getter]
    fn batch_size(&self) -> usize {
        self.games.len()
    }

    #[getter]
    fn target(&self) -> u16 {
        self.target
    }

    #[getter]
    fn max_steps(&self) -> usize {
        self.max_steps
    }

    #[getter]
    fn action_count(&self) -> usize {
        self.actions.len()
    }

    fn reset(&mut self, seed: Option<u64>) -> Vec<u8> {
        for (index, game) in self.games.iter_mut().enumerate() {
            game.reset(seed.map(|seed| seed.wrapping_add(index as u64)));
        }

        self.observations()
    }

    fn reset_at(&mut self, batch_index: usize, seed: Option<u64>) -> PyResult<Vec<u8>> {
        let game = self
            .games
            .get_mut(batch_index)
            .ok_or_else(|| PyValueError::new_err("batch_index is out of range"))?;
        game.reset(seed);

        Ok(game.cells.clone())
    }

    fn set_cells(&mut self, batch_index: usize, cells: Vec<u8>) -> PyResult<Vec<u8>> {
        if cells.len() != self.width * self.height {
            return Err(PyValueError::new_err(
                "cells length must match the simulator board shape",
            ));
        }
        if cells.iter().any(|cell| *cell > 9) {
            return Err(PyValueError::new_err("cells must be in the range 0..=9"));
        }

        let game = self
            .games
            .get_mut(batch_index)
            .ok_or_else(|| PyValueError::new_err("batch_index is out of range"))?;
        game.cells = cells;
        game.score = 0;
        game.steps = 0;
        game.terminated = false;
        game.truncated = false;

        Ok(game.cells.clone())
    }

    fn observations(&self) -> Vec<u8> {
        self.games
            .iter()
            .flat_map(|game| game.cells.iter().copied())
            .collect()
    }

    fn scores(&self) -> Vec<u32> {
        self.games.iter().map(|game| game.score).collect()
    }

    fn terminated(&self) -> Vec<bool> {
        self.games.iter().map(|game| game.terminated).collect()
    }

    fn truncated(&self) -> Vec<bool> {
        self.games.iter().map(|game| game.truncated).collect()
    }

    fn action_to_rectangle(&self, action: usize) -> PyResult<(usize, usize, usize, usize)> {
        let rectangle = self
            .actions
            .get(action)
            .ok_or_else(|| PyValueError::new_err("action is out of range"))?;
        Ok((
            rectangle.left,
            rectangle.top,
            rectangle.right,
            rectangle.bottom,
        ))
    }

    fn rectangle_to_action(
        &self,
        left: usize,
        top: usize,
        right: usize,
        bottom: usize,
    ) -> PyResult<usize> {
        self.actions
            .iter()
            .position(|rectangle| {
                rectangle.left == left
                    && rectangle.top == top
                    && rectangle.right == right
                    && rectangle.bottom == bottom
            })
            .ok_or_else(|| PyValueError::new_err("rectangle is not in the action space"))
    }

    fn action_masks(&self) -> Vec<u8> {
        self.games
            .iter()
            .flat_map(|game| self.action_mask_for_game(game))
            .collect()
    }

    fn legal_actions(&self, batch_index: usize) -> PyResult<Vec<usize>> {
        let game = self
            .games
            .get(batch_index)
            .ok_or_else(|| PyValueError::new_err("batch_index is out of range"))?;

        Ok(self
            .action_mask_for_game(game)
            .into_iter()
            .enumerate()
            .filter_map(|(action, legal)| (legal == 1).then_some(action))
            .collect())
    }

    #[pyo3(signature = (batch_index, max_solutions=None, timeout_ms=None))]
    fn solve_static(
        &self,
        batch_index: usize,
        max_solutions: Option<u64>,
        timeout_ms: Option<u64>,
    ) -> PyResult<StaticSolverResult> {
        let game = self
            .games
            .get(batch_index)
            .ok_or_else(|| PyValueError::new_err("batch_index is out of range"))?;

        Ok(solve_static_cells(
            game.cells.clone(),
            self.width,
            self.target,
            &self.actions,
            normalize_max_solutions(max_solutions),
            timeout_ms,
        ))
    }

    fn step(&mut self, actions: Vec<usize>) -> PyResult<(Vec<u8>, Vec<f32>, Vec<bool>, Vec<bool>)> {
        if actions.len() != self.games.len() {
            return Err(PyValueError::new_err(
                "actions length must match the simulator batch size",
            ));
        }

        for action in &actions {
            if *action >= self.actions.len() {
                return Err(PyValueError::new_err("action is out of range"));
            }
        }

        let actions = actions
            .into_iter()
            .map(|action| self.actions[action])
            .collect::<Vec<_>>();
        let target = self.target;
        let width = self.width;
        let max_steps = self.max_steps;
        let mut rewards = Vec::with_capacity(self.games.len());

        for (game, rectangle) in self.games.iter_mut().zip(actions) {
            rewards.push(step_game(game, width, target, max_steps, rectangle));
        }

        Ok((
            self.observations(),
            rewards,
            self.terminated(),
            self.truncated(),
        ))
    }
}

impl BatchedFruitboxSimulator {
    fn action_mask_for_game(&self, game: &GameState) -> Vec<u8> {
        if game.done() {
            return vec![0; self.actions.len()];
        }

        self.actions
            .iter()
            .map(|rectangle| {
                let sum = sum_rectangle(&game.cells, self.width, *rectangle);
                let score = score_rectangle(&game.cells, self.width, *rectangle);
                u8::from(sum == self.target && score > 0)
            })
            .collect()
    }
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(find_sum_rectangles, m)?)?;
    m.add_function(wrap_pyfunction!(solve_static_board_native, m)?)?;
    m.add_function(wrap_pyfunction!(static_solver_supports_timeout, m)?)?;
    m.add_class::<BatchedFruitboxSimulator>()?;
    m.add_class::<StaticSolverResult>()?;
    Ok(())
}

fn solve_static_cells(
    cells: Vec<u8>,
    width: usize,
    target: u16,
    actions: &[Rectangle],
    max_solutions: Option<u64>,
    timeout_ms: Option<u64>,
) -> StaticSolverResult {
    let deadline = solver_deadline(timeout_ms);
    let mut stack = vec![SearchNode {
        cells,
        score: 0,
        actions: Vec::new(),
        next_action: 0,
    }];
    let mut best_actions = Vec::new();
    let mut best_score = 0_u32;
    let mut solutions_seen = 0_u64;
    let mut timed_out = false;
    let mut exhausted = false;

    while !stack.is_empty() {
        if solver_timed_out(deadline) {
            timed_out = true;
            break;
        }
        if max_solutions.is_some_and(|limit| solutions_seen >= limit) {
            break;
        }

        let top_index = stack.len() - 1;
        if stack[top_index].score > best_score {
            best_score = stack[top_index].score;
            best_actions = stack[top_index].actions.clone();
        }

        let mut next_legal_action = None;
        while stack[top_index].next_action < actions.len() {
            let action = stack[top_index].next_action;
            stack[top_index].next_action += 1;
            let rectangle = actions[action];

            if is_legal_action(&stack[top_index].cells, width, target, rectangle) {
                next_legal_action = Some((action, rectangle));
                break;
            }
        }

        if let Some((action, rectangle)) = next_legal_action {
            let mut cells = stack[top_index].cells.clone();
            let score = score_rectangle(&cells, width, rectangle) as u32;
            apply_rectangle(&mut cells, width, rectangle);

            let mut path = stack[top_index].actions.clone();
            path.push(action);

            stack.push(SearchNode {
                cells,
                score: stack[top_index].score + score,
                actions: path,
                next_action: 0,
            });
        } else {
            if stack[top_index].score > best_score {
                best_score = stack[top_index].score;
                best_actions = stack[top_index].actions.clone();
            }

            solutions_seen = solutions_seen.saturating_add(1);
            stack.pop();

            if stack.is_empty() {
                exhausted = true;
            }
        }
    }

    let rectangles = best_actions
        .iter()
        .map(|action| actions[*action].as_tuple())
        .collect();

    StaticSolverResult {
        actions: best_actions,
        rectangles,
        score: best_score,
        solutions_seen,
        timed_out,
        exhausted,
    }
}

fn step_game(
    game: &mut GameState,
    width: usize,
    target: u16,
    max_steps: usize,
    rectangle: Rectangle,
) -> f32 {
    if game.done() {
        return 0.0;
    }

    game.steps += 1;

    let sum = sum_rectangle(&game.cells, width, rectangle);
    let score = score_rectangle(&game.cells, width, rectangle);
    let reward = if sum == target && score > 0 {
        apply_rectangle(&mut game.cells, width, rectangle);
        game.score += score as u32;
        score as f32
    } else {
        -1.0
    };

    if !has_legal_move(&game.cells, width, target) {
        game.terminated = true;
    }
    if game.steps >= max_steps {
        game.truncated = true;
    }

    reward
}

fn enumerate_rectangles(width: usize, height: usize) -> Vec<Rectangle> {
    let mut rectangles = Vec::new();

    for top in 0..height {
        for bottom in top..height {
            for left in 0..width {
                for right in left..width {
                    rectangles.push(Rectangle {
                        left,
                        top,
                        right,
                        bottom,
                    });
                }
            }
        }
    }

    rectangles
}

fn find_rectangles_for_sum(
    cells: &[u8],
    width: usize,
    height: usize,
    target: u16,
) -> Vec<Rectangle> {
    let mut rectangles = Vec::new();

    for top in 0..height {
        let mut column_sums = vec![0_u16; width];

        for bottom in top..height {
            for x in 0..width {
                column_sums[x] += cells[bottom * width + x] as u16;
            }

            for left in 0..width {
                let mut sum = 0_u16;

                for right in left..width {
                    sum += column_sums[right];

                    if sum == target {
                        rectangles.push(Rectangle {
                            left,
                            top,
                            right,
                            bottom,
                        });
                    }
                    if sum > target {
                        break;
                    }
                }
            }
        }
    }

    rectangles
}

fn has_legal_move(cells: &[u8], width: usize, target: u16) -> bool {
    let height = cells.len() / width;
    find_rectangles_for_sum(cells, width, height, target)
        .into_iter()
        .any(|rectangle| score_rectangle(cells, width, rectangle) > 0)
}

fn is_legal_action(cells: &[u8], width: usize, target: u16, rectangle: Rectangle) -> bool {
    sum_rectangle(cells, width, rectangle) == target && score_rectangle(cells, width, rectangle) > 0
}

fn normalize_max_solutions(max_solutions: Option<u64>) -> Option<u64> {
    max_solutions.and_then(|value| (value > 0).then_some(value))
}

#[cfg(feature = "static-solver-timeout")]
fn solver_deadline(timeout_ms: Option<u64>) -> Option<Instant> {
    timeout_ms.map(|timeout_ms| Instant::now() + Duration::from_millis(timeout_ms))
}

#[cfg(not(feature = "static-solver-timeout"))]
fn solver_deadline(_timeout_ms: Option<u64>) -> Option<()> {
    None
}

#[cfg(feature = "static-solver-timeout")]
fn solver_timed_out(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

#[cfg(not(feature = "static-solver-timeout"))]
fn solver_timed_out(_deadline: Option<()>) -> bool {
    false
}

fn sum_rectangle(cells: &[u8], width: usize, rectangle: Rectangle) -> u16 {
    let mut sum = 0_u16;

    for y in rectangle.top..=rectangle.bottom {
        for x in rectangle.left..=rectangle.right {
            sum += cells[y * width + x] as u16;
        }
    }

    sum
}

fn score_rectangle(cells: &[u8], width: usize, rectangle: Rectangle) -> usize {
    let mut score = 0;

    for y in rectangle.top..=rectangle.bottom {
        for x in rectangle.left..=rectangle.right {
            if cells[y * width + x] > 0 {
                score += 1;
            }
        }
    }

    score
}

fn apply_rectangle(cells: &mut [u8], width: usize, rectangle: Rectangle) {
    for y in rectangle.top..=rectangle.bottom {
        for x in rectangle.left..=rectangle.right {
            cells[y * width + x] = 0;
        }
    }
}
