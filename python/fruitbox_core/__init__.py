"""Python imports for the Rust-backed Fruitbox core."""

from fruitbox_core._native import BatchedFruitboxSimulator, find_sum_rectangles
from fruitbox_core.solver import (
    StaticSolverResult,
    normalize_static_solver_limits,
    solve_static_board,
    supports_static_solver_timeout,
)

__all__ = [
    "BatchedFruitboxSimulator",
    "StaticSolverResult",
    "find_sum_rectangles",
    "normalize_static_solver_limits",
    "solve_static_board",
    "supports_static_solver_timeout",
]
