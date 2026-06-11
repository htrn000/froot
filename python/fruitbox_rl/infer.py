from __future__ import annotations

import argparse
from pathlib import Path

from fruitbox_rl.sb3 import MaskablePPOConfig, load_maskable_ppo, make_batched_env, rollout_batched


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run batched inference for a Fruitbox MaskablePPO model."
    )
    parser.add_argument("model_path", type=Path)
    parser.add_argument("--steps", type=int, default=10)
    parser.add_argument("--width", type=int, default=8)
    parser.add_argument("--height", type=int, default=6)
    parser.add_argument("--target", type=int, default=10)
    parser.add_argument("--batch-size", type=int, default=16)
    parser.add_argument("--max-steps", type=int, default=None)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument(
        "--sample", action="store_true", help="Sample actions instead of greedy inference."
    )
    args = parser.parse_args()

    config = MaskablePPOConfig(
        width=args.width,
        height=args.height,
        target=args.target,
        batch_size=args.batch_size,
        max_steps=args.max_steps,
        seed=args.seed,
    )
    env = make_batched_env(config)
    model = load_maskable_ppo(args.model_path, env=env)
    history = rollout_batched(model, env, steps=args.steps, deterministic=not args.sample)

    for index, step in enumerate(history, start=1):
        rewards = ", ".join(f"{reward:.1f}" for reward in step["rewards"])
        done_count = int(step["dones"].sum())
        print(f"step={index} rewards=[{rewards}] done_count={done_count}")


if __name__ == "__main__":
    main()
