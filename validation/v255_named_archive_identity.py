#!/usr/bin/env python3
"""Read-only, fail-closed identity projection for a named QuickPLS archive case."""

from __future__ import annotations

import argparse
import json
import re
import sys
import zipfile
from pathlib import Path, PurePosixPath
from typing import Any


SAFE_ID = re.compile(r"^[A-Za-z0-9_.:-]+$")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def record(value: Any, label: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{label} must be an object")
    return value


def array(value: Any, label: str) -> list[Any]:
    require(isinstance(value, list), f"{label} must be an array")
    return value


def safe_member(name: str) -> bool:
    member = PurePosixPath(name)
    return (
        bool(name)
        and "\\" not in name
        and not member.is_absolute()
        and all(part not in {"", ".", ".."} for part in member.parts)
    )


def legacy_table_backing(estimation: dict[str, Any], table_id: str) -> tuple[str, int]:
    mediation = estimation.get("mediation")
    moderation = estimation.get("moderation")
    regression = estimation.get("regression")
    cbsem = estimation.get("cbsem")
    process = (
        regression.get("process", {}).get("graph_v2")
        if isinstance(regression, dict) and isinstance(regression.get("process"), dict)
        else None
    )
    mapping: dict[str, tuple[Any, str]] = {
        # The researcher-facing table names are projections over these exact
        # persisted payload collections; never infer their presence from a UI
        # declaration alone.
        "specific_indirect_effects": (mediation, "estimates"),
        "moderation_simple_slopes": (moderation, "estimates"),
        "ols_coefficients": (regression, "coefficients"),
        "logistic_coefficients": (regression, "coefficients"),
        "regression_bootstrap_coefficients": (
            regression.get("bootstrap") if isinstance(regression, dict) else None,
            "coefficients",
        ),
        "process_reference_effects": (process, "reference_effects"),
        "process_simple_slopes": (process, "simple_slopes"),
        "cbsem_fit": (cbsem, "fit"),
        "cbsem_parameters": (cbsem, "parameters"),
        "cbsem_standardized": (cbsem, "standardized_parameters"),
    }
    require(table_id in mapping, f"No fail-closed legacy backing rule exists for {table_id}")
    owner, key = mapping[table_id]
    require(isinstance(owner, dict), f"{table_id} owner payload is absent")
    value = owner.get(key)
    if isinstance(value, list):
        require(len(value) > 0, f"{table_id} backing collection is empty")
        return key, len(value)
    require(value is not None, f"{table_id} backing value is absent")
    return key, 1


def legacy_projection(project: dict[str, Any], result_id: str, table_id: str) -> dict[str, Any]:
    results = array(project.get("results"), "project.results")
    matches = [record(item, "result") for item in results if isinstance(item, dict) and item.get("id") == result_id]
    require(len(matches) == 1, f"Expected exactly one legacy result {result_id}")
    result = matches[0]
    provenance = record(result.get("provenance"), "result.provenance")
    payload = record(result.get("payload"), "result.payload")
    estimation = record(payload.get("estimation"), "result.payload.estimation")
    backing_key, backing_count = legacy_table_backing(estimation, table_id)
    recipe_id = provenance.get("recipe_id")
    recipes = array(project.get("recipes"), "project.recipes")
    recipe = next((record(item, "recipe") for item in recipes if isinstance(item, dict) and item.get("id") == recipe_id), None)
    require(recipe is not None, f"Recipe {recipe_id} is absent")
    recipe_model = recipe.get("model")
    model_id = recipe_model.get("id") if isinstance(recipe_model, dict) else None
    if model_id is None and isinstance(recipe.get("provenance"), dict):
        model_id = recipe["provenance"].get("model_id")
    if model_id is None and isinstance(recipe.get("payload"), dict):
        payload_model = recipe["payload"].get("model")
        model_id = payload_model.get("id") if isinstance(payload_model, dict) else None
    models = array(project.get("models"), "project.models")
    model = next((record(item, "model") for item in models if isinstance(item, dict) and item.get("id") == model_id), None)
    if isinstance(recipe_model, dict):
        legacy_model = recipe_model
    elif model is not None and isinstance(model.get("payload"), dict):
        model_payload = record(model.get("payload"), "model.payload")
        legacy_model = (
            record(model_payload.get("model"), "model.payload.model")
            if isinstance(model_payload.get("model"), dict)
            else model_payload
        )
    else:
        legacy_model = model or {}
    constructs = legacy_model.get("constructs", [])
    paths = legacy_model.get("paths", [])
    interactions = legacy_model.get("interactions", [])
    higher = legacy_model.get("higher_order_constructs", [])
    mediation = estimation.get("mediation") if isinstance(estimation.get("mediation"), dict) else {}
    moderation = estimation.get("moderation") if isinstance(estimation.get("moderation"), dict) else {}
    regression = estimation.get("regression") if isinstance(estimation.get("regression"), dict) else {}
    moderation_estimates = moderation.get("estimates") if isinstance(moderation.get("estimates"), list) else []
    moderation_probe_counts = sorted(
        len(item.get("simple_slopes", []))
        for item in moderation_estimates
        if isinstance(item, dict) and isinstance(item.get("simple_slopes"), list)
    )
    if table_id == "moderation_simple_slopes":
        require(moderation_probe_counts and all(count > 0 for count in moderation_probe_counts), "Moderation evidence has no persisted simple-slope probes")
    mediation_estimates = mediation.get("estimates") if isinstance(mediation.get("estimates"), list) else []
    mediated_effect_count = sum(
        1 for item in mediation_estimates
        if isinstance(item, dict) and isinstance(item.get("indirect"), (int, float)) and abs(item["indirect"]) > 0
    )
    process = regression.get("process") if isinstance(regression.get("process"), dict) else {}
    method_config = recipe.get("method_config") if isinstance(recipe.get("method_config"), dict) else {}
    return {
        "archive_schema": 5,
        "result_id": result_id,
        "status": result.get("status"),
        "method": provenance.get("method"),
        "method_version": provenance.get("method_version"),
        "payload_kind": payload.get("kind"),
        "model_id": model_id,
        "construct_count": len(constructs) if isinstance(constructs, list) else 0,
        "structural_path_count": len(paths) if isinstance(paths, list) else 0,
        "interaction_count": len(interactions) if isinstance(interactions, list) else 0,
        "higher_order_count": len(higher) if isinstance(higher, list) else 0,
        "mediated_effect_count": mediated_effect_count,
        "moderation_probe_counts": moderation_probe_counts,
        "regression_type": regression.get("regression_type"),
        "process_model": process.get("model"),
        "cbsem_model_type": method_config.get("model_type") if method_config.get("kind") == "cbsem" else None,
        "table_id": table_id,
        "table_backing_key": backing_key,
        "table_backing_count": backing_count,
    }


def canonical_projection(project: dict[str, Any], result_id: str, table_id: str) -> dict[str, Any]:
    documents = array(project.get("canonical_result_documents"), "project.canonical_result_documents")
    matches = [
        record(item, "canonical result wrapper")
        for item in documents
        if isinstance(item, dict) and item.get("document_id") == result_id
    ]
    require(len(matches) == 1, f"Expected exactly one canonical result {result_id}")
    wrapper = matches[0]
    document = record(wrapper.get("canonical_document"), "canonical_result_documents[].canonical_document")
    require(document.get("document_id") == result_id, "Canonical wrapper/document ID mismatch")
    provenance = record(document.get("provenance"), "canonical.provenance")
    primary = record(provenance.get("capability_cell"), "canonical.provenance.capability_cell")
    tables = [record(item, "canonical table") for item in array(document.get("tables"), "canonical.tables")]
    table_matches = [item for item in tables if item.get("id") == table_id]
    require(len(table_matches) == 1, f"Expected exactly one canonical table {table_id}")
    table = table_matches[0]
    rows = array(table.get("rows"), f"table {table_id}.rows")
    columns = array(table.get("columns"), f"table {table_id}.columns")
    require(rows and columns, f"Canonical table {table_id} must have rows and columns")
    model_id = provenance.get("model_id")
    models = array(project.get("models"), "project.models")
    model = next((record(item, "model") for item in models if isinstance(item, dict) and (item.get("model_id") == model_id or item.get("id") == model_id)), None)
    model_payload = record(model.get("payload"), "model.payload") if model else {}
    sem_model = record(model_payload.get("model"), "model.payload.model") if isinstance(model_payload.get("model"), dict) else {}
    variables = sem_model.get("variables", [])
    relations = sem_model.get("relations", [])
    derived = sem_model.get("derived_terms", [])
    structural = [item for item in relations if isinstance(item, dict) and item.get("kind") == "structural"] if isinstance(relations, list) else []
    interactions = [item for item in derived if isinstance(item, dict) and item.get("kind") in {"interaction", "interaction_v2"}] if isinstance(derived, list) else []
    higher = [item for item in derived if isinstance(item, dict) and item.get("kind") == "higher_order"] if isinstance(derived, list) else []
    cells = document.get("capability_cells") or [primary]
    require(isinstance(cells, list) and cells, "canonical capability_cells must be non-empty")
    identities = sorted(str(record(cell, "capability cell").get("cell_id")) for cell in cells)
    return {
        "archive_schema": 6,
        "result_id": result_id,
        "status": "completed",
        "method": primary.get("capability_id"),
        "method_version": provenance.get("method_version"),
        "primary_cell_id": primary.get("cell_id"),
        "capability_cell_ids": identities,
        "model_id": model_id,
        "variable_count": len(variables) if isinstance(variables, list) else 0,
        "structural_path_count": len(structural),
        "interaction_count": len(interactions),
        "higher_order_count": len(higher),
        "higher_order_measurement_types": sorted(str(item.get("measurement_type")) for item in higher),
        "table_id": table_id,
        "table_row_count": len(rows),
        "table_column_count": len(columns),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--result-id", required=True)
    parser.add_argument("--table-id", required=True)
    args = parser.parse_args()
    require(args.archive.is_file(), f"Archive is missing: {args.archive}")
    require(SAFE_ID.fullmatch(args.result_id) is not None, "result ID is unsafe")
    require(SAFE_ID.fullmatch(args.table_id) is not None, "table ID is unsafe")
    with zipfile.ZipFile(args.archive, "r") as archive:
        infos = archive.infolist()
        require(all(safe_member(info.filename) for info in infos), "Archive contains an unsafe member path")
        require(sum(info.filename == "project.json" for info in infos) == 1, "Archive must contain exactly one project.json")
        project = record(json.loads(archive.read("project.json")), "project")
    schema = project.get("schema_version")
    projection = canonical_projection(project, args.result_id, args.table_id) if schema == 6 else legacy_projection(project, args.result_id, args.table_id)
    print(json.dumps({"schema_version": 1, "suite_id": "quickpls_v255_named_archive_identity_v1", "passed": True, "identity": projection}, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:  # noqa: BLE001 - CLI must fail closed with one diagnostic.
        print(json.dumps({"schema_version": 1, "suite_id": "quickpls_v255_named_archive_identity_v1", "passed": False, "error": str(error)}, separators=(",", ":")), file=sys.stderr)
        raise SystemExit(1)
