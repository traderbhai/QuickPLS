#!/usr/bin/env python3
"""Fail-closed validator for the QuickPLS QualificationSpec V2 lane.

This module is deliberately independent from the existing method-promotion
validator.  It can inspect a legacy manifest through a report-only adapter, but
it never translates a legacy promotion state into V2 qualification evidence.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from dataclasses import asdict, dataclass
from datetime import datetime, timedelta, timezone
from functools import lru_cache
from itertools import combinations, product
from pathlib import Path, PurePosixPath
from typing import Any, Literal, Mapping, NotRequired, Sequence, TypedDict

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = (
    ROOT / "validation" / "qualification_v2" / "qualification_spec_v2.schema.json"
)

MANDATORY_COMPLEXITY_PROFILES = frozenset(
    {"micro_exact", "applied", "large", "maximum_axis", "compound_stress"}
)
MANDATORY_SCENARIO_AXES = frozenset(
    {
        "model_topology",
        "measurement_model",
        "data_distribution",
        "missingness",
        "input_type",
        "workload",
    }
)
MANDATORY_ARCHIVE_CORRUPTION_CASES = frozenset(
    {
        "feature_identity",
        "method_version",
        "dataset_fingerprint",
        "checksum",
        "duplicate_entry",
        "malformed_payload",
        "legacy_reinterpretation",
        "interrupted_save",
    }
)
MANDATORY_EXPORT_FORMATS = frozenset({"csv", "xlsx", "html", "svg", "pdf", "png"})
MANDATORY_SEMANTIC_READBACK_FORMATS = frozenset({"csv", "xlsx", "html", "svg", "pdf"})
MANDATORY_WINDOWS_PACKAGES = frozenset({"installed", "portable"})
MANDATORY_WINDOWS_VIEWPORTS = frozenset({"1024x700", "1280x720", "1440x900"})
MANDATORY_WINDOWS_SCALE_FACTORS = frozenset({100, 125, 150, 200})
MANDATORY_CANCELLATION_PHASES = frozenset(
    {"validate", "estimate", "resample", "compare", "export"}
)
MANDATORY_RECEIPT_STAGES = frozenset(
    {
        "contract",
        "kernel",
        "oracle",
        "generative",
        "adversarial",
        "persistence_export",
        "packaged_windows",
        "scale_reliability",
    }
)
MANDATORY_RECEIPT_IDENTITY_FIELDS = frozenset(
    {
        "qualification_id",
        "capability_id",
        "cell_id",
        "method_version",
        "source_set_sha256",
        "scenario_set_sha256",
        "build_fingerprint",
    }
)
COMPUTATIONAL_ORACLE_KINDS = frozenset(
    {"published_fixture", "hand_calculation", "independent_implementation"}
)
CAPABILITY_LINK_FIELDS = frozenset(
    {"registry_schema_version", "capability_id", "cell_id", "capability_version"}
)


class DuplicateKeyError(ValueError):
    """Raised when JSON contains an ambiguous duplicate object key."""


class CapabilityCellRef(TypedDict):
    registry_schema_version: int
    capability_id: str
    capability_version: str
    cell_id: str


class QualificationIdentity(TypedDict):
    qualification_id: str
    method_version: str
    execution_kind: Literal["deterministic", "stochastic", "iterative", "hybrid"]
    potentially_long_running: bool
    spec_frozen_at_utc: str
    capability_cell: CapabilityCellRef
    analytical_method_version: NotRequired[str]


class MigrationSpec(TypedDict):
    source_kind: Literal["native_v2", "qualification_v1_manifest"]
    source_schema_version: int
    source_manifest_path: str | None
    status: Literal["native", "completed", "compatibility_only"]
    unresolved_items: list[str]


class EstimandSpec(TypedDict):
    id: str
    label: str
    definition: str
    unit: str
    output_ids: list[str]


class PreprocessingStepSpec(TypedDict):
    id: str
    order: int
    operation: str
    parameters: dict[str, str | int | float | bool | None]
    applies_to: list[str]


class PredicateSpec(TypedDict):
    id: str
    expression: str
    on_violation: Literal["error", "not_applicable", "warning"]
    diagnostic_code: str


class OracleImplementationSpec(TypedDict):
    name: str
    version: str
    maintainer: str


class OracleSpec(TypedDict):
    id: str
    kind: Literal[
        "primary_literature",
        "published_fixture",
        "hand_calculation",
        "independent_implementation",
    ]
    citation: str
    locator: str
    independence_group: str
    runtime_policy: Literal["no_runtime_dependency", "development_validation_only"]
    implementation: OracleImplementationSpec | None
    covered_estimand_ids: list[str]


class OracleExceptionSpec(TypedDict):
    reason: str
    approved_by: str
    approved_at_utc: str
    compensating_evidence: list[str]


class ScientificContractSpec(TypedDict):
    estimands: list[EstimandSpec]
    preprocessing: list[PreprocessingStepSpec]
    model_predicates: list[PredicateSpec]
    data_predicates: list[PredicateSpec]
    oracles: list[OracleSpec]
    oracle_exception: OracleExceptionSpec | None


class ScenarioAxisValueSpec(TypedDict):
    id: str
    description: str


class ScenarioAxisSpec(TypedDict):
    id: str
    label: str
    values: list[ScenarioAxisValueSpec]


class ComplexityWorkloadSpec(TypedDict):
    rows: int | None
    indicators: int | None
    constructs: int | None
    resamples: int | None
    groups: int | None
    candidate_models: int | None


class ComplexityProfileSpec(TypedDict):
    id: Literal["micro_exact", "applied", "large", "maximum_axis", "compound_stress"]
    description: str
    applicability: Literal["required", "not_applicable"]
    not_applicable_reason: str | None
    workload: ComplexityWorkloadSpec


class MandatoryCombinationSpec(TypedDict):
    id: str
    profile_id: Literal[
        "micro_exact", "applied", "large", "maximum_axis", "compound_stress"
    ]
    coverage: Literal["pairwise", "targeted", "compound"]
    purpose: str
    stressed_dimensions: list[
        Literal[
            "rows",
            "indicators",
            "constructs",
            "resamples",
            "groups",
            "candidate_models",
        ]
    ]
    selections: dict[str, list[str]]


class MonteCarloPolicySpec(TypedDict):
    confidence_level: float
    maximum_half_width: float
    failed_fits_in_denominator: Literal[True]


class ScenarioContractSpec(TypedDict):
    axes: list[ScenarioAxisSpec]
    complexity_profiles: list[ComplexityProfileSpec]
    mandatory_combinations: list[MandatoryCombinationSpec]
    monte_carlo_policy: MonteCarloPolicySpec


ComparisonRule = Literal[
    "exact",
    "abs_relative",
    "matrix_norm",
    "sign_orientation",
    "subspace",
    "label_permutation",
    "monte_carlo_interval",
    "bounded_moment",
]


class OutputComparisonSpec(TypedDict):
    output_id: str
    rule: ComparisonRule
    rationale: str
    absolute_tolerance: NotRequired[float]
    relative_tolerance: NotRequired[float]
    norm: NotRequired[Literal["frobenius", "spectral", "maximum"]]
    elementwise_tolerance: NotRequired[float]
    orientation_keys: NotRequired[list[str]]
    maximum_principal_angle_degrees: NotRequired[float]
    projector_tolerance: NotRequired[float]
    assignment_metric: NotRequired[
        Literal["hungarian_l1", "hungarian_l2", "maximum_overlap"]
    ]
    confidence_level: NotRequired[float]
    maximum_half_width: NotRequired[float]
    acceptance_interval: NotRequired[list[float]]
    statistic: NotRequired[Literal["absolute_bias", "rmse"]]
    maximum: NotRequired[float]
    grouping_keys: NotRequired[list[str]]


class ComparisonContractSpec(TypedDict):
    outputs: list[OutputComparisonSpec]


class HardwareClassSpec(TypedDict):
    id: str
    os_family: Literal["windows"]
    architecture: Literal["x86_64"]
    minimum_logical_cores: int
    minimum_memory_gib: float
    notes: str


class PerformanceBudgetSpec(TypedDict):
    profile_id: str
    hardware_class_id: str
    maximum_elapsed_seconds: float
    maximum_peak_working_set_bytes: int
    maximum_result_bytes: int
    maximum_cancellation_latency_seconds: float


class BaselinePolicySpec(TypedDict):
    warmup_runs: int
    measured_runs: int
    statistic: Literal["median", "p95"]
    maximum_runtime_regression_percent: float
    maximum_memory_regression_percent: float


class PerformanceContractSpec(TypedDict):
    hardware_classes: list[HardwareClassSpec]
    baseline_policy: BaselinePolicySpec
    budgets: list[PerformanceBudgetSpec]


class ArchiveContractSpec(TypedDict):
    current_schema_version: int
    readable_schema_versions: list[int]
    writable_schema_versions: list[int]
    future_schema_policy: Literal["verified_read_only"]
    corruption_cases: list[str]


class ExportContractSpec(TypedDict):
    formats: list[str]
    semantic_readback_formats: list[str]
    canonical_projection_id: str
    same_run_required: Literal[True]
    provenance_required: Literal[True]
    validation_witness_excluded: Literal[True]


class WindowsContractSpec(TypedDict):
    package_kinds: list[Literal["installed", "portable"]]
    viewports: list[str]
    display_scale_percent: list[int]
    offline_required: Literal[True]
    keyboard_only_required: Literal[True]
    accessible_tables_required: Literal[True]
    real_pointer_required: Literal[True]


class CancellationPhaseSpec(TypedDict):
    phase: Literal["validate", "estimate", "resample", "compare", "export"]
    applicability: Literal["required", "not_applicable"]
    not_applicable_reason: str | None


class CancellationContractSpec(TypedDict):
    required_for_potentially_long_operations: Literal[True]
    maximum_latency_seconds: float
    phases: list[CancellationPhaseSpec]
    no_partial_visible_result: Literal[True]
    no_partial_committed_result: Literal[True]
    archive_unchanged: Literal[True]
    same_settings_retry: Literal[True]


class OperationalContractSpec(TypedDict):
    performance: PerformanceContractSpec
    archive: ArchiveContractSpec
    export: ExportContractSpec
    windows: WindowsContractSpec
    cancellation: CancellationContractSpec


class HardwareFingerprint(TypedDict):
    os: str
    architecture: Literal["x86_64"]
    cpu: str
    logical_cores: int
    memory_gib: float


class ReceiptDescriptor(TypedDict):
    role: str
    stage: str
    evidence_class: Literal["qualification", "compatibility_fixture"]
    qualification_id: str
    capability_id: str
    cell_id: str
    method_version: str
    analytical_method_version: NotRequired[str]
    path: str
    size_bytes: int
    sha256: str
    generated_at_utc: str
    source_set_sha256: str
    scenario_set_sha256: str
    qualification_contract_sha256: NotRequired[str]
    build_fingerprint: str
    hardware_fingerprint: HardwareFingerprint


class ReceiptContractSpec(TypedDict):
    hash_algorithm: Literal["sha256"]
    identity_fields: list[str]
    source_descriptors_required: Literal[True]
    hardware_fingerprint_required: Literal[True]
    scenario_set_hash_required: Literal[True]
    payload_contract: NotRequired[dict[str, str | int]]


class EvidenceContractSpec(TypedDict):
    required_roles: list[str]
    receipt_contract: ReceiptContractSpec
    receipts: list[ReceiptDescriptor]


class QualificationSpecV2(TypedDict):
    schema_version: Literal[2]
    identity: QualificationIdentity
    migration: MigrationSpec
    scientific_contract: ScientificContractSpec
    scenario_contract: ScenarioContractSpec
    comparison_contract: ComparisonContractSpec
    operational_contract: OperationalContractSpec
    evidence_contract: EvidenceContractSpec


class LegacyManifestProjection(TypedDict):
    adapter_schema_version: int
    source_kind: str
    source_manifest_path: str
    source_identity: dict[str, Any]
    capability_cell_candidate: CapabilityCellRef | None
    mapped_legacy_contract: dict[str, Any]
    evidence_artifacts: list[dict[str, Any]]
    unresolved_v2_requirements: list[str]
    v2_coverage_status: Literal["unassessed"]
    promotion_authority: Literal[False]
    qualification_ready: bool
    source_declared_state_is_informational_only: bool


@dataclass(frozen=True)
class QualificationValidationReport:
    passed: bool
    qualification_ready: bool
    schema_valid: bool
    semantic_valid: bool
    registry_verified: bool
    receipts_verified: bool
    receipt_payload_contract_id: str | None
    receipt_payload_contract_verified: bool
    qualification_id: str | None
    capability_id: str | None
    cell_id: str | None
    method_version: str | None
    errors: tuple[str, ...]
    warnings: tuple[str, ...]

    def to_dict(self) -> dict[str, Any]:
        value = asdict(self)
        value["errors"] = list(self.errors)
        value["warnings"] = list(self.warnings)
        return value


def _strict_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise DuplicateKeyError(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def _reject_constant(token: str) -> None:
    raise ValueError(f"non-finite JSON value: {token}")


def strict_load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8-sig") as handle:
        return json.load(
            handle,
            object_pairs_hook=_strict_pairs,
            parse_constant=_reject_constant,
        )


def canonical_sha256(value: Any) -> str:
    """Hash canonical UTF-8 JSON used for frozen V2 contract identities."""

    encoded = json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


@lru_cache(maxsize=1)
def _load_schema() -> dict[str, Any]:
    value = strict_load_json(SCHEMA_PATH)
    if not isinstance(value, dict):
        raise ValueError("QualificationSpec V2 schema root must be an object")
    Draft202012Validator.check_schema(value)
    return value


@lru_cache(maxsize=1)
def _schema_validator() -> Draft202012Validator:
    return Draft202012Validator(_load_schema(), format_checker=FormatChecker())


def _pointer(error: Any) -> str:
    parts = [
        str(part).replace("~", "~0").replace("/", "~1") for part in error.absolute_path
    ]
    return "/" + "/".join(parts) if parts else "/"


def _schema_errors(document: Any) -> list[str]:
    try:
        validator = _schema_validator()
    except (
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        DuplicateKeyError,
        ValueError,
    ) as error:
        return [
            f"cannot load QualificationSpec V2 schema: {type(error).__name__}: {error}"
        ]
    return [
        f"schema {_pointer(error)}: {error.message}"
        for error in sorted(
            validator.iter_errors(document),
            key=lambda item: tuple(str(part) for part in item.absolute_path),
        )
    ]


def _unique_rows(rows: Any, label: str, errors: list[str]) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    if not isinstance(rows, list):
        return result
    for index, row in enumerate(rows):
        if not isinstance(row, dict):
            continue
        identifier = row.get("id")
        if not isinstance(identifier, str):
            continue
        if identifier in result:
            errors.append(f"{label} id {identifier!r} is duplicated")
        else:
            result[identifier] = row
    return result


def _parse_utc(value: Any, label: str, errors: list[str]) -> datetime | None:
    if not isinstance(value, str):
        errors.append(f"{label} must be an offset-aware timestamp")
        return None
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        errors.append(f"{label} is not a valid ISO-8601 timestamp")
        return None
    if parsed.tzinfo is None:
        errors.append(f"{label} must include a UTC offset")
        return None
    return parsed.astimezone(timezone.utc)


def _scientific_errors(document: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    contract = document["scientific_contract"]
    estimands = _unique_rows(contract["estimands"], "estimand", errors)
    preprocessing = _unique_rows(
        contract["preprocessing"], "preprocessing step", errors
    )
    model_predicates = _unique_rows(
        contract["model_predicates"], "model predicate", errors
    )
    data_predicates = _unique_rows(
        contract["data_predicates"], "data predicate", errors
    )
    oracles = _unique_rows(contract["oracles"], "oracle", errors)

    output_ids: set[str] = set()
    for estimand in estimands.values():
        for output_id in estimand["output_ids"]:
            if output_id in output_ids:
                errors.append(
                    f"scientific output id {output_id!r} is assigned to multiple estimands"
                )
            output_ids.add(output_id)

    orders = [row["order"] for row in preprocessing.values()]
    if len(orders) != len(set(orders)):
        errors.append("preprocessing step orders must be unique")
    if sorted(orders) != list(range(len(orders))):
        errors.append(
            "preprocessing step orders must form a contiguous zero-based sequence"
        )

    diagnostic_codes = [
        row["diagnostic_code"]
        for row in [*model_predicates.values(), *data_predicates.values()]
    ]
    if len(diagnostic_codes) != len(set(diagnostic_codes)):
        errors.append(
            "model and data predicate diagnostic codes must be globally unique"
        )

    primary = [row for row in oracles.values() if row["kind"] == "primary_literature"]
    if not primary:
        errors.append("oracles must include primary literature")
    for row in primary:
        if row["implementation"] is not None:
            errors.append(
                f"primary-literature oracle {row['id']!r} must not claim an implementation"
            )

    computational = [
        row for row in oracles.values() if row["kind"] in COMPUTATIONAL_ORACLE_KINDS
    ]
    computational_groups = {row["independence_group"] for row in computational}
    independent_implementations = [
        row for row in computational if row["kind"] == "independent_implementation"
    ]
    for row in independent_implementations:
        if row["implementation"] is None:
            errors.append(
                f"independent implementation oracle {row['id']!r} needs versioned implementation metadata"
            )
    implementation_groups: dict[tuple[str, str, str], set[str]] = {}
    for row in independent_implementations:
        implementation = row["implementation"]
        if implementation is None:
            continue
        signature = (
            implementation["name"].casefold(),
            implementation["version"].casefold(),
            implementation["maintainer"].casefold(),
        )
        implementation_groups.setdefault(signature, set()).add(
            row["independence_group"]
        )
    if any(len(groups) > 1 for groups in implementation_groups.values()):
        errors.append(
            "the same implementation identity cannot represent multiple oracle independence groups"
        )
    exception = contract["oracle_exception"]
    if (
        len(computational) < 2
        or len(computational_groups) < 2
        or len(independent_implementations) < 1
    ) and exception is None:
        errors.append(
            "oracles require at least two computational sources in separate independence groups, "
            "including one versioned independent implementation, or an approved exception"
        )
    if (
        exception is not None
        and len(computational) >= 2
        and len(computational_groups) >= 2
        and independent_implementations
    ):
        errors.append(
            "oracle_exception must be null when the normal independence requirement is satisfied"
        )
    if exception is not None:
        approved = _parse_utc(
            exception["approved_at_utc"],
            "oracle_exception approved_at_utc",
            errors,
        )
        frozen = _parse_utc(
            document["identity"]["spec_frozen_at_utc"],
            "spec_frozen_at_utc",
            errors,
        )
        if approved is not None and frozen is not None and approved > frozen:
            errors.append(
                "oracle exception approval cannot postdate the frozen specification"
            )

    estimand_ids = set(estimands)
    primary_coverage: dict[str, int] = {identifier: 0 for identifier in estimand_ids}
    computational_coverage: dict[str, set[str]] = {
        identifier: set() for identifier in estimand_ids
    }
    for oracle in oracles.values():
        unknown = sorted(set(oracle["covered_estimand_ids"]) - estimand_ids)
        if unknown:
            errors.append(
                f"oracle {oracle['id']!r} covers unknown estimands: {', '.join(unknown)}"
            )
        if oracle["kind"] == "primary_literature":
            for estimand_id in oracle["covered_estimand_ids"]:
                if estimand_id in primary_coverage:
                    primary_coverage[estimand_id] += 1
        if oracle["kind"] in COMPUTATIONAL_ORACLE_KINDS:
            for estimand_id in oracle["covered_estimand_ids"]:
                if estimand_id in computational_coverage:
                    computational_coverage[estimand_id].add(
                        oracle["independence_group"]
                    )
    for estimand_id, groups in computational_coverage.items():
        required_groups = 1 if exception is not None else 2
        if len(groups) < required_groups:
            errors.append(
                f"estimand {estimand_id!r} is covered by {len(groups)} computational independence "
                f"groups; {required_groups} required"
            )
    for estimand_id, count in primary_coverage.items():
        if count == 0:
            errors.append(f"estimand {estimand_id!r} lacks a primary-literature oracle")

    if not model_predicates or not data_predicates:
        errors.append("qualification requires explicit model and data predicates")
    if not output_ids:
        errors.append("qualification requires at least one typed scientific output")
    return errors


def _scenario_errors(document: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    contract = document["scenario_contract"]
    axes = _unique_rows(contract["axes"], "scenario axis", errors)
    required_axes = set(MANDATORY_SCENARIO_AXES)
    if document["identity"]["execution_kind"] == "stochastic":
        required_axes.add("workers")
    missing_axes = sorted(required_axes - set(axes))
    if missing_axes:
        errors.append(f"missing mandatory scenario axes: {', '.join(missing_axes)}")
    axis_values: dict[str, set[str]] = {}
    for axis_id, axis in axes.items():
        values = _unique_rows(axis["values"], f"scenario axis {axis_id} value", errors)
        axis_values[axis_id] = set(values)
        if len(values) < 2:
            errors.append(f"scenario axis {axis_id!r} requires at least two values")

    profiles = _unique_rows(
        contract["complexity_profiles"], "complexity profile", errors
    )
    if set(profiles) != MANDATORY_COMPLEXITY_PROFILES:
        missing = sorted(MANDATORY_COMPLEXITY_PROFILES - set(profiles))
        extra = sorted(set(profiles) - MANDATORY_COMPLEXITY_PROFILES)
        errors.append(
            "complexity profiles must be exactly the mandatory set"
            + (f"; missing: {', '.join(missing)}" if missing else "")
            + (f"; unexpected: {', '.join(extra)}" if extra else "")
        )
    for profile in profiles.values():
        reason = profile["not_applicable_reason"]
        if profile["applicability"] == "not_applicable" and not (
            isinstance(reason, str) and reason.strip()
        ):
            errors.append(
                f"complexity profile {profile['id']!r} needs a not-applicable reason"
            )
        if profile["applicability"] == "required" and reason is not None:
            errors.append(
                f"required complexity profile {profile['id']!r} must not have a not-applicable reason"
            )
        if profile["applicability"] == "required" and not any(
            isinstance(value, int) and value > 0
            for value in profile["workload"].values()
        ):
            errors.append(
                f"required complexity profile {profile['id']!r} has no positive workload target"
            )
    ordered_profiles = ("micro_exact", "applied", "large")
    if all(profile_id in profiles for profile_id in ordered_profiles):
        for field in (
            "rows",
            "indicators",
            "constructs",
            "resamples",
            "groups",
            "candidate_models",
        ):
            ordered_values = [
                profiles[profile_id]["workload"][field]
                for profile_id in ordered_profiles
            ]
            comparable = [value for value in ordered_values if value is not None]
            if len(comparable) == len(ordered_values) and comparable != sorted(
                comparable
            ):
                errors.append(
                    f"complexity workload {field!r} must be non-decreasing from micro_exact through large"
                )

    combinations_by_id = _unique_rows(
        contract["mandatory_combinations"], "mandatory combination", errors
    )
    profile_coverage: set[str] = set()
    maximum_axis_dimensions: set[str] = set()
    for row in combinations_by_id.values():
        profile_id = row["profile_id"]
        if profile_id not in profiles:
            errors.append(
                f"mandatory combination {row['id']!r} references unknown profile {profile_id!r}"
            )
        else:
            if profiles[profile_id]["applicability"] == "not_applicable":
                # A migrated specification may retain its historical scenario
                # declaration while explicitly deferring that complexity tier.
                # It is non-binding and must not create a coverage obligation.
                continue
            profile_coverage.add(profile_id)
        stressed = set(row["stressed_dimensions"])
        if profile_id == "maximum_axis":
            if row["coverage"] != "targeted" or len(stressed) != 1:
                errors.append(
                    f"maximum-axis combination {row['id']!r} must target exactly one stressed dimension"
                )
            maximum_axis_dimensions.update(stressed)
        elif profile_id == "compound_stress":
            if row["coverage"] != "compound" or len(stressed) < 2:
                errors.append(
                    f"compound-stress combination {row['id']!r} must name at least two stressed dimensions"
                )
        elif stressed:
            errors.append(
                f"combination {row['id']!r} may name stressed dimensions only for maximum-axis or compound stress"
            )
        selections = row["selections"]
        missing = sorted(set(axes) - set(selections))
        extra = sorted(set(selections) - set(axes))
        if missing:
            errors.append(
                f"mandatory combination {row['id']!r} omits axes: {', '.join(missing)}"
            )
        if extra:
            errors.append(
                f"mandatory combination {row['id']!r} references unknown axes: {', '.join(extra)}"
            )
        for axis_id, selected in selections.items():
            unknown = sorted(set(selected) - axis_values.get(axis_id, set()))
            if unknown:
                errors.append(
                    f"mandatory combination {row['id']!r} selects unknown {axis_id} values: "
                    + ", ".join(unknown)
                )
    missing_profile_combinations = sorted(
        {
            profile_id
            for profile_id, profile in profiles.items()
            if profile["applicability"] == "required"
        }
        - profile_coverage
    )
    if missing_profile_combinations:
        errors.append(
            "required complexity profiles lack mandatory combinations: "
            + ", ".join(missing_profile_combinations)
        )
    if (
        all(profile_id in profiles for profile_id in ("applied", "maximum_axis"))
        and profiles["maximum_axis"]["applicability"] == "required"
    ):
        required_maximum_dimensions = {
            field
            for field, maximum in profiles["maximum_axis"]["workload"].items()
            if maximum is not None
            and profiles["applied"]["workload"][field] is not None
            and maximum > profiles["applied"]["workload"][field]
        }
        if not required_maximum_dimensions:
            errors.append(
                "maximum_axis must exceed the applied workload on at least one dimension"
            )
        unexpected_dimensions = sorted(
            maximum_axis_dimensions - required_maximum_dimensions
        )
        if unexpected_dimensions:
            errors.append(
                "maximum-axis combinations name dimensions that do not exceed applied: "
                + ", ".join(unexpected_dimensions)
            )
        missing_dimensions = sorted(
            required_maximum_dimensions - maximum_axis_dimensions
        )
        if missing_dimensions:
            errors.append(
                "maximum-axis combinations omit stressed dimensions: "
                + ", ".join(missing_dimensions)
            )
    if (
        all(profile_id in profiles for profile_id in ("applied", "compound_stress"))
        and profiles["compound_stress"]["applicability"] == "required"
    ):
        compound_dimensions = {
            field
            for field, compound in profiles["compound_stress"]["workload"].items()
            if compound is not None
            and profiles["applied"]["workload"][field] is not None
            and compound > profiles["applied"]["workload"][field]
        }
        if len(compound_dimensions) < 2:
            errors.append(
                "compound_stress must exceed the applied workload on at least two dimensions"
            )
        for row in combinations_by_id.values():
            if row["profile_id"] != "compound_stress":
                continue
            unexpected = sorted(set(row["stressed_dimensions"]) - compound_dimensions)
            if unexpected:
                errors.append(
                    f"compound-stress combination {row['id']!r} names dimensions that do not exceed applied: "
                    + ", ".join(unexpected)
                )

    pairwise_rows = [
        row for row in combinations_by_id.values() if row["coverage"] == "pairwise"
    ]
    if not pairwise_rows:
        errors.append("mandatory combinations require a pairwise coverage row")
    else:
        for left_axis, right_axis in combinations(sorted(axes), 2):
            for left_value, right_value in product(
                sorted(axis_values[left_axis]), sorted(axis_values[right_axis])
            ):
                if not any(
                    left_value in row["selections"].get(left_axis, [])
                    and right_value in row["selections"].get(right_axis, [])
                    for row in pairwise_rows
                ):
                    errors.append(
                        "pairwise coverage misses "
                        f"{left_axis}={left_value} with {right_axis}={right_value}"
                    )
    compound_rows = [
        row for row in combinations_by_id.values() if row["coverage"] == "compound"
    ]
    if profiles.get("compound_stress", {}).get(
        "applicability"
    ) == "required" and not any(
        row["profile_id"] == "compound_stress" for row in compound_rows
    ):
        errors.append(
            "compound_stress requires an explicit compound mandatory combination"
        )
    return errors


_COMPARISON_PARAMETERS = frozenset(
    {
        "absolute_tolerance",
        "relative_tolerance",
        "norm",
        "elementwise_tolerance",
        "orientation_keys",
        "maximum_principal_angle_degrees",
        "projector_tolerance",
        "assignment_metric",
        "confidence_level",
        "maximum_half_width",
        "acceptance_interval",
        "statistic",
        "maximum",
        "grouping_keys",
    }
)


def _comparison_errors(document: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    estimands = document["scientific_contract"]["estimands"]
    expected_outputs = {
        output_id for estimand in estimands for output_id in estimand["output_ids"]
    }
    rows: dict[str, dict[str, Any]] = {}
    for row in document["comparison_contract"]["outputs"]:
        output_id = row["output_id"]
        if output_id in rows:
            errors.append(f"comparison output id {output_id!r} is duplicated")
        rows[output_id] = row
    if set(rows) != expected_outputs:
        missing = sorted(expected_outputs - set(rows))
        extra = sorted(set(rows) - expected_outputs)
        errors.append(
            "comparison outputs must exactly cover scientific outputs"
            + (f"; missing: {', '.join(missing)}" if missing else "")
            + (f"; unexpected: {', '.join(extra)}" if extra else "")
        )

    requirements: dict[str, tuple[set[str], set[str]]] = {
        "exact": (set(), set()),
        "abs_relative": (
            {"absolute_tolerance", "relative_tolerance"},
            {"absolute_tolerance", "relative_tolerance"},
        ),
        "matrix_norm": (
            {
                "absolute_tolerance",
                "relative_tolerance",
                "norm",
                "elementwise_tolerance",
            },
            {
                "absolute_tolerance",
                "relative_tolerance",
                "norm",
                "elementwise_tolerance",
            },
        ),
        "sign_orientation": (
            {"absolute_tolerance", "relative_tolerance", "orientation_keys"},
            {"absolute_tolerance", "relative_tolerance", "orientation_keys"},
        ),
        "subspace": (
            {"maximum_principal_angle_degrees", "projector_tolerance"},
            {"maximum_principal_angle_degrees", "projector_tolerance"},
        ),
        "label_permutation": (
            {"assignment_metric", "absolute_tolerance", "relative_tolerance"},
            {"assignment_metric", "absolute_tolerance", "relative_tolerance"},
        ),
        "monte_carlo_interval": (
            {"confidence_level", "maximum_half_width", "acceptance_interval"},
            {"confidence_level", "maximum_half_width", "acceptance_interval"},
        ),
        "bounded_moment": (
            {"statistic", "maximum", "grouping_keys"},
            {"statistic", "maximum", "grouping_keys"},
        ),
    }
    for output_id, row in rows.items():
        present = set(row) & _COMPARISON_PARAMETERS
        required, allowed = requirements[row["rule"]]
        missing = sorted(required - present)
        forbidden = sorted(present - allowed)
        if missing:
            errors.append(
                f"comparison {output_id!r} using {row['rule']} lacks parameters: {', '.join(missing)}"
            )
        if forbidden:
            errors.append(
                f"comparison {output_id!r} using {row['rule']} has irrelevant parameters: "
                + ", ".join(forbidden)
            )
        if row["rule"] == "monte_carlo_interval" and "acceptance_interval" in row:
            lower, upper = row["acceptance_interval"]
            if not lower < upper:
                errors.append(
                    f"comparison {output_id!r} acceptance interval must increase"
                )
            policy = document["scenario_contract"]["monte_carlo_policy"]
            if row["confidence_level"] != policy["confidence_level"]:
                errors.append(
                    f"comparison {output_id!r} confidence level must match the scenario Monte Carlo policy"
                )
            if row["maximum_half_width"] > policy["maximum_half_width"]:
                errors.append(
                    f"comparison {output_id!r} half-width exceeds the scenario Monte Carlo policy"
                )
        if row["rule"] == "bounded_moment" and "grouping_keys" in row:
            if row["grouping_keys"] != ["family", "target_id"]:
                errors.append(
                    f"comparison {output_id!r} bounded moment grouping keys must be exactly family, target_id"
                )
    return errors


def _operational_errors(document: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    operational = document["operational_contract"]
    profiles = {
        row["id"]: row for row in document["scenario_contract"]["complexity_profiles"]
    }
    performance = operational["performance"]
    hardware = _unique_rows(performance["hardware_classes"], "hardware class", errors)
    budget_keys: set[tuple[str, str]] = set()
    for budget in performance["budgets"]:
        key = (budget["profile_id"], budget["hardware_class_id"])
        if key in budget_keys:
            errors.append(f"performance budget for {key[0]!r}/{key[1]!r} is duplicated")
        budget_keys.add(key)
        if budget["profile_id"] not in profiles:
            errors.append(
                f"performance budget references unknown profile {budget['profile_id']!r}"
            )
        elif profiles[budget["profile_id"]]["applicability"] != "required":
            errors.append(
                f"performance budget cannot target not-applicable profile {budget['profile_id']!r}"
            )
        if budget["hardware_class_id"] not in hardware:
            errors.append(
                f"performance budget references unknown hardware class {budget['hardware_class_id']!r}"
            )
    for profile_id, profile in profiles.items():
        if profile["applicability"] != "required":
            continue
        for hardware_id in hardware:
            if (profile_id, hardware_id) not in budget_keys:
                errors.append(
                    f"required performance budget is missing for {profile_id!r}/{hardware_id!r}"
                )
    budgets_by_hardware = {
        hardware_id: {
            budget["profile_id"]: budget
            for budget in performance["budgets"]
            if budget["hardware_class_id"] == hardware_id
        }
        for hardware_id in hardware
    }
    for hardware_id, rows in budgets_by_hardware.items():
        for field in (
            "maximum_elapsed_seconds",
            "maximum_peak_working_set_bytes",
            "maximum_result_bytes",
        ):
            values = [
                rows[profile_id][field]
                for profile_id in ("micro_exact", "applied", "large")
                if profile_id in rows
            ]
            if len(values) == 3 and values != sorted(values):
                errors.append(
                    f"performance budget {field!r} must be non-decreasing through large on {hardware_id!r}"
                )
            if (
                all(profile_id in rows for profile_id in ("large", "maximum_axis"))
                and rows["maximum_axis"][field] < rows["large"][field]
            ):
                errors.append(
                    f"maximum-axis budget {field!r} must not be below large on {hardware_id!r}"
                )
            if (
                all(profile_id in rows for profile_id in ("large", "compound_stress"))
                and rows["compound_stress"][field] < rows["large"][field]
            ):
                errors.append(
                    f"compound-stress budget {field!r} must not be below large on {hardware_id!r}"
                )

    archive = operational["archive"]
    current = archive["current_schema_version"]
    readable = set(archive["readable_schema_versions"])
    writable = set(archive["writable_schema_versions"])
    if current not in readable or current not in writable:
        errors.append("current archive schema must be both readable and writable")
    if not writable <= readable:
        errors.append("writable archive schemas must be a subset of readable schemas")
    if any(version > current for version in readable | writable):
        errors.append(
            "declared archive schema versions cannot exceed the current version"
        )
    corruption = set(archive["corruption_cases"])
    if corruption != MANDATORY_ARCHIVE_CORRUPTION_CASES:
        errors.append("archive corruption cases must be the complete mandatory set")

    export = operational["export"]
    formats = set(export["formats"])
    readback = set(export["semantic_readback_formats"])
    if not MANDATORY_EXPORT_FORMATS <= formats:
        errors.append(
            "export contract is missing formats: "
            + ", ".join(sorted(MANDATORY_EXPORT_FORMATS - formats))
        )
    if not readback <= formats:
        errors.append(
            "semantic read-back formats must be a subset of generated formats"
        )
    if not MANDATORY_SEMANTIC_READBACK_FORMATS <= readback:
        errors.append(
            "semantic read-back contract is missing formats: "
            + ", ".join(sorted(MANDATORY_SEMANTIC_READBACK_FORMATS - readback))
        )

    windows = operational["windows"]
    if set(windows["package_kinds"]) != MANDATORY_WINDOWS_PACKAGES:
        errors.append("Windows qualification requires installed and portable packages")
    if not MANDATORY_WINDOWS_VIEWPORTS <= set(windows["viewports"]):
        errors.append("Windows qualification is missing mandatory viewport sizes")
    if not MANDATORY_WINDOWS_SCALE_FACTORS <= set(windows["display_scale_percent"]):
        errors.append(
            "Windows qualification is missing 100, 125, 150, or 200 percent scaling"
        )

    cancellation = operational["cancellation"]
    phases: dict[str, dict[str, Any]] = {}
    for row in cancellation["phases"]:
        phase = row["phase"]
        if phase in phases:
            errors.append(f"cancellation phase {phase!r} is duplicated")
        phases[phase] = row
        reason = row["not_applicable_reason"]
        if row["applicability"] == "not_applicable" and not (
            isinstance(reason, str) and reason.strip()
        ):
            errors.append(f"cancellation phase {phase!r} needs a not-applicable reason")
        if row["applicability"] == "required" and reason is not None:
            errors.append(
                f"required cancellation phase {phase!r} must not have a reason"
            )
    missing_phases = sorted(MANDATORY_CANCELLATION_PHASES - set(phases))
    if missing_phases:
        errors.append(
            f"cancellation contract omits phases: {', '.join(missing_phases)}"
        )
    if document["identity"]["potentially_long_running"] and not any(
        row["applicability"] == "required" for row in phases.values()
    ):
        errors.append(
            "potentially long-running methods require at least one cancellable phase"
        )
    if document["identity"]["potentially_long_running"] and (
        "estimate" not in phases or phases["estimate"]["applicability"] != "required"
    ):
        errors.append(
            "potentially long-running estimators require cancellation during estimation"
        )
    for budget in performance["budgets"]:
        if (
            budget["maximum_cancellation_latency_seconds"]
            > cancellation["maximum_latency_seconds"]
        ):
            errors.append(
                f"performance budget {budget['profile_id']!r}/{budget['hardware_class_id']!r} "
                "allows cancellation slower than the product cancellation contract"
            )
    return errors


def _evidence_errors(document: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    evidence = document["evidence_contract"]
    required_roles = set(evidence["required_roles"])
    receipt_roles: set[str] = set()
    receipt_stages: set[str] = set()
    receipt_paths: set[str] = set()
    frozen = _parse_utc(
        document["identity"]["spec_frozen_at_utc"], "spec_frozen_at_utc", errors
    )
    identity = document["identity"]
    cell = identity["capability_cell"]
    payload_contract = evidence["receipt_contract"].get("payload_contract")
    if payload_contract is not None:
        try:
            from validation.general_sem_rank0_receipt_payload_v1 import (
                ANALYTICAL_METHOD_BY_CELL,
                CONTRACT_DESCRIPTOR,
                RANK0_REQUIRED_ROLES,
                ROLE_STAGE,
                qualification_contract_sha256,
            )
        except ModuleNotFoundError:
            from general_sem_rank0_receipt_payload_v1 import (
                ANALYTICAL_METHOD_BY_CELL,
                CONTRACT_DESCRIPTOR,
                RANK0_REQUIRED_ROLES,
                ROLE_STAGE,
                qualification_contract_sha256,
            )
        if payload_contract != CONTRACT_DESCRIPTOR:
            errors.append(
                "receipt payload contract is not the supported exact Rank 0 contract"
            )
        analytical = ANALYTICAL_METHOD_BY_CELL.get(
            (cell["cell_id"], identity["method_version"])
        )
        if (
            analytical is None
            or identity.get("analytical_method_version") != analytical
        ):
            errors.append(
                "strict Rank 0 qualification identity must bind its exact analytical method"
            )
        if required_roles != RANK0_REQUIRED_ROLES:
            missing = sorted(RANK0_REQUIRED_ROLES - required_roles)
            extra = sorted(required_roles - RANK0_REQUIRED_ROLES)
            errors.append(
                "strict Rank 0 required_roles must equal the exact ten-role contract"
                + (f"; missing: {', '.join(missing)}" if missing else "")
                + (f"; unexpected: {', '.join(extra)}" if extra else "")
            )
        if (
            set(document["operational_contract"]["export"]["semantic_readback_formats"])
            != MANDATORY_EXPORT_FORMATS
        ):
            errors.append(
                "strict Rank 0 semantic read-back formats must equal csv/xlsx/html/svg/pdf/png"
            )
        expected_qualification_contract = qualification_contract_sha256(document)
        for receipt in evidence["receipts"]:
            if receipt.get("analytical_method_version") != analytical:
                errors.append(
                    f"receipt {receipt['role']!r} does not bind the exact analytical method"
                )
            if (
                receipt.get("qualification_contract_sha256")
                != expected_qualification_contract
            ):
                errors.append(
                    f"receipt {receipt['role']!r} does not bind the immutable QualificationSpec contract"
                )
            if ROLE_STAGE.get(receipt["role"]) != receipt["stage"]:
                errors.append(
                    f"receipt {receipt['role']!r} stage is not the exact Rank 0 role mapping"
                )
    expected_identity = {
        "qualification_id": identity["qualification_id"],
        "capability_id": cell["capability_id"],
        "cell_id": cell["cell_id"],
        "method_version": identity["method_version"],
    }
    for receipt in evidence["receipts"]:
        role = receipt["role"]
        if role in receipt_roles:
            errors.append(f"receipt role {role!r} is duplicated")
        receipt_roles.add(role)
        receipt_stages.add(receipt["stage"])
        if receipt["path"] in receipt_paths:
            errors.append(
                f"receipt path {receipt['path']!r} is reused by multiple roles"
            )
        receipt_paths.add(receipt["path"])
        generated = _parse_utc(
            receipt["generated_at_utc"], f"receipt {role!r} generated_at_utc", errors
        )
        if frozen is not None and generated is not None and generated < frozen:
            errors.append(f"receipt {role!r} predates the frozen QualificationSpec")
        if generated is not None and generated > datetime.now(timezone.utc) + timedelta(
            minutes=5
        ):
            errors.append(f"receipt {role!r} has an implausible future timestamp")
        for field, expected in expected_identity.items():
            if receipt[field] != expected:
                errors.append(
                    f"receipt {role!r} {field} mismatch: "
                    f"expected {expected!r}, found {receipt[field]!r}"
                )
    receipts = evidence["receipts"]
    shared_fields = ["source_set_sha256", "scenario_set_sha256", "build_fingerprint"]
    if payload_contract is not None:
        shared_fields.append("qualification_contract_sha256")
    for field in shared_fields:
        values = {receipt.get(field) for receipt in receipts}
        if len(values) > 1:
            errors.append(f"immutable receipts disagree on {field}")
    expected_scenarios = canonical_sha256(document["scenario_contract"])
    hardware_classes = document["operational_contract"]["performance"][
        "hardware_classes"
    ]
    for receipt in receipts:
        if receipt["scenario_set_sha256"] != expected_scenarios:
            errors.append(
                f"receipt {receipt['role']!r} scenario_set_sha256 does not bind the frozen scenario contract"
            )
        if receipt["stage"] in {"packaged_windows", "scale_reliability"}:
            fingerprint = receipt["hardware_fingerprint"]
            if not fingerprint["os"].casefold().startswith("windows"):
                errors.append(
                    f"receipt {receipt['role']!r} must be captured on Windows for stage {receipt['stage']!r}"
                )
            if not any(
                fingerprint["logical_cores"] >= hardware["minimum_logical_cores"]
                and fingerprint["memory_gib"] >= hardware["minimum_memory_gib"]
                and fingerprint["architecture"] == hardware["architecture"]
                for hardware in hardware_classes
            ):
                errors.append(
                    f"receipt {receipt['role']!r} does not satisfy a declared performance hardware class"
                )
    migration_ready = document["migration"]["status"] in {"native", "completed"}
    if migration_ready:
        non_qualification_roles = sorted(
            receipt["role"]
            for receipt in receipts
            if receipt["evidence_class"] != "qualification"
        )
        if non_qualification_roles:
            errors.append(
                "qualification-ready migration cannot use compatibility fixture receipts: "
                + ", ".join(non_qualification_roles)
            )
        if required_roles != receipt_roles:
            missing = sorted(required_roles - receipt_roles)
            extra = sorted(receipt_roles - required_roles)
            errors.append(
                "immutable receipt roles must exactly match required roles"
                + (f"; missing: {', '.join(missing)}" if missing else "")
                + (f"; unexpected: {', '.join(extra)}" if extra else "")
            )
        if receipt_stages != MANDATORY_RECEIPT_STAGES:
            errors.append(
                "immutable receipts must cover every qualification stage"
                + f"; missing: {', '.join(sorted(MANDATORY_RECEIPT_STAGES - receipt_stages))}"
            )
        identities = set(evidence["receipt_contract"]["identity_fields"])
        expected_identities = MANDATORY_RECEIPT_IDENTITY_FIELDS
        if payload_contract is not None:
            expected_identities = expected_identities | {
                "qualification_contract_sha256"
            }
        if identities != expected_identities:
            errors.append("receipt identity fields must be the complete mandatory set")
    return errors


def _migration_errors(document: dict[str, Any]) -> list[str]:
    migration = document["migration"]
    errors: list[str] = []
    identity = document["identity"]
    if identity["capability_cell"]["capability_version"] != identity["method_version"]:
        errors.append("capability link version must equal identity.method_version")
    unresolved = migration["unresolved_items"]
    if migration["status"] == "compatibility_only" and not unresolved:
        errors.append(
            "compatibility-only migration must name unresolved V2 requirements"
        )
    if migration["status"] in {"native", "completed"} and unresolved:
        errors.append(
            "qualification-ready migration cannot retain unresolved V2 requirements"
        )
    if migration["source_kind"] == "native_v2":
        if (
            migration["status"] != "native"
            or migration["source_manifest_path"] is not None
            or migration["source_schema_version"] != 2
        ):
            errors.append(
                "native V2 specs require schema version 2, native status, and no legacy source manifest"
            )
    if migration["source_kind"] == "qualification_v1_manifest":
        if (
            migration["status"] == "native"
            or migration["source_manifest_path"] is None
            or migration["source_schema_version"] != 1
        ):
            errors.append(
                "legacy migrations require schema version 1, a source manifest, and non-native status"
            )
    return errors


def _registry_links(
    registry: Mapping[str, Any],
) -> list[tuple[Mapping[str, Any], Any, Mapping[str, Any]]]:
    """Return authoritative ``capabilities[].option_cells[]`` links."""

    capabilities = registry.get("capabilities")
    rows: list[tuple[Mapping[str, Any], Any, Mapping[str, Any]]] = []
    if isinstance(capabilities, list):
        for capability in capabilities:
            if not isinstance(capability, Mapping) or not isinstance(
                capability.get("option_cells"), list
            ):
                continue
            for cell in capability["option_cells"]:
                if not isinstance(cell, Mapping):
                    continue
                specification = cell.get("qualification_spec")
                links = (
                    specification.get("links")
                    if isinstance(specification, Mapping)
                    else None
                )
                if (
                    not isinstance(links, list)
                    or len(links) != 1
                    or not isinstance(links[0], Mapping)
                ):
                    continue
                rows.append((links[0], capability.get("capability_id"), cell))
    return rows


def _registry_errors(document: dict[str, Any], registry: Any) -> list[str]:
    if not isinstance(registry, Mapping):
        return ["Capability Registry V2 root must be an object"]
    if registry.get("registry_schema_version") != 2:
        return ["Capability Registry link requires registry_schema_version 2"]
    expected = document["identity"]["capability_cell"]
    errors: list[str] = []
    if set(expected) != CAPABILITY_LINK_FIELDS:
        errors.append(
            "QualificationSpec capability link must use the exact Registry V2 fields"
        )
    if expected["capability_version"] != document["identity"]["method_version"]:
        errors.append(
            "QualificationSpec capability_version must equal identity.method_version"
        )
    links = _registry_links(registry)
    for link, owner_id, cell in links:
        if set(link) != CAPABILITY_LINK_FIELDS:
            errors.append("Capability Registry contains a malformed qualification link")
        if link.get("capability_id") != owner_id:
            errors.append(
                f"Capability Registry link {link.get('cell_id')!r} does not match its containing capability"
            )
        if (
            cell.get("capability_id") != owner_id
            or cell.get("capability_id") != link.get("capability_id")
            or cell.get("cell_id") != link.get("cell_id")
            or cell.get("capability_version") != link.get("capability_version")
        ):
            errors.append(
                "Capability Registry option-cell identity does not match its qualification link"
            )
    matches = [
        link
        for link, _owner_id, _cell in links
        if set(link) == CAPABILITY_LINK_FIELDS and dict(link) == dict(expected)
    ]
    authoritative = {
        tuple(link.get(field) for field in sorted(CAPABILITY_LINK_FIELDS))
        for link, _owner_id, _cell in links
        if set(link) == CAPABILITY_LINK_FIELDS
    }
    compatibility: set[tuple[Any, ...]] = set()
    for capability in registry.get("capabilities", []):
        if not isinstance(capability, Mapping):
            continue
        row_links = capability.get("qualification_links")
        if not isinstance(row_links, list):
            errors.append(
                "Capability Registry compatibility qualification_links must be a list"
            )
            continue
        for link in row_links:
            if not isinstance(link, Mapping) or set(link) != CAPABILITY_LINK_FIELDS:
                errors.append(
                    "Capability Registry contains a malformed compatibility qualification link"
                )
                continue
            compatibility.add(
                tuple(link.get(field) for field in sorted(CAPABILITY_LINK_FIELDS))
            )
    if compatibility != authoritative:
        errors.append(
            "Capability Registry compatibility qualification_links drift from option cells"
        )
    if len(matches) != 1:
        errors.append(
            "Capability Registry must contain exactly one qualification link "
            f"{dict(expected)!r}; found {len(matches)}"
        )
    return errors


def _safe_repository_path(root: Path, relative: str) -> Path | None:
    pure = PurePosixPath(relative)
    if pure.is_absolute() or ".." in pure.parts or "\\" in relative:
        return None
    candidate = (root / Path(*pure.parts)).resolve()
    try:
        candidate.relative_to(root.resolve())
    except ValueError:
        return None
    return candidate


def _stream_sha256(path: Path) -> tuple[int, str]:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as handle:
        while chunk := handle.read(1024 * 1024):
            size += len(chunk)
            digest.update(chunk)
    return size, digest.hexdigest()


def _receipt_verification_errors(document: dict[str, Any], root: Path) -> list[str]:
    errors: list[str] = []
    migration_ready = document["migration"]["status"] in {"native", "completed"}
    payload_contract = document["evidence_contract"]["receipt_contract"].get(
        "payload_contract"
    )
    strict_rank0_payload = payload_contract is not None
    for receipt in document["evidence_contract"]["receipts"]:
        path = _safe_repository_path(root, receipt["path"])
        if path is None:
            errors.append(f"receipt {receipt['role']!r} path escapes the repository")
            continue
        if not path.is_file():
            errors.append(f"receipt {receipt['role']!r} is missing: {receipt['path']}")
            continue
        try:
            size, digest = _stream_sha256(path)
        except OSError as error:
            errors.append(
                f"receipt {receipt['role']!r} cannot be read: {type(error).__name__}: {error}"
            )
            continue
        if size != receipt["size_bytes"]:
            errors.append(f"receipt {receipt['role']!r} size mismatch")
        if digest != receipt["sha256"]:
            errors.append(f"receipt {receipt['role']!r} SHA-256 mismatch")
        if strict_rank0_payload:
            if path.suffix.casefold() != ".json":
                errors.append(
                    f"receipt {receipt['role']!r} strict payload must be JSON"
                )
                continue
            try:
                from validation.general_sem_rank0_receipt_payload_v1 import (
                    validate_payload_path,
                )
            except ModuleNotFoundError:
                from general_sem_rank0_receipt_payload_v1 import validate_payload_path
            payload_errors = validate_payload_path(
                path,
                receipt=receipt,
                specification=document,
                repository_root=root,
            )
            errors.extend(
                f"receipt {receipt['role']!r} payload: {problem}"
                for problem in payload_errors
            )
            continue
        if (
            migration_ready
            and path.suffix.casefold() == ".json"
            and size <= 1024 * 1024
        ):
            try:
                payload = strict_load_json(path)
            except (
                OSError,
                UnicodeError,
                json.JSONDecodeError,
                DuplicateKeyError,
                ValueError,
            ):
                payload = None
            if isinstance(payload, Mapping) and payload.get("fixture_only") is True:
                errors.append(
                    f"receipt {receipt['role']!r} is explicitly fixture-only and cannot promote a cell"
                )
    return errors


def validate_spec_document(
    document: Any,
    *,
    repository_root: Path | None = None,
    verify_receipts: bool = False,
    registry_document: Any | None = None,
    require_registry: bool = False,
) -> dict[str, Any]:
    """Validate a V2 specification and report qualification readiness.

    ``passed`` means schema and semantic validity.  ``qualification_ready`` is
    stricter: migration must be complete and immutable receipts plus the linked
    registry cell must be verified during this call.
    """

    errors = _schema_errors(document)
    schema_valid = not errors and isinstance(document, dict)
    warnings: list[str] = []
    registry_verified = False
    receipts_verified = False
    receipt_payload_contract_id: str | None = None
    receipt_payload_contract_verified = False
    if schema_valid:
        selected_payload_contract = document["evidence_contract"][
            "receipt_contract"
        ].get("payload_contract")
        if isinstance(selected_payload_contract, Mapping):
            value = selected_payload_contract.get("contract_id")
            receipt_payload_contract_id = value if isinstance(value, str) else None
        errors.extend(_migration_errors(document))
        errors.extend(_scientific_errors(document))
        errors.extend(_scenario_errors(document))
        errors.extend(_comparison_errors(document))
        errors.extend(_operational_errors(document))
        errors.extend(_evidence_errors(document))

        if registry_document is not None:
            registry_problems = _registry_errors(document, registry_document)
            errors.extend(registry_problems)
            registry_verified = not registry_problems
        elif require_registry:
            errors.append(
                "strict qualification requires the linked Capability Registry V2 document"
            )
        else:
            warnings.append(
                "Capability Registry V2 link was not verified in this validation pass"
            )

        if verify_receipts:
            if repository_root is None:
                errors.append("receipt verification requires repository_root")
            else:
                receipt_problems = _receipt_verification_errors(
                    document, repository_root
                )
                errors.extend(receipt_problems)
                receipts_verified = not receipt_problems
                receipt_payload_contract_verified = bool(
                    selected_payload_contract is not None and not receipt_problems
                )
        else:
            warnings.append(
                "immutable receipt bytes were not verified in this validation pass"
            )

    semantic_valid = schema_valid and not errors
    migration_ready = bool(
        schema_valid and document["migration"]["status"] in {"native", "completed"}
    )
    qualification_ready = (
        semantic_valid and migration_ready and registry_verified and receipts_verified
    )
    identity = document.get("identity", {}) if isinstance(document, dict) else {}
    cell = identity.get("capability_cell", {}) if isinstance(identity, dict) else {}
    report = QualificationValidationReport(
        passed=semantic_valid,
        qualification_ready=qualification_ready,
        schema_valid=schema_valid,
        semantic_valid=semantic_valid,
        registry_verified=registry_verified,
        receipts_verified=receipts_verified,
        receipt_payload_contract_id=receipt_payload_contract_id,
        receipt_payload_contract_verified=receipt_payload_contract_verified,
        qualification_id=identity.get("qualification_id")
        if isinstance(identity, dict)
        else None,
        capability_id=cell.get("capability_id") if isinstance(cell, dict) else None,
        cell_id=cell.get("cell_id") if isinstance(cell, dict) else None,
        method_version=identity.get("method_version")
        if isinstance(identity, dict)
        else None,
        errors=tuple(errors),
        warnings=tuple(warnings),
    )
    return report.to_dict()


def validate_spec_path(
    path: Path,
    *,
    repository_root: Path | None = None,
    verify_receipts: bool = False,
    registry_path: Path | None = None,
    require_registry: bool = False,
) -> dict[str, Any]:
    try:
        document = strict_load_json(path)
        registry = (
            strict_load_json(registry_path) if registry_path is not None else None
        )
    except (
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        DuplicateKeyError,
        ValueError,
    ) as error:
        return QualificationValidationReport(
            passed=False,
            qualification_ready=False,
            schema_valid=False,
            semantic_valid=False,
            registry_verified=False,
            receipts_verified=False,
            receipt_payload_contract_id=None,
            receipt_payload_contract_verified=False,
            qualification_id=None,
            capability_id=None,
            cell_id=None,
            method_version=None,
            errors=(f"{type(error).__name__}: {error}",),
            warnings=(),
        ).to_dict()
    return validate_spec_document(
        document,
        repository_root=repository_root,
        verify_receipts=verify_receipts,
        registry_document=registry,
        require_registry=require_registry,
    )


_LEGACY_UNRESOLVED = (
    "capability_cell.registry_mapping_confirmation",
    "scientific_contract.explicit_estimand_output_map",
    "scientific_contract.ordered_preprocessing",
    "scientific_contract.typed_model_predicates",
    "scientific_contract.typed_data_predicates",
    "scientific_contract.two_computational_oracles_per_estimand",
    "scenario_contract.mandatory_axes_and_pairwise_coverage",
    "scenario_contract.complexity_profiles_and_compound_stress",
    "comparison_contract.per_output_rules",
    "operational_contract.performance_budgets_and_hardware",
    "operational_contract.cross_format_semantic_readback",
    "operational_contract.windows_scaling_and_workload_cancellation",
    "evidence_contract.v2_stage_receipts",
)


def adapt_v1_manifest_report(
    document: Any,
    *,
    source_manifest_path: str,
    capability_cell: CapabilityCellRef | None = None,
) -> LegacyManifestProjection:
    """Read a V1 manifest into a non-promoting compatibility report.

    The result is intentionally *not* a QualificationSpec V2 document.  Legacy
    ``declared_state`` and evidence are reported verbatim and cannot satisfy V2
    scenario, oracle, comparison, scale, or receipt requirements.
    """

    if not isinstance(document, dict):
        raise ValueError("legacy method-promotion manifest root must be an object")
    for key in (
        "schema_version",
        "feature",
        "claim",
        "scientific_contract",
        "product_contract",
        "qualification",
    ):
        if key not in document:
            raise ValueError(f"legacy method-promotion manifest is missing {key}")
    if document["schema_version"] != 1:
        raise ValueError(
            f"legacy adapter supports schema_version 1, found {document['schema_version']!r}"
        )
    if not isinstance(source_manifest_path, str) or not source_manifest_path.strip():
        raise ValueError("legacy adapter requires a non-empty source_manifest_path")
    feature = document["feature"]
    if not isinstance(feature, dict):
        raise ValueError("legacy feature must be an object")
    for key in ("id", "method_version", "catalogue_snapshot_date"):
        if not isinstance(feature.get(key), str) or not feature[key]:
            raise ValueError(f"legacy feature is missing {key}")
    qualification = document["qualification"]
    if not isinstance(qualification, dict):
        raise ValueError("legacy qualification must be an object")
    scientific_contract = document["scientific_contract"]
    if not isinstance(scientific_contract, dict):
        raise ValueError("legacy scientific_contract must be an object")
    product_contract = document["product_contract"]
    if not isinstance(product_contract, dict):
        raise ValueError("legacy product_contract must be an object")

    if capability_cell is not None:
        required_cell_fields = {
            "registry_schema_version",
            "capability_id",
            "capability_version",
            "cell_id",
        }
        if set(capability_cell) != required_cell_fields:
            raise ValueError("capability_cell must contain the exact V2 link fields")
        if capability_cell["registry_schema_version"] != 2:
            raise ValueError("capability_cell registry_schema_version must be 2")
        for key in required_cell_fields - {"registry_schema_version"}:
            if not isinstance(capability_cell[key], str) or not capability_cell[key]:
                raise ValueError(f"capability_cell {key} must be a non-empty string")
    evidence_artifacts: list[dict[str, Any]] = []
    evidence = qualification.get("evidence", {})
    if isinstance(evidence, dict):
        for stage, artifacts in evidence.items():
            if not isinstance(artifacts, list):
                continue
            for artifact in artifacts:
                if isinstance(artifact, dict):
                    evidence_artifacts.append(
                        {
                            "legacy_stage": stage,
                            "path": artifact.get("path"),
                            "roles": artifact.get("roles"),
                        }
                    )
    return {
        "adapter_schema_version": 1,
        "source_kind": "qualification_v1_manifest",
        "source_manifest_path": source_manifest_path,
        "source_identity": {
            "feature_id": feature["id"],
            "method_version": feature["method_version"],
            "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
            "declared_state": qualification.get("declared_state"),
            "target_state": qualification.get("target_state"),
        },
        "capability_cell_candidate": capability_cell,
        "mapped_legacy_contract": {
            "claim": document["claim"],
            "equations": scientific_contract.get("equations", []),
            "references": scientific_contract.get("references", []),
            "simulations": scientific_contract.get("simulations", []),
            "boundaries": scientific_contract.get("boundaries", []),
            "product_contract": product_contract,
        },
        "evidence_artifacts": evidence_artifacts,
        "unresolved_v2_requirements": list(_LEGACY_UNRESOLVED),
        "v2_coverage_status": "unassessed",
        "promotion_authority": False,
        "qualification_ready": False,
        "source_declared_state_is_informational_only": True,
    }


def adapt_v1_manifest_path(
    path: Path, *, capability_cell: CapabilityCellRef | None = None
) -> LegacyManifestProjection:
    document = strict_load_json(path)
    try:
        relative = path.resolve().relative_to(ROOT.resolve()).as_posix()
    except ValueError:
        relative = path.as_posix()
    return adapt_v1_manifest_report(
        document,
        source_manifest_path=relative,
        capability_cell=capability_cell,
    )


def _main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    validate = subparsers.add_parser(
        "validate", help="Validate one QualificationSpec V2 JSON file"
    )
    validate.add_argument("path", type=Path)
    validate.add_argument("--repository-root", type=Path, default=ROOT)
    validate.add_argument("--registry", type=Path)
    validate.add_argument(
        "--strict",
        action="store_true",
        help="Verify receipts and require the registry link",
    )
    adapt = subparsers.add_parser(
        "adapt-v1", help="Print a report-only legacy compatibility projection"
    )
    adapt.add_argument("path", type=Path)
    args = parser.parse_args(argv)

    if args.command == "adapt-v1":
        try:
            report: dict[str, Any] = adapt_v1_manifest_path(args.path)
        except (
            OSError,
            UnicodeError,
            json.JSONDecodeError,
            DuplicateKeyError,
            ValueError,
        ) as error:
            report = {
                "qualification_ready": False,
                "errors": [f"{type(error).__name__}: {error}"],
            }
        print(json.dumps(report, indent=2, sort_keys=True, allow_nan=False))
        return 0 if not report.get("errors") else 1

    report = validate_spec_path(
        args.path,
        repository_root=args.repository_root,
        verify_receipts=args.strict,
        registry_path=args.registry,
        require_registry=args.strict,
    )
    print(json.dumps(report, indent=2, sort_keys=True, allow_nan=False))
    return (
        0 if (report["qualification_ready"] if args.strict else report["passed"]) else 1
    )


if __name__ == "__main__":
    sys.exit(_main())
