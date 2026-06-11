import type { Board, Rectangle } from "./solver";
import { findStaticMoves } from "./solver";

export type GameMode = {
  id: string;
  label: string;
  offline_capable: boolean;
  description: string;
};

export type SolverResult = {
  move: Rectangle | null;
  source: "api" | "local";
};

export async function loadModes(): Promise<GameMode[]> {
  const response = await fetch("/api/v1/modes");

  if (!response.ok) {
    throw new Error(`Failed to load modes: ${response.status}`);
  }

  return response.json() as Promise<GameMode[]>;
}

export async function findBestMove(board: Board): Promise<SolverResult> {
  try {
    const response = await fetch("/api/v1/solver/static-move", {
      method: "POST",
      headers: {
        "content-type": "application/json"
      },
      body: JSON.stringify(board)
    });

    if (response.ok) {
      const payload = (await response.json()) as { move: Rectangle | null };
      return { move: payload.move, source: "api" };
    }
  } catch {
    // Offline PWA mode intentionally falls through to the local deterministic solver.
  }

  return { move: findStaticMoves(board)[0] ?? null, source: "local" };
}

export function fallbackModes(): GameMode[] {
  return [
    {
      id: "singleplayer",
      label: "Singleplayer",
      offline_capable: true,
      description: "Playable now in the browser with local scoring."
    },
    {
      id: "bot-static",
      label: "Static solver bot",
      offline_capable: true,
      description: "Available as a local solver fallback when the API is unreachable."
    },
    {
      id: "multiplayer",
      label: "Multiplayer",
      offline_capable: false,
      description: "Planned online mode."
    }
  ];
}
