from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import numpy as np
from numpy.typing import NDArray

from fruitbox_rl.env import FruitboxBatch, FruitboxEnv, Observation

try:
    from sb3_contrib import MaskablePPO
    from stable_baselines3.common.vec_env import VecEnv
except ImportError as import_error:  # pragma: no cover - exercised when RL group is absent.
    MaskablePPO = None  # type: ignore[assignment]
    VecEnv = object  # type: ignore[assignment,misc]
    _SB3_IMPORT_ERROR: ImportError | None = import_error
else:
    _SB3_IMPORT_ERROR = None


@dataclass(frozen=True)
class MaskablePPOConfig:
    width: int = 8
    height: int = 6
    target: int = 10
    batch_size: int = 16
    max_steps: int | None = None
    seed: int = 0
    policy: str = "MlpPolicy"
    learning_rate: float = 3e-4
    n_steps: int = 1024
    batch_size_train: int = 256
    gamma: float = 0.99
    verbose: int = 1
    model_kwargs: dict[str, Any] = field(default_factory=dict)


class FruitboxVecEnv(VecEnv):  # type: ignore[misc]
    """Stable-Baselines3 VecEnv backed by one Rust batched simulator."""

    def __init__(
        self,
        *,
        width: int = 8,
        height: int = 6,
        batch_size: int = 16,
        target: int = 10,
        max_steps: int | None = None,
        seed: int = 0,
    ) -> None:
        _require_sb3()
        self.batch = FruitboxBatch(
            width=width,
            height=height,
            batch_size=batch_size,
            target=target,
            max_steps=max_steps,
            seed=seed,
        )
        self._pending_actions: NDArray[np.int64] | None = None
        self._seeds = [seed + index for index in range(batch_size)]
        self.render_mode = None
        probe_env = FruitboxEnv(width=width, height=height, target=target, max_steps=max_steps)
        super().__init__(batch_size, probe_env.observation_space, probe_env.action_space)

    def reset(self) -> Observation:
        return self.batch.reset(seed=self._seeds[0])

    def step_async(self, actions: NDArray[np.integer[Any]] | list[int]) -> None:
        self._pending_actions = np.asarray(actions, dtype=np.int64)

    def step_wait(
        self,
    ) -> tuple[Observation, NDArray[np.float32], NDArray[np.bool_], list[dict[str, Any]]]:
        if self._pending_actions is None:
            raise RuntimeError("step_async must be called before step_wait")

        observations, rewards, terminated, truncated = self.batch.step(self._pending_actions)
        dones = np.logical_or(terminated, truncated)
        infos: list[dict[str, Any]] = []

        for index, done in enumerate(dones):
            info: dict[str, Any] = {
                "action_mask": self.batch.action_masks()[index],
                "score": int(self.batch.scores[index]),
                "TimeLimit.truncated": bool(truncated[index] and not terminated[index]),
            }
            if done:
                info["terminal_observation"] = observations[index].copy()
                self._seeds[index] += self.num_envs
                observations[index] = self.batch.reset_at(index, seed=self._seeds[index])
            infos.append(info)

        self._pending_actions = None
        return observations, rewards, dones.astype(np.bool_), infos

    def close(self) -> None:
        self._pending_actions = None

    def action_masks(self) -> NDArray[np.bool_]:
        return self.batch.action_masks()

    def get_attr(self, attr_name: str, indices: Any = None) -> list[Any]:
        return [getattr(self, attr_name) for _ in self._resolve_indices(indices)]

    def set_attr(self, attr_name: str, value: Any, indices: Any = None) -> None:
        for _ in self._resolve_indices(indices):
            setattr(self, attr_name, value)

    def env_method(
        self, method_name: str, *method_args: Any, indices: Any = None, **method_kwargs: Any
    ) -> list[Any]:
        resolved_indices = self._resolve_indices(indices)
        if method_name == "action_masks":
            masks = self.action_masks()
            return [masks[index] for index in resolved_indices]
        if method_name == "render":
            observations = self.batch.observations()
            return [_render_observation(observations[index]) for index in resolved_indices]
        if method_name == "solve_static":
            return [
                self.batch.solve_static(index, *method_args, **method_kwargs)
                for index in resolved_indices
            ]

        raise AttributeError(f"FruitboxVecEnv has no env method {method_name!r}")

    def env_is_wrapped(self, wrapper_class: type[Any], indices: Any = None) -> list[bool]:
        del wrapper_class
        return [False for _ in self._resolve_indices(indices)]

    def get_images(self) -> list[None]:
        return [None for _ in range(self.num_envs)]

    def _resolve_indices(self, indices: Any = None) -> list[int]:
        if indices is None:
            return list(range(self.num_envs))
        if isinstance(indices, int):
            return [indices]
        return [int(index) for index in indices]


def make_batched_env(config: MaskablePPOConfig) -> FruitboxVecEnv:
    return FruitboxVecEnv(
        width=config.width,
        height=config.height,
        batch_size=config.batch_size,
        target=config.target,
        max_steps=config.max_steps,
        seed=config.seed,
    )


def train_maskable_ppo(
    config: MaskablePPOConfig,
    *,
    total_timesteps: int,
    save_path: str | Path | None = None,
) -> Any:
    _require_sb3()
    env = make_batched_env(config)
    model = MaskablePPO(  # type: ignore[misc]
        config.policy,
        env,
        learning_rate=config.learning_rate,
        n_steps=config.n_steps,
        batch_size=config.batch_size_train,
        gamma=config.gamma,
        verbose=config.verbose,
        **config.model_kwargs,
    )
    model.learn(total_timesteps=total_timesteps)

    if save_path is not None:
        model.save(Path(save_path))

    return model


def load_maskable_ppo(path: str | Path, env: FruitboxVecEnv | None = None) -> Any:
    _require_sb3()
    return MaskablePPO.load(Path(path), env=env)  # type: ignore[union-attr]


def predict_batched(
    model: Any,
    env: FruitboxVecEnv,
    observations: Observation | None = None,
    *,
    deterministic: bool = True,
) -> NDArray[np.int64]:
    if observations is None:
        observations = env.batch.observations()

    actions, _ = model.predict(
        observations,
        deterministic=deterministic,
        action_masks=env.action_masks(),
    )
    return np.asarray(actions, dtype=np.int64)


def rollout_batched(
    model: Any,
    env: FruitboxVecEnv,
    *,
    steps: int,
    deterministic: bool = True,
) -> list[dict[str, Any]]:
    observations = env.reset()
    history: list[dict[str, Any]] = []

    for _ in range(steps):
        actions = predict_batched(model, env, observations, deterministic=deterministic)
        observations, rewards, dones, infos = env.step(actions)
        history.append(
            {
                "actions": actions,
                "rewards": rewards,
                "dones": dones,
                "infos": infos,
            }
        )

    return history


def _render_observation(observation: Observation) -> str:
    return "\n".join(" ".join(str(value) for value in row) for row in observation)


def _require_sb3() -> None:
    if _SB3_IMPORT_ERROR is not None:
        raise ImportError(
            "Stable-Baselines3 support is optional. Install the RL dependency group with "
            "`uv sync --group rl` or `uv add --group rl stable-baselines3 sb3-contrib`."
        ) from _SB3_IMPORT_ERROR
