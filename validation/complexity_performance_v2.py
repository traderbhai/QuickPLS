#!/usr/bin/env python3
"""Fail-closed Product-finalization complexity and performance contract V2.

This module validates target budgets and supplied measurement receipts. It does
not run maximum-axis or compound workloads and it never promotes capability or
catalogue state. Contract-only validation is explicit; normal CLI validation
requires the complete current measurement matrix and therefore fails while no
receipts have been supplied.
"""

from __future__ import annotations

import argparse
import math
import statistics
from datetime import datetime
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence

from jsonschema import Draft202012Validator, FormatChecker
from jsonschema.exceptions import SchemaError

try:
    from validation.capability_registry_v2 import (
        DEFAULT_REGISTRY_PATH,
        REPOSITORY_ROOT,
        canonical_sha256,
        load_json,
        qualification_link_identity,
        validate_registry_document,
    )
except ModuleNotFoundError:  # Direct `python validation/...py` execution.
    from capability_registry_v2 import (
        DEFAULT_REGISTRY_PATH,
        REPOSITORY_ROOT,
        canonical_sha256,
        load_json,
        qualification_link_identity,
        validate_registry_document,
    )


DEFAULT_MANIFEST_PATH = (
    REPOSITORY_ROOT
    / "validation/capabilities/complexity_performance_profiles_v2.manifest.json"
)
DEFAULT_SCHEMA_PATH = (
    REPOSITORY_ROOT
    / "validation/capabilities/complexity_performance_profiles_v2.schema.json"
)
DEFAULT_MEASUREMENT_SCHEMA_PATH = (
    REPOSITORY_ROOT
    / "validation/capabilities/complexity_performance_measurement_v2.schema.json"
)

CONTRACT_ID = "quickpls.product_finalization.complexity_performance.v2"
HARDWARE_IDS = (
    "standard_windows_6c16g",
    "workstation_windows_12c32g",
)
PROFILE_IDS = (
    "micro_exact",
    "applied",
    "large",
    "maximum_axis",
    "compound_stress",
)
UI_SCENARIO_IDS = (
    "applied_diagram",
    "stress_diagram",
    "typical_preflight",
    "stress_preflight",
)
TOP_LEVEL_KEYS = frozenset(
    {
        "schema_version",
        "contract_id",
        "contract_version",
        "status",
        "registry_binding",
        "hardware_profiles",
        "complexity_profiles",
        "ui_budgets",
        "measurement_policy",
        "operation_requirements",
        "budget_classes",
        "capability_budget_resolution",
        "measurement_artifacts",
    }
)
REFERENCE_KEYS = frozenset(
    {"registry_schema_version", "capability_id", "cell_id", "capability_version"}
)
BUDGET_KEYS = frozenset(
    {
        "hardware_profile_id",
        "profile_id",
        "maximum_median_elapsed_seconds",
        "maximum_p95_elapsed_seconds",
        "maximum_median_working_set_bytes",
        "maximum_p95_working_set_bytes",
        "maximum_p95_result_bytes",
    }
)
CAPABILITY_RECEIPT_KEYS = frozenset(
    {
        "schema_version",
        "document_kind",
        "measurement_id",
        "measurement_role",
        "captured_at_utc",
        "contract_id",
        "contract_sha256",
        "capability_reference",
        "hardware_profile_id",
        "hardware_fingerprint",
        "budget_class_id",
        "profile_id",
        "case_id",
        "applicability",
        "not_applicable_reason",
        "predicate_references",
        "command",
        "warmup_runs",
        "measured_runs",
        "aggregates",
        "progress_observation",
        "cancellation_observation",
        "memory_growth_observation",
        "baseline_reference",
        "receipt_complete",
    }
)
UI_RECEIPT_KEYS = frozenset(
    {
        "schema_version",
        "document_kind",
        "measurement_id",
        "measurement_role",
        "captured_at_utc",
        "contract_id",
        "contract_sha256",
        "hardware_profile_id",
        "hardware_fingerprint",
        "scenario_id",
        "warmup_runs",
        "measured_runs",
        "aggregates",
        "baseline_reference",
        "receipt_complete",
    }
)
RUN_KEYS = frozenset(
    {
        "phase",
        "index",
        "exit_code",
        "elapsed_seconds",
        "peak_working_set_bytes",
        "result_bytes",
        "progress_values",
        "orphan_processes",
    }
)
AGGREGATE_KEYS = frozenset(
    {
        "median_elapsed_seconds",
        "p95_elapsed_seconds",
        "median_working_set_bytes",
        "p95_working_set_bytes",
        "p95_result_bytes",
    }
)
UI_SAMPLE_KEYS = frozenset(
    {"open_seconds", "edit_response_ms", "pan_zoom_fps", "preflight_seconds"}
)
UI_AGGREGATE_KEYS = frozenset(
    {
        "median_open",
        "p95_open",
        "median_edit_response",
        "p95_edit_response",
        "median_pan_zoom_fps",
        "p05_pan_zoom_fps",
        "median_preflight",
        "p95_preflight",
    }
)


def _is_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def _unique_rows(
    rows: Any, key: str, label: str, errors: list[str]
) -> dict[str, Mapping[str, Any]]:
    if not isinstance(rows, list):
        errors.append(f"{label} must be a list")
        return {}
    result: dict[str, Mapping[str, Any]] = {}
    for index, row in enumerate(rows):
        if not isinstance(row, Mapping):
            errors.append(f"{label}[{index}] must be an object")
            continue
        value = row.get(key)
        if not isinstance(value, str) or not value:
            errors.append(f"{label}[{index}].{key} must be non-empty")
            continue
        if value in result:
            errors.append(f"{label} duplicates {value!r}")
        result[value] = row
    return result


def _finite_nonnegative(value: Any) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(float(value))
        and float(value) >= 0
    )


def _finite_positive(value: Any) -> bool:
    return _finite_nonnegative(value) and float(value) > 0


def active_registry_entries(
    registry: Mapping[str, Any],
) -> list[tuple[dict[str, Any], Mapping[str, Any], Mapping[str, Any]]]:
    """Derive exact active option-cell references in official catalogue order."""

    entries: list[tuple[dict[str, Any], Mapping[str, Any], Mapping[str, Any]]] = []
    for row in registry.get("capabilities", []):
        if not isinstance(row, Mapping) or row.get("official_lifecycle") != "active":
            continue
        for cell in row.get("option_cells", []):
            if not isinstance(cell, Mapping):
                continue
            reference = {
                "registry_schema_version": 2,
                "capability_id": cell.get("capability_id"),
                "cell_id": cell.get("cell_id"),
                "capability_version": cell.get("capability_version"),
            }
            qualification_link_identity(reference)
            entries.append((reference, row, cell))
    return entries


def _identity(reference: Mapping[str, Any]) -> tuple[int, str, str, str]:
    return qualification_link_identity(reference)


