from fastapi.testclient import TestClient

from fruitbox_api.app import create_app
from fruitbox_api.config import get_settings
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


def test_static_solver_scores_non_empty_cells() -> None:
    client = TestClient(create_app())

    response = client.post(
        "/api/v1/solver/static-move",
        json={
            "width": 4,
            "height": 1,
            "cells": [1, 0, 9, 0],
        },
    )

    assert response.status_code == 200
    assert response.json()["move"]["score"] == 2


def test_static_solver_validates_board_shape() -> None:
    client = TestClient(create_app())

    response = client.post(
        "/api/v1/solver/static-move",
        json={"width": 3, "height": 2, "cells": [1, 2, 3]},
    )

    assert response.status_code == 422


def test_rust_core_finds_sum_rectangles() -> None:
    assert (0, 0, 1, 0) in find_sum_rectangles([1, 9, 4, 6], 2, 10)


def test_frontend_dist_is_served_when_available(tmp_path, monkeypatch) -> None:
    (tmp_path / "assets").mkdir()
    (tmp_path / "index.html").write_text("<h1>Fruitbox shell</h1>")
    (tmp_path / "icon.svg").write_text("<svg />")
    monkeypatch.setenv("FRUITBOX_FRONTEND_DIST_PATH", str(tmp_path))
    get_settings.cache_clear()

    try:
        client = TestClient(create_app())

        index_response = client.get("/")
        fallback_response = client.get("/singleplayer")
        asset_response = client.get("/icon.svg")
        api_response = client.get("/api/missing")
    finally:
        get_settings.cache_clear()

    assert index_response.status_code == 200
    assert "Fruitbox shell" in index_response.text
    assert fallback_response.status_code == 200
    assert asset_response.status_code == 200
    assert api_response.status_code == 404
