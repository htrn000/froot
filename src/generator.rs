use crate::board::{Board, BoardError, TARGET_SUM};
use crate::solver::{has_empty_solution, MoveOrdering, SearchError, SolverLimits};

#[derive(Clone, Debug)]
/// Deterministic RNG for reproducible benchmark boards. It keeps a 256-bit
/// xorshift state, while accepting a simple `u64` seed for CLI ergonomics.
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
/// Generates solvable-by-construction boards by partitioning the full board
/// into rectangular sum-10 moves. The partition strategy is injectable so
/// experiments can compare a simple strip partition against random backtracking.
pub struct FungsterConfig {
    pub width: usize,
    pub height: usize,
    pub attempts: usize,
    pub min_tuple: usize,
    pub max_tuple: usize,
    pub partition_strategy: FungsterPartitionStrategy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FungsterPartitionStrategy {
    StraightStrips,
    RandomBacktracking,
}

impl Default for FungsterConfig {
    fn default() -> Self {
        Self {
            width: 17,
            height: 10,
            attempts: 32,
            min_tuple: 2,
            max_tuple: 4,
            partition_strategy: FungsterPartitionStrategy::StraightStrips,
        }
    }
}

#[derive(Clone, Debug)]
/// Rejection sampling wrapper around `RandomConfig`. It only returns boards
/// when the selected single-solution solver finds an empty-board witness.
pub struct RejectionConfig {
    pub width: usize,
    pub height: usize,
    pub max_attempts: usize,
    pub solver_limits: SolverLimits,
}

#[derive(Clone, Debug)]
/// Official-style random board generation: all cells are positive apples and
/// the whole board sum is divisible by 10, but solvability is not guaranteed.
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
                max_empty_solutions: None,
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