def type7_percentile(values: Sequence[float | int], probability: float) -> float:
    """Hyndman-Fan Type 7 percentile used by the V2 measurement contract."""

    if not values:
        raise ValueError("a percentile requires at least one value")
    if not 0 <= probability <= 1:
        raise ValueError("probability must be within [0, 1]")
    ordered = sorted(float(value) for value in values)
    if any(not math.isfinite(value) for value in ordered):
        raise ValueError("percentile values must be finite")
    if len(ordered) == 1:
        return ordered[0]
    position = (len(ordered) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    fraction = position - lower
    return ordered[lower] + fraction * (ordered[upper] - ordered[lower])


def aggregate_runs(runs: Sequence[Mapping[str, Any]]) -> dict[str, float]:
    return {
        "median_elapsed_seconds": statistics.median(
            float(run["elapsed_seconds"]) for run in runs
        ),
        "p95_elapsed_seconds": type7_percentile(
            [float(run["elapsed_seconds"]) for run in runs], 0.95
        ),
        "median_working_set_bytes": statistics.median(
            int(run["peak_working_set_bytes"]) for run in runs
        ),
        "p95_working_set_bytes": type7_percentile(
            [int(run["peak_working_set_bytes"]) for run in runs], 0.95
        ),
        "p95_result_bytes": type7_percentile(
            [int(run["result_bytes"]) for run in runs], 0.95
        ),
    }


def validate_schema_documents(
    contract_schema: Mapping[str, Any],
    measurement_schema: Mapping[str, Any],
) -> list[str]:
    errors: list[str] = []
    for label, schema in (
        ("contract schema", contract_schema),
        ("measurement schema", measurement_schema),
    ):
        try:
            Draft202012Validator.check_schema(schema)
        except SchemaError as exc:
            errors.append(f"{label} is not valid draft 2020-12 JSON Schema: {exc.message}")
        if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
            errors.append(f"{label} must use JSON Schema draft 2020-12")
        if not isinstance(schema.get("$defs"), Mapping):
            errors.append(f"{label} must define reusable strict shapes")
    properties = contract_schema.get("properties", {})
    if properties.get("schema_version", {}).get("const") != 2:
        errors.append("contract schema must freeze schema_version at 2")
    if set(contract_schema.get("required", [])) != TOP_LEVEL_KEYS:
        errors.append("contract schema top-level required fields are not exact")
    if contract_schema.get("additionalProperties") is not False:
        errors.append("contract schema must reject additional top-level properties")
    if measurement_schema.get("title") != (
        "QuickPLS complexity and performance measurement receipt V2"
    ):
        errors.append("measurement schema identity is invalid")
    return errors


def _json_schema_instance_errors(
    value: Mapping[str, Any], schema: Mapping[str, Any], label: str
) -> list[str]:
    try:
        validator = Draft202012Validator(schema, format_checker=FormatChecker())
        schema_errors = sorted(validator.iter_errors(value), key=lambda row: list(row.path))
    except SchemaError as exc:
        return [f"{label} schema is invalid: {exc.message}"]
    errors: list[str] = []
    for error in schema_errors:
        location = ".".join(str(value) for value in error.absolute_path)
        errors.append(f"{label} schema {location or '<root>'}: {error.message}")
    return errors


def _validate_profiles(
    manifest: Mapping[str, Any], errors: list[str]
) -> tuple[dict[str, Mapping[str, Any]], list[tuple[str, str]]]:
    profiles = _unique_rows(
        manifest.get("complexity_profiles"), "profile_id", "complexity profile", errors
    )
    if tuple(profiles) != PROFILE_IDS:
        errors.append("complexity profiles must be in the frozen five-profile order")
    expected_cases: list[tuple[str, str]] = []
    case_ids: set[str] = set()
    for profile_id, profile in profiles.items():
        cases = profile.get("cases")
        if not isinstance(cases, list) or not cases:
            errors.append(f"complexity profile {profile_id!r} needs at least one case")
            continue
        for case in cases:
            if not isinstance(case, Mapping):
                errors.append(f"complexity profile {profile_id!r} has a non-object case")
                continue
            case_id = case.get("case_id")
            if not isinstance(case_id, str) or not case_id:
                errors.append(f"complexity profile {profile_id!r} has an invalid case ID")
                continue
            if case_id in case_ids:
                errors.append(f"complexity case {case_id!r} is duplicated")
            case_ids.add(case_id)
            expected_cases.append((profile_id, case_id))
            workload = case.get("workload")
            if not isinstance(workload, Mapping) or set(workload) != {
                "rows",
                "indicators",
                "constructs",
                "resamples",
                "groups",
                "candidate_models",
            }:
                errors.append(f"complexity case {case_id!r} workload is not exact")
            elif any(
                not isinstance(value, int) or isinstance(value, bool) or value < 0
                for value in workload.values()
            ):
                errors.append(f"complexity case {case_id!r} workload must be integers")
    maximum = {
        case.get("axis"): case.get("workload", {})
        for case in profiles.get("maximum_axis", {}).get("cases", [])
        if isinstance(case, Mapping)
    }
    required_maxima = {
        "rows": ("rows", 100_000),
        "indicators": ("indicators", 300),
        "constructs": ("constructs", 100),
        "resamples": ("resamples", 10_000),
    }
    if set(maximum) != set(required_maxima):
        errors.append("maximum_axis must contain separate rows/indicators/constructs/resamples cases")
    else:
        for axis, (field, expected) in required_maxima.items():
            if maximum[axis].get(field) != expected:
                errors.append(f"maximum_axis {axis!r} must set {field}={expected}")
    large_cases = profiles.get("large", {}).get("cases", [])
    if large_cases:
        workload = large_cases[0].get("workload", {})
        if workload.get("constructs", 0) < 20 or workload.get("indicators", 0) < 80:
            errors.append("large profile must cover at least 20 constructs and 80 indicators")
    repeat_profiles = [
        profile_id
        for profile_id, profile in profiles.items()
        if profile.get("repeat_memory_gate") is True
    ]
    if repeat_profiles != ["applied"]:
        errors.append("the lightweight repeated-run memory gate must use the applied profile")
    return profiles, expected_cases


def _validate_ui_budgets(
    manifest: Mapping[str, Any], errors: list[str]
) -> dict[str, Mapping[str, Any]]:
    budgets = _unique_rows(manifest.get("ui_budgets"), "scenario_id", "UI budget", errors)
    if tuple(budgets) != UI_SCENARIO_IDS:
        errors.append("UI budgets must contain the four frozen scenarios in order")
        return budgets
    expected = {
        "applied_diagram": (2, 100, 45, None),
        "stress_diagram": (10, 250, 30, None),
        "typical_preflight": (None, None, None, 0.5),
        "stress_preflight": (None, None, None, 3),
    }
    for scenario_id, values in expected.items():
        budget = budgets[scenario_id]
        actual = (
            budget.get("maximum_open_seconds"),
            budget.get("maximum_edit_response_p95_ms"),
            budget.get("minimum_pan_zoom_fps"),
            budget.get("maximum_preflight_seconds"),
        )
        if actual != values:
            errors.append(f"UI budget {scenario_id!r} differs from the product target")
    return budgets


def _validate_policy(manifest: Mapping[str, Any], errors: list[str]) -> None:
    policy = manifest.get("measurement_policy")
    if not isinstance(policy, Mapping):
        errors.append("measurement_policy must be an object")
        return
    if policy.get("warmup_runs") != 1 or policy.get("measured_runs") != 5:
        errors.append("measurement policy requires one warm-up and five measured runs")
    if policy.get("gate_statistics") != ["median", "p95"]:
        errors.append("measurement policy must gate on median and p95")
    if policy.get("high_percentile") != 0.95 or policy.get("percentile_method") != "type_7":
        errors.append("measurement policy must use the Type-7 p95")
    for field in (
        "maximum_runtime_regression_percent",
        "maximum_memory_regression_percent",
    ):
        value = policy.get(field)
        if not _finite_nonnegative(value) or float(value) > 20:
            errors.append(f"{field} cannot exceed the 20 percent product rule")
    if policy.get("accepted_baseline_required") is not True:
        errors.append("an accepted baseline must be required")
    if policy.get("failed_runs_fail_measurement") is not True:
        errors.append("failed measured runs must fail the measurement")

    operations = manifest.get("operation_requirements")
    if not isinstance(operations, Mapping):
        errors.append("operation_requirements must be an object")
        return
    progress = operations.get("progress", {})
    if (
        not _finite_positive(progress.get("elapsed_threshold_seconds"))
        or float(progress["elapsed_threshold_seconds"]) > 2
        or progress.get("real_progress_required") is not True
        or progress.get("minimum_distinct_progress_values", 0) < 2
        or progress.get("monotonic_progress_required") is not True
    ):
        errors.append("progress requirements do not enforce real progress after two seconds")
    cancellation = operations.get("cancellation", {})
    if (
        not _finite_positive(cancellation.get("maximum_terminal_latency_seconds"))
        or float(cancellation["maximum_terminal_latency_seconds"]) > 1
        or cancellation.get("terminal_state") != "cancelled"
        or any(
            cancellation.get(field) is not True
            for field in (
                "no_partial_visible_result",
                "no_partial_committed_result",
                "archive_unchanged",
            )
        )
    ):
        errors.append("cancellation requirements do not enforce the one-second atomic contract")
    memory = operations.get("memory_growth", {})
    if (
        memory.get("minimum_repeated_accepted_runs", 0) < 2
        or not _finite_nonnegative(memory.get("maximum_material_growth_percent"))
        or memory.get("no_orphan_process") is not True
    ):
        errors.append("memory-growth requirements are incomplete")


def _validate_and_index_budgets(
    manifest: Mapping[str, Any],
    hardware: Mapping[str, Mapping[str, Any]],
    errors: list[str],
) -> dict[str, dict[tuple[str, str], Mapping[str, Any]]]:
    classes = _unique_rows(
        manifest.get("budget_classes"), "budget_class_id", "budget class", errors
    )
    indexed: dict[str, dict[tuple[str, str], Mapping[str, Any]]] = {}
    required_keys = {(hardware_id, profile_id) for hardware_id in HARDWARE_IDS for profile_id in PROFILE_IDS}
    for class_id, row in classes.items():
        budgets: dict[tuple[str, str], Mapping[str, Any]] = {}
        values = row.get("budgets")
        if not isinstance(values, list):
            errors.append(f"budget class {class_id!r} budgets must be a list")
            continue
        for index, budget in enumerate(values):
            if not isinstance(budget, Mapping) or set(budget) != BUDGET_KEYS:
                errors.append(f"budget class {class_id!r} budget {index} shape is not exact")
                continue
            key = (budget.get("hardware_profile_id"), budget.get("profile_id"))
            if key in budgets:
                errors.append(f"budget class {class_id!r} duplicates {key!r}")
            budgets[key] = budget
            for field in BUDGET_KEYS - {"hardware_profile_id", "profile_id"}:
                if not _finite_positive(budget.get(field)):
                    errors.append(f"budget class {class_id!r} {key!r} has invalid {field}")
            if _finite_positive(budget.get("maximum_median_elapsed_seconds")) and _finite_positive(
                budget.get("maximum_p95_elapsed_seconds")
            ) and budget["maximum_median_elapsed_seconds"] > budget["maximum_p95_elapsed_seconds"]:
                errors.append(f"budget class {class_id!r} {key!r} median time exceeds p95")
            if _finite_positive(budget.get("maximum_median_working_set_bytes")) and _finite_positive(
                budget.get("maximum_p95_working_set_bytes")
            ) and budget["maximum_median_working_set_bytes"] > budget["maximum_p95_working_set_bytes"]:
                errors.append(f"budget class {class_id!r} {key!r} median memory exceeds p95")
            hardware_row = hardware.get(str(key[0]))
            if hardware_row and _finite_positive(budget.get("maximum_p95_working_set_bytes")):
                if budget["maximum_p95_working_set_bytes"] > hardware_row.get(
                    "maximum_qualified_working_set_bytes", 0
                ):
                    errors.append(f"budget class {class_id!r} {key!r} exceeds its hardware memory cap")
        if set(budgets) != required_keys:
            missing = sorted(required_keys - set(budgets))
            unexpected = sorted(set(budgets) - required_keys)
            errors.append(
                f"budget class {class_id!r} matrix differs; missing={missing!r}, unexpected={unexpected!r}"
            )
        indexed[class_id] = budgets
    return indexed


def resolve_capability_budget_classes(
    manifest: Mapping[str, Any], registry: Mapping[str, Any], errors: list[str]
) -> dict[tuple[int, str, str, str], str]:
    entries = active_registry_entries(registry)
    active = {_identity(reference): (row, cell) for reference, row, cell in entries}
    resolution = manifest.get("capability_budget_resolution")
    if not isinstance(resolution, Mapping):
        errors.append("capability_budget_resolution must be an object")
        return {}
    defaults = _unique_rows(
        resolution.get("family_defaults"),
        "official_family",
        "capability family default",
        errors,
    )
    active_families = {
        str(row.get("official_family")) for _, row, _ in entries
    }
    if set(defaults) != active_families:
        errors.append("family defaults must cover every and only active registry family")
    overrides: dict[tuple[int, str, str, str], str] = {}
    raw_overrides = resolution.get("exact_overrides")
    if not isinstance(raw_overrides, list):
        errors.append("exact_overrides must be a list")
        raw_overrides = []
    for index, override in enumerate(raw_overrides):
        if not isinstance(override, Mapping) or set(override) != {
            "reference",
            "budget_class_id",
        }:
            errors.append(f"exact override {index} shape is not exact")
            continue
        try:
            identity = _identity(override["reference"])
        except (KeyError, TypeError, ValueError) as exc:
            errors.append(f"exact override {index} reference is invalid: {exc}")
            continue
        if identity not in active:
            errors.append(f"exact override {index} does not reference an active option cell")
        if identity in overrides:
            errors.append(f"exact override {index} duplicates {identity!r}")
        overrides[identity] = str(override.get("budget_class_id"))
    resolved: dict[tuple[int, str, str, str], str] = {}
    for identity, (row, _) in active.items():
        class_id = overrides.get(identity)
        if class_id is None:
            default = defaults.get(str(row.get("official_family")))
            class_id = str(default.get("budget_class_id")) if default else ""
        if not class_id:
            errors.append(f"active option cell {identity!r} has no performance budget class")
        else:
            resolved[identity] = class_id
    return resolved


def validate_contract_documents(
    manifest: Mapping[str, Any],
    registry: Mapping[str, Any],
    contract_schema: Mapping[str, Any],
    measurement_schema: Mapping[str, Any],
) -> dict[str, Any]:
    errors = validate_schema_documents(contract_schema, measurement_schema)
    errors.extend(_json_schema_instance_errors(manifest, contract_schema, "manifest"))
    if set(manifest) != TOP_LEVEL_KEYS:
        errors.append("manifest top-level fields are not exact")
    if manifest.get("schema_version") != 2 or manifest.get("contract_id") != CONTRACT_ID:
        errors.append("manifest V2 identity is invalid")
    if manifest.get("status") != "targets_only_no_measurements":
        errors.append("manifest must identify itself as targets only")
    if manifest.get("measurement_artifacts") != []:
        errors.append("the target contract must not contain measurement claims")

    registry_report = validate_registry_document(registry, check_references=False)
    if not registry_report["passed"]:
        errors.extend(f"registry: {error}" for error in registry_report["errors"])
    binding = manifest.get("registry_binding")
    entries = active_registry_entries(registry)
    references = [reference for reference, _, _ in entries]
    active_rows = {
        row.get("capability_id")
        for _, row, _ in entries
    }
    if not isinstance(binding, Mapping):
        errors.append("registry_binding must be an object")
    else:
        if binding.get("path") != (
            "validation/capabilities/capability_registry_v2.json"
        ):
            errors.append("registry binding path is invalid")
        if binding.get("selection") != "active_option_cells":
            errors.append("registry binding selection is invalid")
        if binding.get("registry_schema_version") != 2:
            errors.append("registry binding schema version must equal 2")
        if binding.get("registry_version") != registry.get("registry_version"):
            errors.append("registry binding version differs from Registry V2")
        if binding.get("registry_sha256") != canonical_sha256(registry):
            errors.append("registry binding SHA-256 differs from Registry V2")
        if binding.get("expected_active_row_count") != len(active_rows):
            errors.append("registry binding active row count differs")
        if binding.get("expected_active_option_cell_count") != len(references):
            errors.append("registry binding active option-cell count differs")
        if binding.get("derived_reference_set_sha256") != canonical_sha256(references):
            errors.append("registry binding derived reference set differs")

    hardware = _unique_rows(
        manifest.get("hardware_profiles"), "hardware_profile_id", "hardware profile", errors
    )
    if tuple(hardware) != HARDWARE_IDS:
        errors.append("hardware profiles must be Standard then Workstation")
    for hardware_id, expected in (
        ("standard_windows_6c16g", (6, 16, 12_884_901_888)),
        ("workstation_windows_12c32g", (12, 32, 25_769_803_776)),
    ):
        row = hardware.get(hardware_id, {})
        actual = (
            row.get("minimum_physical_cores"),
            row.get("minimum_memory_gib"),
            row.get("maximum_qualified_working_set_bytes"),
        )
        if actual != expected:
            errors.append(f"hardware profile {hardware_id!r} differs from the product target")

    profiles, expected_cases = _validate_profiles(manifest, errors)
    ui_budgets = _validate_ui_budgets(manifest, errors)
    _validate_policy(manifest, errors)
    budgets = _validate_and_index_budgets(manifest, hardware, errors)
    resolved = resolve_capability_budget_classes(manifest, registry, errors)
    capability_predicates: dict[tuple[int, str, str, str], frozenset[str]] = {}
    for reference, _, cell in entries:
        predicates: set[str] = set()
        for field in ("supported_model_predicate", "supported_data_predicate"):
            definition = cell.get(field, {})
            if not isinstance(definition, Mapping):
                continue
            for layer in ("official", "quickpls"):
                values = definition.get(layer, [])
                if isinstance(values, list):
                    predicates.update(
                        value for value in values if isinstance(value, str) and value
                    )
        capability_predicates[_identity(reference)] = frozenset(predicates)
    for identity, class_id in resolved.items():
        if class_id not in budgets:
            errors.append(f"active option cell {identity!r} references unknown budget class {class_id!r}")

    return {
        "contract_valid": not errors,
        "error_count": len(errors),
        "errors": errors,
        "active_row_count": len(active_rows),
        "active_option_cell_count": len(references),
        "derived_reference_set_sha256": canonical_sha256(references),
        "resolved_capability_count": len(resolved),
        "resolved_budget_count": len(resolved) * len(HARDWARE_IDS) * len(PROFILE_IDS),
        "expected_capability_current_measurements": len(resolved)
        * len(HARDWARE_IDS)
        * len(expected_cases),
        "expected_ui_current_measurements": len(HARDWARE_IDS) * len(ui_budgets),
        "expected_cases": expected_cases,
        "profiles": profiles,
        "hardware": hardware,
        "budget_index": budgets,
        "resolved_classes": resolved,
        "ui_budget_index": ui_budgets,
        "contract_sha256": canonical_sha256(manifest),
        "measurement_schema": measurement_schema,
        "capability_predicates": capability_predicates,
    }


def load_contract(
    manifest_path: Path | str = DEFAULT_MANIFEST_PATH,
    registry_path: Path | str = DEFAULT_REGISTRY_PATH,
    schema_path: Path | str = DEFAULT_SCHEMA_PATH,
    measurement_schema_path: Path | str = DEFAULT_MEASUREMENT_SCHEMA_PATH,
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any], dict[str, Any]]:
    return (
        load_json(manifest_path),
        load_json(registry_path),
        load_json(schema_path),
        load_json(measurement_schema_path),
    )


