import numpy as np
import pytest

from fruitbox_core import (
    BatchedFruitboxSimulator,
    solve_static_board,
    supports_static_solver_timeout,
)
import fruitbox_core.solver as solver_module
from fruitbox_rl import FruitboxBatch, FruitboxEnv


def test_rust_batched_simulator_steps_multiple_games() -> None:
    simulator = BatchedFruitboxSimulator(2, 2, 2, seed=7)
    simulator.set_cells(0, [1, 9, 1, 1])
    simulator.set_cells(1, [5, 5, 0, 0])
    action = simulator.rectangle_to_action(0, 0, 1, 0)

    observations, rewards, terminated, truncated = simulator.step([action, action])

    assert rewards == [2.0, 2.0]
    assert list(observations) == [0, 0, 1, 1, 0, 0, 0, 0]
    assert terminated == [True, True]
    assert truncated == [False, False]


def test_rust_batched_simulator_exposes_legal_action_masks() -> None:
    simulator = BatchedFruitboxSimulator(2, 2, 1, seed=7)
    simulator.set_cells(0, [1, 9, 4, 6])
    action = simulator.rectangle_to_action(0, 0, 1, 0)

    mask = simulator.action_masks()

    assert mask[action] == 1
    assert sum(mask) >= 1


def test_numpy_batch_wrapper_shapes_outputs() -> None:
    batch = FruitboxBatch(width=2, height=2, batch_size=2, target=10, seed=7)

    observations = batch.reset(seed=11)
    masks = batch.action_masks()

    assert observations.shape == (2, 2, 2)
    assert observations.dtype == np.uint8
    assert masks.shape == (2, batch.action_count)
    assert masks.dtype == np.bool_


def test_gymnasium_env_uses_rust_batch_size_one() -> None:
    env = FruitboxEnv(width=2, height=2, target=10, max_steps=4, seed=7)
    env.batch.set_cells(0, [1, 9, 1, 1])
    action = env.rectangle_to_action(0, 0, 1, 0)

    observation, reward, terminated, truncated, info = env.step(action)

    assert reward == 2.0
    assert terminated is True
    assert truncated is False
    assert observation.tolist() == [[0, 0], [1, 1]]
    assert info["score"] == 2
    assert info["rectangle"] == (0, 0, 1, 0)
    assert env.action_masks().shape == (env.action_space.n,)


def test_static_solver_uses_depth_first_backtracking() -> None:
    result = solve_static_board(
        [1, 9, 4, 6],
        2,
        target=10,
        max_solutions=0,
    )

    assert result.score == 4
    assert result.solutions_seen >= 1
    assert result.exhausted is True
    assert result.timed_out is False
    assert result.rectangles == [(0, 0, 1, 0), (0, 0, 1, 1)]


def test_static_solver_respects_solution_limit() -> None:
    limited = solve_static_board(
        [1, 9, 4, 6],
        2,
        target=10,
        max_solutions=1,
    )

    assert limited.solutions_seen == 1
    assert limited.exhausted is False
    assert limited.score >= 2


def test_batch_can_call_static_solver_for_batch_slot() -> None:
    batch = FruitboxBatch(width=2, height=2, batch_size=1, target=10, seed=7)
    batch.set_cells(0, [1, 9, 4, 6])

    result = batch.solve_static(max_solutions=0)

    assert result.score == 4
    assert result.actions == [
        batch.rectangle_to_action(0, 0, 1, 0),
        batch.rectangle_to_action(0, 0, 1, 1),
    ]


def test_timeout_support_flag_is_boolean() -> None:
    assert isinstance(supports_static_solver_timeout(), bool)


def test_static_solver_timeout_when_feature_is_enabled() -> None:
    if not supports_static_solver_timeout():
        pytest.skip("native extension was built without static solver timeout support")

    result = solve_static_board(
        [1, 9, 4, 6],
        2,
        target=10,
        max_solutions=0,
        timeout_ms=0,
    )

    assert result.timed_out is True


def test_python_solver_warns_when_timeout_feature_is_missing(monkeypatch) -> None:
    monkeypatch.setattr(solver_module, "supports_static_solver_timeout", lambda: False)

    with pytest.warns(RuntimeWarning, match="timeout support is disabled"):
        max_solutions, timeout_ms = solver_module.normalize_static_solver_limits(
            max_solutions=0,
            timeout_ms=10,
        )

    assert max_solutions == 3
    assert timeout_ms is None


def test_sb3_batched_vec_env_when_rl_group_is_installed() -> None:
    pytest.importorskip("sb3_contrib")
    from fruitbox_rl import MaskablePPOConfig, make_batched_env

    env = make_batched_env(MaskablePPOConfig(width=2, height=2, batch_size=2, n_steps=4))
    observations = env.reset()
    masks = env.action_masks()

    assert observations.shape == (2, 2, 2)
    assert masks.shape == (2, env.action_space.n)
