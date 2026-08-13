#!/usr/bin/env python3
"""Fail-closed validator for QuickPLS method-promotion manifests.

Only Python's standard library is used.  A manifest may describe planned work
at ``absent`` without evidence, but every listed evidence artifact must be a
strict JSON report that binds the current capability identity and exact source
bytes.  Promotion remains sequential and is derived from evidence on disk.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import sys
from collections import Counter
from datetime import date, datetime, timezone
from pathlib import Path
from typing import Any, Iterable
from urllib.parse import urlparse


VALIDATION_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = VALIDATION_DIR.parent
MANIFEST_DIR = VALIDATION_DIR / "methods"
SCHEMA_PATH = MANIFEST_DIR / "method_promotion_manifest.schema.json"

SCHEMA_RELATIVE_PATH = "validation/methods/method_promotion_manifest.schema.json"
VALIDATOR_RELATIVE_PATH = "validation/method_promotion_manifest.py"
FOCUSED_TEST_RELATIVE_PATH = "validation/test_method_promotion_manifest.py"

STATE_ORDER = (
    "absent",
    "engine_only",
    "archive_qualified",
    "native_qualified",
    "release_qualified",
)

REQUIRED_STAGE_ROLES = {
    "engine_only": frozenset(
        {
            "method_spec",
            "independent_reference",
            "simulation_report",
            "boundary_report",
        }
    ),
    "archive_qualified": frozenset({"persistence_report"}),
    "native_qualified": frozenset({"frontend_report", "export_report"}),
    "release_qualified": frozenset({"method_audit", "packaged_acceptance"}),
}
ALL_EVIDENCE_ROLES = frozenset().union(*REQUIRED_STAGE_ROLES.values())

REQUIRED_BOUNDARY_CATEGORIES = frozenset(
    {"data_pathology", "unsupported_scope", "metamorphic", "determinism", "tamper"}
)
REQUIRED_TAMPER_CATEGORIES = frozenset(
    {
        "feature_identity",
        "method_version",
        "dataset_fingerprint",
        "checksum",
        "malformed_payload",
        "legacy_reinterpretation",
    }
)
REQUIRED_PACKAGED_STEPS = frozenset(
    {
        "setup",
        "invalid_setup_blocked",
        "execute",
        "inspect_results",
        "export",
        "save",
        "close",
        "reopen_same_run",
        "cleanup",
    }
)
REQUIRED_VIEWPORTS = frozenset({"1024x700", "1280x720", "1440x900"})
REQUIRED_AUDIT_IDENTITIES = frozenset(
    {"passed", "feature_id", "method_version", "catalogue_snapshot_date"}
)
COMPUTATIONAL_REFERENCE_KINDS = frozenset(
    {"published_fixture", "hand_calculation", "independent_python", "independent_r"}
)
SHA256 = re.compile(r"^[0-9a-f]{64}$")
POINTER_PART = re.compile(r"~(?:0|1)|[^~]+")


class DuplicateKeyError(ValueError):
    """Raised when JSON contains an ambiguous duplicate object key."""


def _strict_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateKeyError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def strict_load_json(path: Path) -> Any:
    """Load UTF-8 JSON while rejecting duplicate keys and non-finite values."""

    def reject_constant(value: str) -> None:
        raise ValueError(f"non-finite JSON number: {value}")

    with path.open("r", encoding="utf-8-sig") as handle:
        return json.load(
            handle,
            object_pairs_hook=_strict_pairs,
            parse_constant=reject_constant,
        )


def json_pointer(document: Any, pointer: str) -> Any:
    """Resolve an RFC 6901 pointer without accepting malformed escapes."""

    if pointer == "":
        return document
    if not isinstance(pointer, str) or not pointer.startswith("/"):
        raise ValueError("JSON pointer must start with '/'")
    current = document
    for raw_part in pointer[1:].split("/"):
        if "~" in raw_part and "".join(POINTER_PART.findall(raw_part)) != raw_part:
            raise ValueError(f"invalid JSON pointer escape in {pointer!r}")
        part = raw_part.replace("~1", "/").replace("~0", "~")
        if isinstance(current, list):
            if not part.isdigit():
                raise KeyError(pointer)
            current = current[int(part)]
        elif isinstance(current, dict):
            current = current[part]
        else:
            raise KeyError(pointer)
    return current


def _join_location(path: str, part: str | int) -> str:
    escaped = str(part).replace("~", "~0").replace("/", "~1")
    return f"{path}/{escaped}" if path else f"/{escaped}"


def _schema_type_matches(value: Any, expected: str) -> bool:
    if expected == "object":
        return isinstance(value, dict)
    if expected == "array":
        return isinstance(value, list)
    if expected == "string":
        return isinstance(value, str)
    if expected == "boolean":
        return isinstance(value, bool)
    if expected == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if expected == "number":
        return (
            isinstance(value, (int, float))
            and not isinstance(value, bool)
            and math.isfinite(float(value))
        )
    if expected == "null":
        return value is None
    return False


def _format_matches(value: str, format_name: str) -> bool:
    try:
        if format_name == "date":
            return date.fromisoformat(value).isoformat() == value
        if format_name == "date-time":
            parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
            return parsed.tzinfo is not None
        if format_name == "uri":
            parsed = urlparse(value)
            return bool(parsed.scheme and parsed.netloc)
    except ValueError:
        return False
    return True


def _resolve_schema_reference(root_schema: dict[str, Any], reference: str) -> Any:
    if not reference.startswith("#"):
        raise ValueError(f"external schema reference is not supported: {reference}")
    return json_pointer(root_schema, reference[1:])


def _validate_schema_instance(
    value: Any,
    schema: dict[str, Any],
    root_schema: dict[str, Any],
    path: str = "",
) -> list[str]:
    """Validate the JSON-Schema subset used by the factory schema."""

    if "$ref" in schema:
        try:
            target = _resolve_schema_reference(root_schema, schema["$ref"])
        except (KeyError, TypeError, ValueError) as error:
            return [f"{path or '/'}: invalid schema reference: {error}"]
        return _validate_schema_instance(value, target, root_schema, path)

    if "oneOf" in schema:
        outcomes = [
            _validate_schema_instance(value, branch, root_schema, path)
            for branch in schema["oneOf"]
        ]
        matches = sum(not outcome for outcome in outcomes)
        if matches != 1:
            return [f"{path or '/'}: value must match exactly one schema branch"]
        return []

    errors: list[str] = []
    if "const" in schema and value != schema["const"]:
        errors.append(f"{path or '/'}: value must equal {schema['const']!r}")
    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{path or '/'}: value is not one of {schema['enum']!r}")

    expected_type = schema.get("type")
    if expected_type is not None:
        expected_types = [expected_type] if isinstance(expected_type, str) else expected_type
        if not any(_schema_type_matches(value, item) for item in expected_types):
            errors.append(f"{path or '/'}: expected type {expected_type!r}")
            return errors

    if isinstance(value, dict):
        required = schema.get("required", [])
        for key in required:
            if key not in value:
                errors.append(f"{_join_location(path, key)}: required property is missing")
        properties = schema.get("properties", {})
        if schema.get("additionalProperties") is False:
            for key in value:
                if key not in properties:
                    errors.append(f"{_join_location(path, key)}: additional property is not allowed")
        for key, child_schema in properties.items():
            if key in value:
                errors.extend(
                    _validate_schema_instance(
                        value[key], child_schema, root_schema, _join_location(path, key)
                    )
                )

    if isinstance(value, list):
        minimum_items = schema.get("minItems")
        if isinstance(minimum_items, int) and len(value) < minimum_items:
            errors.append(f"{path or '/'}: requires at least {minimum_items} items")
        if schema.get("uniqueItems") is True:
            canonical = [
                json.dumps(item, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
                for item in value
            ]
            if len(canonical) != len(set(canonical)):
                errors.append(f"{path or '/'}: array items must be unique")
        item_schema = schema.get("items")
        if isinstance(item_schema, dict):
            for index, item in enumerate(value):
                errors.extend(
                    _validate_schema_instance(
                        item, item_schema, root_schema, _join_location(path, index)
                    )
                )

    if isinstance(value, str):
        minimum_length = schema.get("minLength")
        if isinstance(minimum_length, int) and len(value) < minimum_length:
            errors.append(f"{path or '/'}: string is shorter than {minimum_length}")
        pattern = schema.get("pattern")
        if isinstance(pattern, str) and re.search(pattern, value) is None:
            errors.append(f"{path or '/'}: value does not match {pattern!r}")
        format_name = schema.get("format")
        if isinstance(format_name, str) and not _format_matches(value, format_name):
            errors.append(f"{path or '/'}: value is not a valid {format_name}")

    if isinstance(value, (int, float)) and not isinstance(value, bool):
        minimum = schema.get("minimum")
        maximum = schema.get("maximum")
        exclusive_minimum = schema.get("exclusiveMinimum")
        if minimum is not None and value < minimum:
            errors.append(f"{path or '/'}: value must be at least {minimum}")
        if maximum is not None and value > maximum:
            errors.append(f"{path or '/'}: value must be no more than {maximum}")
        if exclusive_minimum is not None and value <= exclusive_minimum:
            errors.append(f"{path or '/'}: value must be greater than {exclusive_minimum}")
    return errors


def _unique_ids(rows: Any, label: str, errors: list[str]) -> set[str]:
    if not isinstance(rows, list):
        return set()
    identifiers = [row.get("id") for row in rows if isinstance(row, dict)]
    duplicates = sorted(
        str(identifier)
        for identifier, count in Counter(identifiers).items()
        if identifier is not None and count > 1
    )
    if duplicates:
        errors.append(f"{label} IDs must be unique: {', '.join(duplicates)}")
    return {identifier for identifier in identifiers if isinstance(identifier, str)}


def _safe_repository_path(root: Path, relative: Any) -> Path | None:
    if not isinstance(relative, str) or not relative or "\\" in relative:
        return None
    candidate = Path(relative)
    if candidate.is_absolute() or ".." in candidate.parts:
        return None
    resolved_root = root.resolve()
    resolved = (resolved_root / candidate).resolve()
    try:
        resolved.relative_to(resolved_root)
    except ValueError:
        return None
    return resolved


def _normalize_relative_path(path: Path, root: Path) -> str | None:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return None


def _parse_utc(value: Any) -> datetime:
    if not isinstance(value, str):
        raise ValueError("timestamp must be a string")
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        raise ValueError("timestamp must include an offset")
    return parsed.astimezone(timezone.utc)


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _role_source_errors(source_requirements: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    method_specs = source_requirements.get("method_spec", [])
    if any(
        not path.startswith("docs/methods/") or not path.endswith(".md")
        for path in method_specs
    ):
        errors.append("method_spec sources must be Markdown files under docs/methods/")

    independent = source_requirements.get("independent_reference", [])
    if not any(
        path.startswith("validation/") and Path(path).suffix in {".py", ".R"}
        for path in independent
    ):
        errors.append("independent_reference sources must include a validation Python or R file")

    for role in ("simulation_report", "boundary_report", "persistence_report"):
        paths = source_requirements.get(role, [])
        if not any(path.startswith("validation/") for path in paths):
            errors.append(f"{role} sources must include a validation/ path")

    frontend = source_requirements.get("frontend_report", [])
    if not any(path.startswith(("src/", "validation/")) for path in frontend):
        errors.append("frontend_report sources must include a src/ or validation/ path")

    export = source_requirements.get("export_report", [])
    if not any(path.startswith(("src/", "validation/")) for path in export):
        errors.append("export_report sources must include a src/ or validation/ path")

    audits = source_requirements.get("method_audit", [])
    if not any(
        path.startswith("validation/") and path.endswith("_audit.py") for path in audits
    ):
        errors.append("method_audit sources must include a validation/*_audit.py file")

    packaged = source_requirements.get("packaged_acceptance", [])
    if not any(
        path.startswith("validation/run_") and path.endswith(".ps1")
        for path in packaged
    ):
        errors.append("packaged_acceptance sources must include a validation/run_*.ps1 file")

    governance_paths = {
        SCHEMA_RELATIVE_PATH,
        VALIDATOR_RELATIVE_PATH,
        FOCUSED_TEST_RELATIVE_PATH,
    }
    for role, paths in source_requirements.items():
        overlap = sorted(set(paths) & governance_paths)
        if overlap:
            errors.append(
                f"{role} role-specific sources cannot substitute governance files: "
                + ", ".join(overlap)
            )
    return errors


def _semantic_errors(
    document: dict[str, Any], actual_manifest_path: str | None
) -> list[str]:
    errors: list[str] = []
    governance = document.get("governance", {})
    feature = document.get("feature", {})
    scientific = document.get("scientific_contract", {})
    product = document.get("product_contract", {})
    qualification = document.get("qualification", {})

    if governance.get("schema_path") != SCHEMA_RELATIVE_PATH:
        errors.append(f"governance.schema_path must be {SCHEMA_RELATIVE_PATH}")
    if governance.get("validator_path") != VALIDATOR_RELATIVE_PATH:
        errors.append(f"governance.validator_path must be {VALIDATOR_RELATIVE_PATH}")
    if governance.get("focused_test_path") != FOCUSED_TEST_RELATIVE_PATH:
        errors.append(f"governance.focused_test_path must be {FOCUSED_TEST_RELATIVE_PATH}")
    manifest_declared = governance.get("manifest_path")
    if not (
        isinstance(manifest_declared, str)
        and manifest_declared.startswith("validation/methods/")
        and manifest_declared.endswith(".manifest.json")
    ):
        errors.append("governance.manifest_path must name validation/methods/*.manifest.json")
    if actual_manifest_path is not None and manifest_declared != actual_manifest_path:
        errors.append(
            f"governance.manifest_path mismatch: expected {actual_manifest_path!r}, "
            f"found {manifest_declared!r}"
        )

    references = scientific.get("references", [])
    reference_ids = _unique_ids(references, "reference", errors)
    reference_kinds = {
        row.get("kind") for row in references if isinstance(row, dict)
    }
    independence_groups = {
        row.get("independence_group")
        for row in references
        if isinstance(row, dict) and isinstance(row.get("independence_group"), str)
    }
    if "primary_paper" not in reference_kinds:
        errors.append("references must include at least one primary_paper")
    if not (reference_kinds & COMPUTATIONAL_REFERENCE_KINDS):
        errors.append("references must include an independent computational reference")
    if len(independence_groups) < 2:
        errors.append("references must contain at least two independence groups")

    equations = scientific.get("equations", [])
    _unique_ids(equations, "equation", errors)
    for equation in equations if isinstance(equations, list) else []:
        if isinstance(equation, dict) and equation.get("source_reference_id") not in reference_ids:
            errors.append(
                f"equation {equation.get('id')!r} cites unknown reference "
                f"{equation.get('source_reference_id')!r}"
            )

    _unique_ids(scientific.get("simulations", []), "simulation", errors)
    boundaries = scientific.get("boundaries", [])
    _unique_ids(boundaries, "boundary", errors)
    boundary_categories = {
        row.get("category") for row in boundaries if isinstance(row, dict)
    }
    missing_boundaries = sorted(REQUIRED_BOUNDARY_CATEGORIES - boundary_categories)
    if missing_boundaries:
        errors.append(f"missing boundary categories: {', '.join(missing_boundaries)}")

    persistence = product.get("persistence", {})
    tamper_tests = persistence.get("tamper_tests", [])
    tamper_categories = [
        row.get("category") for row in tamper_tests if isinstance(row, dict)
    ]
    duplicate_tamper = sorted(
        str(category)
        for category, count in Counter(tamper_categories).items()
        if category is not None and count > 1
    )
    if duplicate_tamper:
        errors.append(f"tamper-test categories must be unique: {', '.join(duplicate_tamper)}")
    missing_tamper = sorted(REQUIRED_TAMPER_CATEGORIES - set(tamper_categories))
    if missing_tamper:
        errors.append(f"missing persistence tamper tests: {', '.join(missing_tamper)}")

    packaged = product.get("packaged", {})
    workflow_steps = set(packaged.get("workflow_steps", []))
    missing_steps = sorted(REQUIRED_PACKAGED_STEPS - workflow_steps)
    if missing_steps:
        errors.append(f"missing packaged workflow steps: {', '.join(missing_steps)}")
    viewports = set(packaged.get("viewports", []))
    if viewports != REQUIRED_VIEWPORTS:
        errors.append(
            "packaged viewports must be exactly " + ", ".join(sorted(REQUIRED_VIEWPORTS))
        )
    if feature.get("method_kind") == "stochastic":
        if packaged.get("cancellation_required") is not True:
            errors.append("stochastic methods must require packaged cancellation")
        if "cancel_retry" not in workflow_steps:
            errors.append("stochastic methods must include the cancel_retry packaged step")

    audit_identities = set(product.get("audit", {}).get("exact_identity_fields", []))
    if audit_identities != REQUIRED_AUDIT_IDENTITIES:
        errors.append(
            "audit identity fields must be exactly "
            + ", ".join(sorted(REQUIRED_AUDIT_IDENTITIES))
        )

    source_requirements = qualification.get("source_requirements", {})
    if isinstance(source_requirements, dict):
        errors.extend(_role_source_errors(source_requirements))

    declared = qualification.get("declared_state")
    target = qualification.get("target_state")
    if declared in STATE_ORDER and target in STATE_ORDER:
        if STATE_ORDER.index(target) < STATE_ORDER.index(declared):
            errors.append("target_state cannot be lower than declared_state")
    return errors


def _verify_source_descriptors(
    descriptors: Any,
    repository_root: Path,
    required_paths: set[str],
) -> tuple[bool, list[str]]:
    errors: list[str] = []
    if not isinstance(descriptors, list) or not descriptors:
        return False, ["source_artifacts must be a non-empty descriptor list"]

    seen: set[str] = set()
    for index, descriptor in enumerate(descriptors):
        prefix = f"source_artifacts[{index}]"
        if not isinstance(descriptor, dict) or set(descriptor) != {"path", "size", "sha256"}:
            errors.append(f"{prefix} must contain exactly path, size, and sha256")
            continue
        relative = descriptor.get("path")
        size = descriptor.get("size")
        digest = descriptor.get("sha256")
        if relative in seen:
            errors.append(f"{prefix} duplicates source path {relative!r}")
            continue
        if isinstance(relative, str):
            seen.add(relative)
        source_path = _safe_repository_path(repository_root, relative)
        if source_path is None:
            errors.append(f"{prefix} has unsafe source path {relative!r}")
            continue
        if not source_path.is_file():
            errors.append(f"{prefix} source file is missing: {relative}")
            continue
        if not isinstance(size, int) or isinstance(size, bool) or size < 0:
            errors.append(f"{prefix}.size must be a non-negative integer")
        elif source_path.stat().st_size != size:
            errors.append(
                f"{prefix} size mismatch for {relative}: expected {size}, "
                f"found {source_path.stat().st_size}"
            )
        if not isinstance(digest, str) or SHA256.fullmatch(digest) is None:
            errors.append(f"{prefix}.sha256 must be a lowercase SHA-256 digest")
        else:
            actual_digest = _sha256_file(source_path)
            if actual_digest != digest:
                errors.append(
                    f"{prefix} SHA-256 mismatch for {relative}: expected {digest}, "
                    f"found {actual_digest}"
                )

    missing = sorted(required_paths - seen)
    if missing:
        errors.append("source_artifacts do not bind required sources: " + ", ".join(missing))
    return not errors, errors


def _verify_artifact(
    artifact: dict[str, Any],
    document: dict[str, Any],
    repository_root: Path,
    expected_identity: dict[str, Any],
) -> tuple[bool, list[str]]:
    errors: list[str] = []
    relative = artifact.get("path")
    report_path = _safe_repository_path(repository_root, relative)
    if report_path is None:
        return False, [f"unsafe evidence path: {relative!r}"]
    if not report_path.is_file():
        return False, [f"evidence file is missing: {relative}"]

    verification = artifact.get("verification", {})
    if verification.get("kind") != "identity_report":
        return False, [f"{relative}: verification kind must be identity_report"]

    try:
        report = strict_load_json(report_path)
        if not isinstance(report, dict):
            raise TypeError("report root must be an object")
        pointers = verification.get("identity_pointers", {})
        if set(pointers) != REQUIRED_AUDIT_IDENTITIES:
            errors.append(
                f"{relative}: identity pointers must bind exactly passed, feature_id, "
                "method_version, and catalogue_snapshot_date"
            )
        else:
            for field, pointer in pointers.items():
                actual = json_pointer(report, pointer)
                expected = expected_identity[field]
                if actual != expected:
                    errors.append(
                        f"{relative}: {field} identity mismatch "
                        f"(expected {expected!r}, found {actual!r})"
                    )

        generated_at = json_pointer(report, verification["generated_at_pointer"])
        report_time = _parse_utc(generated_at)
        frozen_time = _parse_utc(document["governance"]["contract_frozen_at_utc"])
        if report_time < frozen_time:
            errors.append(
                f"{relative}: report is stale ({generated_at} precedes contract freeze "
                f"{document['governance']['contract_frozen_at_utc']})"
            )

        source_descriptors = json_pointer(
            report, verification["source_artifacts_pointer"]
        )
        governance = document["governance"]
        required_sources = {
            governance["manifest_path"],
            governance["schema_path"],
            governance["validator_path"],
            governance["focused_test_path"],
        }
        source_requirements = document["qualification"]["source_requirements"]
        for role in artifact.get("roles", []):
            required_sources.update(source_requirements[role])
        _, descriptor_errors = _verify_source_descriptors(
            source_descriptors, repository_root, required_sources
        )
        errors.extend(f"{relative}: {error}" for error in descriptor_errors)
    except (
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        DuplicateKeyError,
        TypeError,
        KeyError,
        IndexError,
        ValueError,
    ) as error:
        errors.append(f"{relative}: {type(error).__name__}: {error}")
    return not errors, errors


def _evaluate_evidence(
    document: dict[str, Any], repository_root: Path, verify_evidence: bool
) -> tuple[str, list[dict[str, Any]], list[str]]:
    feature = document["feature"]
    evidence = document["qualification"]["evidence"]
    expected_identity = {
        "passed": True,
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
    }
    stage_results: list[dict[str, Any]] = []
    errors: list[str] = []
    derived = "absent"
    previous_passed = True

    for stage in STATE_ORDER[1:]:
        artifacts = evidence.get(stage, [])
        role_counts: Counter[str] = Counter()
        artifact_results: list[dict[str, Any]] = []
        for artifact in artifacts:
            roles = artifact.get("roles", [])
            role_counts.update(roles)
            wrong_roles = sorted(set(roles) - REQUIRED_STAGE_ROLES[stage])
            if wrong_roles:
                errors.append(
                    f"{stage} contains roles assigned to another stage: {', '.join(wrong_roles)}"
                )
            if verify_evidence:
                passed, artifact_errors = _verify_artifact(
                    artifact, document, repository_root, expected_identity
                )
            else:
                passed, artifact_errors = True, []
            artifact_results.append(
                {
                    "path": artifact.get("path"),
                    "roles": roles,
                    "passed": passed,
                    "errors": artifact_errors,
                }
            )
            errors.extend(artifact_errors)

        duplicate_roles = sorted(role for role, count in role_counts.items() if count > 1)
        if duplicate_roles:
            errors.append(f"{stage} evidence repeats roles: {', '.join(duplicate_roles)}")
        missing_roles = sorted(REQUIRED_STAGE_ROLES[stage] - set(role_counts))
        if artifacts and missing_roles:
            errors.append(f"{stage} evidence is missing roles: {', '.join(missing_roles)}")
        stage_passed = (
            previous_passed
            and bool(artifacts)
            and not missing_roles
            and not duplicate_roles
            and all(row["passed"] for row in artifact_results)
        )
        stage_results.append(
            {
                "state": stage,
                "passed": stage_passed,
                "missing_roles": missing_roles,
                "artifacts": artifact_results,
            }
        )
        if stage_passed:
            derived = stage
        elif artifacts and not previous_passed:
            errors.append(f"{stage} evidence is orphaned before prior-stage qualification")
        previous_passed = stage_passed
    return derived, stage_results, errors


def validate_manifest_document(
    document: Any,
    repository_root: Path = REPOSITORY_ROOT,
    *,
    manifest_path: Path | None = None,
    verify_evidence: bool = True,
) -> dict[str, Any]:
    """Validate one parsed manifest and derive its evidence-backed state."""

    try:
        schema = strict_load_json(SCHEMA_PATH)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        return {
            "passed": False,
            "declared_state": None,
            "derived_state": "absent",
            "errors": [f"cannot load manifest schema: {type(error).__name__}: {error}"],
        }
    if not isinstance(schema, dict):
        return {
            "passed": False,
            "declared_state": None,
            "derived_state": "absent",
            "errors": ["manifest schema root must be an object"],
        }

    schema_errors = _validate_schema_instance(document, schema, schema)
    if schema_errors or not isinstance(document, dict):
        return {
            "passed": False,
            "declared_state": None,
            "derived_state": "absent",
            "stage_results": [],
            "errors": schema_errors or ["manifest root must be an object"],
        }

    actual_manifest_path = (
        _normalize_relative_path(manifest_path, repository_root)
        if manifest_path is not None
        else None
    )
    errors = _semantic_errors(document, actual_manifest_path)
    derived, stage_results, evidence_errors = _evaluate_evidence(
        document, repository_root.resolve(), verify_evidence
    )
    errors.extend(evidence_errors)
    declared = document["qualification"]["declared_state"]
    if STATE_ORDER.index(declared) > STATE_ORDER.index(derived):
        errors.append(
            f"declared {declared} but current evidence derives only {derived}"
        )

    return {
        "passed": not errors,
        "feature_id": document["feature"]["id"],
        "method_version": document["feature"]["method_version"],
        "catalogue_snapshot_date": document["feature"]["catalogue_snapshot_date"],
        "declared_state": declared,
        "target_state": document["qualification"]["target_state"],
        "derived_state": derived,
        "stage_results": stage_results,
        "errors": errors,
    }


def validate_manifest(
    path: Path,
    repository_root: Path = REPOSITORY_ROOT,
    *,
    verify_evidence: bool = True,
) -> dict[str, Any]:
    """Load and validate one manifest without trusting its declared state."""

    try:
        document = strict_load_json(path)
    except (OSError, UnicodeError, json.JSONDecodeError, DuplicateKeyError, ValueError) as error:
        return {
            "passed": False,
            "path": str(path),
            "declared_state": None,
            "derived_state": "absent",
            "errors": [f"cannot load manifest: {type(error).__name__}: {error}"],
        }
    result = validate_manifest_document(
        document,
        repository_root,
        manifest_path=path,
        verify_evidence=verify_evidence,
    )
    result["path"] = str(path)
    return result


def discover_manifests(paths: Iterable[Path] | None = None) -> list[Path]:
    """Return explicit manifests or every factory manifest in stable order."""

    if paths:
        return sorted(path.resolve() for path in paths)
    return sorted(MANIFEST_DIR.glob("*.manifest.json"))


def validate_all(
    paths: Iterable[Path] | None = None,
    repository_root: Path = REPOSITORY_ROOT,
) -> dict[str, Any]:
    manifests = discover_manifests(paths)
    if not manifests:
        return {
            "passed": False,
            "manifest_count": 0,
            "manifests": [],
            "errors": ["no method-promotion manifests found"],
        }
    results = [validate_manifest(path, repository_root) for path in manifests]
    feature_ids = [
        result.get("feature_id") for result in results if result.get("feature_id")
    ]
    duplicate_ids = sorted(
        feature_id
        for feature_id, count in Counter(feature_ids).items()
        if count > 1
    )
    errors = []
    if duplicate_ids:
        errors.append(f"duplicate feature IDs across manifests: {', '.join(duplicate_ids)}")
    return {
        "passed": all(result["passed"] for result in results) and not errors,
        "manifest_count": len(results),
        "manifests": results,
        "errors": errors,
    }


def _human_summary(report: dict[str, Any]) -> str:
    lines = [
        f"QuickPLS method-promotion manifests: {'PASS' if report['passed'] else 'FAIL'}",
        f"Manifests: {report['manifest_count']}",
    ]
    for result in report["manifests"]:
        lines.append(
            f"- {result.get('feature_id', result['path'])}: "
            f"declared={result.get('declared_state')} derived={result.get('derived_state')} "
            f"{'PASS' if result['passed'] else 'FAIL'}"
        )
        lines.extend(f"  ERROR: {error}" for error in result.get("errors", []))
    lines.extend(f"ERROR: {error}" for error in report.get("errors", []))
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", type=Path)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY_ROOT)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    report = validate_all(args.paths or None, args.repository_root)
    if args.json:
        json.dump(report, sys.stdout, indent=2, ensure_ascii=False, sort_keys=True)
        sys.stdout.write("\n")
    else:
        print(_human_summary(report))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
