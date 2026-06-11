use wasm_bindgen::prelude::*;

#[derive(Clone, Copy)]
struct Rectangle {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

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

#[wasm_bindgen]
pub struct WasmRectangle {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
    score: usize,
}

#[wasm_bindgen]
impl WasmRectangle {
    #[wasm_bindgen(getter)]
    pub fn left(&self) -> usize {
        self.left
    }

    #[wasm_bindgen(getter)]
    pub fn top(&self) -> usize {
        self.top
    }

    #[wasm_bindgen(getter)]
    pub fn right(&self) -> usize {
        self.right
    }

    #[wasm_bindgen(getter)]
    pub fn bottom(&self) -> usize {
        self.bottom
    }

    #[wasm_bindgen(getter)]
    pub fn score(&self) -> usize {
        self.score
    }
}

#[wasm_bindgen]
pub struct WasmMoveList {
    moves: Vec<WasmRectangle>,
}

#[wasm_bindgen]
impl WasmMoveList {
    #[wasm_bindgen(getter)]
    pub fn length(&self) -> usize {
        self.moves.len()
    }

    pub fn get(&self, index: usize) -> Option<WasmRectangle> {
        self.moves.get(index).map(|rectangle| WasmRectangle {
            left: rectangle.left,
            top: rectangle.top,
            right: rectangle.right,
            bottom: rectangle.bottom,
            score: rectangle.score,
        })
    }
}

#[wasm_bindgen]
pub fn create_board_cells(width: usize, height: usize, seed: u32) -> Vec<u8> {
    let mut rng = SplitMix64::new(seed as u64);
    (0..width * height).map(|_| rng.fruit_value()).collect()
}

#[wasm_bindgen]
pub fn rectangle_from_cells(start_index: usize, end_index: usize, width: usize) -> WasmRectangle {
    let start_x = start_index % width;
    let start_y = start_index / width;
    let end_x = end_index % width;
    let end_y = end_index / width;

    WasmRectangle {
        left: start_x.min(end_x),
        top: start_y.min(end_y),
        right: start_x.max(end_x),
        bottom: start_y.max(end_y),
        score: 0,
    }
}

#[wasm_bindgen]
pub fn sum_rectangle(
    cells: &[u8],
    width: usize,
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
) -> u16 {
    let rectangle = Rectangle {
        left,
        top,
        right,
        bottom,
    };
    rectangle_sum(cells, width, rectangle)
}

#[wasm_bindgen]
pub fn score_rectangle(
    cells: &[u8],
    width: usize,
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
) -> WasmRectangle {
    let rectangle = Rectangle {
        left,
        top,
        right,
        bottom,
    };

    WasmRectangle {
        left,
        top,
        right,
        bottom,
        score: rectangle_score(cells, width, rectangle),
    }
}

#[wasm_bindgen]
pub fn apply_rectangle(
    cells: &[u8],
    width: usize,
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
) -> Vec<u8> {
    let mut next_cells = cells.to_vec();
    clear_rectangle(
        &mut next_cells,
        width,
        Rectangle {
            left,
            top,
            right,
            bottom,
        },
    );
    next_cells
}

#[wasm_bindgen]
pub fn find_static_moves(cells: &[u8], width: usize, target: u16) -> WasmMoveList {
    let height = cells.len() / width;
    let mut moves = find_rectangles_for_sum(cells, width, height, target)
        .into_iter()
        .filter_map(|rectangle| {
            let score = rectangle_score(cells, width, rectangle);
            (score > 0).then_some(WasmRectangle {
                left: rectangle.left,
                top: rectangle.top,
                right: rectangle.right,
                bottom: rectangle.bottom,
                score,
            })
        })
        .collect::<Vec<_>>();

    moves.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| area(a).cmp(&area(b)))
    });

    WasmMoveList { moves }
}

#[wasm_bindgen]
pub fn is_inside(
    index: usize,
    width: usize,
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
) -> bool {
    let x = index % width;
    let y = index / width;
    x >= left && x <= right && y >= top && y <= bottom
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

fn rectangle_sum(cells: &[u8], width: usize, rectangle: Rectangle) -> u16 {
    let mut sum = 0_u16;

    for y in rectangle.top..=rectangle.bottom {
        for x in rectangle.left..=rectangle.right {
            sum += cells[y * width + x] as u16;
        }
    }

    sum
}

fn rectangle_score(cells: &[u8], width: usize, rectangle: Rectangle) -> usize {
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

fn clear_rectangle(cells: &mut [u8], width: usize, rectangle: Rectangle) {
    for y in rectangle.top..=rectangle.bottom {
        for x in rectangle.left..=rectangle.right {
            cells[y * width + x] = 0;
        }
    }
}

fn area(rectangle: &WasmRectangle) -> usize {
    (rectangle.right - rectangle.left + 1) * (rectangle.bottom - rectangle.top + 1)
}