def validate_contract(
    manifest_path: Path | str = DEFAULT_MANIFEST_PATH,
    registry_path: Path | str = DEFAULT_REGISTRY_PATH,
    schema_path: Path | str = DEFAULT_SCHEMA_PATH,
    measurement_schema_path: Path | str = DEFAULT_MEASUREMENT_SCHEMA_PATH,
) -> dict[str, Any]:
    documents = load_contract(
        manifest_path, registry_path, schema_path, measurement_schema_path
    )
    return validate_contract_documents(*documents)


def _hardware_errors(
    receipt: Mapping[str, Any], hardware: Mapping[str, Any]
) -> list[str]:
    errors: list[str] = []
    fingerprint = receipt.get("hardware_fingerprint")
    if not isinstance(fingerprint, Mapping):
        return ["hardware_fingerprint must be an object"]
    if set(fingerprint) != {
        "os",
        "architecture",
        "cpu",
        "physical_cores",
        "logical_cores",
        "memory_bytes",
    }:
        errors.append("hardware_fingerprint fields are not exact")
    if fingerprint.get("os") != hardware.get("os") or fingerprint.get(
        "architecture"
    ) != hardware.get("architecture"):
        errors.append("hardware fingerprint OS or architecture differs from the profile")
    physical_cores = fingerprint.get("physical_cores")
    logical_cores = fingerprint.get("logical_cores")
    memory_bytes = fingerprint.get("memory_bytes")
    if (
        not isinstance(physical_cores, int)
        or isinstance(physical_cores, bool)
        or physical_cores < hardware.get("minimum_physical_cores", 0)
    ):
        errors.append("hardware fingerprint has too few physical cores")
    if (
        not isinstance(memory_bytes, int)
        or isinstance(memory_bytes, bool)
        or memory_bytes
        < int(float(hardware.get("minimum_memory_gib", 0)) * 1024**3)
    ):
        errors.append("hardware fingerprint has too little memory")
    if not isinstance(fingerprint.get("cpu"), str) or not fingerprint["cpu"].strip():
        errors.append("hardware fingerprint CPU must be non-empty")
    if (
        not isinstance(logical_cores, int)
        or isinstance(logical_cores, bool)
        or not isinstance(physical_cores, int)
        or isinstance(physical_cores, bool)
        or logical_cores < physical_cores
    ):
        errors.append("logical core count cannot be below physical core count")
    return errors


