# RL training notes

## Environment boundary

The game transition loop lives in Rust as `BatchedFruitboxSimulator`.

Python provides Gymnasium compatibility:

- `fruitbox_rl.FruitboxBatch`: NumPy-shaped wrapper for Rust batch simulation.
- `fruitbox_rl.FruitboxEnv`: `gymnasium.Env` backed by a Rust simulator with
  `batch_size=1`.
- `fruitbox_rl.FruitboxVecEnv`: Stable-Baselines3 `VecEnv` backed by one Rust
  batch simulator.

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

Install the optional RL group before training:

```bash
uv sync --group rl
```

Recommended first pass from Python:

```python
from fruitbox_rl import MaskablePPOConfig, train_maskable_ppo

config = MaskablePPOConfig(width=8, height=6, batch_size=16)
model = train_maskable_ppo(
    config,
    total_timesteps=100_000,
    save_path="models/fruitbox-maskable-ppo",
)
```

Or use the provided CLIs:

```bash
uv run --group rl fruitbox-train-maskable-ppo \
  --total-timesteps 100000 \
  --save-path models/fruitbox-maskable-ppo

uv run --group rl fruitbox-infer-maskable-ppo \
  models/fruitbox-maskable-ppo.zip \
  --batch-size 16 \
  --steps 10
```

Use these later only when there is a specific reason:

- Core SB3 `PPO`: acceptable baseline if you temporarily ignore action masks.
- Core SB3 `DQN`: possible because the action space is discrete, but inefficient
  without action masking.
- Core SB3 `A2C`: useful as a faster smoke-test baseline, still weaker than
  masked PPO for this action space.
- `SAC`, `TD3`, `DDPG`: not a natural fit because Fruitbox actions are discrete.

## Batched vectorization

Stable-Baselines3 expects its own vector-env interface. `FruitboxVecEnv` maps
one SB3 vector step to one `FruitboxBatch.step(...)` call instead of running many
independent Python `FruitboxEnv` objects.

## Rust static solver

The static solver is implemented in Rust with explicit DFS/backtracking over a
stack. It repeatedly finds the next legal rectangle action, applies it to a
cloned board state, and backtracks when no further action is available.

Important parameters:

- `max_solutions`: stop after this many terminal solutions. `0` or `None` means
  no solution-count limit.
- `timeout_ms`: optional wall-clock timeout when the crate is built with the
  `static-solver-timeout` Cargo feature.

The Cargo feature is enabled by default:

```toml
[features]
default = ["static-solver-timeout"]
static-solver-timeout = []
```

If a Python caller requests a timeout but the native extension was built without
timeout support, the wrapper emits a runtime warning, disables the timeout, and
uses `max_solutions=3` unless the caller already supplied a finite limit.
