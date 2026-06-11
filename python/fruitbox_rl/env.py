from typing import Any

import gymnasium as gym
import numpy as np
from gymnasium import spaces
from numpy.typing import NDArray

from fruitbox_core import (
    BatchedFruitboxSimulator,
    StaticSolverResult,
    normalize_static_solver_limits,
)


Observation = NDArray[np.uint8]
ActionMask = NDArray[np.bool_]


class FruitboxBatch:
    """NumPy-friendly wrapper around the Rust batched simulator."""

    def __init__(
        self,
        *,
        width: int = 8,
        height: int = 6,
        batch_size: int = 1,
        target: int = 10,
        max_steps: int | None = None,
        seed: int = 0,
    ) -> None:
        self.width = width
        self.height = height
        self.batch_size = batch_size
        self.target = target
        self._native = BatchedFruitboxSimulator(
            width,
            height,
            batch_size,
            target=target,
            max_steps=max_steps,
            seed=seed,
        )

    @property
    def action_count(self) -> int:
        return self._native.action_count

    @property
    def scores(self) -> NDArray[np.uint32]:
        return np.asarray(self._native.scores(), dtype=np.uint32)

    def reset(self, seed: int | None = None) -> Observation:
        return self._reshape_observations(self._native.reset(seed))

    def reset_at(self, batch_index: int, seed: int | None = None) -> Observation:
        return self._uint8_array(self._native.reset_at(batch_index, seed)).reshape(
            self.height,
            self.width,
        )

    def observations(self) -> Observation:
        return self._reshape_observations(self._native.observations())

    def set_cells(
        self, batch_index: int, cells: list[int] | NDArray[np.integer[Any]]
    ) -> Observation:
        return self._uint8_array(
            self._native.set_cells(batch_index, [int(cell) for cell in cells])
        ).reshape(self.height, self.width)

    def action_masks(self) -> ActionMask:
        return (
            self._uint8_array(self._native.action_masks())
            .astype(np.bool_)
            .reshape(
                self.batch_size,
                self.action_count,
            )
        )

    def legal_actions(self, batch_index: int = 0) -> NDArray[np.int64]:
        return np.asarray(self._native.legal_actions(batch_index), dtype=np.int64)

    def solve_static(
        self,
        batch_index: int = 0,
        *,
        max_solutions: int | None = 3,
        timeout_ms: int | None = None,
    ) -> StaticSolverResult:
        max_solutions, timeout_ms = normalize_static_solver_limits(max_solutions, timeout_ms)
        return StaticSolverResult.from_native(
            self._native.solve_static(
                batch_index,
                max_solutions=max_solutions,
                timeout_ms=timeout_ms,
            )
        )

    def action_to_rectangle(self, action: int) -> tuple[int, int, int, int]:
        return self._native.action_to_rectangle(action)

    def rectangle_to_action(self, left: int, top: int, right: int, bottom: int) -> int:
        return self._native.rectangle_to_action(left, top, right, bottom)

    def step(
        self,
        actions: list[int] | NDArray[np.integer[Any]],
    ) -> tuple[Observation, NDArray[np.float32], NDArray[np.bool_], NDArray[np.bool_]]:
        observations, rewards, terminated, truncated = self._native.step(
            [int(action) for action in actions]
        )
        return (
            self._reshape_observations(observations),
            np.asarray(rewards, dtype=np.float32),
            np.asarray(terminated, dtype=np.bool_),
            np.asarray(truncated, dtype=np.bool_),
        )

    def _reshape_observations(self, observations: list[int]) -> Observation:
        return self._uint8_array(observations).reshape(
            self.batch_size,
            self.height,
            self.width,
        )

    @staticmethod
    def _uint8_array(values: bytes | bytearray | memoryview | list[int]) -> NDArray[np.uint8]:
        if isinstance(values, bytes | bytearray | memoryview):
            return np.frombuffer(values, dtype=np.uint8).copy()

        return np.asarray(values, dtype=np.uint8)


class FruitboxEnv(gym.Env[Observation, int]):
    """Gymnasium environment backed by the Rust simulator with batch size 1."""

    metadata = {"render_modes": ["ansi", "human"], "render_fps": 4}

    def __init__(
        self,
        *,
        width: int = 8,
        height: int = 6,
        target: int = 10,
        max_steps: int | None = None,
        seed: int = 0,
        render_mode: str | None = None,
    ) -> None:
        super().__init__()
        if render_mode not in (None, "ansi", "human"):
            raise ValueError("render_mode must be one of None, 'ansi', or 'human'")

        self.width = width
        self.height = height
        self.target = target
        self.render_mode = render_mode
        self.batch = FruitboxBatch(
            width=width,
            height=height,
            batch_size=1,
            target=target,
            max_steps=max_steps,
            seed=seed,
        )

        self.action_space = spaces.Discrete(self.batch.action_count)
        self.observation_space = spaces.Box(
            low=0,
            high=9,
            shape=(height, width),
            dtype=np.uint8,
        )

    def reset(
        self,
        *,
        seed: int | None = None,
        options: dict[str, Any] | None = None,
    ) -> tuple[Observation, dict[str, Any]]:
        super().reset(seed=seed)
        del options
        observation = self.batch.reset(seed=seed)[0]
        return observation, self._info()

    def step(self, action: int) -> tuple[Observation, float, bool, bool, dict[str, Any]]:
        observation, rewards, terminated, truncated = self.batch.step([int(action)])
        return (
            observation[0],
            float(rewards[0]),
            bool(terminated[0]),
            bool(truncated[0]),
            self._info(action=int(action)),
        )

    def render(self) -> str | None:
        board = self.batch.observations()[0]
        rendered = "\n".join(" ".join(str(value) for value in row) for row in board)

        if self.render_mode == "human":
            print(rendered)
            return None

        return rendered

    def action_masks(self) -> ActionMask:
        """SB3-contrib MaskablePPO reads this method when present."""

        return self.batch.action_masks()[0]

    def legal_actions(self) -> NDArray[np.int64]:
        return self.batch.legal_actions(0)

    def solve_static(
        self,
        *,
        max_solutions: int | None = 3,
        timeout_ms: int | None = None,
    ) -> StaticSolverResult:
        return self.batch.solve_static(
            0,
            max_solutions=max_solutions,
            timeout_ms=timeout_ms,
        )

    def action_to_rectangle(self, action: int) -> tuple[int, int, int, int]:
        return self.batch.action_to_rectangle(action)

    def rectangle_to_action(self, left: int, top: int, right: int, bottom: int) -> int:
        return self.batch.rectangle_to_action(left, top, right, bottom)

    def _info(self, action: int | None = None) -> dict[str, Any]:
        info: dict[str, Any] = {
            "action_mask": self.action_masks(),
            "score": int(self.batch.scores[0]),
        }

        if action is not None:
            info["rectangle"] = self.action_to_rectangle(action)

        return info
