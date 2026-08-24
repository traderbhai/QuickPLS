#!/usr/bin/env python3
"""Fail-closed verifier for the MultiMod semantic export qualification matrix."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


FORMATS = {"csv", "xlsx", "json", "html", "pdf", "svg", "png"}
FAMILY_LABELS = {
    "qpls.multimod.mga_multigroup_v1": "mga",
    "qpls.multimod.pls_heterogeneity_v2": "heterogeneity",
    "qpls.multimod.general_sem_conditional_process_v2": "conditional",
    "qpls.multimod.interventional_causal_mediation_v1": "causal",
}


def load_object(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain one JSON object")
    return value


def manifest_cells(
    root: Path, index: dict[str, Any]
) -> tuple[set[str], set[str], dict[str, str]]:
    profiles: set[str] = set()
    cells: set[str] = set()
    profile_families: dict[str, str] = {}
    for family in index.get("families", []):
        if not isinstance(family, dict):
            raise ValueError("capability family entries must be objects")
        family_id = str(family["family_id"])
        manifest = load_object(root / str(family["manifest"]))
        for profile in manifest.get("profile_matrix", []):
            profile_id = str(profile["profile_id"])
            if profile_id in profile_families:
                raise ValueError(f"profile appears in multiple families: {profile_id}")
            profiles.add(profile_id)
            profile_families[profile_id] = family_id
            for procedure in profile.get("procedure_cells", []):
                cells.add(str(procedure["evidence_identity_template"]))
    return profiles, cells, profile_families


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--capability-index", type=Path, required=True)
    parser.add_argument("--repository-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    root = args.repository_root.resolve()
    report = load_object(args.report)
    index = load_object(args.capability_index)
    expected_profiles, declared_cells, profile_families = manifest_cells(root, index)
    failures: list[str] = []
    if report.get("schema_version") != 1:
        failures.append("report.schema_version")
    if report.get("report_id") != "qpls.multimod.semantic-export-qualification.v1":
        failures.append("report.report_id")
    if report.get("status") != "passed":
        failures.append("report.status")
    rows = report.get("rows")
    if not isinstance(rows, list):
        failures.append("report.rows")
        rows = []

    observed: dict[str, set[str]] = {}
    for index_value, row in enumerate(rows):
        if not isinstance(row, dict):
            failures.append(f"rows[{index_value}].object")
            continue
        profile = str(row.get("profile", ""))
        format_id = str(row.get("format", ""))
        cell = str(row.get("required_cell", ""))
        if profile not in expected_profiles:
            failures.append(f"rows[{index_value}].profile")
        if cell not in declared_cells or not cell.startswith(f"{profile}::"):
            failures.append(f"rows[{index_value}].required_cell")
        if row.get("family_capability") != profile_families.get(profile):
            failures.append(f"rows[{index_value}].family_capability")
        if row.get("family") != FAMILY_LABELS.get(profile_families.get(profile, "")):
            failures.append(f"rows[{index_value}].family")
        if row.get("publication_qualification") != "release_qualified_candidate":
            failures.append(f"rows[{index_value}].publication_qualification")
        if format_id not in FORMATS:
            failures.append(f"rows[{index_value}].format")
        if any(row.get(flag) is not True for flag in (
            "exact_semantic_match",
            "digest_match",
            "rendered_surface_match",
            "receipt_bound",
            "suppression_preserved",
        )):
            failures.append(f"rows[{index_value}].semantic_guards")
        digest = row.get("semantic_sha256")
        if not isinstance(digest, str) or len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
            failures.append(f"rows[{index_value}].semantic_sha256")
        observed.setdefault(profile, set()).add(format_id)

    if set(observed) != expected_profiles:
        failures.append("profile_inventory")
    for profile in expected_profiles:
        if observed.get(profile) != FORMATS:
            failures.append(f"format_inventory:{profile}")
    expected_rows = len(expected_profiles) * len(FORMATS)
    if len(rows) != expected_rows or report.get("profile_count") != len(expected_profiles):
        failures.append("matrix_dimensions")
    if set(report.get("formats", [])) != FORMATS:
        failures.append("report.formats")

    result = {
        "schema_version": 1,
        "verification_id": "qpls.multimod.semantic-export-readback-verifier.v1",
        "status": "passed" if not failures else "failed",
        "profile_count": len(expected_profiles),
        "format_count": len(FORMATS),
        "matrix_row_count": len(rows),
        "required_matrix_row_count": expected_rows,
        "failures": sorted(set(failures)),
        "source_report": str(args.report.resolve()),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if not failures else 1


if __name__ == "__main__":
    raise SystemExit(main())
