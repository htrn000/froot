import type { CellValue, GameConfig, GameState } from "./types";

function mulberry32(seed: number): () => number {
  let state = seed >>> 0;

  return () => {
    state += 0x6d2b79f5;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export function indexAt(width: number, x: number, y: number): number {
  return y * width + x;
}

export function cellAt(state: Pick<GameState, "width" | "cells">, x: number, y: number): CellValue {
  return state.cells[indexAt(state.width, x, y)] ?? 0;
}

export function createRandomBoard(width: number, height: number, seed = Date.now()): CellValue[] {
  const random = mulberry32(seed);
  const cells: CellValue[] = [];

  for (let index = 0; index < width * height; index += 1) {
    cells.push(Math.floor(random() * 9) + 1);
  }

  return cells;
}

export function createGame(config: GameConfig): GameState {
  const cells = config.cells ?? createRandomBoard(config.width, config.height, config.seed);

  if (cells.length !== config.width * config.height) {
    throw new Error("cells length must match width * height");
  }

  return {
    width: config.width,
    height: config.height,
    target: config.target,
    cells: [...cells],
    score: 0,
    moves: 0,
    status: "playing",
  };
}

export function cloneState(state: GameState): GameState {
  return {
    ...state,
    cells: [...state.cells],
  };
}

export function boardIsCleared(state: Pick<GameState, "cells">): boolean {
  return state.cells.every((cell) => cell === 0);
}
