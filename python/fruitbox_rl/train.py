from __future__ import annotations

import argparse
from pathlib import Path

from fruitbox_rl.sb3 import MaskablePPOConfig, train_maskable_ppo


def main() -> None:
    parser = argparse.ArgumentParser(description="Train a MaskablePPO Fruitbox agent.")
    parser.add_argument("--total-timesteps", type=int, default=100_000)
    parser.add_argument("--save-path", type=Path, default=Path("models/fruitbox-maskable-ppo"))
    parser.add_argument("--width", type=int, default=8)
    parser.add_argument("--height", type=int, default=6)
    parser.add_argument("--target", type=int, default=10)
    parser.add_argument("--batch-size", type=int, default=16)
    parser.add_argument("--max-steps", type=int, default=None)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--n-steps", type=int, default=1024)
    parser.add_argument("--train-batch-size", type=int, default=256)
    parser.add_argument("--learning-rate", type=float, default=3e-4)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--verbose", type=int, default=1)
    args = parser.parse_args()

    args.save_path.parent.mkdir(parents=True, exist_ok=True)
    config = MaskablePPOConfig(
        width=args.width,
        height=args.height,
        target=args.target,
        batch_size=args.batch_size,
        max_steps=args.max_steps,
        seed=args.seed,
        n_steps=args.n_steps,
        batch_size_train=args.train_batch_size,
        learning_rate=args.learning_rate,
        gamma=args.gamma,
        verbose=args.verbose,
    )
    train_maskable_ppo(config, total_timesteps=args.total_timesteps, save_path=args.save_path)


if __name__ == "__main__":
    main()