def _receipt_common_errors(
    receipt: Mapping[str, Any], context: Mapping[str, Any]
) -> list[str]:
    errors = _json_schema_instance_errors(
        receipt, context["measurement_schema"], "receipt"
    )
    kind = receipt.get("document_kind")
    expected_keys = (
        CAPABILITY_RECEIPT_KEYS
        if kind == "capability_performance_measurement"
        else UI_RECEIPT_KEYS
        if kind == "ui_performance_measurement"
        else frozenset()
    )
    if not expected_keys:
        errors.append("document_kind is invalid")
    elif set(receipt) != expected_keys:
        errors.append("measurement receipt fields are not exact")
    if receipt.get("schema_version") != 2:
        errors.append("schema_version must equal 2")
    if receipt.get("contract_id") != CONTRACT_ID:
        errors.append("contract_id is invalid")
    if not _is_sha256(receipt.get("contract_sha256")) or receipt.get(
        "contract_sha256"
    ) != context["contract_sha256"]:
        errors.append("contract_sha256 differs from the target contract")
    if receipt.get("measurement_role") not in {"accepted_baseline", "current"}:
        errors.append("measurement_role is invalid")
    if receipt.get("receipt_complete") is not True:
        errors.append("receipt_complete must be true")
    hardware_id = receipt.get("hardware_profile_id")
    hardware = context["hardware"].get(hardware_id)
    if hardware is None:
        errors.append("hardware_profile_id is unknown")
    else:
        errors.extend(_hardware_errors(receipt, hardware))
    if not isinstance(receipt.get("measurement_id"), str) or not receipt[
        "measurement_id"
    ].strip():
        errors.append("measurement_id must be non-empty")
    captured = receipt.get("captured_at_utc")
    try:
        if not isinstance(captured, str):
            raise ValueError
        parsed = datetime.fromisoformat(captured.replace("Z", "+00:00"))
        if parsed.tzinfo is None:
            raise ValueError
    except ValueError:
        errors.append("captured_at_utc must be an offset-aware timestamp")
    baseline_reference = receipt.get("baseline_reference")
    if baseline_reference is not None and (
        not isinstance(baseline_reference, Mapping)
        or set(baseline_reference) != {"measurement_id", "receipt_sha256"}
        or not isinstance(baseline_reference.get("measurement_id"), str)
        or not baseline_reference["measurement_id"].strip()
        or not _is_sha256(baseline_reference.get("receipt_sha256"))
    ):
        errors.append("baseline_reference shape is invalid")
    if receipt.get("measurement_role") == "accepted_baseline" and receipt.get(
        "baseline_reference"
    ) is not None:
        errors.append("accepted baselines cannot reference another baseline")
    return errors


