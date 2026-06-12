import { createGame } from "../src/game/board";
import {
  applyMove,
  createSelection,
  playHeadlessGame,
  previewSelection,
} from "../src/game/engine";
import { bestMove, findSumRectangles, listValidMoves } from "../src/game/solver";
import { describe, expect, it } from "vitest";

describe("solver", () => {
  it("finds rectangles that sum to the target", () => {
    const rectangles = findSumRectangles([1, 9, 4, 6], 2, 10);

    expect(rectangles).toContainEqual({ left: 0, top: 0, right: 1, bottom: 0 });
  });
});

describe("engine", () => {
  it("accepts valid moves and clears cells", () => {
    const state = createGame({
      width: 3,
      height: 2,
      target: 10,
      cells: [1, 2, 4, 3, 4, 6],
    });

    const move = { left: 0, top: 0, right: 1, bottom: 1 };
    const preview = previewSelection(state, move);

    expect(preview.valid).toBe(true);

    const { state: next, result } = applyMove(state, move);

    expect(result.ok).toBe(true);
    expect(next.score).toBe(4);
    expect(next.cells).toEqual([0, 0, 4, 0, 0, 6]);
  });

  it("rejects invalid sums without mutating state", () => {
    const state = createGame({
      width: 2,
      height: 2,
      target: 10,
      cells: [1, 2, 3, 4],
    });

    const move = { left: 0, top: 0, right: 0, bottom: 0 };
    const { state: next, result } = applyMove(state, move);

    expect(result).toEqual({ ok: false, reason: "invalid-sum" });
    expect(next).toEqual(state);
  });

  it("plays a full headless game using the best-move bot", () => {
    const finalState = playHeadlessGame(
      {
        width: 3,
        height: 2,
        target: 10,
        cells: [1, 2, 4, 3, 4, 6],
      },
      (state) => bestMove(state),
    );

    expect(finalState.moves).toBeGreaterThan(0);
    expect(listValidMoves(finalState)).toEqual([]);
  });

  it("normalizes drag selections", () => {
    const selection = createSelection({ x: 2, y: 1 }, { x: 0, y: 0 });

    expect(selection).toEqual({ left: 0, top: 0, right: 2, bottom: 1 });
  });
});
