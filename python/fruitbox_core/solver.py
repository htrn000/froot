from __future__ import annotations

import warnings
from dataclasses import dataclass
from typing import Protocol

from fruitbox_core._native import solve_static_board_native, static_solver_supports_timeout


class NativeStaticSolverResult(Protocol):
    actions: list[int]
    rectangles: list[tuple[int, int, int, int]]
    score: int
    solutions_seen: int
    timed_out: bool
    exhausted: bool


@dataclass(frozen=True)
class StaticSolverResult:
    actions: list[int]
    rectangles: list[tuple[int, int, int, int]]
    score: int
    solutions_seen: int
    timed_out: bool
    exhausted: bool

    @classmethod
    def from_native(cls, result: NativeStaticSolverResult) -> "StaticSolverResult":
        return cls(
            actions=list(result.actions),
            rectangles=[tuple(rectangle) for rectangle in result.rectangles],
            score=int(result.score),
            solutions_seen=int(result.solutions_seen),
            timed_out=bool(result.timed_out),
            exhausted=bool(result.exhausted),
        )


def supports_static_solver_timeout() -> bool:
    return bool(static_solver_supports_timeout())


def solve_static_board(
    cells: list[int],
    width: int,
    *,
    target: int = 10,
    max_solutions: int | None = 3,
    timeout_ms: int | None = None,
) -> StaticSolverResult:
    max_solutions, timeout_ms = normalize_static_solver_limits(max_solutions, timeout_ms)
    native_result = solve_static_board_native(
        cells,
        width,
        target,
        max_solutions=max_solutions,
        timeout_ms=timeout_ms,
    )
    return StaticSolverResult.from_native(native_result)


def normalize_static_solver_limits(
    max_solutions: int | None,
    timeout_ms: int | None,
) -> tuple[int | None, int | None]:
    if timeout_ms is None or supports_static_solver_timeout():
        return max_solutions, timeout_ms

    if max_solutions is None or max_solutions <= 0:
        max_solutions = 3

    warnings.warn(
        "Rust static solver timeout support is disabled; running without timeout "
        f"and limiting search to max_solutions={max_solutions}.",
        RuntimeWarning,
        stacklevel=2,
    )
    return max_solutions, None