def _validate_runs(
    receipt: Mapping[str, Any],
    profile: Mapping[str, Any],
    budget: Mapping[str, Any],
    contract: Mapping[str, Any],
    allowed_predicates: frozenset[str] | None = None,
    hardware: Mapping[str, Any] | None = None,
) -> list[str]:
    errors: list[str] = []
    if receipt.get("applicability") == "not_applicable":
        if not isinstance(receipt.get("not_applicable_reason"), str) or not receipt[
            "not_applicable_reason"
        ].strip():
            errors.append("not-applicable measurements require a reason")
        predicates = receipt.get("predicate_references")
        if not isinstance(predicates, list) or not predicates or not all(
            isinstance(value, str) and value.strip() for value in predicates
        ):
            errors.append("not-applicable measurements require predicate references")
        elif allowed_predicates is not None:
            unsupported = sorted(set(predicates) - allowed_predicates)
            if unsupported:
                errors.append(
                    "not-applicable predicate references are not declared by Registry V2: "
                    + ", ".join(unsupported)
                )
        if receipt.get("warmup_runs") or receipt.get("measured_runs"):
            errors.append("not-applicable measurements cannot contain runs")
        if receipt.get("aggregates") is not None:
            errors.append("not-applicable measurements cannot contain aggregates")
        if receipt.get("command") is not None:
            errors.append("not-applicable measurements cannot contain command identity")
        if receipt.get("progress_observation") != {
            "operation_exceeded_threshold": False,
            "real_progress_shown": False,
            "distinct_progress_values": 0,
            "monotonic": True,
        }:
            errors.append("not-applicable measurements need the empty progress observation")
        if receipt.get("cancellation_observation") is not None:
            errors.append("not-applicable measurements cannot contain cancellation evidence")
        if receipt.get("memory_growth_observation") is not None:
            errors.append("not-applicable measurements cannot contain memory-growth evidence")
        if receipt.get("baseline_reference") is not None:
            errors.append("not-applicable measurements cannot reference a baseline")
        if receipt.get("measurement_role") == "accepted_baseline":
            errors.append("an accepted baseline cannot be not applicable")
        return errors
    if receipt.get("applicability") != "measured":
        return ["applicability must be measured or not_applicable"]
    if receipt.get("not_applicable_reason") is not None or receipt.get(
        "predicate_references"
    ) != []:
        errors.append("measured receipts cannot carry not-applicable metadata")
    command = receipt.get("command")
    if not isinstance(command, Mapping):
        errors.append("measured receipts require command identity")
    else:
        if set(command) != {
            "argv",
            "working_directory",
            "build_fingerprint",
            "workload_fingerprint",
            "process_tree_measured",
        }:
            errors.append("command identity fields are not exact")
        if (
            not isinstance(command.get("argv"), list)
            or not command["argv"]
            or not all(isinstance(value, str) and value for value in command["argv"])
            or not isinstance(command.get("working_directory"), str)
            or not command["working_directory"].strip()
            or not _is_sha256(command.get("build_fingerprint"))
            or not _is_sha256(command.get("workload_fingerprint"))
        ):
            errors.append("command identity values are invalid")
        if command.get("process_tree_measured") is not True:
            errors.append("the complete process tree must be measured")
    policy = contract["measurement_policy"]
    warmups = receipt.get("warmup_runs")
    measured = receipt.get("measured_runs")
    if not isinstance(warmups, list) or len(warmups) != policy["warmup_runs"]:
        errors.append("warm-up run count differs from the contract")
        warmups = []
    if not isinstance(measured, list) or len(measured) != policy["measured_runs"]:
        errors.append("measured run count differs from the contract")
        measured = []
    for phase, runs in (("warmup", warmups), ("measured", measured)):
        for index, run in enumerate(runs):
            if not isinstance(run, Mapping):
                errors.append(f"{phase} run {index} must be an object")
                continue
            if set(run) != RUN_KEYS:
                errors.append(f"{phase} run {index} fields are not exact")
            if run.get("phase") != phase or run.get("index") != index:
                errors.append(f"{phase} run {index} identity is invalid")
            if run.get("exit_code") != 0:
                errors.append(f"{phase} run {index} did not exit successfully")
            for field in ("elapsed_seconds", "peak_working_set_bytes", "result_bytes"):
                if not _finite_positive(run.get(field)):
                    errors.append(f"{phase} run {index} has invalid {field}")
            if (
                hardware is not None
                and _finite_positive(run.get("peak_working_set_bytes"))
                and float(run["peak_working_set_bytes"])
                > float(hardware["maximum_qualified_working_set_bytes"])
            ):
                errors.append(
                    f"{phase} run {index} exceeds the hardware working-set ceiling"
                )
            if run.get("orphan_processes") != 0:
                errors.append(f"{phase} run {index} left an orphan process")
            progress = run.get("progress_values")
            if not isinstance(progress, list) or any(
                not _finite_nonnegative(value) or float(value) > 1 for value in progress
            ):
                errors.append(f"{phase} run {index} has invalid progress values")
            elif progress != sorted(progress):
                errors.append(f"{phase} run {index} progress is not monotonic")
    aggregates = receipt.get("aggregates")
    if measured and isinstance(aggregates, Mapping):
        if set(aggregates) != AGGREGATE_KEYS:
            errors.append("aggregate fields are not exact")
        try:
            expected = aggregate_runs(measured)
        except (KeyError, TypeError, ValueError, OverflowError):
            errors.append("aggregates could not be reproduced from invalid measured runs")
        else:
            for field, value in expected.items():
                observed = aggregates.get(field)
                if not _finite_nonnegative(observed) or not math.isclose(
                    float(observed), value, rel_tol=1e-12, abs_tol=1e-9
                ):
                    errors.append(
                        f"aggregate {field} was not reproduced from measured runs"
                    )
        budget_fields = {
            "median_elapsed_seconds": "maximum_median_elapsed_seconds",
            "p95_elapsed_seconds": "maximum_p95_elapsed_seconds",
            "median_working_set_bytes": "maximum_median_working_set_bytes",
            "p95_working_set_bytes": "maximum_p95_working_set_bytes",
            "p95_result_bytes": "maximum_p95_result_bytes",
        }
        for field, budget_field in budget_fields.items():
            if _finite_nonnegative(aggregates.get(field)) and float(
                aggregates[field]
            ) > float(budget[budget_field]):
                errors.append(f"aggregate {field} exceeds its absolute budget")
    else:
        errors.append("measured receipts require aggregates")

    threshold = contract["operation_requirements"]["progress"]
    exceeded = any(
        _finite_nonnegative(run.get("elapsed_seconds"))
        and float(run["elapsed_seconds"]) > threshold["elapsed_threshold_seconds"]
        for run in measured
        if isinstance(run, Mapping)
    )
    progress_observation = receipt.get("progress_observation")
    if progress_observation is not None and (
        not isinstance(progress_observation, Mapping)
        or set(progress_observation)
        != {
            "operation_exceeded_threshold",
            "real_progress_shown",
            "distinct_progress_values",
            "monotonic",
        }
    ):
        errors.append("progress observation fields are not exact")
    if exceeded:
        if not isinstance(progress_observation, Mapping):
            errors.append("operations exceeding two seconds require a progress observation")
        else:
            all_values = {
                float(value)
                for run in measured
                if isinstance(run, Mapping)
                for value in run.get("progress_values", [])
                if _finite_nonnegative(value)
            }
            long_runs_have_progress = all(
                len(
                    {
                        float(value)
                        for value in run.get("progress_values", [])
                        if _finite_nonnegative(value)
                    }
                )
                >= threshold["minimum_distinct_progress_values"]
                for run in measured
                if isinstance(run, Mapping)
                and _finite_nonnegative(run.get("elapsed_seconds"))
                and float(run["elapsed_seconds"])
                > threshold["elapsed_threshold_seconds"]
            )
            if (
                progress_observation.get("operation_exceeded_threshold") is not True
                or progress_observation.get("real_progress_shown") is not True
                or progress_observation.get("monotonic") is not True
                or progress_observation.get("distinct_progress_values") != len(all_values)
                or len(all_values) < threshold["minimum_distinct_progress_values"]
                or not long_runs_have_progress
            ):
                errors.append("progress observation does not prove real monotonic progress")
    cancellation = receipt.get("cancellation_observation")
    if cancellation is not None and (
        not isinstance(cancellation, Mapping)
        or set(cancellation)
        != {
            "terminal_latency_seconds",
            "terminal_state",
            "no_partial_visible_result",
            "no_partial_committed_result",
            "archive_unchanged",
        }
    ):
        errors.append("cancellation observation fields are not exact")
    if profile.get("potentially_long") is True:
        requirement = contract["operation_requirements"]["cancellation"]
        if not isinstance(cancellation, Mapping):
            errors.append("potentially long profiles require a cancellation exercise")
        elif (
            not _finite_nonnegative(cancellation.get("terminal_latency_seconds"))
            or cancellation["terminal_latency_seconds"]
            > requirement["maximum_terminal_latency_seconds"]
            or cancellation.get("terminal_state") != requirement["terminal_state"]
            or any(
                cancellation.get(field) is not True
                for field in (
                    "no_partial_visible_result",
                    "no_partial_committed_result",
                    "archive_unchanged",
                )
            )
        ):
            errors.append("cancellation exercise does not satisfy the atomic one-second contract")
    memory = receipt.get("memory_growth_observation")
    if memory is not None and (
        not isinstance(memory, Mapping)
        or set(memory)
        != {
            "accepted_runs",
            "first_settled_working_set_bytes",
            "last_settled_working_set_bytes",
            "growth_percent",
            "orphan_processes",
        }
    ):
        errors.append("memory-growth observation fields are not exact")
    if profile.get("repeat_memory_gate") is True:
        requirement = contract["operation_requirements"]["memory_growth"]
        if not isinstance(memory, Mapping):
            errors.append("applied profiles require repeated-run memory observation")
        elif (
            not isinstance(memory.get("accepted_runs"), int)
            or isinstance(memory.get("accepted_runs"), bool)
            or memory["accepted_runs"]
            < requirement["minimum_repeated_accepted_runs"]
            or not _finite_nonnegative(memory.get("growth_percent"))
            or memory["growth_percent"] > requirement["maximum_material_growth_percent"]
            or memory.get("orphan_processes") != 0
        ):
            errors.append("repeated-run memory observation exceeds the material-growth rule")
    return errors


