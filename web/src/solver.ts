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

export function createBoard(width = 8, height = 6, target = 10): Board {
  return {
    width,
    height,
    target,
    cells: Array.from({ length: width * height }, () => randomFruitValue())
  };
}

export function rectangleFromCells(
  startIndex: number,
  endIndex: number,
  width: number
): Omit<Rectangle, "score"> {
  const startX = startIndex % width;
  const startY = Math.floor(startIndex / width);
  const endX = endIndex % width;
  const endY = Math.floor(endIndex / width);

  return {
    left: Math.min(startX, endX),
    top: Math.min(startY, endY),
    right: Math.max(startX, endX),
    bottom: Math.max(startY, endY)
  };
}

export function scoreRectangle(board: Board, rectangle: Omit<Rectangle, "score">): Rectangle {
  let score = 0;

  forEachRectangleIndex(board, rectangle, (index) => {
    if (board.cells[index] > 0) {
      score += 1;
    }
  });

  return { ...rectangle, score };
}

export function sumRectangle(board: Board, rectangle: Omit<Rectangle, "score">): number {
  let sum = 0;

  forEachRectangleIndex(board, rectangle, (index) => {
    sum += board.cells[index];
  });

  return sum;
}

export function applyRectangle(board: Board, rectangle: Rectangle): Board {
  const cells = [...board.cells];

  forEachRectangleIndex(board, rectangle, (index) => {
    cells[index] = 0;
  });

  return { ...board, cells };
}

export function findStaticMoves(board: Board): Rectangle[] {
  const moves: Rectangle[] = [];

  for (let top = 0; top < board.height; top += 1) {
    const columnSums = Array.from({ length: board.width }, () => 0);

    for (let bottom = top; bottom < board.height; bottom += 1) {
      for (let x = 0; x < board.width; x += 1) {
        columnSums[x] += board.cells[bottom * board.width + x];
      }

      for (let left = 0; left < board.width; left += 1) {
        let sum = 0;

        for (let right = left; right < board.width; right += 1) {
          sum += columnSums[right];

          if (sum === board.target) {
            const move = scoreRectangle(board, { left, top, right, bottom });
            if (move.score > 0) {
              moves.push(move);
            }
          }

          if (sum > board.target) {
            break;
          }
        }
      }
    }
  }

  return moves.sort((a, b) => b.score - a.score || area(a) - area(b));
}

export function isInside(rectangle: Omit<Rectangle, "score"> | null, index: number, width: number): boolean {
  if (!rectangle) {
    return false;
  }

  const x = index % width;
  const y = Math.floor(index / width);
  return x >= rectangle.left && x <= rectangle.right && y >= rectangle.top && y <= rectangle.bottom;
}

function forEachRectangleIndex(
  board: Board,
  rectangle: Omit<Rectangle, "score">,
  callback: (index: number) => void
) {
  for (let y = rectangle.top; y <= rectangle.bottom; y += 1) {
    for (let x = rectangle.left; x <= rectangle.right; x += 1) {
      callback(y * board.width + x);
    }
  }
}

function area(rectangle: Omit<Rectangle, "score">): number {
  return (rectangle.right - rectangle.left + 1) * (rectangle.bottom - rectangle.top + 1);
}

function randomFruitValue(): number {
  const values = new Uint32Array(1);
  crypto.getRandomValues(values);
  return (values[0] % 9) + 1;
}
