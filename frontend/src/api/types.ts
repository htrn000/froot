export type GameMode = {
  id: string;
  label: string;
  offline_capable: boolean;
  description: string;
};

export type Rectangle = {
  left: number;
  top: number;
  right: number;
  bottom: number;
  score: number;
};

export type StaticMoveResponse = {
  move: Rectangle | null;
  candidates: Rectangle[];
};