def _measurement_key(receipt: Mapping[str, Any]) -> tuple[Any, ...]:
    if receipt.get("document_kind") == "capability_performance_measurement":
        return (
            "capability",
            *_identity(receipt["capability_reference"]),
            receipt.get("hardware_profile_id"),
            receipt.get("profile_id"),
            receipt.get("case_id"),
        )
    return (
        "ui",
        receipt.get("hardware_profile_id"),
        receipt.get("scenario_id"),
    )


def _regression_errors(
    current: Mapping[str, Any],
    baseline: Mapping[str, Any],
    policy: Mapping[str, Any],
) -> list[str]:
    errors: list[str] = []
    current_values = current.get("aggregates")
    baseline_values = baseline.get("aggregates")
    if not isinstance(current_values, Mapping) or not isinstance(baseline_values, Mapping):
        return ["current and baseline aggregates are required for regression checks"]
    thresholds = {
        "median_elapsed_seconds": policy["maximum_runtime_regression_percent"],
        "p95_elapsed_seconds": policy["maximum_runtime_regression_percent"],
        "median_working_set_bytes": policy["maximum_memory_regression_percent"],
        "p95_working_set_bytes": policy["maximum_memory_regression_percent"],
    }
    for field, percentage in thresholds.items():
        previous_value = baseline_values.get(field)
        observed_value = current_values.get(field)
        if not _finite_positive(previous_value) or not _finite_nonnegative(
            observed_value
        ):
            errors.append(f"baseline regression field {field} is invalid")
            continue
        previous = float(previous_value)
        observed = float(observed_value)
        if observed > previous * (1 + float(percentage) / 100):
            errors.append(f"{field} regressed by more than {percentage:g} percent")
    return errors


