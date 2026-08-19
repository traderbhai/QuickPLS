#!/usr/bin/env python3
"""Run the method-scoped Rank 0 General SEM performance qualification matrix.

This is an orchestration layer, not a second measurement implementation.  Each
of the four frozen Rank 0 capability cells is exercised against every
scientifically applicable case across the five contract profiles.  Point cells
omit the bootstrap-only maximum-resamples axis; bootstrap cells include it.
and delegated to
``complexity_performance_measure.py``.  That existing harness remains the
authority for process-tree measurement, one warm-up plus five measured runs,
progress capture, receipt shape, and non-overwrite behavior.

The bundled, source-frozen driver is the only accepted qualification adapter
to an actual packaged QuickPLS build.  Custom drivers are rejected because an
accepted receipt cannot be allowed to substitute unfrozen execution semantics.

* ``prepare`` authors and saves the exact packaged workload outside the timed
  operation boundary.
* ``observe`` writes cancellation evidence for every potentially-long case and
  a ten-accepted-run memory-soak observation for the applied case.
* ``measure`` executes one real workload, reports monotonic progress through
  ``QPLS_PERFORMANCE_PROGRESS_PATH``, and writes the result named by
``QPLS_PERFORMANCE_RESULT_PATH``.

Both subcommands receive ``--quickpls-executable``.  The orchestrator rejects
the run unless ``--build-fingerprint`` is the SHA-256 of those exact package
bytes, so a receipt cannot float free of the executable the adapter must run.

Accepted-baseline and current runs use identical workload fingerprints.  A
current run is rejected before measurement unless every exact accepted receipt
exists, validates, and is comparable.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Mapping, Sequence

try:
    from validation.capability_registry_v2 import canonical_sha256, load_json
    from validation.complexity_performance_v2 import (
        DEFAULT_MANIFEST_PATH,
        DEFAULT_MEASUREMENT_SCHEMA_PATH,
        DEFAULT_REGISTRY_PATH,
        DEFAULT_SCHEMA_PATH,
        _baseline_comparability_errors,
        _measurement_key,
        _receipt_common_errors,
        _regression_errors,
        _validate_runs,
        load_contract,
        qualification_link_identity,
        validate_contract_documents,
    )
    from validation.general_sem_rank0_packaged_acceptance import (
        DEFAULT_CONTRACT as DEFAULT_RANK0_CONTRACT,
        ContractError,
        qualification_contract_authorities,
        validate_contract as validate_rank0_contract,
    )
    from validation.general_sem_rank0_receipt_payload_v1 import (
        qualification_contract_sha256,
        unified_rank0_source_receipt,
        validate_unified_rank0_source_receipt,
    )
except ModuleNotFoundError:  # Direct ``python validation/...py`` execution.
    from capability_registry_v2 import canonical_sha256, load_json
    from complexity_performance_v2 import (
        DEFAULT_MANIFEST_PATH,
        DEFAULT_MEASUREMENT_SCHEMA_PATH,
        DEFAULT_REGISTRY_PATH,
        DEFAULT_SCHEMA_PATH,
        _baseline_comparability_errors,
        _measurement_key,
        _receipt_common_errors,
        _regression_errors,
        _validate_runs,
        load_contract,
        qualification_link_identity,
        validate_contract_documents,
    )
    from general_sem_rank0_packaged_acceptance import (
        DEFAULT_CONTRACT as DEFAULT_RANK0_CONTRACT,
        ContractError,
        qualification_contract_authorities,
        validate_contract as validate_rank0_contract,
    )
    from general_sem_rank0_receipt_payload_v1 import (
        qualification_contract_sha256,
        unified_rank0_source_receipt,
        validate_unified_rank0_source_receipt,
    )


ROOT = Path(__file__).resolve().parents[1]
MEASUREMENT_HARNESS = ROOT / "validation/complexity_performance_measure.py"
BUNDLED_DRIVER = ROOT / "validation/general_sem_rank0_performance_driver.mjs"
PERFORMANCE_RESULTS_ROOT = ROOT / "validation/results"
HARDWARE_PROFILE_ID = "standard_windows_6c16g"
PROFILE_CASES = (
    ("micro_exact", "micro_exact"),
    ("applied", "applied"),
    ("large", "large"),
    ("maximum_axis", "maximum_rows_100000"),
    ("maximum_axis", "maximum_indicators_300"),
    ("maximum_axis", "maximum_constructs_100"),
    ("maximum_axis", "maximum_resamples_10000"),
    ("compound_stress", "compound_stress"),
)
QUALIFICATION_SPEC_PATHS = {
    "mediation_point": ROOT
    / "validation/qualification_v2/mediation_v1.qualification.json",
    "multiple_mediation_bootstrap": ROOT
    / "validation/qualification_v2/general_sem_pls_multiple_mediation_bootstrap_v1.qualification.json",
    "multiple_two_way_moderation_point": ROOT
    / "validation/qualification_v2/general_sem_pls_multiple_moderation_point_v1.qualification.json",
    "multiple_two_way_moderation_bootstrap": ROOT
    / "validation/qualification_v2/general_sem_pls_multiple_moderation_bootstrap_v1.qualification.json",
}
MANDATORY_COMBINATION_IDS = {
    "micro_exact": "micro_exact_contract",
    "applied": "applied_pairwise_matrix",
    "maximum_rows_100000": "maximum_rows",
    "maximum_indicators_300": "maximum_indicators",
    "maximum_constructs_100": "maximum_constructs",
    "maximum_resamples_10000": "maximum_resamples",
    "compound_stress": "compound_reliability",
}
WORKLOAD_FIELDS = {
    "rows",
    "indicators",
    "constructs",
    "resamples",
    "groups",
    "candidate_models",
}
SHA256 = "0123456789abcdef"


def _require_sha256(value: str, subject: str) -> None:
    if len(value) != 64 or any(character not in SHA256 for character in value):
        raise ContractError(f"{subject} must be a lowercase SHA-256")


def _write_new_json(path: Path, value: Mapping[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = (
        json.dumps(
            value, indent=2, sort_keys=False, ensure_ascii=False, allow_nan=False
        )
        + "\n"
    )
    with path.open("x", encoding="utf-8", newline="\n") as handle:
        handle.write(payload)
        handle.flush()
        os.fsync(handle.fileno())


def _descriptor(path: Path, root: Path) -> dict[str, Any]:
    resolved = path.resolve()
    try:
        relative = resolved.relative_to(root.resolve()).as_posix()
    except ValueError as error:
        raise ContractError(
            f"performance artifact leaves output root: {resolved}"
        ) from error
    if not resolved.is_file() or resolved.is_symlink() or resolved.stat().st_size <= 0:
        raise ContractError(f"performance artifact is missing or empty: {resolved}")
    return {
        "path": relative,
        "size": resolved.stat().st_size,
        "sha256": hashlib.sha256(resolved.read_bytes()).hexdigest(),
    }


def _reference_object(reference: Sequence[Any]) -> dict[str, Any]:
    return {
        "registry_schema_version": reference[0],
        "capability_id": reference[1],
        "cell_id": reference[2],
        "capability_version": reference[3],
    }


def _profile_case(
    context: Mapping[str, Any], profile_id: str, case_id: str
) -> Mapping[str, Any]:
    profile = context["profiles"].get(profile_id)
    if not isinstance(profile, Mapping):
        raise ContractError(f"performance profile is unavailable: {profile_id}")
    rows = [
        row
        for row in profile.get("cases", [])
        if isinstance(row, Mapping) and row.get("case_id") == case_id
    ]
    if len(rows) != 1:
        raise ContractError(f"performance case is not exact: {profile_id}/{case_id}")
    return rows[0]


def _qualification_spec_descriptor(
    path: Path, spec: Mapping[str, Any]
) -> dict[str, Any]:
    identity = spec.get("identity")
    if not isinstance(identity, Mapping):
        raise ContractError(f"QualificationSpec identity is missing: {path}")
    relative = path.resolve().relative_to(ROOT.resolve()).as_posix()
    return {
        "path": relative,
        "qualification_id": identity.get("qualification_id"),
        "spec_frozen_at_utc": identity.get("spec_frozen_at_utc"),
        "qualification_contract_sha256": qualification_contract_sha256(spec),
    }


def workload_fingerprint_authority(value: Mapping[str, Any]) -> dict[str, Any]:
    """Exclude mutable/raw spec bytes while binding its normalized contract."""

    authority = {
        key: item
        for key, item in value.items()
        if key
        not in {
            "workload_fingerprint",
            "potentially_long",
            "repeat_memory_gate",
        }
    }
    qualification = authority.get("qualification_spec")
    if not isinstance(qualification, Mapping):
        raise ContractError(
            "performance workload QualificationSpec authority is missing"
        )
    authority["qualification_spec"] = dict(qualification)
    return authority


def _exact_qualification_workload(
    *,
    variant: Mapping[str, Any],
    profile_id: str,
    case_id: str,
    performance_case: Mapping[str, Any],
) -> tuple[dict[str, Any], dict[str, Any], str]:
    variant_id = str(variant["variant_id"])
    path = QUALIFICATION_SPEC_PATHS.get(variant_id)
    if path is None or not path.is_file():
        raise ContractError(f"QualificationSpec is unavailable for {variant_id}")
    spec = load_json(path)
    if not isinstance(spec, Mapping) or spec.get("schema_version") != 2:
        raise ContractError(f"QualificationSpec schema is invalid for {variant_id}")
    identity = spec.get("identity")
    reference = _reference_object(variant["reference"])
    if (
        not isinstance(identity, Mapping)
        or identity.get("capability_cell") != reference
        or identity.get("spec_frozen_at_utc") != "2026-08-19T00:00:00Z"
    ):
        raise ContractError(
            f"QualificationSpec identity differs from the exact Rank 0 cell: {variant_id}"
        )
    scenario = spec.get("scenario_contract")
    profiles = (
        scenario.get("complexity_profiles") if isinstance(scenario, Mapping) else None
    )
    combinations = (
        scenario.get("mandatory_combinations")
        if isinstance(scenario, Mapping)
        else None
    )
    if not isinstance(profiles, list) or not isinstance(combinations, list):
        raise ContractError(f"QualificationSpec scenarios are missing for {variant_id}")
    profile_rows = [
        row
        for row in profiles
        if isinstance(row, Mapping) and row.get("id") == profile_id
    ]
    if len(profile_rows) != 1 or profile_rows[0].get("applicability") != "required":
        raise ContractError(
            f"QualificationSpec profile is not exactly required: {variant_id}/{profile_id}"
        )
    profile_workload = profile_rows[0].get("workload")
    if (
        not isinstance(profile_workload, Mapping)
        or set(profile_workload) != WORKLOAD_FIELDS
    ):
        raise ContractError(
            f"QualificationSpec workload fields are not exact: {variant_id}/{profile_id}"
        )
    workload = dict(profile_workload)
    if profile_id == "maximum_axis":
        controlled = performance_case.get("workload")
        if not isinstance(controlled, Mapping) or set(controlled) != WORKLOAD_FIELDS:
            raise ContractError(
                f"maximum-axis control workload is invalid: {variant_id}/{case_id}"
            )
        workload = {
            "rows": controlled["rows"],
            "indicators": controlled["indicators"],
            "constructs": controlled["constructs"],
            "resamples": profile_workload["resamples"],
            "groups": profile_workload["groups"],
            "candidate_models": profile_workload["candidate_models"],
        }
        stressed = {
            "maximum_rows_100000": "rows",
            "maximum_indicators_300": "indicators",
            "maximum_constructs_100": "constructs",
            "maximum_resamples_10000": "resamples",
        }.get(case_id)
        if stressed is None or workload[stressed] != profile_workload[stressed]:
            raise ContractError(
                f"maximum-axis workload does not reach its QualificationSpec limit: {variant_id}/{case_id}"
            )
    expected_resamples = (
        0
        if not variant["bootstrap"]
        else 199
        if profile_id == "micro_exact"
        else 5_000
        if profile_id in {"applied", "large"}
        else 10_000
    )
    if (
        workload.get("groups") != 1
        or workload.get("candidate_models") != 1
        or workload.get("resamples") != expected_resamples
        or any(
            not isinstance(workload.get(field), int)
            or isinstance(workload.get(field), bool)
            or workload[field] < (0 if field == "resamples" else 1)
            for field in WORKLOAD_FIELDS
        )
    ):
        raise ContractError(
            f"QualificationSpec workload is not the frozen single-group/single-candidate cell: {variant_id}/{case_id}"
        )
    combination_id = (
        "large_worker_replay"
        if case_id == "large" and variant["bootstrap"]
        else "large_point_replay"
        if case_id == "large"
        else MANDATORY_COMBINATION_IDS[case_id]
    )
    combination_rows = [
        row
        for row in combinations
        if isinstance(row, Mapping) and row.get("id") == combination_id
    ]
    if (
        len(combination_rows) != 1
        or combination_rows[0].get("profile_id") != profile_id
    ):
        raise ContractError(
            f"QualificationSpec mandatory combination is not exact: {variant_id}/{combination_id}"
        )
    return workload, _qualification_spec_descriptor(path, spec), combination_id


def build_plan(
    *,
    manifest_path: Path = DEFAULT_MANIFEST_PATH,
    registry_path: Path = DEFAULT_REGISTRY_PATH,
    schema_path: Path = DEFAULT_SCHEMA_PATH,
    measurement_schema_path: Path = DEFAULT_MEASUREMENT_SCHEMA_PATH,
    rank0_contract_path: Path = DEFAULT_RANK0_CONTRACT,
) -> tuple[Mapping[str, Any], Mapping[str, Any], list[dict[str, Any]]]:
    manifest, registry, schema, measurement_schema = load_contract(
        manifest_path, registry_path, schema_path, measurement_schema_path
    )
    context = validate_contract_documents(
        manifest, registry, schema, measurement_schema
    )
    if not context["contract_valid"]:
        raise ContractError(
            "complexity-performance contract is invalid: "
            + "; ".join(context["errors"])
        )
    rank0 = validate_rank0_contract(load_json(rank0_contract_path), registry)
    if tuple(PROFILE_CASES) != (
        ("micro_exact", "micro_exact"),
        ("applied", "applied"),
        ("large", "large"),
        ("maximum_axis", "maximum_rows_100000"),
        ("maximum_axis", "maximum_indicators_300"),
        ("maximum_axis", "maximum_constructs_100"),
        ("maximum_axis", "maximum_resamples_10000"),
        ("compound_stress", "compound_stress"),
    ):
        raise AssertionError("Rank 0 performance profile selection drifted")
    rows: list[dict[str, Any]] = []
    for variant in rank0["variants"]:
        reference = variant["reference"]
        if reference not in context["resolved_classes"]:
            raise ContractError(
                f"Rank 0 cell is not in the active performance contract: {reference!r}"
            )
        reference_object = _reference_object(reference)
        for profile_id, case_id in PROFILE_CASES:
            if case_id == "maximum_resamples_10000" and not variant["bootstrap"]:
                continue
            case = _profile_case(context, profile_id, case_id)
            workload, qualification_spec, combination_id = (
                _exact_qualification_workload(
                    variant=variant,
                    profile_id=profile_id,
                    case_id=case_id,
                    performance_case=case,
                )
            )
            workload_authority = {
                "schema_version": 1,
                "workload_kind": "general_sem_rank0_performance_workload",
                "variant_id": variant["variant_id"],
                "capability_reference": reference_object,
                "qualification_spec": qualification_spec,
                "profile_id": profile_id,
                "case_id": case_id,
                "mandatory_combination_id": combination_id,
                "workload": workload,
            }
            workload_fingerprint = canonical_sha256(
                workload_fingerprint_authority(workload_authority)
            )
            rows.append(
                {
                    **workload_authority,
                    "workload_fingerprint": workload_fingerprint,
                    "potentially_long": context["profiles"][profile_id][
                        "potentially_long"
                    ],
                    "repeat_memory_gate": context["profiles"][profile_id][
                        "repeat_memory_gate"
                    ],
                }
            )
    if (
        len(rows) != 30
        or len({(row["variant_id"], row["profile_id"], row["case_id"]) for row in rows})
        != 30
    ):
        raise ContractError("Rank 0 performance plan must contain 30 applicable cells")
    max_and_compound = [
        row for row in rows if row["profile_id"] in {"maximum_axis", "compound_stress"}
    ]
    if len(max_and_compound) != 18:
        raise ContractError(
            "Rank 0 performance plan must contain 18 exact max/compound cells"
        )
    return manifest, context, rows


def validate_cell_performance_index(
    index: Any,
    capability_reference: Mapping[str, Any],
    *,
    repository_root: Path = ROOT,
) -> dict[str, Any]:
    """Validate one cell's complete performance ledger without cross-cell gating."""

    expected_fields = {
        "schema_version",
        "evidence_kind",
        "measurement_role",
        "contract_id",
        "contract_sha256",
        "rank0_contract_sha256",
        "hardware_profile_id",
        "hardware_fingerprint",
        "build_fingerprint",
        "source_receipt",
        "qualification_contracts",
        "generated_at_utc",
        "case_count",
        "cases",
        "cell_pass_ledger",
        "passed",
    }
    if not isinstance(index, Mapping) or set(index) != expected_fields:
        raise ContractError("performance index fields are not exact")
    if (
        index.get("schema_version") != 1
        or index.get("evidence_kind") != "general_sem_rank0_method_performance_index"
        or index.get("measurement_role") not in {"accepted_baseline", "current"}
        or index.get("hardware_profile_id") != HARDWARE_PROFILE_ID
        or not isinstance(index.get("generated_at_utc"), str)
        or not index["generated_at_utc"]
    ):
        raise ContractError("performance index root authority is invalid")
    build_fingerprint = index.get("build_fingerprint")
    if not isinstance(build_fingerprint, str):
        raise ContractError("performance build fingerprint is unavailable")
    _require_sha256(build_fingerprint, "performance build_fingerprint")
    try:
        source_receipt = validate_unified_rank0_source_receipt(
            index.get("source_receipt"),
            repository_root,
            subject="performance index source_receipt",
        )
    except ValueError as error:
        raise ContractError(str(error)) from error

    manifest, context, plan_rows = build_plan()
    rank0_contract = load_json(DEFAULT_RANK0_CONTRACT)
    rank0_context = validate_rank0_contract(
        rank0_contract, load_json(DEFAULT_REGISTRY_PATH)
    )
    if (
        index.get("contract_id") != manifest["contract_id"]
        or index.get("contract_sha256") != context["contract_sha256"]
        or index.get("rank0_contract_sha256") != canonical_sha256(rank0_contract)
    ):
        raise ContractError("performance index does not bind frozen contracts")
    reference_fields = {
        "registry_schema_version",
        "capability_id",
        "cell_id",
        "capability_version",
    }
    if (
        not isinstance(capability_reference, Mapping)
        or set(capability_reference) != reference_fields
    ):
        raise ContractError("performance capability reference fields are not exact")
    target_reference = dict(capability_reference)
    target_plan = [
        row for row in plan_rows if row["capability_reference"] == target_reference
    ]
    if not target_plan:
        raise ContractError("performance capability reference is not a Rank 0 cell")

    expected_contracts = qualification_contract_authorities(
        rank0_context, repository_root
    )
    target_contracts = [
        row
        for row in index.get("qualification_contracts", [])
        if isinstance(row, Mapping)
        and row.get("capability_reference") == target_reference
    ]
    expected_target_contracts = [
        row
        for row in expected_contracts
        if row["capability_reference"] == target_reference
    ]
    if target_contracts != expected_target_contracts or len(target_contracts) != 1:
        raise ContractError(
            "performance target normalized qualification contract is not exact"
        )

    hardware = index.get("hardware_fingerprint")
    if not isinstance(hardware, Mapping) or set(hardware) != {
        "os",
        "architecture",
        "cpu",
        "physical_cores",
        "logical_cores",
        "memory_bytes",
    }:
        raise ContractError("performance hardware fingerprint fields are not exact")
    physical = hardware.get("physical_cores")
    logical = hardware.get("logical_cores")
    memory = hardware.get("memory_bytes")
    if (
        hardware.get("os") != "windows_11"
        or hardware.get("architecture") != "x86_64"
        or not isinstance(hardware.get("cpu"), str)
        or not hardware["cpu"].strip()
        or not isinstance(physical, int)
        or isinstance(physical, bool)
        or physical < 6
        or not isinstance(logical, int)
        or isinstance(logical, bool)
        or logical < physical
        or not isinstance(memory, int)
        or isinstance(memory, bool)
        or memory < 16 * 1024**3
    ):
        raise ContractError(
            "performance hardware does not satisfy standard_windows_6c16g"
        )

    cases = index.get("cases")
    if not isinstance(cases, list) or index.get("case_count") != len(cases):
        raise ContractError("performance case inventory is invalid")
    target_cases = [
        row
        for row in cases
        if isinstance(row, Mapping)
        and row.get("capability_reference") == target_reference
    ]
    plan_by_key = {
        (row["variant_id"], row["profile_id"], row["case_id"]): row
        for row in target_plan
    }
    observed: set[tuple[str, str, str]] = set()
    row_fields = {
        "variant_id",
        "capability_reference",
        "profile_id",
        "case_id",
        "workload_fingerprint",
        "warmup_runs",
        "measured_runs",
        "cancellation_observed",
        "memory_soak_accepted_runs",
        "receipt",
        "baseline",
        "receipt_complete",
    }
    for row in target_cases:
        if set(row) != row_fields:
            raise ContractError("target performance case fields are not exact")
        key = (str(row["variant_id"]), str(row["profile_id"]), str(row["case_id"]))
        plan = plan_by_key.get(key)
        receipt = row.get("receipt")
        if (
            plan is None
            or key in observed
            or row.get("workload_fingerprint") != plan["workload_fingerprint"]
            or row.get("warmup_runs") != 1
            or row.get("measured_runs") != 5
            or row.get("cancellation_observed") is not bool(plan["potentially_long"])
            or row.get("memory_soak_accepted_runs")
            != (10 if plan["repeat_memory_gate"] else 0)
            or row.get("receipt_complete") is not True
            or not isinstance(receipt, Mapping)
            or set(receipt) != {"path", "size", "sha256"}
            or not isinstance(receipt.get("path"), str)
            or not isinstance(receipt.get("size"), int)
            or receipt["size"] <= 0
            or not isinstance(receipt.get("sha256"), str)
        ):
            raise ContractError("target performance case is incomplete or differs")
        _require_sha256(receipt["sha256"], "target performance receipt sha256")
        observed.add(key)
    if observed != set(plan_by_key):
        raise ContractError(
            "performance index does not exactly cover target cell cases"
        )

    ledger = index.get("cell_pass_ledger")
    target_ledger = (
        [
            row
            for row in ledger
            if isinstance(ledger, list)
            and isinstance(row, Mapping)
            and row.get("capability_reference") == target_reference
        ]
        if isinstance(ledger, list)
        else []
    )
    if target_ledger != [
        {
            "capability_reference": target_reference,
            "expected_case_count": len(target_plan),
            "accepted_case_count": len(target_plan),
            "passed": True,
        }
    ]:
        raise ContractError("performance target cell pass ledger is not exact")
    return {
        "passed": True,
        "case_count": len(target_cases),
        "build_fingerprint": build_fingerprint,
        "hardware_fingerprint": dict(hardware),
        "source_set_sha256": source_receipt["source_set_sha256"],
        "qualification_contract_sha256": target_contracts[0][
            "qualification_contract_sha256"
        ],
    }


