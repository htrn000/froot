use crate::board::{Board, BoardError, TARGET_SUM};
use crate::solver::{solve_first_empty, MoveOrdering, SearchError, SolverLimits};

#[derive(Clone, Debug)]
pub struct Rng64 {
    state: [u64; 4],
}

impl Rng64 {
    pub fn new(seed: u64) -> Self {
        let mut splitmix = SplitMix64::new(seed);
        let state = [
            splitmix.next_u64(),
            splitmix.next_u64(),
            splitmix.next_u64(),
            splitmix.next_u64(),
        ];
        Self { state }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.state[0].wrapping_add(self.state[3]);
        let t = self.state[1] << 17;

        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];
        self.state[2] ^= t;
        self.state[3] = self.state[3].rotate_left(45);

        result
    }

    pub fn range(&mut self, start: usize, end: usize) -> usize {
        debug_assert!(start < end);
        start + (self.next_u64() as usize % (end - start))
    }
}

#[derive(Clone, Debug)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
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
    for y in 0..config.height {
        let segments = partition_line(config.width, config.min_tuple, config.max_tuple, rng)
            .ok_or(GeneratorError::InvalidConfig(
                "width cannot be tiled by the configured tuple bounds",
            ))?;
        let mut left = 0;
        for segment_len in segments {
            let tuple = positive_tuple_sum(TARGET_SUM as u8, segment_len, rng);
            for (offset, value) in tuple.into_iter().enumerate() {
                cells[y * config.width + left + offset] = value;
            }
            left += segment_len;
        }
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

fn partition_line(
    width: usize,
    min_tuple: usize,
    max_tuple: usize,
    rng: &mut Rng64,
) -> Option<Vec<usize>> {
    fn can_partition(width: usize, min_tuple: usize, max_tuple: usize) -> bool {
        width == 0
            || (min_tuple..=max_tuple)
                .any(|len| width >= len && can_partition(width - len, min_tuple, max_tuple))
    }

    if !can_partition(width, min_tuple, max_tuple) {
        return None;
    }

    let mut remaining = width;
    let mut segments = Vec::new();
    while remaining > 0 {
        let mut candidates = (min_tuple..=max_tuple)
            .filter(|len| {
                remaining >= *len && can_partition(remaining - *len, min_tuple, max_tuple)
            })
            .collect::<Vec<_>>();
        for index in (1..candidates.len()).rev() {
            let swap = rng.range(0, index + 1);
            candidates.swap(index, swap);
        }
        let len = candidates[0];
        segments.push(len);
        remaining -= len;
    }
    Some(segments)
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
