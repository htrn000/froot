use crate::board::{Board, BoardError, TARGET_SUM};
use crate::solver::{solve_first_empty, MoveOrdering, SearchError, SolverLimits};

#[derive(Clone, Debug)]
pub struct Rng64 {
    state: u64,
}

impl Rng64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    pub fn range(&mut self, start: usize, end: usize) -> usize {
        debug_assert!(start < end);
        start + (self.next_u64() as usize % (end - start))
    }
}

#[derive(Clone, Debug)]
pub struct FungsterConfig {
    pub width: usize,
    pub height: usize,
    pub groups: usize,
    pub min_tuple: usize,
    pub max_tuple: usize,
}

impl Default for FungsterConfig {
    fn default() -> Self {
        Self {
            width: 17,
            height: 10,
            groups: 32,
            min_tuple: 2,
            max_tuple: 4,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RejectionConfig {
    pub width: usize,
    pub height: usize,
    pub max_attempts: usize,
    pub solver_limits: SolverLimits,
}

#[derive(Clone, Debug)]
pub struct RandomConfig {
    pub width: usize,
    pub height: usize,
}

impl Default for RandomConfig {
    fn default() -> Self {
        Self {
            width: 17,
            height: 10,
        }
    }
}

impl Default for RejectionConfig {
    fn default() -> Self {
        Self {
            width: 17,
            height: 10,
            max_attempts: 100,
            solver_limits: SolverLimits {
                max_states: 250_000,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GeneratorError {
    Board(BoardError),
    Search(SearchError),
    ExhaustedAttempts { attempts: usize },
    InvalidConfig(&'static str),
}

impl From<BoardError> for GeneratorError {
    fn from(error: BoardError) -> Self {
        Self::Board(error)
    }
}

impl From<SearchError> for GeneratorError {
    fn from(error: SearchError) -> Self {
        Self::Search(error)
    }
}

pub fn generate_fungster_board(
    config: &FungsterConfig,
    rng: &mut Rng64,
) -> Result<Board, GeneratorError> {
    if config.width == 0 || config.height == 0 {
        return Err(GeneratorError::InvalidConfig(
            "width and height must be positive",
        ));
    }
    if config.min_tuple < 2 || config.min_tuple > config.max_tuple || config.max_tuple > 10 {
        return Err(GeneratorError::InvalidConfig(
            "tuple bounds must satisfy 2 <= min_tuple <= max_tuple <= 10",
        ));
    }

    let mut cells = vec![0_u8; config.width * config.height];
    let mut placed = 0;
    let mut misses = 0;
    let max_misses = config.groups.saturating_mul(200).max(500);

    while placed < config.groups && misses < max_misses {
        let area = rng.range(config.min_tuple, config.max_tuple + 1);
        let shapes = rectangle_shapes(area);
        let (rect_width, rect_height) = shapes[rng.range(0, shapes.len())];
        if rect_width > config.width || rect_height > config.height {
            misses += 1;
            continue;
        }

        let left = rng.range(0, config.width - rect_width + 1);
        let top = rng.range(0, config.height - rect_height + 1);
        let indices: Vec<usize> = (top..top + rect_height)
            .flat_map(|y| (left..left + rect_width).map(move |x| y * config.width + x))
            .collect();

        if indices.iter().any(|&index| cells[index] != 0) {
            misses += 1;
            continue;
        }

        let tuple = positive_tuple_sum(TARGET_SUM as u8, area, rng);
        for (&index, value) in indices.iter().zip(tuple) {
            cells[index] = value;
        }
        placed += 1;
    }

    Board::new(cells, config.width).map_err(GeneratorError::from)
}

pub fn generate_rejection_solvable_board(
    config: &RejectionConfig,
    rng: &mut Rng64,
) -> Result<Board, GeneratorError> {
    if config.width == 0 || config.height == 0 {
        return Err(GeneratorError::InvalidConfig(
            "width and height must be positive",
        ));
    }

    for _ in 0..config.max_attempts {
        let board = generate_random_board(
            &RandomConfig {
                width: config.width,
                height: config.height,
            },
            rng,
        )?;
        let solution = match solve_first_empty(
            &board,
            MoveOrdering::LargestScoreFirst,
            config.solver_limits,
        ) {
            Ok(solution) => solution,
            Err(SearchError::StateLimitExceeded { .. }) => continue,
        };
        if solution.empty_solvable {
            return Ok(board);
        }
    }

    Err(GeneratorError::ExhaustedAttempts {
        attempts: config.max_attempts,
    })
}

pub fn generate_random_board(
    config: &RandomConfig,
    rng: &mut Rng64,
) -> Result<Board, GeneratorError> {
    if config.width == 0 || config.height == 0 {
        return Err(GeneratorError::InvalidConfig(
            "width and height must be positive",
        ));
    }

    let mut cells = vec![0_u8; config.width * config.height];
    loop {
        let mut sum = 0_u16;
        for cell in &mut cells {
            *cell = rng.range(1, 10) as u8;
            sum += *cell as u16;
        }
        if sum % TARGET_SUM == 0 {
            break;
        }
    }

    Board::new(cells, config.width).map_err(GeneratorError::from)
}

fn rectangle_shapes(area: usize) -> Vec<(usize, usize)> {
    (1..=area)
        .filter(|width| area % width == 0)
        .map(|width| (width, area / width))
        .collect()
}

fn positive_tuple_sum(target: u8, len: usize, rng: &mut Rng64) -> Vec<u8> {
    let mut values = vec![1_u8; len];
    let mut remaining = target - len as u8;

    while remaining > 0 {
        let index = rng.range(0, len);
        if values[index] < 9 {
            values[index] += 1;
            remaining -= 1;
        }
    }

    for index in (1..len).rev() {
        let swap = rng.range(0, index + 1);
        values.swap(index, swap);
    }
    values
}
