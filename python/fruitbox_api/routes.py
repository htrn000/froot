from fastapi import APIRouter

from fruitbox_api.models import BoardRequest, GameMode, Rectangle, StaticMoveResponse
from fruitbox_core import find_sum_rectangles

router = APIRouter()


@router.get("/modes", response_model=list[GameMode])
async def list_modes() -> list[GameMode]:
    return [
        GameMode(
            id="singleplayer",
            label="Singleplayer",
            offline_capable=True,
            description="Local play with cached assets and deterministic rules.",
        ),
        GameMode(
            id="multiplayer",
            label="Multiplayer",
            offline_capable=False,
            description="Online sessions coordinated by the backend.",
        ),
        GameMode(
            id="bot-static",
            label="Static solver bot",
            offline_capable=True,
            description="Deterministic Rust solver suitable for Python and Wasm targets.",
        ),
        GameMode(
            id="bot-rl-nn",
            label="RL/NN bot",
            offline_capable=False,
            description="Model-backed bot; offline support depends on model size and browser runtime.",
        ),
    ]


@router.post("/solver/static-move", response_model=StaticMoveResponse)
async def static_move(board: BoardRequest) -> StaticMoveResponse:
    rectangles = [
        Rectangle(
            left=left,
            top=top,
            right=right,
            bottom=bottom,
            score=_score_rectangle(board, left, top, right, bottom),
        )
        for left, top, right, bottom in find_sum_rectangles(
            [int(cell) for cell in board.cells],
            board.width,
            board.target,
        )
    ]
    rectangles = [rectangle for rectangle in rectangles if rectangle.score > 0]
    rectangles.sort(key=lambda rectangle: rectangle.score, reverse=True)

    return StaticMoveResponse(
        move=rectangles[0] if rectangles else None,
        candidates=rectangles,
    )


def _score_rectangle(
    board: BoardRequest,
    left: int,
    top: int,
    right: int,
    bottom: int,
) -> int:
    score = 0

    for y in range(top, bottom + 1):
        for x in range(left, right + 1):
            if board.cells[y * board.width + x] > 0:
                score += 1

    return score