def _receipt_path(root: Path, row: Mapping[str, Any]) -> Path:
    return (
        root
        / "receipts"
        / str(row["variant_id"])
        / f"{row['profile_id']}__{row['case_id']}.json"
    )


def _validate_observations(
    value: Any,
    row: Mapping[str, Any],
    manifest: Mapping[str, Any],
) -> None:
    if not isinstance(value, Mapping):
        raise ContractError("driver observation must be one JSON object")
    expected: set[str] = set()
    if row["potentially_long"]:
        expected.add("cancellation_observation")
    if row["repeat_memory_gate"]:
        expected.add("memory_growth_observation")
    if set(value) != expected:
        raise ContractError(
            f"{row['variant_id']}/{row['profile_id']} observation fields are not exact"
        )
    if row["potentially_long"]:
        observed = value.get("cancellation_observation")
        requirement = manifest["operation_requirements"]["cancellation"]
        if not isinstance(observed, Mapping) or set(observed) != {
            "terminal_latency_seconds",
            "terminal_state",
            "no_partial_visible_result",
            "no_partial_committed_result",
            "archive_unchanged",
        }:
            raise ContractError("cancellation observation fields are not exact")
        latency = observed.get("terminal_latency_seconds")
        if (
            not isinstance(latency, (int, float))
            or isinstance(latency, bool)
            or latency < 0
            or latency > requirement["maximum_terminal_latency_seconds"]
            or observed.get("terminal_state") != requirement["terminal_state"]
            or any(
                observed.get(field) is not True
                for field in (
                    "no_partial_visible_result",
                    "no_partial_committed_result",
                    "archive_unchanged",
                )
            )
        ):
            raise ContractError("cancellation observation did not pass the atomic gate")
    if row["repeat_memory_gate"]:
        observed = value.get("memory_growth_observation")
        requirement = manifest["operation_requirements"]["memory_growth"]
        if not isinstance(observed, Mapping) or set(observed) != {
            "accepted_runs",
            "first_settled_working_set_bytes",
            "last_settled_working_set_bytes",
            "growth_percent",
            "orphan_processes",
        }:
            raise ContractError("memory-soak observation fields are not exact")
        numeric_fields = (
            "first_settled_working_set_bytes",
            "last_settled_working_set_bytes",
            "growth_percent",
        )
        if (
            not isinstance(observed.get("accepted_runs"), int)
            or isinstance(observed.get("accepted_runs"), bool)
            or observed["accepted_runs"] < requirement["minimum_repeated_accepted_runs"]
            or any(
                not isinstance(observed.get(field), (int, float))
                or isinstance(observed.get(field), bool)
                or observed[field] < 0
                for field in numeric_fields
            )
            or observed["growth_percent"]
            > requirement["maximum_material_growth_percent"]
            or observed.get("orphan_processes") != 0
        ):
            raise ContractError("ten-run memory-soak observation did not pass")


