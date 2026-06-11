import initWasm, {
  apply_rectangle,
  create_board_cells,
  find_static_moves,
  is_inside,
  rectangle_from_cells,
  score_rectangle,
  sum_rectangle,
  type WasmRectangle
} from "./wasm/fruitbox_wasm";

export type Rectangle = {
  left: number;
  top: number;
  right: number;
  bottom: number;
  score: number;
};

export type Board = {
  width: number;
  height: number;
  cells: number[];
  target: number;
};

let wasmInitialized = false;

export async function initFruitboxWasm(): Promise<void> {
  if (!wasmInitialized) {
    await initWasm();
    wasmInitialized = true;
  }
}

export function createBoard(width = 8, height = 6, target = 10): Board {
  return {
    width,
    height,
    target,
    cells: Array.from(create_board_cells(width, height, randomSeed()))
  };
}

export function rectangleFromCells(
  startIndex: number,
  endIndex: number,
  width: number
): Omit<Rectangle, "score"> {
  return copyRectangle(rectangle_from_cells(startIndex, endIndex, width));
}

export function scoreRectangle(board: Board, rectangle: Omit<Rectangle, "score">): Rectangle {
  return copyRectangle(
    score_rectangle(
      Uint8Array.from(board.cells),
      board.width,
      rectangle.left,
      rectangle.top,
      rectangle.right,
      rectangle.bottom
    )
  );
}

export function sumRectangle(board: Board, rectangle: Omit<Rectangle, "score">): number {
  return sum_rectangle(
    Uint8Array.from(board.cells),
    board.width,
    rectangle.left,
    rectangle.top,
    rectangle.right,
    rectangle.bottom
  );
}

export function applyRectangle(board: Board, rectangle: Rectangle): Board {
  return {
    ...board,
    cells: Array.from(
      apply_rectangle(
        Uint8Array.from(board.cells),
        board.width,
        rectangle.left,
        rectangle.top,
        rectangle.right,
        rectangle.bottom
      )
    )
  };
}

export function findStaticMoves(board: Board): Rectangle[] {
  const wasmMoves = find_static_moves(Uint8Array.from(board.cells), board.width, board.target);
  const moves: Rectangle[] = [];

  try {
    for (let index = 0; index < wasmMoves.length; index += 1) {
      const move = wasmMoves.get(index);
      if (move) {
        moves.push(copyRectangle(move));
      }
    }
  } finally {
    wasmMoves.free();
  }

  return moves;
}

export function isInside(rectangle: Omit<Rectangle, "score"> | null, index: number, width: number): boolean {
  if (!rectangle) {
    return false;
  }

  return is_inside(index, width, rectangle.left, rectangle.top, rectangle.right, rectangle.bottom);
}

function copyRectangle(rectangle: WasmRectangle): Rectangle {
  try {
    return {
      left: rectangle.left,
      top: rectangle.top,
      right: rectangle.right,
      bottom: rectangle.bottom,
      score: rectangle.score
    };
  } finally {
    rectangle.free();
  }
}

function randomSeed(): number {
  const values = new Uint32Array(1);
  crypto.getRandomValues(values);
  return values[0];
}
