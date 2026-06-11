from typing import Annotated

from pydantic import BaseModel, Field, model_validator


CellValue = Annotated[int, Field(ge=0, le=9)]


class BoardRequest(BaseModel):
    width: int = Field(gt=0, le=64)
    height: int = Field(gt=0, le=64)
    cells: list[CellValue]
    target: int = Field(default=10, gt=0, le=255)

    @model_validator(mode="after")
    def validate_cell_count(self) -> "BoardRequest":
        expected = self.width * self.height
        if len(self.cells) != expected:
            raise ValueError(f"cells must contain exactly {expected} values")
        return self


class Rectangle(BaseModel):
    left: int
    top: int
    right: int
    bottom: int
    score: int


class StaticMoveResponse(BaseModel):
    move: Rectangle | None
    candidates: list[Rectangle]


class GameMode(BaseModel):
    id: str
    label: str
    offline_capable: bool
    description: str
