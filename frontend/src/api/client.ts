import type { GameMode, Rectangle, StaticMoveResponse } from "./types";

export type { GameMode, Rectangle, StaticMoveResponse };

export async function fetchHealth(): Promise<boolean> {
  try {
    const response = await fetch("/health");
    return response.ok;
  } catch {
    return false;
  }
}

export async function fetchModes(): Promise<GameMode[]> {
  const response = await fetch("/api/v1/modes");
  if (!response.ok) {
    throw new Error("Failed to load game modes");
  }

  return response.json() as Promise<GameMode[]>;
}

export async function fetchStaticMove(payload: {
  width: number;
  height: number;
  cells: number[];
  target?: number;
}): Promise<StaticMoveResponse> {
  const response = await fetch("/api/v1/solver/static-move", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ ...payload, target: payload.target ?? 10 }),
  });

  if (!response.ok) {
    throw new Error("Solver request failed");
  }

  return response.json() as Promise<StaticMoveResponse>;
}