def _validate_result_payloads(
    *,
    output_root: Path,
    result_path: Path,
    workload_path: Path,
    row: Mapping[str, Any],
    executable_sha256: str,
    driver_sha256: str,
    receipt: Mapping[str, Any],
) -> dict[str, Any]:
    expected_runs = [("warmup", 0), *[("measured", index) for index in range(5)]]
    receipt_runs = [*receipt.get("warmup_runs", []), *receipt.get("measured_runs", [])]
    if [
        (run.get("phase"), run.get("index"))
        for run in receipt_runs
        if isinstance(run, Mapping)
    ] != expected_runs:
        raise ContractError(
            "performance receipt run phase/index inventory is not exact"
        )
    workload_sha256 = hashlib.sha256(workload_path.read_bytes()).hexdigest()
    expected_fields = {
        "schema_version",
        "evidence_kind",
        "variant_id",
        "capability_reference",
        "qualification_contract_sha256",
        "profile_id",
        "case_id",
        "mandatory_combination_id",
        "workload_fingerprint",
        "workload_document_sha256",
        "performance_driver_sha256",
        "prepared_project",
        "package_executable_sha256",
        "phase",
        "run_index",
        "operation_elapsed_seconds",
        "canonical_identity",
        "offline",
        "completed",
    }
    payloads: list[dict[str, Any]] = []
    qualification = row.get("qualification_spec")
    if not isinstance(qualification, Mapping):
        raise ContractError("planned QualificationSpec authority is missing")
    for ordinal, ((phase, index), receipt_run) in enumerate(
        zip(expected_runs, receipt_runs)
    ):
        path = (
            result_path.parent
            / f"{result_path.stem}.payloads"
            / f"{phase}-{index}.json"
        )
        payload = load_json(path)
        if not isinstance(payload, Mapping) or set(payload) != expected_fields:
            raise ContractError(
                f"performance result payload fields are not exact: {phase}/{index}"
            )
        identity = payload.get("canonical_identity")
        offline = payload.get("offline")
        prepared_project = payload.get("prepared_project")
        if (
            payload.get("schema_version") != 1
            or payload.get("evidence_kind")
            != "general_sem_rank0_packaged_performance_result"
            or payload.get("variant_id") != row["variant_id"]
            or payload.get("capability_reference") != row["capability_reference"]
            or payload.get("qualification_contract_sha256")
            != qualification.get("qualification_contract_sha256")
            or payload.get("profile_id") != row["profile_id"]
            or payload.get("case_id") != row["case_id"]
            or payload.get("mandatory_combination_id")
            != row["mandatory_combination_id"]
            or payload.get("workload_fingerprint") != row["workload_fingerprint"]
            or payload.get("workload_document_sha256") != workload_sha256
            or payload.get("performance_driver_sha256") != driver_sha256
            or payload.get("package_executable_sha256") != executable_sha256
            or payload.get("phase") != phase
            or payload.get("run_index") != index
            or payload.get("completed") is not True
            or not isinstance(payload.get("operation_elapsed_seconds"), (int, float))
            or isinstance(payload.get("operation_elapsed_seconds"), bool)
            or payload["operation_elapsed_seconds"] <= 0
            or not isinstance(prepared_project, str)
            or not Path(prepared_project).is_absolute()
            or not Path(prepared_project).is_file()
            or not isinstance(identity, Mapping)
            or any(
                not isinstance(identity.get(field), str) or not identity[field]
                for field in (
                    "documentId",
                    "runId",
                    "methodVersion",
                    "datasetFingerprint",
                )
            )
            or not isinstance(identity.get("tableIds"), list)
            or len(identity["tableIds"]) < 1
            or not isinstance(identity.get("chartIds"), list)
            or len(identity["chartIds"]) < 1
            or not isinstance(offline, Mapping)
            or offline.get("passed") is not True
            or offline.get("externalRequestCount") != 0
            or not isinstance(offline.get("observedRequestCount"), int)
            or isinstance(offline.get("observedRequestCount"), bool)
            or offline["observedRequestCount"] <= 0
            or receipt_run.get("result_bytes") != path.stat().st_size
        ):
            raise ContractError(
                f"performance result payload does not bind its exact run: {phase}/{index}"
            )
        payloads.append(
            {
                "phase": phase,
                "index": index,
                "artifact": _descriptor(path, output_root),
            }
        )
        if (
            ordinal == len(expected_runs) - 1
            and result_path.read_bytes() != path.read_bytes()
        ):
            raise ContractError(
                "final harness result does not equal the exact last measured payload"
            )
    return {
        "schema_version": 1,
        "evidence_kind": "general_sem_rank0_performance_result_payload_binding",
        "variant_id": row["variant_id"],
        "capability_reference": row["capability_reference"],
        "profile_id": row["profile_id"],
        "case_id": row["case_id"],
        "workload_fingerprint": row["workload_fingerprint"],
        "workload_document_sha256": workload_sha256,
        "package_executable_sha256": executable_sha256,
        "performance_driver_sha256": driver_sha256,
        "payloads": payloads,
        "passed": len(payloads) == 6,
    }


