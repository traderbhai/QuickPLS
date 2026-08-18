#!/usr/bin/env python3
"""Strict project/archive persistence gate for PLS power v1."""

from __future__ import annotations

import hashlib
import json
import zipfile
from copy import deepcopy
from pathlib import Path
from typing import Any

from pls_sample_size_power_simulation import (
    CLI,
    EXECUTION_SOURCES,
    METHOD_VERSION,
    WORK_ROOT,
    canonical_recipe,
    ensure_fixture_dataset,
    repository_path,
    require_current_cli,
    require_stable_execution_sources,
    run_command,
    run_product_power,
    sha256_file,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/pls_sample_size_power_persistence_gate.py"
PROJECT_SOURCE = "crates/qpls-project/src/lib.rs"
RESAMPLING_SOURCE = "crates/qpls-resampling/src/power.rs"

ARCHIVE_CLAIM_BOUNDARY = {
    "verified": (
        "Archive validation proves structural, accounting, identity, and "
        "deterministic-summary integrity from the stored replicate ledger and rejects "
        "isolated mutations."
    ),
    "not_verified": (
        "It does not replay generated data, PLS estimates, or bootstrap fits and does "
        "not authenticate a coordinated rewrite of target estimates, p values, "
        "rejection flags, outcome digest, derived power rows, grid decision, and outer "
        "archive checksum."
    ),
    "semantic_replay_performed": False,
    "coordinated_rewrite_authenticated": False,
    "qualification_use": (
        "This structural persistence result may support archive/native qualification "
        "only when composed with separate identity-bound scientific and native "
        "evidence; it is not semantic replay or tamper-proof provenance."
    ),
}


def _strict_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n"
    ).encode("utf-8")


def _archive_entries(path: Path) -> dict[str, bytes]:
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
        if len(names) != len(set(names)):
            raise ValueError("source project contains duplicate ZIP entries")
        return {name: archive.read(name) for name in names}


def write_project_archive(
    base: Path,
    output: Path,
    *,
    project_document: dict[str, Any] | None = None,
    raw_project: bytes | None = None,
    recompute_checksums: bool = True,
) -> None:
    """Rewrite a method-scoped archive while preserving its dataset bytes."""

    if (project_document is None) == (raw_project is None):
        raise ValueError("provide exactly one project representation")
    entries = _archive_entries(base)
    manifest = json.loads(entries["manifest.json"])
    entries["project.json"] = (
        _strict_bytes(project_document) if project_document is not None else raw_project
    )
    if recompute_checksums:
        manifest["checksums"] = {
            name: hashlib.sha256(content).hexdigest()
            for name, content in sorted(entries.items())
            if name != "manifest.json"
        }
    entries["manifest.json"] = _strict_bytes(manifest)
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        output.unlink()
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, content in sorted(entries.items(), key=lambda row: row[0] == "manifest.json"):
            archive.writestr(name, content)


def inspect_archive(path: Path, *, expect_success: bool) -> tuple[dict[str, Any], dict[str, Any]]:
    completed, execution = run_command([str(CLI), "inspect", str(path), "--json"], timeout=180)
    if expect_success:
        summary = json.loads(completed.stdout) if completed.returncode == 0 else None
        passed = (
            completed.returncode == 0
            and isinstance(summary, dict)
            and summary.get("recipes") == 1
            and summary.get("results") == 1
            and summary.get("recovered") is False
        )
        return {"passed": passed, "summary": summary}, execution
    combined = (completed.stdout + "\n" + completed.stderr).lower()
    return {
        "passed": completed.returncode != 0,
        "nonzero_exit": completed.returncode != 0,
        "error_tail": combined[-1_500:],
    }, execution


def _mutated_archive(
    *,
    base: Path,
    document: dict[str, Any],
    name: str,
) -> tuple[dict[str, Any], dict[str, Any]]:
    output = WORK_ROOT / f"persistence_{name}.qpls"
    write_project_archive(base, output, project_document=document)
    check, execution = inspect_archive(output, expect_success=False)
    check.update(
        {
            "archive": repository_path(output),
            "archive_sha256": sha256_file(output),
        }
    )
    return check, execution