def _baseline_comparability_errors(
    current: Mapping[str, Any], baseline: Mapping[str, Any]
) -> list[str]:
    errors: list[str] = []
    if current.get("hardware_fingerprint") != baseline.get("hardware_fingerprint"):
        errors.append("current and baseline hardware fingerprints differ")
    if current.get("document_kind") == "capability_performance_measurement":
        current_command = current.get("command")
        baseline_command = baseline.get("command")
        current_workload = (
            current_command.get("workload_fingerprint")
            if isinstance(current_command, Mapping)
            else None
        )
        baseline_workload = (
            baseline_command.get("workload_fingerprint")
            if isinstance(baseline_command, Mapping)
            else None
        )
        if current_workload != baseline_workload:
            errors.append("current and baseline workload fingerprints differ")
    return errors


def validate_measurement_documents(
    manifest: Mapping[str, Any],
    registry: Mapping[str, Any],
    context: Mapping[str, Any],
    receipts: Iterable[Mapping[str, Any]],
) -> dict[str, Any]:
    errors: list[str] = []
    if not context.get("contract_valid"):
        return {
            "measurement_qualification_passed": False,
            "error_count": 1,
            "errors": ["the performance contract is invalid"],
        }
    expected_capability = {
        (
            "capability",
            *identity,
            hardware_id,
            profile_id,
            case_id,
        )
        for identity in context["resolved_classes"]
        for hardware_id in HARDWARE_IDS
        for profile_id, case_id in context["expected_cases"]
    }
    expected_ui = {
        ("ui", hardware_id, scenario_id)
        for hardware_id in HARDWARE_IDS
        for scenario_id in UI_SCENARIO_IDS
    }
    baselines: dict[tuple[Any, ...], Mapping[str, Any]] = {}
    current: dict[tuple[Any, ...], Mapping[str, Any]] = {}
    receipt_by_id: dict[str, Mapping[str, Any]] = {}
    receipt_sha_by_id: dict[str, str] = {}
    for position, receipt in enumerate(receipts, start=1):
        if not isinstance(receipt, Mapping):
            errors.append(f"measurement {position} must be an object")
            continue
        measurement_id = receipt.get("measurement_id")
        if isinstance(measurement_id, str):
            if measurement_id in receipt_by_id:
                errors.append(f"measurement ID {measurement_id!r} is duplicated")
            receipt_by_id[measurement_id] = receipt
            receipt_sha_by_id[measurement_id] = canonical_sha256(receipt)
        try:
            key = _measurement_key(receipt)
        except (KeyError, TypeError, ValueError) as exc:
            errors.append(f"measurement {position} identity is invalid: {exc}")
            continue
        target = baselines if receipt.get("measurement_role") == "accepted_baseline" else current
        if key in target:
            errors.append(f"measurement key {key!r} is duplicated for its role")
        target[key] = receipt

    for key, receipt in [*baselines.items(), *current.items()]:
        prefix = f"{receipt.get('measurement_id', key)!r}"
        item_errors = _receipt_common_errors(receipt, context)
        if key[0] == "capability":
            identity = key[1:5]
            hardware_id = key[5]
            profile_id = key[6]
            if key not in expected_capability:
                item_errors.append("capability measurement key is not required by the contract")
            class_id = context["resolved_classes"].get(identity)
            if receipt.get("budget_class_id") != class_id:
                item_errors.append("budget_class_id differs from registry-derived resolution")
            profile = context["profiles"].get(profile_id)
            budget = context["budget_index"].get(class_id, {}).get(
                (hardware_id, profile_id)
            )
            if profile is None or budget is None:
                item_errors.append("profile or budget could not be resolved")
            else:
                item_errors.extend(
                    _validate_runs(
                        receipt,
                        profile,
                        budget,
                        manifest,
                        context["capability_predicates"].get(identity, frozenset()),
                        context["hardware"].get(hardware_id),
                    )
                )
        elif key[0] == "ui":
            if key not in expected_ui:
                item_errors.append("UI measurement key is not required by the contract")
            item_errors.extend(_validate_ui_receipt(receipt, context))
        else:
            item_errors.append("document_kind is invalid")
        errors.extend(f"measurement {prefix}: {error}" for error in item_errors)

    for key, receipt in current.items():
        if receipt.get("applicability") == "not_applicable":
            continue
        reference = receipt.get("baseline_reference")
        if not isinstance(reference, Mapping):
            errors.append(f"current measurement {receipt.get('measurement_id')!r} lacks its accepted baseline")
            continue
        baseline_id = reference.get("measurement_id")
        baseline = receipt_by_id.get(baseline_id)
        if baseline is None or baseline.get("measurement_role") != "accepted_baseline":
            errors.append(f"current measurement {receipt.get('measurement_id')!r} references an unavailable baseline")
            continue
        if reference.get("receipt_sha256") != receipt_sha_by_id.get(baseline_id):
            errors.append(f"current measurement {receipt.get('measurement_id')!r} baseline digest differs")
        try:
            baseline_key = _measurement_key(baseline)
        except (KeyError, TypeError, ValueError):
            baseline_key = None
        if baseline_key != key:
            errors.append(f"current measurement {receipt.get('measurement_id')!r} baseline key differs")
            continue
        comparability_errors = _baseline_comparability_errors(receipt, baseline)
        errors.extend(
            f"current measurement {receipt.get('measurement_id')!r}: {error}"
            for error in comparability_errors
        )
        if comparability_errors:
            continue
        if key[0] == "capability":
            errors.extend(
                f"current measurement {receipt.get('measurement_id')!r}: {error}"
                for error in _regression_errors(
                    receipt, baseline, manifest["measurement_policy"]
                )
            )
        else:
            errors.extend(
                f"current UI measurement {receipt.get('measurement_id')!r}: {error}"
                for error in _ui_regression_errors(
                    receipt, baseline, manifest["measurement_policy"]
                )
            )

    missing_capability = sorted(expected_capability - set(current), key=repr)
    missing_ui = sorted(expected_ui - set(current), key=repr)
    if missing_capability:
        errors.append(
            f"missing {len(missing_capability)} current capability measurements"
        )
    if missing_ui:
        errors.append(f"missing {len(missing_ui)} current UI measurements")
    return {
        "measurement_qualification_passed": not errors,
        "error_count": len(errors),
        "errors": errors,
        "accepted_baseline_count": len(baselines),
        "current_capability_measurement_count": len(
            set(current) & expected_capability
        ),
        "current_ui_measurement_count": len(set(current) & expected_ui),
        "missing_capability_measurement_count": len(missing_capability),
        "missing_ui_measurement_count": len(missing_ui),
    }


def _ui_aggregate(samples: Sequence[Mapping[str, Any]]) -> dict[str, float | None]:
    result: dict[str, float | None] = {}
    for field in (
        "open_seconds",
        "edit_response_ms",
        "pan_zoom_fps",
        "preflight_seconds",
    ):
        values = [float(sample[field]) for sample in samples if sample.get(field) is not None]
        prefix = field.removesuffix("_ms").removesuffix("_seconds")
        if not values:
            result[f"median_{prefix}"] = None
            result[("p05_" if field == "pan_zoom_fps" else "p95_") + prefix] = None
        else:
            result[f"median_{prefix}"] = statistics.median(values)
            result[("p05_" if field == "pan_zoom_fps" else "p95_") + prefix] = type7_percentile(
                values, 0.05 if field == "pan_zoom_fps" else 0.95
            )
    return result


