export type CellValue = number;

export type Rectangle = {
  left: number;
  top: number;
  right: number;
  bottom: number;
};

export type GameStatus = "playing" | "won" | "stuck";

export type GameConfig = {
  width: number;
  height: number;
  target: number;
  seed?: number;
  cells?: CellValue[];
};

export type GameState = {
  width: number;
  height: number;
  target: number;
  cells: CellValue[];
  score: number;
  moves: number;
  status: GameStatus;
};

export type MoveFailureReason = "invalid-sum" | "empty-selection" | "game-over" | "out-of-bounds";

export type MoveResult =
  | { ok: true; cleared: number; score: number }
  | { ok: false; reason: MoveFailureReason };

export type Selection = Rectangle | null;

export type GameEvent =
  | { type: "move-applied"; rectangle: Rectangle; cleared: number; score: number }
  | { type: "move-rejected"; rectangle: Rectangle; reason: MoveFailureReason }
  | { type: "game-ended"; status: Exclude<GameStatus, "playing"> };

export type Point = {
  x: number;
  y: number;
};

export function normalizeRectangle(start: Point, end: Point): Rectangle {
  return {
    left: Math.min(start.x, end.x),
    right: Math.max(start.x, end.x),
    top: Math.min(start.y, end.y),
    bottom: Math.max(start.y, end.y),
  };
}

export function rectangleArea(rectangle: Rectangle): number {
  return (rectangle.right - rectangle.left + 1) * (rectangle.bottom - rectangle.top + 1);
}

export function pointInRectangle(point: Point, rectangle: Rectangle): boolean {
  return (
    point.x >= rectangle.left &&
    point.x <= rectangle.right &&
    point.y >= rectangle.top &&
    point.y <= rectangle.bottom
  );
}

export function rectanglesEqual(a: Rectangle, b: Rectangle): boolean {
  return a.left === b.left && a.top === b.top && a.right === b.right && a.bottom === b.bottom;
}
