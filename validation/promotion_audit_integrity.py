#!/usr/bin/env python3
"""Fail-closed evidence primitives for QuickPLS promotion audits.

Promotion code must not infer success from the mere presence of a JSON file or
from an arbitrary ``checks`` collection.  A report needs an explicit overall
pass state, and method-specific audits can additionally bind it to exact JSON
values, companion result payloads, source freshness, and content hashes.
"""

from __future__ import annotations

import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
_MISSING = object()


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"expected a JSON object in {path}")
    return value


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def json_value(value: Any, dotted_path: str, default: Any = _MISSING) -> Any:
    current = value
    for part in dotted_path.split("."):
        if not isinstance(current, dict) or part not in current:
            return default
        current = current[part]
    return current


def _state_value(value: Any) -> bool:
    return value is True or value == "passed"


def explicit_pass_state(value: dict[str, Any], pass_paths: list[str] | None = None) -> dict[str, Any]:
    """Return an explicit overall pass decision, rejecting contradictory states."""

    states: list[dict[str, Any]] = []
    if pass_paths:
        for path in pass_paths:
            observed = json_value(value, path)
            states.append({
                "path": path,
                "present": observed is not _MISSING,
                "value": None if observed is _MISSING else observed,
                "passed": observed is not _MISSING and _state_value(observed),
            })
    else:
        for path in ("passed", "qualification_passed", "qualification.passed"):
            observed = json_value(value, path)
            if observed is not _MISSING:
                states.append({"path": path, "present": True, "value": observed, "passed": observed is True})
        status = json_value(value, "status")
        if status is not _MISSING and status in {"passed", "failed"}:
            states.append({"path": "status", "present": True, "value": status, "passed": status == "passed"})
        promotion_ready = json_value(value, "promotion_ready")
        if promotion_ready is not _MISSING:
            artifacts_present = json_value(value, "all_listed_artifacts_present")
            artifacts_passed = json_value(value, "all_listed_artifacts_passed")
            states.append({
                "path": "promotion_ready+artifact_states",
                "present": True,
                "value": {
                    "promotion_ready": promotion_ready,
                    "all_listed_artifacts_present": None if artifacts_present is _MISSING else artifacts_present,
                    "all_listed_artifacts_passed": None if artifacts_passed is _MISSING else artifacts_passed,
                },
                "passed": (
                    promotion_ready is True
                    and artifacts_present is True
                    and artifacts_passed is True
                ),
            })

    return {
        "present": bool(states),
        "passed": bool(states) and all(item["passed"] for item in states),
        "states": states,
    }


def _requirement_checks(value: dict[str, Any], spec: dict[str, Any]) -> list[dict[str, Any]]:
    checks: list[dict[str, Any]] = []
    for path, expected in spec.get("required_values", {}).items():
        observed = json_value(value, path)
        checks.append({
            "path": path,
            "expected": expected,
            "observed": None if observed is _MISSING else observed,
            "passed": observed is not _MISSING and observed == expected,
        })
    for path, allowed in spec.get("required_any_values", {}).items():
        observed = json_value(value, path)
        checks.append({
            "path": path,
            "allowed": allowed,
            "observed": None if observed is _MISSING else observed,
            "passed": observed is not _MISSING and observed in allowed,
        })
    for path in spec.get("required_true", []):
        observed = json_value(value, path)
        checks.append({
            "path": path,
            "expected": True,
            "observed": None if observed is _MISSING else observed,
            "passed": observed is True,
        })
    for path in spec.get("required_nonempty", []):
        observed = json_value(value, path)
        checks.append({
            "path": path,
            "expected": "nonempty",
            "observed_type": None if observed is _MISSING else type(observed).__name__,
            "passed": observed is not _MISSING and bool(observed),
        })

    for list_requirement in spec.get("required_list_items", []):
        path = list_requirement["path"]
        values = json_value(value, path)
        where = list_requirement.get("where", {})
        matches = []
        if isinstance(values, list):
            for item in values:
                if not isinstance(item, dict):
                    continue
                if all(json_value(item, key) == expected for key, expected in where.items()):
                    matches.append(item)
        item_passed = len(matches) == 1
        item_checks: list[dict[str, Any]] = []
        if item_passed:
            item_checks = _requirement_checks(matches[0], list_requirement)
            item_passed = all(item["passed"] for item in item_checks)
        checks.append({
            "path": path,
            "where": where,
            "matching_items": len(matches),
            "item_checks": item_checks,
            "passed": item_passed,
        })
    return checks


