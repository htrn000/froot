from fastapi.testclient import TestClient

from fruitbox_api.app import create_app
from fruitbox_core import find_sum_rectangles


def test_health() -> None:
    client = TestClient(create_app())

    response = client.get("/health")

    assert response.status_code == 200
    assert response.json() == {"status": "ok"}


def test_static_solver_endpoint_returns_best_rectangle() -> None:
    client = TestClient(create_app())

    response = client.post(
        "/api/v1/solver/static-move",
        json={
            "width": 3,
            "height": 2,
            "cells": [
                1,
                2,
                4,
                3,
                4,
                6,
            ],
        },
    )

    assert response.status_code == 200
    assert response.json()["move"] == {
        "left": 0,
        "top": 0,
        "right": 1,
        "bottom": 1,
        "score": 4,
    }


def test_static_solver_validates_board_shape() -> None:
    client = TestClient(create_app())

    response = client.post(
        "/api/v1/solver/static-move",
        json={"width": 3, "height": 2, "cells": [1, 2, 3]},
    )

    assert response.status_code == 422


def test_rust_core_finds_sum_rectangles() -> None:
    assert (0, 0, 1, 0) in find_sum_rectangles([1, 9, 4, 6], 2, 10)
