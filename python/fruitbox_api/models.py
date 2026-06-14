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


class SampleSet(BaseModel):
    id: str
    label: str
    description: str
    source_name: str
    source_kind: str
    purpose: str
    exclude_from_training: bool
    sample_count: int
    metadata: dict[str, object] = Field(default_factory=dict)


class SampleRecord(BaseModel):
    id: str
    sample_set_id: str
    source_name: str
    source_kind: str
    image_uri: str
    image_sha256: str | None = None
    perceptual_hash: str | None = None
    board_signature: str | None = None
    width: int | None = None
    height: int | None = None
    captured_at: str | None = None
    metadata: dict[str, object] = Field(default_factory=dict)


class TrainingExclusion(BaseModel):
    sample_set_id: str
    record_id: str
    source_name: str
    source_kind: str
    image_sha256: str | None = None
    perceptual_hash: str | None = None
    board_signature: str | None = None