def _iso_utc(timestamp: float) -> str:
    return datetime.fromtimestamp(timestamp, tz=timezone.utc).isoformat().replace("+00:00", "Z")


def _freshness(path: Path, root: Path, spec: dict[str, Any], now: datetime) -> dict[str, Any]:
    sources = []
    for relative in spec.get("source_paths", []):
        source = root / relative
        sources.append({
            "path": str(source.relative_to(root)),
            "present": source.is_file(),
            "modified_at_utc": _iso_utc(source.stat().st_mtime) if source.is_file() else None,
        })

    modified_at = datetime.fromtimestamp(path.stat().st_mtime, tz=timezone.utc)
    age_days = max(0.0, (now - modified_at).total_seconds() / 86_400.0)
    sources_passed = all(
        item["present"] and path.stat().st_mtime_ns >= (root / item["path"]).stat().st_mtime_ns
        for item in sources
    )
    max_age_days = spec.get("max_age_days")
    age_passed = max_age_days is None or age_days <= float(max_age_days)
    return {
        "checked": bool(sources) or max_age_days is not None,
        "passed": sources_passed and age_passed,
        "report_modified_at_utc": _iso_utc(path.stat().st_mtime),
        "age_days": round(age_days, 6),
        "max_age_days": max_age_days,
        "sources": sources,
    }


def _companion_evidence(root: Path, spec: dict[str, Any]) -> dict[str, Any]:
    relative = spec["path"]
    path = root / relative
    evidence: dict[str, Any] = {
        "path": str(relative),
        "present": path.is_file(),
        "bytes": path.stat().st_size if path.is_file() else None,
        "sha256": sha256_file(path) if path.is_file() else None,
        "readable": False,
        "checks": [],
        "passed": False,
    }
    if not path.is_file():
        return evidence
    try:
        value = load_json(path)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        evidence["error"] = f"{type(error).__name__}: {error}"
        return evidence
    evidence["readable"] = True
    checks = _requirement_checks(value, spec)
    evidence["checks"] = checks
    explicit = None
    if spec.get("require_explicit_pass"):
        explicit = explicit_pass_state(value, spec.get("pass_paths"))
        evidence["explicit_pass_state"] = explicit
    evidence["passed"] = all(item["passed"] for item in checks) and (explicit is None or explicit["passed"])
    return evidence


def normalize_report_spec(spec: str | dict[str, Any]) -> dict[str, Any]:
    return {"name": spec} if isinstance(spec, str) else dict(spec)


