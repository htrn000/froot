import { cellAt } from "./board";
import type { GameState, Rectangle } from "./types";
import { rectangleArea } from "./types";

export function findSumRectangles(
  cells: number[],
  width: number,
  target: number,
): Rectangle[] {
  if (width <= 0 || target <= 0 || cells.length === 0 || cells.length % width !== 0) {
    return [];
  }

  const height = cells.length / width;
  const rectangles: Rectangle[] = [];

  for (let top = 0; top < height; top += 1) {
    const columnSums = new Array<number>(width).fill(0);

    for (let bottom = top; bottom < height; bottom += 1) {
      for (let x = 0; x < width; x += 1) {
        columnSums[x] += cells[bottom * width + x] ?? 0;
      }

      for (let left = 0; left < width; left += 1) {
        let sum = 0;

        for (let right = left; right < width; right += 1) {
          sum += columnSums[right] ?? 0;

          if (sum === target) {
            rectangles.push({ left, top, right, bottom });
          }
          if (sum > target) {
            break;
          }
        }
      }
    }
  }

  return rectangles;
}

export function sumRectangle(state: Pick<GameState, "width" | "cells">, rectangle: Rectangle): number {
  let sum = 0;

  for (let y = rectangle.top; y <= rectangle.bottom; y += 1) {
    for (let x = rectangle.left; x <= rectangle.right; x += 1) {
      sum += cellAt(state, x, y);
    }
  }

  return sum;
}

export function listValidMoves(state: GameState): Rectangle[] {
  return findSumRectangles(state.cells, state.width, state.target).filter((rectangle) => {
    return sumRectangle(state, rectangle) === state.target && rectangleArea(rectangle) > 0;
  });
}

export function bestMove(state: GameState): Rectangle | null {
  const moves = listValidMoves(state);
  moves.sort((a, b) => rectangleArea(b) - rectangleArea(a));
  return moves[0] ?? null;
}
