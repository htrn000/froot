# RL training notes

## Environment boundary

The game transition loop lives in Rust as `BatchedFruitboxSimulator`.

Python provides Gymnasium compatibility:

- `fruitbox_rl.FruitboxBatch`: NumPy-shaped wrapper for Rust batch simulation.
- `fruitbox_rl.FruitboxEnv`: `gymnasium.Env` backed by a Rust simulator with
  `batch_size=1`.

This keeps PyTorch and Stable-Baselines3 integration idiomatic while allowing
the hot loop to move through many Fruitbox boards in Rust.

## Action space

The environment uses `spaces.Discrete(action_count)`, where each action encodes
one axis-aligned rectangle on the board. The env exposes:

- `env.action_masks()`: boolean legal-action mask for SB3-contrib.
- `env.action_to_rectangle(action)`: decode action to `(left, top, right, bottom)`.
- `env.rectangle_to_action(left, top, right, bottom)`: encode a rectangle.

Invalid actions currently receive `-1.0` reward and do not change the board.
Legal actions receive reward equal to the number of non-empty fruits cleared.

## Stable-Baselines3 subset recommendation

Start with `sb3-contrib` `MaskablePPO`.

Fruitbox has a large discrete rectangle action space and only a subset of those
rectangles are legal at any state. Core Stable-Baselines3 algorithms can train,
but most will waste samples exploring invalid rectangles unless you heavily tune
invalid-action penalties. `MaskablePPO` consumes `env.action_masks()` directly.

Recommended first pass:

```python
from sb3_contrib import MaskablePPO

from fruitbox_rl import FruitboxEnv

env = FruitboxEnv(width=8, height=6, target=10)
model = MaskablePPO(
    "MlpPolicy",
    env,
    n_steps=1024,
    batch_size=256,
    gamma=0.99,
    verbose=1,
)
model.learn(total_timesteps=100_000)
```

Use these later only when there is a specific reason:

- Core SB3 `PPO`: acceptable baseline if you temporarily ignore action masks.
- Core SB3 `DQN`: possible because the action space is discrete, but inefficient
  without action masking.
- Core SB3 `A2C`: useful as a faster smoke-test baseline, still weaker than
  masked PPO for this action space.
- `SAC`, `TD3`, `DDPG`: not a natural fit because Fruitbox actions are discrete.

## Future vectorization

Stable-Baselines3 expects its own vector-env interface. The Rust simulator is
already batched, so the next performance step is a custom SB3 `VecEnv` adapter
that maps one SB3 vector step to one `FruitboxBatch.step(...)` call instead of
running many independent Python `FruitboxEnv` objects.