def evaluate_report(
    root: Path,
    results: Path,
    spec: str | dict[str, Any],
    *,
    now: datetime | None = None,
) -> dict[str, Any]:
    requirement = normalize_report_spec(spec)
    path = results / requirement["name"]
    evidence: dict[str, Any] = {
        "path": str(path.relative_to(root)),
        "present": path.is_file(),
        "bytes": path.stat().st_size if path.is_file() else None,
        "sha256": sha256_file(path) if path.is_file() else None,
        "readable": False,
        "explicit_pass_state": {"present": False, "passed": False, "states": []},
        "requirement_checks": [],
        "companions": [],
        "passed": False,
    }
    if not path.is_file():
        return evidence
    try:
        value = load_json(path)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        evidence["error"] = f"{type(error).__name__}: {error}"
        return evidence

    evidence["readable"] = True
    explicit = explicit_pass_state(value, requirement.get("pass_paths"))
    checks = _requirement_checks(value, requirement)
    companions = [_companion_evidence(root, companion) for companion in requirement.get("companions", [])]
    freshness = _freshness(path, root, requirement, now or datetime.now(timezone.utc))
    generated_at = next(
        (
            json_value(value, candidate)
            for candidate in ("generated_at_utc", "generated_at", "generatedAt", "timestamp_utc")
            if json_value(value, candidate) is not _MISSING
        ),
        None,
    )
    evidence.update({
        "explicit_pass_state": explicit,
        "requirement_checks": checks,
        "companions": companions,
        "freshness": freshness,
        "generated_at": generated_at,
        "passed": (
            explicit["passed"]
            and all(item["passed"] for item in checks)
            and all(item["passed"] for item in companions)
            and freshness["passed"]
        ),
    })
    return evidence


def report_passed(path: Path) -> bool:
    """Compatibility wrapper with strict, explicit overall-state semantics."""

    if not path.is_file():
        return False
    try:
        value = load_json(path)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError):
        return False
    return explicit_pass_state(value)["passed"]


def normalize_doc_spec(spec: str | dict[str, Any]) -> dict[str, Any]:
    return {"name": spec} if isinstance(spec, str) else dict(spec)


def evaluate_document(root: Path, spec: str | dict[str, Any]) -> dict[str, Any]:
    requirement = normalize_doc_spec(spec)
    relative = requirement.get("path") or f"docs/methods/{requirement['name']}"
    path = root / relative
    evidence: dict[str, Any] = {
        "path": str(path.relative_to(root)),
        "present": path.is_file(),
        "bytes": path.stat().st_size if path.is_file() else None,
        "sha256": sha256_file(path) if path.is_file() else None,
        "required_phrases": [],
        "forbidden_phrases": [],
        "passed": False,
    }
    if not path.is_file():
        return evidence
    try:
        content = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        evidence["error"] = f"{type(error).__name__}: {error}"
        return evidence
    lowered = content.casefold()
    required = [
        {"phrase": phrase, "present": phrase.casefold() in lowered}
        for phrase in requirement.get("required_phrases", [])
    ]
    forbidden = [
        {"phrase": phrase, "absent": phrase.casefold() not in lowered}
        for phrase in requirement.get("forbidden_phrases", [])
    ]
    evidence["required_phrases"] = required
    evidence["forbidden_phrases"] = forbidden
    evidence["passed"] = bool(content.strip()) and all(item["present"] for item in required) and all(item["absent"] for item in forbidden)
    return evidence


def write_method_audit(
    *,
    target: str,
    method_id: str,
    promoted_scope: str,
    required_reports: list[str | dict[str, Any]],
    required_docs: list[str | dict[str, Any]],
    extra_checks: list[dict[str, Any]] | None = None,
    root: Path = ROOT,
    results: Path = RESULTS,
) -> int:
    results.mkdir(parents=True, exist_ok=True)
    reports = [evaluate_report(root, results, spec) for spec in required_reports]
    docs = [evaluate_document(root, spec) for spec in required_docs]
    checks = extra_checks or []
    passed = (
        all(item["passed"] for item in reports)
        and all(item["passed"] for item in docs)
        and all(item.get("passed") is True for item in checks)
    )
    output = results / f"{method_id}_method_promotion_audit.json"
    report = {
        "schema_version": 2,
        "integrity_contract": "explicit_pass_state_and_bound_evidence_v1",
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "target": target,
        "method_id": method_id,
        "promoted_scope": promoted_scope,
        "passed": passed,
        "reports": reports,
        "docs": docs,
        "checks": checks,
        "note": "Promotion is limited to this documented scope; missing, stale, contradictory, or unbound evidence fails closed.",
    }
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {output} | passed={passed}")
    return 0 if passed else 1
