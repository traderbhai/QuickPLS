#!/usr/bin/env python3
"""Validate and evaluate the QuickPLS 3 parity ledger.

The ledger records claims; this module derives the highest state supported by
the evidence currently on disk.  A report is never considered passing merely
because the ledger says that it passed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import sys
from collections import Counter
from copy import deepcopy
from pathlib import Path
from typing import Any


STATE_ORDER = (
    "absent",
    "engine_only",
    "archive_qualified",
    "native_qualified",
    "release_qualified",
)

EXPECTED_CATALOG_KINDS = frozenset(
    {
        "pls_algorithm",
        "plsc",
        "wpls",
        "gsca",
        "cca",
        "ipma",
        "cbsem",
        "pls_bootstrap",
        "pls_permutation",
        "mga",
        "predict",
        "nca",
        "pca",
        "regression",
    }
)

# The catalog is a closed UI inventory, while a single entry may expose more
# than one independently qualified analytical capability. Regression is the
# first such entry: OLS, logistic, regression bootstrap, and PROCESS must never share one promotion
# state merely because they share a launcher.
EXPECTED_CATALOG_CAPABILITIES = {
    "pls_algorithm": frozenset({"qpls3.pls.algorithm"}),
    "plsc": frozenset({"qpls3.pls.consistent"}),
    "wpls": frozenset({"qpls3.pls.weighted"}),
    "gsca": frozenset({"qpls3.gsca.als"}),
    "cca": frozenset({"qpls3.assessment.cca_residuals"}),
    "ipma": frozenset({"qpls3.assessment.ipma"}),
    "cbsem": frozenset({"qpls3.cbsem.ml"}),
    "pls_bootstrap": frozenset({"qpls3.inference.bootstrap"}),
    "pls_permutation": frozenset({"qpls3.inference.structural_path_randomization"}),
    "mga": frozenset({"qpls3.groups.micom_permutation_mga"}),
    "predict": frozenset({"qpls3.prediction.plspredict_cvpat"}),
    "nca": frozenset({"qpls3.standalone.nca"}),
    "pca": frozenset({"qpls3.standalone.pca"}),
    "regression": frozenset(
        {
            "qpls3.standalone.ols",
            "qpls3.standalone.logistic",
            "qpls3.standalone.regression_bootstrap",
            "qpls3.standalone.process",
        }
    ),
}

FEATURE_ID = re.compile(r"^qpls3\.[a-z0-9][a-z0-9_.-]*$")
TRUTH_ASSERTION_KEYS = frozenset(
    {
        "passed",
        "fresh",
        "fresh_report",
        "source_freshness",
        "reported_freshness",
        "not_older_than_sources",
        "binary_not_older",
        "report_not_older",
        "tested_release_cli_is_current_and_bound",
        "tested_desktop_binary_is_current_and_bound",
        "fresh_after_packaged_report",
        "test_executables_are_current_and_bound",
        "desktop_bound_fresh",
        "provisioning_cli_bound_fresh",
        "fresh_sources_and_dist",
        "executables_bound_fresh",
        "inside_repository",
        "present",
        "exists",
        "valid",
        "matched",
        "matches",
        "success",
        "ok",
    }
)

# These named subtrees assert that prohibited/undesired content is absent. A
# nested ``present: false`` is success there, not failed evidence.
NEGATIVE_PRESENCE_CONTAINERS = frozenset(
    {"bundleObsoleteStrings", "prohibited", "forbidden", "obsolete"}
)


def _strict_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_nonfinite(token: str) -> None:
    raise ValueError(f"non-finite JSON number: {token}")


def _strict_float(token: str) -> float:
    value = float(token)
    if not math.isfinite(value):
        raise ValueError(f"non-finite JSON number: {token}")
    return value


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(
            handle,
            object_pairs_hook=_strict_object,
            parse_constant=_reject_nonfinite,
            parse_float=_strict_float,
        )


def _file_descriptor(path: Path, repository_root: Path) -> dict[str, Any]:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            size += len(block)
            digest.update(block)
    try:
        relative = path.resolve().relative_to(repository_root.resolve()).as_posix()
    except ValueError:
        relative = str(path.resolve())
    return {"path": relative, "size": size, "sha256": digest.hexdigest()}


def _semantic_report_failures(document: Any) -> list[str]:
    """Reject nested failed checks and non-empty failure/error collections."""

    failures: list[str] = []

    def walk(value: Any, pointer: str = "", negative_presence: bool = False) -> None:
        if isinstance(value, dict):
            if "reported_size" in value or "reported_sha256" in value:
                if value.get("reported_size") != value.get("actual_size"):
                    failures.append(f"{pointer or '/'} reported_size differs from actual_size")
                reported_hash = value.get("reported_sha256")
                actual_hash = value.get("actual_sha256")
                if not (
                    isinstance(reported_hash, str)
                    and isinstance(actual_hash, str)
                    and reported_hash.lower() == actual_hash.lower()
                ):
                    failures.append(f"{pointer or '/'} reported_sha256 differs from actual_sha256")
            for key, child in value.items():
                escaped = key.replace("~", "~0").replace("/", "~1")
                child_pointer = f"{pointer}/{escaped}"
                child_negative_presence = negative_presence or key in NEGATIVE_PRESENCE_CONTAINERS
                if (
                    key in TRUTH_ASSERTION_KEYS
                    and child is False
                    and not (key in {"present", "exists"} and child_negative_presence)
                ):
                    failures.append(f"{child_pointer}=false")
                if key in {"failures", "errors", "console_errors"}:
                    if child not in (None, [], {}):
                        failures.append(f"{child_pointer} is not empty")
                walk(child, child_pointer, child_negative_presence)
        elif isinstance(value, list):
            for index, child in enumerate(value):
                walk(child, f"{pointer}/{index}", negative_presence)

    walk(document)
    return failures


def json_pointer(document: Any, pointer: str) -> Any:
    """Resolve an RFC 6901 JSON pointer."""

    if pointer == "":
        return document
    if not pointer.startswith("/"):
        raise ValueError(f"JSON pointer must start with '/': {pointer!r}")
    current = document
    for raw_part in pointer[1:].split("/"):
        part = raw_part.replace("~1", "/").replace("~0", "~")
        if isinstance(current, list):
            current = current[int(part)]
        elif isinstance(current, dict):
            current = current[part]
        else:
            raise KeyError(pointer)
    return current


def _matches(actual: Any, rule: dict[str, Any]) -> bool:
    if "equals" in rule:
        return actual == rule["equals"]
    expected = rule.get("contains")
    if expected is None:
        raise ValueError("evidence rule must define 'equals' or 'contains'")
    if isinstance(actual, str):
        return isinstance(expected, str) and expected in actual
    if isinstance(actual, (list, tuple, set, dict)):
        return expected in actual
    return False


def evaluate_rule(rule: dict[str, Any], repository_root: Path) -> dict[str, Any]:
    result: dict[str, Any] = {
        "description": rule.get("description", "unnamed evidence rule"),
        "path": rule.get("path"),
        "passed": False,
    }
    relative_path = rule.get("path")
    if not isinstance(relative_path, str) or not relative_path:
        result["error"] = "evidence path is missing"
        return result

    evidence_path = repository_root / Path(relative_path)
    if not evidence_path.is_file():
        result["error"] = "evidence file is missing"
        return result

    try:
        pointer = rule.get("pointer")
        if pointer is None:
            actual = evidence_path.read_text(encoding="utf-8")
        else:
            actual = json_pointer(load_json(evidence_path), pointer)
            result["pointer"] = pointer
        result["passed"] = _matches(actual, rule)
        if not result["passed"]:
            result["actual"] = actual
            result["expected"] = (
                {"equals": rule["equals"]}
                if "equals" in rule
                else {"contains": rule.get("contains")}
            )
    except (OSError, UnicodeError, json.JSONDecodeError, KeyError, IndexError, TypeError, ValueError) as exc:
        result["error"] = f"{type(exc).__name__}: {exc}"
    return result


def evaluate_group(
    feature: dict[str, Any], group_name: str, repository_root: Path
) -> dict[str, Any]:
    rules = feature.get("evidence", {}).get(group_name, [])
    if not isinstance(rules, list) or not rules:
        return {"passed": False, "checks": [], "error": "no evidence rules declared"}
    checks = [evaluate_rule(rule, repository_root) for rule in rules]
    return {"passed": all(check["passed"] for check in checks), "checks": checks}


def _evaluate_release_report(
    specification: dict[str, Any],
    repository_root: Path,
    feature: dict[str, Any],
    catalogue_snapshot_date: str,
) -> dict[str, Any]:
    result: dict[str, Any] = {
        "path": specification.get("path"),
        "passed": False,
        "checks": {},
    }
    path_value = specification.get("path")
    required_pointers = {
        "passed": "passed_pointer",
        "feature_id": "feature_id_pointer",
        "method_version": "method_version_pointer",
        "catalogue_snapshot_date": "catalogue_snapshot_date_pointer",
    }
    if not isinstance(path_value, str) or not path_value:
        result["error"] = "release evidence path is missing"
        return result
    if any(not isinstance(specification.get(key), str) for key in required_pointers.values()):
        result["error"] = "release evidence is missing one or more identity pointers"
        return result

    evidence_path = repository_root / Path(path_value)
    if not evidence_path.is_file():
        result["error"] = "release evidence file is missing"
        return result

    try:
        result["descriptor"] = _file_descriptor(evidence_path, repository_root)
        document = load_json(evidence_path)
        expected = {
            "passed": True,
            "feature_id": feature["id"],
            "method_version": feature["method_version"],
            "catalogue_snapshot_date": catalogue_snapshot_date,
        }
        for label, pointer_key in required_pointers.items():
            actual = json_pointer(document, specification[pointer_key])
            result["checks"][label] = {
                "passed": actual == expected[label],
                "actual": actual,
                "expected": expected[label],
            }
        semantic_failures = _semantic_report_failures(document)
        result["semantic_integrity"] = {
            "passed": not semantic_failures,
            "failures": semantic_failures,
        }
        result["passed"] = (
            all(check["passed"] for check in result["checks"].values())
            and not semantic_failures
        )
    except (OSError, json.JSONDecodeError, KeyError, IndexError, TypeError, ValueError) as exc:
        result["error"] = f"{type(exc).__name__}: {exc}"
    return result


def evaluate_release(
    feature: dict[str, Any], repository_root: Path, catalogue_snapshot_date: str
) -> dict[str, Any]:
    """Require two current, independently scoped reports for release promotion."""

    release = feature.get("release_evidence")
    if not isinstance(release, dict):
        return {
            "passed": False,
            "method_audit": None,
            "packaged_acceptance": None,
            "error": "release evidence is not declared",
        }
    method_audit = release.get("current_scoped_method_audit")
    packaged = release.get("packaged_acceptance")
    if not isinstance(method_audit, dict) or not isinstance(packaged, dict):
        return {
            "passed": False,
            "method_audit": None,
            "packaged_acceptance": None,
            "error": "both method audit and packaged acceptance reports are required",
        }
    method_result = _evaluate_release_report(
        method_audit, repository_root, feature, catalogue_snapshot_date
    )
    packaged_result = _evaluate_release_report(
        packaged, repository_root, feature, catalogue_snapshot_date
    )
    return {
        "passed": method_result["passed"] and packaged_result["passed"],
        "method_audit": method_result,
        "packaged_acceptance": packaged_result,
    }


def evaluate_feature(
    feature: dict[str, Any], repository_root: Path, catalogue_snapshot_date: str
) -> dict[str, Any]:
    groups = {
        name: evaluate_group(feature, name, repository_root)
        for name in ("engine", "archive", "native")
    }
    release = evaluate_release(feature, repository_root, catalogue_snapshot_date)

    derived_state = "absent"
    if groups["engine"]["passed"]:
        derived_state = "engine_only"
        if groups["archive"]["passed"]:
            derived_state = "archive_qualified"
            if groups["native"]["passed"]:
                derived_state = "native_qualified"
                if release["passed"]:
                    derived_state = "release_qualified"

    declared_state = feature.get("state", "absent")
    declared_supported = (
        declared_state in STATE_ORDER
        and STATE_ORDER.index(declared_state) <= STATE_ORDER.index(derived_state)
    )
    return {
        "id": feature.get("id"),
        "catalog_kind": feature.get("catalog_kind"),
        "method_version": feature.get("method_version"),
        "declared_state": declared_state,
        "derived_state": derived_state,
        "declared_state_supported": declared_supported,
        "evidence": groups,
        "release_evidence": release,
    }


def _schema_errors(document: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if document.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    if tuple(document.get("allowed_states", [])) != STATE_ORDER:
        errors.append("allowed_states must contain the promotion ladder in order")

    catalogue = document.get("official_catalogue_snapshot")
    if not isinstance(catalogue, dict):
        errors.append("official_catalogue_snapshot is required")
    else:
        if catalogue.get("date") != "2026-08-12":
            errors.append("official catalogue snapshot date must be 2026-08-12")
        url = catalogue.get("url")
        if not isinstance(url, str) or not url.startswith("https://smartpls.com/"):
            errors.append("official catalogue snapshot URL must be an HTTPS SmartPLS URL")

    features = document.get("features")
    if not isinstance(features, list):
        return errors + ["features must be a list"]
    expected_feature_ids = set().union(*EXPECTED_CATALOG_CAPABILITIES.values())
    if len(features) != len(expected_feature_ids):
        errors.append(
            f"features must contain exactly {len(expected_feature_ids)} analytical capabilities, "
            f"found {len(features)}"
        )

    ids: list[str] = []
    kinds: list[str] = []
    for index, feature in enumerate(features):
        prefix = f"features[{index}]"
        if not isinstance(feature, dict):
            errors.append(f"{prefix} must be an object")
            continue
        feature_id = feature.get("id")
        if not isinstance(feature_id, str) or not FEATURE_ID.fullmatch(feature_id):
            errors.append(f"{prefix}.id is not a stable QuickPLS 3 feature ID")
        else:
            ids.append(feature_id)
        kind = feature.get("catalog_kind")
        if not isinstance(kind, str):
            errors.append(f"{prefix}.catalog_kind is required")
        else:
            kinds.append(kind)
        if feature.get("state") not in STATE_ORDER:
            errors.append(f"{prefix}.state is invalid")
        for field in (
            "label",
            "official_family",
            "quickpls_scope",
            "method_version",
        ):
            if not isinstance(feature.get(field), str) or not feature[field].strip():
                errors.append(f"{prefix}.{field} must be a non-empty string")
        differences = feature.get("known_differences")
        if not isinstance(differences, list) or not differences or not all(
            isinstance(item, str) and item.strip() for item in differences
        ):
            errors.append(f"{prefix}.known_differences must be a non-empty string list")
        evidence = feature.get("evidence")
        if not isinstance(evidence, dict):
            errors.append(f"{prefix}.evidence must be an object")

    duplicate_ids = sorted(key for key, count in Counter(ids).items() if count > 1)
    if duplicate_ids:
        errors.append(f"duplicate feature IDs: {', '.join(duplicate_ids)}")
    actual_feature_ids = set(ids)
    if actual_feature_ids != expected_feature_ids:
        missing = sorted(expected_feature_ids - actual_feature_ids)
        extra = sorted(actual_feature_ids - expected_feature_ids)
        errors.append(
            f"capability IDs differ from the frozen inventory (missing={missing}, extra={extra})"
        )
    if set(kinds) != EXPECTED_CATALOG_KINDS:
        missing = sorted(EXPECTED_CATALOG_KINDS - set(kinds))
        extra = sorted(set(kinds) - EXPECTED_CATALOG_KINDS)
        errors.append(f"catalog kinds differ from the accepted 14 (missing={missing}, extra={extra})")
    for kind, expected_ids in EXPECTED_CATALOG_CAPABILITIES.items():
        actual_ids = {
            feature.get("id")
            for feature in features
            if isinstance(feature, dict) and feature.get("catalog_kind") == kind
        }
        if actual_ids != expected_ids:
            errors.append(
                f"catalog capability mapping differs for {kind} "
                f"(expected={sorted(expected_ids)}, actual={sorted(actual_ids)})"
            )
    return errors


def validate_ledger_document(
    document: dict[str, Any], repository_root: Path
) -> dict[str, Any]:
    errors = _schema_errors(document)
    snapshot = document.get("official_catalogue_snapshot", {})
    snapshot_date = snapshot.get("date", "") if isinstance(snapshot, dict) else ""
    evaluated = [
        evaluate_feature(feature, repository_root, snapshot_date)
        for feature in document.get("features", [])
        if isinstance(feature, dict)
    ]
    for feature in evaluated:
        if not feature["declared_state_supported"]:
            errors.append(
                f"{feature['id']}: declared {feature['declared_state']} but current evidence "
                f"derives only {feature['derived_state']}"
            )

    states = Counter(feature["declared_state"] for feature in evaluated)
    derived_states = Counter(feature["derived_state"] for feature in evaluated)
    evidence_descriptors: dict[str, dict[str, Any]] = {}
    for feature in evaluated:
        release = feature.get("release_evidence", {})
        if not isinstance(release, dict):
            continue
        for role in ("method_audit", "packaged_acceptance"):
            report = release.get(role)
            if not isinstance(report, dict):
                continue
            descriptor = report.get("descriptor")
            if isinstance(descriptor, dict) and isinstance(descriptor.get("path"), str):
                evidence_descriptors[descriptor["path"]] = descriptor
    return {
        "passed": not errors,
        "schema_version": document.get("schema_version"),
        "catalogue_snapshot": snapshot,
        "feature_count": len(evaluated),
        "declared_states": dict(sorted(states.items())),
        "derived_states": dict(sorted(derived_states.items())),
        "features": evaluated,
        "release_evidence_descriptors": [
            evidence_descriptors[path] for path in sorted(evidence_descriptors)
        ],
        "errors": errors,
    }


def validate_ledger(
    ledger_path: Path, repository_root: Path | None = None
) -> dict[str, Any]:
    ledger_path = ledger_path.resolve()
    root = repository_root.resolve() if repository_root else ledger_path.parent.parent
    try:
        document = load_json(ledger_path)
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        return {
            "passed": False,
            "feature_count": 0,
            "features": [],
            "errors": [f"cannot load ledger: {type(exc).__name__}: {exc}"],
        }
    if not isinstance(document, dict):
        return {
            "passed": False,
            "feature_count": 0,
            "features": [],
            "errors": ["ledger root must be an object"],
        }
    return validate_ledger_document(deepcopy(document), root)


def _human_summary(report: dict[str, Any]) -> str:
    lines = [
        f"QuickPLS 3 parity ledger: {'PASS' if report['passed'] else 'FAIL'}",
        f"Features: {report.get('feature_count', 0)}",
        f"Declared states: {json.dumps(report.get('declared_states', {}), sort_keys=True)}",
        f"Derived states: {json.dumps(report.get('derived_states', {}), sort_keys=True)}",
    ]
    for error in report.get("errors", []):
        lines.append(f"ERROR: {error}")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--ledger",
        type=Path,
        default=Path(__file__).with_name("quickpls_3_parity_ledger.json"),
    )
    parser.add_argument("--repository-root", type=Path)
    parser.add_argument("--json", action="store_true", help="print the full evidence report")
    args = parser.parse_args(argv)

    report = validate_ledger(args.ledger, args.repository_root)
    if args.json:
        json.dump(report, sys.stdout, indent=2, ensure_ascii=False, sort_keys=True)
        sys.stdout.write("\n")
    else:
        print(_human_summary(report))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
