import numpy as np

from fruitbox_core import BatchedFruitboxSimulator
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