    match config.partition_strategy {
        FungsterPartitionStrategy::StraightStrips => {
            let mut cells = vec![0_u8; config.width * config.height];
            fill_straight_strips(config, rng, &mut cells)?;
            Board::new(cells, config.width).map_err(GeneratorError::from)
        }
        FungsterPartitionStrategy::RandomBacktracking => {
            for _ in 0..config.attempts.max(1) {
                let mut cells = vec![0_u8; config.width * config.height];
                if tile_random_rectangles(config, rng, &mut cells) {
                    return Board::new(cells, config.width).map_err(GeneratorError::from);
                }
            }

            Err(GeneratorError::ExhaustedAttempts {
                attempts: config.attempts.max(1),
            })
        }
    }
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
        let solution = match has_empty_solution(
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Region {
    left: usize,
    top: usize,
    width: usize,
    height: usize,
}

impl Region {
    fn area(self) -> usize {
        self.width * self.height
    }
}

fn fill_straight_strips(
    config: &FungsterConfig,
    rng: &mut Rng64,
    cells: &mut [u8],
) -> Result<(), GeneratorError> {
    let horizontal_height = config.height / 2;
    for y in 0..horizontal_height {
        fill_horizontal_strip(config, rng, cells, y)?;
    }
    for x in 0..config.width {
        fill_vertical_strip(config, rng, cells, x, horizontal_height)?;
    }
    Ok(())
}

fn fill_horizontal_strip(
    config: &FungsterConfig,
    rng: &mut Rng64,
    cells: &mut [u8],
    y: usize,
) -> Result<(), GeneratorError> {
    let segments = partition_line(config.width, config.min_tuple, config.max_tuple, rng).ok_or(
        GeneratorError::InvalidConfig("width cannot be tiled by the configured tuple bounds"),
    )?;
    let mut left = 0;
    for segment_len in segments {
        fill_rect_region(
            config.width,
            rng,
            cells,
            Region {
                left,
                top: y,
                width: segment_len,
                height: 1,
            },
        );
        left += segment_len;
    }
    Ok(())
}

fn fill_vertical_strip(
    config: &FungsterConfig,
    rng: &mut Rng64,
    cells: &mut [u8],
    x: usize,
    top: usize,
) -> Result<(), GeneratorError> {
    let strip_height = config.height - top;
    let segments = partition_line(strip_height, config.min_tuple, config.max_tuple, rng).ok_or(
        GeneratorError::InvalidConfig("height cannot be tiled by the configured tuple bounds"),
    )?;
    let mut current_top = top;
    for segment_len in segments {
        fill_rect_region(
            config.width,
            rng,
            cells,
            Region {
                left: x,
                top: current_top,
                width: 1,
                height: segment_len,
            },
        );
        current_top += segment_len;
    }
    Ok(())
}

fn partition_line(
    length: usize,
    min_tuple: usize,
    max_tuple: usize,
    rng: &mut Rng64,
) -> Option<Vec<usize>> {
    fn can_partition(length: usize, min_tuple: usize, max_tuple: usize) -> bool {
        length == 0
            || (min_tuple..=max_tuple)
                .any(|len| length >= len && can_partition(length - len, min_tuple, max_tuple))
    }

    if !can_partition(length, min_tuple, max_tuple) {
        return None;
    }

    let mut remaining = length;
    let mut segments = Vec::new();
    while remaining > 0 {
        let mut candidates = (min_tuple..=max_tuple)
            .filter(|len| remaining >= *len && can_partition(remaining - *len, min_tuple, max_tuple))
            .collect::<Vec<_>>();
        shuffle(rng, &mut candidates);
        let len = candidates[0];
        segments.push(len);
        remaining -= len;
    }
    Some(segments)
}

fn tile_random_rectangles(config: &FungsterConfig, rng: &mut Rng64, cells: &mut [u8]) -> bool {
    let Some(index) = cells.iter().position(|cell| *cell == 0) else {
        return true;
    };
    let x = index % config.width;
    let y = index / config.width;
    let mut candidates = candidate_rectangles(config, cells, x, y);
    shuffle(rng, &mut candidates);

    for region in candidates {
        fill_rect_region(config.width, rng, cells, region);
        if tile_random_rectangles(config, rng, cells) {
            return true;
        }
        clear_rect(config.width, cells, region);
    }
    false
}

fn candidate_rectangles(config: &FungsterConfig, cells: &[u8], x: usize, y: usize) -> Vec<Region> {
    let mut candidates = Vec::new();
    for height in 1..=config.max_tuple {
        for width in 1..=config.max_tuple {
            let area = width * height;
            if !(config.min_tuple..=config.max_tuple).contains(&area) {
                continue;
            }
            if width > config.width || height > config.height {
                continue;
            }
            let min_left = x.saturating_add(1).saturating_sub(width);
            let max_left = x.min(config.width - width);
            let min_top = y.saturating_add(1).saturating_sub(height);
            let max_top = y.min(config.height - height);
            for top in min_top..=max_top {
                for left in min_left..=max_left {
                    let region = Region {
                        left,
                        top,
                        width,
                        height,
                    };
                    if rect_is_empty(config.width, cells, region) {
                        candidates.push(region);
                    }
                }
            }
        }
    }
    candidates
}

fn fill_rect_region(board_width: usize, rng: &mut Rng64, cells: &mut [u8], region: Region) {
    debug_assert!(region.area() > 0);
    // A fungster iteration places apples into one rectangular partition tile;
    // later iterations only need the remaining empty cells to still admit a
    // valid rectangle partition, so random backtracking owns that choice here.
    let tuple = positive_tuple_sum(TARGET_SUM as u8, region.area(), rng);
    for (offset, value) in tuple.into_iter().enumerate() {
        let x = offset % region.width;
        let y = offset / region.width;
        cells[(region.top + y) * board_width + region.left + x] = value;
    }
}

fn clear_rect(board_width: usize, cells: &mut [u8], region: Region) {
    for y in region.top..region.top + region.height {
        for x in region.left..region.left + region.width {
            cells[y * board_width + x] = 0;
        }
    }
}

fn rect_is_empty(board_width: usize, cells: &[u8], region: Region) -> bool {
    (region.top..region.top + region.height)
        .all(|y| (region.left..region.left + region.width).all(|x| cells[y * board_width + x] == 0))
}

fn shuffle<T>(rng: &mut Rng64, values: &mut [T]) {
    for index in (1..values.len()).rev() {
        let swap = rng.range(0, index + 1);
        values.swap(index, swap);
    }
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