def validate_method_receipt(
    receipt: Mapping[str, Any],
    *,
    role: str,
    row: Mapping[str, Any],
    manifest: Mapping[str, Any],
    context: Mapping[str, Any],
    baseline: Mapping[str, Any] | None,
) -> None:
    reference = qualification_link_identity(row["capability_reference"])
    expected_key = (
        "capability",
        *reference,
        HARDWARE_PROFILE_ID,
        row["profile_id"],
        row["case_id"],
    )
    errors = _receipt_common_errors(receipt, context)
    try:
        observed_key = _measurement_key(receipt)
    except (KeyError, TypeError, ValueError) as error:
        raise ContractError(
            f"measurement receipt identity is invalid: {error}"
        ) from error
    if observed_key != expected_key:
        errors.append("measurement receipt differs from the exact planned cell")
    if receipt.get("measurement_role") != role:
        errors.append("measurement role differs from orchestration role")
    class_id = context["resolved_classes"].get(reference)
    if receipt.get("budget_class_id") != class_id:
        errors.append("budget class differs from Registry-derived authority")
    profile = context["profiles"].get(row["profile_id"])
    budget = (
        context["budget_index"]
        .get(class_id, {})
        .get((HARDWARE_PROFILE_ID, row["profile_id"]))
    )
    if not isinstance(profile, Mapping) or not isinstance(budget, Mapping):
        errors.append("profile or budget could not be resolved")
    else:
        errors.extend(
            _validate_runs(
                receipt,
                profile,
                budget,
                manifest,
                context["capability_predicates"].get(reference, frozenset()),
                context["hardware"].get(HARDWARE_PROFILE_ID),
            )
        )
    command = receipt.get("command")
    if (
        not isinstance(command, Mapping)
        or command.get("workload_fingerprint") != row["workload_fingerprint"]
    ):
        errors.append("receipt workload fingerprint differs from the frozen plan")
    if role == "accepted_baseline":
        if baseline is not None:
            errors.append("accepted baseline cannot receive a baseline")
    elif baseline is None:
        errors.append("current receipt requires its exact accepted baseline")
    else:
        baseline_errors = _receipt_common_errors(baseline, context)
        try:
            baseline_key = _measurement_key(baseline)
        except (KeyError, TypeError, ValueError):
            baseline_key = None
        if baseline_key != expected_key:
            baseline_errors.append("accepted baseline key differs")
        if baseline.get("measurement_role") != "accepted_baseline":
            baseline_errors.append("baseline receipt role is not accepted_baseline")
        baseline_command = baseline.get("command")
        if (
            not isinstance(baseline_command, Mapping)
            or baseline_command.get("workload_fingerprint")
            != row["workload_fingerprint"]
        ):
            baseline_errors.append("baseline workload fingerprint differs")
        reference_value = receipt.get("baseline_reference")
        if not isinstance(reference_value, Mapping) or reference_value != {
            "measurement_id": baseline.get("measurement_id"),
            "receipt_sha256": canonical_sha256(baseline),
        }:
            errors.append("current receipt is not hash-bound to its exact baseline")
        errors.extend(f"baseline: {error}" for error in baseline_errors)
        comparability = _baseline_comparability_errors(receipt, baseline)
        errors.extend(comparability)
        if not comparability:
            errors.extend(
                _regression_errors(receipt, baseline, manifest["measurement_policy"])
            )
    if errors:
        raise ContractError(
            f"{row['variant_id']}/{row['profile_id']}/{row['case_id']} receipt failed: "
            + "; ".join(errors)
        )