def _validate_ui_receipt(
    receipt: Mapping[str, Any], context: Mapping[str, Any]
) -> list[str]:
    errors: list[str] = []
    warmups = receipt.get("warmup_runs")
    measured = receipt.get("measured_runs")
    if not isinstance(warmups, list) or len(warmups) != 1:
        errors.append("UI receipt requires one warm-up run")
    if not isinstance(measured, list) or len(measured) != 5:
        errors.append("UI receipt requires five measured runs")
        return errors
    for phase, samples in (("warm-up", warmups), ("measured", measured)):
        if not isinstance(samples, list):
            continue
        for index, sample in enumerate(samples):
            if not isinstance(sample, Mapping) or set(sample) != UI_SAMPLE_KEYS:
                errors.append(f"UI {phase} sample {index} fields are not exact")
                continue
            if any(
                value is not None and not _finite_nonnegative(value)
                for value in sample.values()
            ):
                errors.append(f"UI {phase} sample {index} contains an invalid metric")
    measured_valid = all(
        isinstance(sample, Mapping)
        and set(sample) == UI_SAMPLE_KEYS
        and all(
            value is None or _finite_nonnegative(value) for value in sample.values()
        )
        for sample in measured
    )
    if not measured_valid:
        return errors
    aggregates = receipt.get("aggregates")
    expected = _ui_aggregate(measured)
    if not isinstance(aggregates, Mapping):
        errors.append("UI receipt requires aggregate metrics")
        return errors
    if set(aggregates) != UI_AGGREGATE_KEYS:
        errors.append("UI aggregate fields are not exact")
    for field, value in expected.items():
        observed = aggregates.get(field)
        if value is None:
            if observed is not None:
                errors.append(f"UI aggregate {field} must be null")
        elif not _finite_nonnegative(observed) or not math.isclose(
            float(observed), value, rel_tol=1e-12, abs_tol=1e-9
        ):
            errors.append(f"UI aggregate {field} was not reproduced")
    budget = context["ui_budget_index"].get(receipt.get("scenario_id"), {})
    checks = (
        ("p95_open", "maximum_open_seconds", "maximum"),
        ("p95_edit_response", "maximum_edit_response_p95_ms", "maximum"),
        ("p05_pan_zoom_fps", "minimum_pan_zoom_fps", "minimum"),
        ("p95_preflight", "maximum_preflight_seconds", "maximum"),
    )
    for metric, budget_field, direction in checks:
        limit = budget.get(budget_field)
        observed = aggregates.get(metric)
        if limit is None:
            if observed is not None:
                errors.append(f"UI metric {metric} is not applicable to this scenario")
        elif not _finite_nonnegative(observed):
            errors.append(f"UI metric {metric} is missing")
        elif direction == "maximum" and float(observed) > float(limit):
            errors.append(f"UI metric {metric} exceeds its budget")
        elif direction == "minimum" and float(observed) < float(limit):
            errors.append(f"UI metric {metric} is below its budget")
    return errors


def _ui_regression_errors(
    current: Mapping[str, Any],
    baseline: Mapping[str, Any],
    policy: Mapping[str, Any],
) -> list[str]:
    errors: list[str] = []
    current_values = current.get("aggregates")
    baseline_values = baseline.get("aggregates")
    if not isinstance(current_values, Mapping) or not isinstance(
        baseline_values, Mapping
    ):
        return ["current and baseline UI aggregates are required for regression checks"]
    percentage = float(policy["maximum_runtime_regression_percent"])
    for field in (
        "median_open",
        "p95_open",
        "median_edit_response",
        "p95_edit_response",
        "median_preflight",
        "p95_preflight",
    ):
        previous = baseline_values.get(field)
        observed = current_values.get(field)
        if previous is None and observed is None:
            continue
        if not _finite_positive(previous) or not _finite_nonnegative(observed):
            errors.append(f"UI regression field {field} is invalid")
        elif float(observed) > float(previous) * (1 + percentage / 100):
            errors.append(f"UI {field} regressed by more than {percentage:g} percent")
    for field in ("median_pan_zoom_fps", "p05_pan_zoom_fps"):
        previous = baseline_values.get(field)
        observed = current_values.get(field)
        if previous is None and observed is None:
            continue
        if not _finite_positive(previous) or not _finite_nonnegative(observed):
            errors.append(f"UI regression field {field} is invalid")
        elif float(observed) < float(previous) * (1 - percentage / 100):
            errors.append(f"UI {field} regressed by more than {percentage:g} percent")
    return errors


def validate_measurement_paths(
    paths: Iterable[Path | str],
    manifest_path: Path | str = DEFAULT_MANIFEST_PATH,
    registry_path: Path | str = DEFAULT_REGISTRY_PATH,
    schema_path: Path | str = DEFAULT_SCHEMA_PATH,
    measurement_schema_path: Path | str = DEFAULT_MEASUREMENT_SCHEMA_PATH,
) -> dict[str, Any]:
    manifest, registry, contract_schema, measurement_schema = load_contract(
        manifest_path, registry_path, schema_path, measurement_schema_path
    )
    context = validate_contract_documents(
        manifest, registry, contract_schema, measurement_schema
    )
    receipts = [load_json(path) for path in paths]
    measurements = validate_measurement_documents(manifest, registry, context, receipts)
    return {
        "schema_version": 2,
        "contract_valid": context["contract_valid"],
        "contract_errors": context["errors"],
        **measurements,
        "product_finalization_performance_passed": context["contract_valid"]
        and measurements["measurement_qualification_passed"],
    }


def _contract_cli_report(context: Mapping[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": 2,
        "contract_valid": context["contract_valid"],
        "errors": context["errors"],
        "active_option_cell_count": context["active_option_cell_count"],
        "resolved_budget_count": context["resolved_budget_count"],
        "expected_capability_current_measurements": context[
            "expected_capability_current_measurements"
        ],
        "expected_ui_current_measurements": context[
            "expected_ui_current_measurements"
        ],
        "measurements_verified": False,
        "product_finalization_performance_passed": False,
    }


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST_PATH)
    parser.add_argument("--registry", type=Path, default=DEFAULT_REGISTRY_PATH)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA_PATH)
    parser.add_argument(
        "--measurement-schema", type=Path, default=DEFAULT_MEASUREMENT_SCHEMA_PATH
    )
    parser.add_argument(
        "--contract-only",
        action="store_true",
        help="validate target definitions without treating absent measurements as a pass",
    )
    parser.add_argument("measurements", nargs="*", type=Path)
    args = parser.parse_args(argv)
    manifest, registry, schema, measurement_schema = load_contract(
        args.manifest, args.registry, args.schema, args.measurement_schema
    )
    context = validate_contract_documents(
        manifest, registry, schema, measurement_schema
    )
    if args.contract_only:
        report = _contract_cli_report(context)
        print_report(report)
        return 0 if context["contract_valid"] else 1
    receipts = [load_json(path) for path in args.measurements]
    measurement_report = validate_measurement_documents(
        manifest, registry, context, receipts
    )
    report = {
        "schema_version": 2,
        "contract_valid": context["contract_valid"],
        "contract_errors": context["errors"],
        **measurement_report,
        "product_finalization_performance_passed": context["contract_valid"]
        and measurement_report["measurement_qualification_passed"],
    }
    print_report(report)
    return 0 if report["product_finalization_performance_passed"] else 1


def print_report(report: Mapping[str, Any]) -> None:
    import json

    print(json.dumps(report, indent=2, sort_keys=True, allow_nan=False))


if __name__ == "__main__":
    raise SystemExit(main())
