import { boardIsCleared, cellAt, cloneState, createGame, indexAt } from "./board";
import { bestMove, listValidMoves, sumRectangle } from "./solver";
import type {
  GameConfig,
  GameEvent,
  GameState,
  MoveResult,
  Rectangle,
  Selection,
} from "./types";
import { normalizeRectangle, pointInRectangle, rectangleArea } from "./types";

export function isRectangleInBounds(state: Pick<GameState, "width" | "height">, rectangle: Rectangle): boolean {
  return (
    rectangle.left >= 0 &&
    rectangle.top >= 0 &&
    rectangle.right < state.width &&
    rectangle.bottom < state.height &&
    rectangle.left <= rectangle.right &&
    rectangle.top <= rectangle.bottom
  );
}

export function selectionHasCells(state: GameState, rectangle: Rectangle): boolean {
  for (let y = rectangle.top; y <= rectangle.bottom; y += 1) {
    for (let x = rectangle.left; x <= rectangle.right; x += 1) {
      if (cellAt(state, x, y) > 0) {
        return true;
      }
    }
  }

  return false;
}

function resolveStatus(state: GameState): GameState["status"] {
  if (boardIsCleared(state)) {
    return "won";
  }

  if (listValidMoves(state).length === 0) {
    return "stuck";
  }

  return "playing";
}

export function applyMove(state: GameState, rectangle: Rectangle): { state: GameState; result: MoveResult; events: GameEvent[] } {
  if (state.status !== "playing") {
    return {
      state,
      result: { ok: false, reason: "game-over" },
      events: [],
    };
  }

  if (!isRectangleInBounds(state, rectangle)) {
    return {
      state,
      result: { ok: false, reason: "out-of-bounds" },
      events: [],
    };
  }

  if (!selectionHasCells(state, rectangle)) {
    return {
      state,
      result: { ok: false, reason: "empty-selection" },
      events: [{ type: "move-rejected", rectangle, reason: "empty-selection" }],
    };
  }

  const sum = sumRectangle(state, rectangle);
  if (sum !== state.target) {
    return {
      state,
      result: { ok: false, reason: "invalid-sum" },
      events: [{ type: "move-rejected", rectangle, reason: "invalid-sum" }],
    };
  }

  const next = cloneState(state);

  for (let y = rectangle.top; y <= rectangle.bottom; y += 1) {
    for (let x = rectangle.left; x <= rectangle.right; x += 1) {
      next.cells[indexAt(next.width, x, y)] = 0;
    }
  }

  const cleared = rectangleArea(rectangle);
  next.score += cleared;
  next.moves += 1;
  next.status = resolveStatus(next);

  const events: GameEvent[] = [
    {
      type: "move-applied",
      rectangle,
      cleared,
      score: next.score,
    },
  ];

  if (next.status !== "playing") {
    events.push({ type: "game-ended", status: next.status });
  }

  return {
    state: next,
    result: { ok: true, cleared, score: next.score },
    events,
  };
}

export function createSelection(start: { x: number; y: number }, end: { x: number; y: number }): Selection {
  return normalizeRectangle(start, end);
}

export function previewSelection(state: GameState, selection: Selection): {
  sum: number;
  valid: boolean;
  area: number;
} {
  if (!selection) {
    return { sum: 0, valid: false, area: 0 };
  }

  const sum = sumRectangle(state, selection);
  return {
    sum,
    valid: sum === state.target && selectionHasCells(state, selection),
    area: rectangleArea(selection),
  };
}

export function playBotMove(state: GameState): { state: GameState; move: Rectangle | null; events: GameEvent[] } {
  const move = bestMove(state);
  if (!move) {
    return { state, move: null, events: [] };
  }

  const { state: nextState, events } = applyMove(state, move);
  return { state: nextState, move, events };
}

export function playHeadlessGame(config: GameConfig, pickMove: (state: GameState, moves: Rectangle[]) => Rectangle | null): GameState {
  let state = createGame(config);

  while (state.status === "playing") {
    const moves = listValidMoves(state);
    const move = pickMove(state, moves);
    if (!move) {
      break;
    }

    state = applyMove(state, move).state;
  }

  return state;
}

export function cellsInSelection(selection: Selection): Array<{ x: number; y: number }> {
  if (!selection) {
    return [];
  }

  const cells: Array<{ x: number; y: number }> = [];
  for (let y = selection.top; y <= selection.bottom; y += 1) {
    for (let x = selection.left; x <= selection.right; x += 1) {
      cells.push({ x, y });
    }
  }

  return cells;
}

export function isCellSelected(selection: Selection, x: number, y: number): boolean {
  return selection ? pointInRectangle({ x, y }, selection) : false;
}

export { createGame, listValidMoves, bestMove, sumRectangle };
