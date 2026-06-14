from collections.abc import Iterator

import pytest
from fastapi.testclient import TestClient

from fruitbox_api.app import create_app
from fruitbox_api.config import get_settings
from fruitbox_api.sample_catalog import upsert_sample_record, upsert_sample_set
from fruitbox_core import find_sum_rectangles


@pytest.fixture(autouse=True)
def clear_settings_cache() -> Iterator[None]:
    get_settings.cache_clear()
    yield
    get_settings.cache_clear()


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


def test_sample_catalog_endpoints_merge_provisioned_and_environment_sources(
    tmp_path,
    monkeypatch,
) -> None:
    provisioned = tmp_path / "provisioned.sqlite3"
    environment = tmp_path / "environment.sqlite3"
    upsert_sample_set(
        provisioned,
        sample_set_id="fruitbox-provisioned-v1",
        label="Provisioned Fruitbox captures",
        description="Reviewed in-repo captures.",
    )
    upsert_sample_record(
        provisioned,
        record_id="capture-0001",
        sample_set_id="fruitbox-provisioned-v1",
        image_uri="data/provisioned/images/capture-0001.png",
        image_sha256="sha256-provisioned",
        perceptual_hash="phash-provisioned",
        board_signature="board-provisioned",
        width=640,
        height=480,
    )
    upsert_sample_set(
        environment,
        sample_set_id="fruitbox-deployment-v1",
        label="Deployment captures",
        description="Impure deployment captures.",
    )
    upsert_sample_record(
        environment,
        record_id="capture-env-0001",
        sample_set_id="fruitbox-deployment-v1",
        image_uri="/var/lib/fruitbox/images/capture-env-0001.png",
        image_sha256="sha256-environment",
    )
    monkeypatch.setenv("FRUITBOX_SAMPLE_CATALOG_PATH", str(provisioned))
    monkeypatch.setenv("FRUITBOX_SAMPLE_CATALOG_OVERLAYS", str(environment))

    client = TestClient(create_app())

    sets_response = client.get("/api/v1/samples/sets")
    records_response = client.get("/api/v1/samples/sets/fruitbox-provisioned-v1/records")
    exclusions_response = client.get("/api/v1/samples/training-exclusions")

    assert sets_response.status_code == 200
    assert records_response.status_code == 200
    assert exclusions_response.status_code == 200
    assert sets_response.json() == [
        {
            "id": "fruitbox-provisioned-v1",
            "label": "Provisioned Fruitbox captures",
            "description": "Reviewed in-repo captures.",
            "source_name": "provisioned",
            "source_kind": "provisioned",
            "purpose": "test",
            "exclude_from_training": True,
            "sample_count": 1,
            "metadata": {},
        },
        {
            "id": "fruitbox-deployment-v1",
            "label": "Deployment captures",
            "description": "Impure deployment captures.",
            "source_name": "environment-1",
            "source_kind": "environment",
            "purpose": "test",
            "exclude_from_training": True,
            "sample_count": 1,
            "metadata": {},
        },
    ]
    assert records_response.json()[0]["image_sha256"] == "sha256-provisioned"
    assert exclusions_response.json() == [
        {
            "sample_set_id": "fruitbox-provisioned-v1",
            "record_id": "capture-0001",
            "source_name": "provisioned",
            "source_kind": "provisioned",
            "image_sha256": "sha256-provisioned",
            "perceptual_hash": "phash-provisioned",
            "board_signature": "board-provisioned",
        },
        {
            "sample_set_id": "fruitbox-deployment-v1",
            "record_id": "capture-env-0001",
            "source_name": "environment-1",
            "source_kind": "environment",
            "image_sha256": "sha256-environment",
            "perceptual_hash": None,
            "board_signature": None,
        },
    ]
