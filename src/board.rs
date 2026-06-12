use std::fmt;

pub const TARGET_SUM: u16 = 10;
pub const MAX_CELLS: usize = 192;
const MASK_WORDS: usize = 3;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Mask([u64; MASK_WORDS]);

impl Mask {
    pub const EMPTY: Self = Self([0; MASK_WORDS]);

    pub fn full(cell_count: usize) -> Result<Self, BoardError> {
        if cell_count > MAX_CELLS {
            return Err(BoardError::TooManyCells {
                cells: cell_count,
                max: MAX_CELLS,
            });
        }

        let mut mask = Self::EMPTY;
        for index in 0..cell_count {
            mask.set(index);
        }
        Ok(mask)
    }

    pub fn from_indices(indices: &[usize]) -> Self {
        let mut mask = Self::EMPTY;
        for &index in indices {
            mask.set(index);
        }
        mask
    }

    pub fn is_empty(self) -> bool {
        self.0.iter().all(|word| *word == 0)
    }

    pub fn set(&mut self, index: usize) {
        self.0[index / 64] |= 1_u64 << (index % 64);
    }

    pub fn and(self, other: Self) -> Self {
        let mut words = [0; MASK_WORDS];
        for (index, word) in words.iter_mut().enumerate() {
            *word = self.0[index] & other.0[index];
        }
        Self(words)
    }

    pub fn and_not(self, other: Self) -> Self {
        let mut words = [0; MASK_WORDS];
        for (index, word) in words.iter_mut().enumerate() {
            *word = self.0[index] & !other.0[index];
        }
        Self(words)
    }

    pub fn intersects(self, other: Self) -> bool {
        self.0
            .iter()
            .zip(other.0.iter())
            .any(|(left, right)| left & right != 0)
    }

    pub fn count(self) -> u16 {
        self.0.iter().map(|word| word.count_ones() as u16).sum()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rectangle {
    pub left: usize,
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
    pub mask: Mask,
    pub area: u16,
}

impl Rectangle {
    pub fn coordinates(self) -> (usize, usize, usize, usize) {
        (self.left, self.top, self.right, self.bottom)
    }
}

#[derive(Clone, Debug)]
/// Immutable board metadata shared by all static solvers. Rectangles and digit
/// masks are precomputed once so recursive searches can represent progress as a
/// cheap `Mask` and evaluate each candidate move with bit operations.
pub struct Board {
    width: usize,
    height: usize,
    cells: Vec<u8>,
    live_mask: Mask,
    digit_masks: [Mask; 10],
    rectangles: Vec<Rectangle>,
}

impl Board {
    pub fn new(cells: Vec<u8>, width: usize) -> Result<Self, BoardError> {
        if width == 0 {
            return Err(BoardError::ZeroWidth);
        }
        if cells.is_empty() || cells.len() % width != 0 {
            return Err(BoardError::InvalidCellCount {
                cells: cells.len(),
                width,
            });
        }
        if cells.len() > MAX_CELLS {
            return Err(BoardError::TooManyCells {
                cells: cells.len(),
                max: MAX_CELLS,
            });
        }

        let height = cells.len() / width;
        let mut live_mask = Mask::EMPTY;
        let mut digit_masks = [Mask::EMPTY; 10];

        for (index, &cell) in cells.iter().enumerate() {
            if cell > 9 {
                return Err(BoardError::InvalidCellValue { index, value: cell });
            }
            digit_masks[cell as usize].set(index);
            if cell > 0 {
                live_mask.set(index);
            }
        }

        let rectangles = build_rectangles(width, height)?;

        Ok(Self {
            width,
            height,
            cells,
            live_mask,
            digit_masks,
            rectangles,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn cells(&self) -> &[u8] {
        &self.cells
    }

    pub fn initial_state(&self) -> Mask {
        self.live_mask
    }

    pub fn rectangles(&self) -> &[Rectangle] {
        &self.rectangles
    }

    pub fn live_sum(&self, state: Mask, rectangle: Rectangle) -> u16 {
        let live_rect = state.and(rectangle.mask);
        (1..=9)
            .map(|digit| live_rect.and(self.digit_masks[digit]).count() * digit as u16)
            .sum()
    }

    pub fn live_score(&self, state: Mask, rectangle: Rectangle) -> u16 {
        state.and(rectangle.mask).count()
    }

    pub fn apply(&self, state: Mask, rectangle: Rectangle) -> Mask {
        state.and_not(rectangle.mask)
    }

    pub fn valid_moves<'a>(
        &'a self,
        state: Mask,
        target: u16,
    ) -> impl Iterator<Item = Rectangle> + 'a {
        self.rectangles.iter().copied().filter(move |rectangle| {
            state.intersects(rectangle.mask) && self.live_sum(state, *rectangle) == target
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoardError {
    ZeroWidth,
    InvalidCellCount { cells: usize, width: usize },
    TooManyCells { cells: usize, max: usize },
    InvalidCellValue { index: usize, value: u8 },
}

impl fmt::Display for BoardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroWidth => write!(formatter, "width must be greater than zero"),
            Self::InvalidCellCount { cells, width } => write!(
                formatter,
                "cells length must be a non-empty multiple of width (cells={cells}, width={width})"
            ),
            Self::TooManyCells { cells, max } => {
                write!(
                    formatter,
                    "board has {cells} cells but this solver supports at most {max}"
                )
            }
            Self::InvalidCellValue { index, value } => {
                write!(
                    formatter,
                    "cell {index} has invalid value {value}; expected 0..=9"
                )
            }
        }
    }
}

impl std::error::Error for BoardError {}

pub fn find_sum_rectangles_core(
    cells: &[u8],
    width: usize,
    target: u16,
) -> Result<Vec<(usize, usize, usize, usize)>, BoardError> {
    let board = Board::new(cells.to_vec(), width)?;
    Ok(board
        .valid_moves(board.initial_state(), target)
        .map(Rectangle::coordinates)
        .collect())
}

fn build_rectangles(width: usize, height: usize) -> Result<Vec<Rectangle>, BoardError> {
    let mut rectangles = Vec::with_capacity(width * (width + 1) * height * (height + 1) / 4);

    for top in 0..height {
        for bottom in top..height {
            for left in 0..width {
                for right in left..width {
                    let mut indices = Vec::with_capacity((right - left + 1) * (bottom - top + 1));
                    for y in top..=bottom {
                        for x in left..=right {
                            indices.push(y * width + x);
                        }
                    }
                    let area = indices.len() as u16;
                    rectangles.push(Rectangle {
                        left,
                        top,
                        right,
                        bottom,
                        mask: Mask::from_indices(&indices),
                        area,
                    });
                }
            }
        }
    }

    Ok(rectangles)
}