def main() -> int:
    cli_identity = require_current_cli()
    before = cli_identity["execution_sources"]
    dataset, fingerprint, fixture_execution = ensure_fixture_dataset()
    base_archive = WORK_ROOT / "prospective_power_envelope.qpls"
    recipe = canonical_recipe(
        fingerprint,
        name="persistence_roundtrip",
        population_path=0.45,
        sample_size_grid=(60, 120),
        monte_carlo_replicates=100,
        bootstrap_replicates=99,
        target_power=0.70,
        seed=20_260_820,
        workers=2,
    )
    run = run_product_power(name="persistence_roundtrip", recipe=recipe, dataset=dataset)
    base_entries = _archive_entries(base_archive)
    project_document = json.loads(base_entries["project.json"])
    project_document["recipes"] = [recipe]
    project_document["results"] = [run["document"]]
    valid_archive = WORK_ROOT / "persistence_valid.qpls"
    write_project_archive(base_archive, valid_archive, project_document=project_document)
    valid_first, valid_first_execution = inspect_archive(valid_archive, expect_success=True)
    first_hash = sha256_file(valid_archive)
    valid_second, valid_second_execution = inspect_archive(valid_archive, expect_success=True)
    valid_roundtrip = {
        "passed": valid_first["passed"]
        and valid_second["passed"]
        and first_hash == sha256_file(valid_archive)
        and valid_first["summary"] == valid_second["summary"],
        "first": valid_first,
        "second": valid_second,
        "archive": repository_path(valid_archive),
        "archive_sha256": first_hash,
        "byte_stable_across_reopen": first_hash == sha256_file(valid_archive),
    }

    executions: list[dict[str, Any]] = [
        fixture_execution,
        run["execution"],
        valid_first_execution,
        valid_second_execution,
    ]
    mutations: dict[str, dict[str, Any]] = {}

    feature = deepcopy(project_document)
    feature["results"][0]["payload"]["analysis"]["capability_id"] = "qpls3.pls.power_forgery"
    mutations["feature_identity"], execution = _mutated_archive(
        base=base_archive, document=feature, name="feature_identity"
    )
    executions.append(execution)

    version = deepcopy(project_document)
    version["results"][0]["payload"]["analysis"]["method_version"] = (
        "pls_sample_size_power_v2"
    )
    mutations["method_version"], execution = _mutated_archive(
        base=base_archive, document=version, name="method_version"
    )
    executions.append(execution)

    fingerprint_tamper = deepcopy(project_document)
    fingerprint_tamper["results"][0]["provenance"]["dataset_fingerprint"] = (
        "v2:" + "0" * 64
    )
    mutations["dataset_fingerprint"], execution = _mutated_archive(
        base=base_archive, document=fingerprint_tamper, name="dataset_fingerprint"
    )
    executions.append(execution)

    decision = deepcopy(project_document)
    decision["results"][0]["payload"]["analysis"]["decision"] = {
        "status": "reached",
        "sample_size": 60,
    }
    mutations["decision_recomputation"], execution = _mutated_archive(
        base=base_archive, document=decision, name="decision_recomputation"
    )
    executions.append(execution)

    malformed = deepcopy(project_document)
    malformed["results"][0]["payload"]["analysis"]["undeclared_field"] = True
    mutations["malformed_payload"], execution = _mutated_archive(
        base=base_archive, document=malformed, name="malformed_payload"
    )
    executions.append(execution)

    duplicate_raw = _strict_bytes(project_document)
    closing = duplicate_raw.rfind(b"}")
    duplicate_raw = duplicate_raw[:closing] + b',\n  "results": []\n' + duplicate_raw[closing:]
    duplicate_archive = WORK_ROOT / "persistence_duplicate_key.qpls"
    write_project_archive(base_archive, duplicate_archive, raw_project=duplicate_raw)
    duplicate_check, duplicate_execution = inspect_archive(
        duplicate_archive, expect_success=False
    )
    duplicate_check.update(
        {
            "archive": repository_path(duplicate_archive),
            "archive_sha256": sha256_file(duplicate_archive),
        }
    )
    mutations["duplicate_json_key"] = duplicate_check
    executions.append(duplicate_execution)

    checksum_archive = WORK_ROOT / "persistence_checksum_mismatch.qpls"
    checksum_raw = _strict_bytes(project_document).replace(
        METHOD_VERSION.encode("ascii"), b"pls_sample_size_power_v9", 1
    )
    write_project_archive(
        base_archive,
        checksum_archive,
        raw_project=checksum_raw,
        recompute_checksums=False,
    )
    checksum_check, checksum_execution = inspect_archive(
        checksum_archive, expect_success=False
    )
    checksum_check.update(
        {
            "archive": repository_path(checksum_archive),
            "archive_sha256": sha256_file(checksum_archive),
        }
    )
    mutations["checksum"] = checksum_check
    executions.append(checksum_execution)

    # Historical legacy payloads remain readable, but the current CLI refuses
    # to reinterpret or export them as typed prospective power output.
    legacy = deepcopy(project_document)
    legacy_result = legacy["results"][0]
    legacy_result["payload"] = {
        "kind": "legacy",
        "value": {
            "label": "historical heuristic preview",
            "claimed_method_version": METHOD_VERSION,
        },
    }
    legacy_archive = WORK_ROOT / "persistence_legacy_read_only.qpls"
    write_project_archive(base_archive, legacy_archive, project_document=legacy)
    legacy_load, legacy_load_execution = inspect_archive(legacy_archive, expect_success=True)
    legacy_result_path = WORK_ROOT / "persistence_legacy_result.json"
    legacy_result_path.write_text(
        json.dumps(legacy_result, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    legacy_export = WORK_ROOT / "persistence_legacy_export.must_not_exist.csv"
    if legacy_export.exists():
        legacy_export.unlink()
    legacy_completed, legacy_export_execution = run_command(
        [
            str(CLI),
            "export",
            str(legacy_result_path),
            "--format",
            "csv",
            "--output",
            str(legacy_export),
        ],
        timeout=120,
    )
    legacy_policy = {
        "passed": legacy_load["passed"]
        and legacy_completed.returncode != 0
        and not legacy_export.exists(),
        "historical_archive_readable": legacy_load["passed"],
        "legacy_export_rejected": legacy_completed.returncode != 0,
        "no_typed_export_created": not legacy_export.exists(),
    }
    executions.extend([legacy_load_execution, legacy_export_execution])

    rust_tests, rust_test_execution = run_command(
        [
            "cargo",
            "test",
            "-p",
            "qpls-resampling",
            "strict_archive",
            "--",
            "--nocapture",
        ],
        timeout=900,
    )
    executions.append(rust_test_execution)
    project_source = (WORK_ROOT.parents[3] / PROJECT_SOURCE).read_text(encoding="utf-8")
    project_contract = all(
        token in project_source
        for token in (
            "validate_pls_sample_size_power_contract",
            "validate_pls_sample_size_power_result",
            "verify_archive_checksums",
            "reject_duplicate_json_object_keys",
        )
    )
    source_stability = require_stable_execution_sources(before)
    claim_boundary_explicit = (
        ARCHIVE_CLAIM_BOUNDARY["semantic_replay_performed"] is False
        and ARCHIVE_CLAIM_BOUNDARY["coordinated_rewrite_authenticated"] is False
        and "stored replicate ledger" in ARCHIVE_CLAIM_BOUNDARY["verified"]
        and "coordinated rewrite" in ARCHIVE_CLAIM_BOUNDARY["not_verified"]
    )
    checks = {
        "valid_project_reopens_same_run": valid_roundtrip["passed"],
        "all_isolated_tamper_mutations_rejected": all(
            row["passed"] for row in mutations.values()
        ),
        "archive_claim_boundary_is_structural_not_semantic": claim_boundary_explicit,
        "legacy_never_reinterpreted_as_typed_power": legacy_policy["passed"],
        "typed_archive_unit_tests_pass": rust_tests.returncode == 0,
        "project_strict_validator_present_and_executed": project_contract,
        "source_stable_during_gate": source_stability["passed"],
    }
    passed = all(checks.values())
    report = {
        "passed": passed,
        "checks": checks,
        "valid_roundtrip": valid_roundtrip,
        "tamper_mutations": mutations,
        "legacy_policy": legacy_policy,
        "source_stability": source_stability,
        "archive_claim_boundary": ARCHIVE_CLAIM_BOUNDARY,
        "archive_scope": (
            "This gate rebuilds real schema-v5 project ZIPs with their Arrow dataset, exact "
            "schema-v3 recipe, typed result, and recomputed manifest checksums, then reopens "
            "them through the production CLI loader. It checks structural consistency from "
            "the stored ledger and isolated tamper rejection; it does not regenerate datasets "
            "or replay PLS/bootstrap fits."
        ),
    }
    path = write_identity_report(
        "persistence_report",
        passed=passed,
        checks=report,
        execution=executions,
        extras=[SOURCE, PROJECT_SOURCE, RESAMPLING_SOURCE, *EXECUTION_SOURCES],
    )
    print(json.dumps({"passed": passed, "output": str(path)}, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