def _resolve_program(program: str) -> str:
    candidate = Path(program)
    if candidate.is_file():
        return str(candidate.resolve())
    resolved = shutil.which(program)
    if resolved is None:
        raise ContractError(f"performance driver executable is unavailable: {program}")
    return resolved


def run_matrix(
    *,
    output_root: Path,
    role: str,
    build_fingerprint: str,
    quickpls_executable: Path,
    driver_program: str | None,
    driver_args: Sequence[str],
    physical_cores: int,
    cpu: str | None,
    memory_bytes: int | None,
    accepted_root: Path | None,
    variant_id: str | None = None,
    manifest_path: Path = DEFAULT_MANIFEST_PATH,
    registry_path: Path = DEFAULT_REGISTRY_PATH,
    schema_path: Path = DEFAULT_SCHEMA_PATH,
    measurement_schema_path: Path = DEFAULT_MEASUREMENT_SCHEMA_PATH,
    rank0_contract_path: Path = DEFAULT_RANK0_CONTRACT,
) -> dict[str, Any]:
    _require_sha256(build_fingerprint, "build_fingerprint")
    if (
        not quickpls_executable.is_file()
        or quickpls_executable.is_symlink()
        or quickpls_executable.suffix.lower() != ".exe"
    ):
        raise ContractError(
            f"actual packaged QuickPLS executable is unavailable: {quickpls_executable}"
        )
    executable_sha256 = hashlib.sha256(quickpls_executable.read_bytes()).hexdigest()
    if build_fingerprint != executable_sha256:
        raise ContractError(
            "build_fingerprint must equal the exact packaged QuickPLS executable SHA-256"
        )
    if role not in {"accepted_baseline", "current"}:
        raise ContractError("measurement role must be accepted_baseline or current")
    if output_root.exists():
        raise ContractError(f"performance output root must be new: {output_root}")
    try:
        output_root.resolve().relative_to(PERFORMANCE_RESULTS_ROOT.resolve())
    except ValueError as error:
        raise ContractError(
            "performance output root must remain below validation/results"
        ) from error
    if role == "current":
        if accepted_root is None or not accepted_root.is_dir():
            raise ContractError(
                "current measurements require an accepted-baseline root"
            )
    elif accepted_root is not None:
        raise ContractError("accepted-baseline capture cannot reference accepted-root")
    if physical_cores < 6:
        raise ContractError(
            "Standard performance capture requires at least 6 physical cores"
        )
    if memory_bytes is not None and memory_bytes < 16 * 1024**3:
        raise ContractError(
            "Standard performance capture requires at least 16 GiB memory"
        )
    if driver_program is not None or driver_args:
        raise ContractError(
            "accepted Rank 0 qualification requires the frozen bundled performance driver; custom driver programs/arguments are forbidden"
        )
    if not BUNDLED_DRIVER.is_file():
        raise ContractError("frozen bundled Rank 0 performance driver is unavailable")
    program = _resolve_program("node")
    effective_driver_args = [str(BUNDLED_DRIVER.resolve())]
    driver_sha256 = hashlib.sha256(BUNDLED_DRIVER.read_bytes()).hexdigest()
    manifest, context, all_rows = build_plan(
        manifest_path=manifest_path,
        registry_path=registry_path,
        schema_path=schema_path,
        measurement_schema_path=measurement_schema_path,
        rank0_contract_path=rank0_contract_path,
    )
    rows = [
        row for row in all_rows if variant_id is None or row["variant_id"] == variant_id
    ]
    if not rows or (
        variant_id is not None and {row["variant_id"] for row in rows} != {variant_id}
    ):
        raise ContractError("requested performance variant is unavailable")
    rank0_contract = load_json(rank0_contract_path)
    rank0_context = validate_rank0_contract(rank0_contract, load_json(registry_path))
    source_receipt_before = unified_rank0_source_receipt(ROOT)
    qualification_contracts_before = qualification_contract_authorities(
        rank0_context, ROOT
    )

    baselines: dict[tuple[str, str, str], Mapping[str, Any]] = {}
    if role == "current":
        assert accepted_root is not None
        for row in rows:
            path = _receipt_path(accepted_root, row)
            if not path.is_file():
                raise ContractError(f"accepted baseline is unavailable: {path}")
            baseline = load_json(path)
            validate_method_receipt(
                baseline,
                role="accepted_baseline",
                row=row,
                manifest=manifest,
                context=context,
                baseline=None,
            )
            baselines[(row["variant_id"], row["profile_id"], row["case_id"])] = baseline

    output_root.mkdir(parents=True)
    index_rows: list[dict[str, Any]] = []
    result_payload_bindings: list[dict[str, Any]] = []
    observed_hardware: Mapping[str, Any] | None = None
    for row in rows:
        stem = f"{row['profile_id']}__{row['case_id']}"
        workload_path = output_root / "workloads" / row["variant_id"] / f"{stem}.json"
        workload_document = {
            key: row[key]
            for key in (
                "schema_version",
                "workload_kind",
                "variant_id",
                "capability_reference",
                "qualification_spec",
                "profile_id",
                "case_id",
                "mandatory_combination_id",
                "workload",
                "workload_fingerprint",
            )
        }
        _write_new_json(workload_path, workload_document)
        prepared_project_path = (
            output_root / "prepared" / row["variant_id"] / f"{stem}.qpls"
        )
        prepared_project_path.parent.mkdir(parents=True, exist_ok=True)
        prepare_argv = [
            program,
            *effective_driver_args,
            "prepare",
            "--variant-id",
            row["variant_id"],
            "--profile-id",
            row["profile_id"],
            "--case-id",
            row["case_id"],
            "--workload-json",
            str(workload_path.resolve()),
            "--quickpls-executable",
            str(quickpls_executable.resolve()),
            "--python-executable",
            sys.executable,
            "--prepared-project",
            str(prepared_project_path.resolve()),
            "--driver-sha256",
            driver_sha256,
        ]
        completed = subprocess.run(
            prepare_argv,
            cwd=ROOT,
            check=False,
            stdin=subprocess.DEVNULL,
        )
        if (
            completed.returncode != 0
            or not prepared_project_path.is_file()
            or prepared_project_path.stat().st_size <= 0
        ):
            raise ContractError(
                f"packaged workload preparation failed for {row['variant_id']}/{stem}"
            )
        observation_path: Path | None = None
        if row["potentially_long"] or row["repeat_memory_gate"]:
            observation_path = (
                output_root / "observations" / row["variant_id"] / f"{stem}.json"
            )
            observation_path.parent.mkdir(parents=True, exist_ok=True)
            observation_argv = [
                program,
                *effective_driver_args,
                "observe",
                "--variant-id",
                row["variant_id"],
                "--profile-id",
                row["profile_id"],
                "--case-id",
                row["case_id"],
                "--workload-json",
                str(workload_path.resolve()),
                "--quickpls-executable",
                str(quickpls_executable.resolve()),
                "--python-executable",
                sys.executable,
                "--prepared-project",
                str(prepared_project_path.resolve()),
                "--driver-sha256",
                driver_sha256,
                "--output",
                str(observation_path.resolve()),
                "--accepted-runs",
                "10",
            ]
            completed = subprocess.run(
                observation_argv,
                cwd=ROOT,
                check=False,
                stdin=subprocess.DEVNULL,
            )
            if completed.returncode != 0 or not observation_path.is_file():
                raise ContractError(
                    f"performance observer failed for {row['variant_id']}/{stem}"
                )
            _validate_observations(load_json(observation_path), row, manifest)

        receipt_path = _receipt_path(output_root, row)
        result_path = output_root / "results" / row["variant_id"] / f"{stem}.json"
        result_path.parent.mkdir(parents=True, exist_ok=True)
        measure_driver_argv = [
            program,
            *effective_driver_args,
            "measure",
            "--variant-id",
            row["variant_id"],
            "--profile-id",
            row["profile_id"],
            "--case-id",
            row["case_id"],
            "--workload-json",
            str(workload_path.resolve()),
            "--quickpls-executable",
            str(quickpls_executable.resolve()),
            "--python-executable",
            sys.executable,
            "--prepared-project",
            str(prepared_project_path.resolve()),
            "--driver-sha256",
            driver_sha256,
        ]
        harness_argv = [
            sys.executable,
            str(MEASUREMENT_HARNESS),
            "--manifest",
            str(manifest_path.resolve()),
            "--registry",
            str(registry_path.resolve()),
            "--schema",
            str(schema_path.resolve()),
            "--measurement-schema",
            str(measurement_schema_path.resolve()),
            "--capability-id",
            row["capability_reference"]["capability_id"],
            "--cell-id",
            row["capability_reference"]["cell_id"],
            "--capability-version",
            row["capability_reference"]["capability_version"],
            "--hardware-profile",
            HARDWARE_PROFILE_ID,
            "--physical-cores",
            str(physical_cores),
            "--profile",
            row["profile_id"],
            "--case",
            row["case_id"],
            "--measurement-role",
            role,
            "--build-fingerprint",
            build_fingerprint,
            "--workload-fingerprint",
            row["workload_fingerprint"],
            "--cwd",
            str(ROOT),
            "--result-path",
            str(result_path.resolve()),
            "--output",
            str(receipt_path.resolve()),
        ]
        if cpu:
            harness_argv.extend(("--cpu", cpu))
        if memory_bytes is not None:
            harness_argv.extend(("--memory-bytes", str(memory_bytes)))
        if observation_path is not None:
            harness_argv.extend(("--observations", str(observation_path.resolve())))
        key = (row["variant_id"], row["profile_id"], row["case_id"])
        baseline_path: Path | None = None
        if role == "current":
            assert accepted_root is not None
            baseline_path = _receipt_path(accepted_root, row)
            harness_argv.extend(("--baseline", str(baseline_path.resolve())))
        harness_argv.extend(("--", *measure_driver_argv))
        completed = subprocess.run(
            harness_argv,
            cwd=ROOT,
            check=False,
            stdin=subprocess.DEVNULL,
        )
        if completed.returncode != 0 or not receipt_path.is_file():
            raise ContractError(
                f"measurement harness failed for {row['variant_id']}/{stem}"
            )
        receipt = load_json(receipt_path)
        baseline = baselines.get(key)
        validate_method_receipt(
            receipt,
            role=role,
            row=row,
            manifest=manifest,
            context=context,
            baseline=baseline,
        )
        receipt_hardware = receipt.get("hardware_fingerprint")
        if not isinstance(receipt_hardware, Mapping):
            raise ContractError(
                "performance receipt hardware fingerprint is unavailable"
            )
        if observed_hardware is None:
            observed_hardware = dict(receipt_hardware)
        elif receipt_hardware != observed_hardware:
            raise ContractError(
                "performance receipts were captured on different hardware"
            )
        payload_binding = _validate_result_payloads(
            output_root=output_root,
            result_path=result_path,
            workload_path=workload_path,
            row=row,
            executable_sha256=executable_sha256,
            driver_sha256=driver_sha256,
            receipt=receipt,
        )
        payload_binding_path = (
            output_root / "result-payload-bindings" / row["variant_id"] / f"{stem}.json"
        )
        _write_new_json(payload_binding_path, payload_binding)
        result_payload_bindings.append(
            {
                "variant_id": row["variant_id"],
                "profile_id": row["profile_id"],
                "case_id": row["case_id"],
                "workload_fingerprint": row["workload_fingerprint"],
                "binding": _descriptor(payload_binding_path, output_root),
            }
        )
        index_rows.append(
            {
                "variant_id": row["variant_id"],
                "capability_reference": row["capability_reference"],
                "profile_id": row["profile_id"],
                "case_id": row["case_id"],
                "workload_fingerprint": row["workload_fingerprint"],
                "warmup_runs": len(receipt["warmup_runs"]),
                "measured_runs": len(receipt["measured_runs"]),
                "cancellation_observed": receipt["cancellation_observation"]
                is not None,
                "memory_soak_accepted_runs": (
                    receipt["memory_growth_observation"]["accepted_runs"]
                    if receipt["memory_growth_observation"] is not None
                    else 0
                ),
                "receipt": _descriptor(receipt_path, output_root),
                "baseline": (
                    {
                        "path": baseline_path.resolve().as_posix(),
                        "measurement_id": baseline["measurement_id"],
                        "sha256": canonical_sha256(baseline),
                    }
                    if baseline is not None and baseline_path is not None
                    else None
                ),
                "receipt_complete": True,
            }
        )

    if observed_hardware is None:
        raise ContractError("performance matrix produced no hardware authority")
    source_receipt_after = unified_rank0_source_receipt(ROOT)
    if source_receipt_after != source_receipt_before:
        raise ContractError(
            "unified Rank 0 source inventory changed during measurement"
        )
    qualification_contracts_after = qualification_contract_authorities(
        rank0_context, ROOT
    )
    if qualification_contracts_after != qualification_contracts_before:
        raise ContractError("QualificationSpec contract changed during measurement")
    selected_references = {
        canonical_sha256(row["capability_reference"]): row["capability_reference"]
        for row in rows
    }
    cell_pass_ledger = []
    for reference in selected_references.values():
        planned = [row for row in rows if row["capability_reference"] == reference]
        accepted = [
            row
            for row in index_rows
            if row["capability_reference"] == reference
            and row["receipt_complete"] is True
        ]
        cell_pass_ledger.append(
            {
                "capability_reference": reference,
                "expected_case_count": len(planned),
                "accepted_case_count": len(accepted),
                "passed": len(accepted) == len(planned),
            }
        )

    index = {
        "schema_version": 1,
        "evidence_kind": "general_sem_rank0_method_performance_index",
        "measurement_role": role,
        "contract_id": manifest["contract_id"],
        "contract_sha256": context["contract_sha256"],
        "rank0_contract_sha256": canonical_sha256(load_json(rank0_contract_path)),
        "hardware_profile_id": HARDWARE_PROFILE_ID,
        "hardware_fingerprint": dict(observed_hardware),
        "build_fingerprint": build_fingerprint,
        "source_receipt": source_receipt_before,
        "qualification_contracts": qualification_contracts_before,
        "generated_at_utc": datetime.now(timezone.utc)
        .isoformat()
        .replace("+00:00", "Z"),
        "case_count": len(index_rows),
        "cases": index_rows,
        "cell_pass_ledger": cell_pass_ledger,
        "passed": len(index_rows) == len(rows)
        and all(row["passed"] is True for row in cell_pass_ledger),
    }
    _write_new_json(
        output_root / "performance-result-payload-index.json",
        {
            "schema_version": 1,
            "evidence_kind": "general_sem_rank0_performance_result_payload_index",
            "hardware_profile_id": HARDWARE_PROFILE_ID,
            "hardware_fingerprint": dict(observed_hardware),
            "build_fingerprint": build_fingerprint,
            "source_receipt": source_receipt_before,
            "qualification_contracts": qualification_contracts_before,
            "performance_driver": {
                "path": BUNDLED_DRIVER.relative_to(ROOT).as_posix(),
                "size": BUNDLED_DRIVER.stat().st_size,
                "sha256": driver_sha256,
            },
            "case_count": len(result_payload_bindings),
            "cases": result_payload_bindings,
            "passed": len(result_payload_bindings) == len(rows),
        },
    )
    _write_new_json(output_root / "performance-index.json", index)
    return index


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument(
        "--measurement-role",
        choices=("accepted_baseline", "current"),
        required=True,
    )
    parser.add_argument("--accepted-root", type=Path)
    parser.add_argument("--build-fingerprint", required=True)
    parser.add_argument("--quickpls-executable", type=Path, required=True)
    parser.add_argument(
        "--driver-program",
        help="Optional custom driver executable; defaults to Node plus the bundled package/CDP driver.",
    )
    parser.add_argument("--driver-arg", action="append", default=[])
    parser.add_argument("--variant-id")
    parser.add_argument("--physical-cores", type=int, required=True)
    parser.add_argument("--cpu")
    parser.add_argument("--memory-bytes", type=int)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST_PATH)
    parser.add_argument("--registry", type=Path, default=DEFAULT_REGISTRY_PATH)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA_PATH)
    parser.add_argument(
        "--measurement-schema",
        type=Path,
        default=DEFAULT_MEASUREMENT_SCHEMA_PATH,
    )
    parser.add_argument("--rank0-contract", type=Path, default=DEFAULT_RANK0_CONTRACT)
    args = parser.parse_args(argv)
    try:
        index = run_matrix(
            output_root=args.output_root.resolve(),
            role=args.measurement_role,
            build_fingerprint=args.build_fingerprint,
            quickpls_executable=args.quickpls_executable.resolve(),
            driver_program=args.driver_program,
            driver_args=args.driver_arg,
            physical_cores=args.physical_cores,
            cpu=args.cpu,
            memory_bytes=args.memory_bytes,
            accepted_root=(
                args.accepted_root.resolve() if args.accepted_root is not None else None
            ),
            variant_id=args.variant_id,
            manifest_path=args.manifest.resolve(),
            registry_path=args.registry.resolve(),
            schema_path=args.schema.resolve(),
            measurement_schema_path=args.measurement_schema.resolve(),
            rank0_contract_path=args.rank0_contract.resolve(),
        )
        result = {
            "passed": True,
            "output": str((args.output_root / "performance-index.json").resolve()),
            "case_count": index["case_count"],
            "measurement_role": index["measurement_role"],
        }
    except (ContractError, OSError, UnicodeError, json.JSONDecodeError) as error:
        result = {"passed": False, "errors": [str(error)]}
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
