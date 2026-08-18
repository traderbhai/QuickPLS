"""Source-bound evidence utilities for the HTMT/HTMT+ qualification lane.

The factory has two deliberately separate outputs:

* ``*.source_audit.json`` proves that a gate and its frozen inputs are present
  and internally consistent.  It is never promotion evidence.
* ``*.identity.json`` is reserved for a completed qualification execution.  A
  caller cannot create one while blockers remain or without explicitly marking
  the report as qualification evidence.

Nothing in this module changes the capability registry or the v1 promotion
manifest.  Admission remains a separate, reviewed action.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Sequence


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
MANIFEST_PATH = VALIDATION / "methods" / "htmt_plus_v1.manifest.json"
QUALIFICATION_SPEC_PATH = (
    VALIDATION / "qualification_v2" / "htmt_plus_v1.qualification.json"
)
REGISTRY_PATH = VALIDATION / "capabilities" / "capability_registry_v2.json"
REPORT_ROOT = VALIDATION / "results" / "method_factory" / "htmt_plus_v1"
COMMON_SOURCE = "validation/htmt_plus_v1_factory_common.py"
FOCUSED_TEST = "validation/test_htmt_qualification_v2.py"

EXPECTED_FEATURE = {
    "id": "qpls3.assessment.htmt",
    "method_version": "ringle_et_al_htmt_plus_v1",
    "catalogue_snapshot_date": "2026-08-12",
}
EXPECTED_QUALIFICATION_ID = "qpls3.assessment.htmt.qualification_v2"
EXPECTED_CAPABILITY = {
    "registry_schema_version": 2,
    "capability_id": "smartpls.htmt",
    "cell_id": "qpls3.assessment.htmt",
    "capability_version": "ringle_et_al_htmt_plus_v1",
}


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
    with path.open("r", encoding="utf-8-sig") as handle:
        return json.load(
            handle,
            object_pairs_hook=_strict_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                ValueError(f"non-finite JSON number: {value}")
            ),
        )


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")


def canonical_sha256(value: Any) -> str:
    return hashlib.sha256(canonical_json_bytes(value)).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def repository_path(path: Path) -> str:
    return path.resolve().relative_to(ROOT.resolve()).as_posix()


def source_descriptors(paths: Iterable[str]) -> list[dict[str, Any]]:
    descriptors: list[dict[str, Any]] = []
    for relative in sorted(set(paths)):
        candidate = Path(relative)
        if candidate.is_absolute() or ".." in candidate.parts or "\\" in relative:
            raise ValueError(f"unsafe repository source path: {relative!r}")
        path = ROOT / candidate
        if not path.is_file():
            raise FileNotFoundError(f"required HTMT source is missing: {relative}")
        descriptors.append(
            {
                "path": relative,
                "size_bytes": path.stat().st_size,
                "sha256": sha256_file(path),
            }
        )
    return descriptors


def source_set_sha256(descriptors: Sequence[dict[str, Any]]) -> str:
    return canonical_sha256(
        [
            {
                "path": row["path"],
                "size_bytes": row["size_bytes"],
                "sha256": row["sha256"],
            }
            for row in descriptors
        ]
    )


def manifest() -> dict[str, Any]:
    document = strict_load_json(MANIFEST_PATH)
    feature = document.get("feature", {})
    for key, expected in EXPECTED_FEATURE.items():
        if feature.get(key) != expected:
            raise ValueError(
                f"HTMT manifest identity mismatch for {key}: "
                f"expected {expected!r}, found {feature.get(key)!r}"
            )
    qualification = document.get("qualification", {})
    if qualification.get("declared_state") != "absent":
        raise ValueError("HTMT scaffold must remain declared absent")
    evidence = qualification.get("evidence", {})
    if any(evidence.get(stage) for stage in evidence):
        raise ValueError("HTMT scaffold must not admit promotion evidence")
    return document


def qualification_spec() -> dict[str, Any]:
    document = strict_load_json(QUALIFICATION_SPEC_PATH)
    identity = document.get("identity", {})
    if identity.get("qualification_id") != EXPECTED_QUALIFICATION_ID:
        raise ValueError("HTMT QualificationSpec identity mismatch")
    if identity.get("method_version") != EXPECTED_FEATURE["method_version"]:
        raise ValueError("HTMT QualificationSpec method version mismatch")
    if identity.get("capability_cell") != EXPECTED_CAPABILITY:
        raise ValueError("HTMT QualificationSpec capability link mismatch")
    migration = document.get("migration", {})
    if migration.get("status") != "compatibility_only":
        raise ValueError("HTMT QualificationSpec must remain compatibility-only")
    if not migration.get("unresolved_items"):
        raise ValueError("HTMT QualificationSpec must name unresolved work")
    if document.get("evidence_contract", {}).get("receipts"):
        raise ValueError("HTMT QualificationSpec must not contain evidence receipts")
    return document


def role_sources(role: str, extras: Iterable[str] = ()) -> list[str]:
    document = manifest()
    requirements = document["qualification"]["source_requirements"]
    if role not in requirements:
        raise ValueError(f"unknown HTMT evidence role: {role}")
    governance = document["governance"]
    required = {
        governance["manifest_path"],
        governance["schema_path"],
        governance["validator_path"],
        governance["focused_test_path"],
        QUALIFICATION_SPEC_PATH.relative_to(ROOT).as_posix(),
        "validation/qualification_spec_v2.py",
        "validation/qualification_v2/qualification_spec_v2.schema.json",
        COMMON_SOURCE,
        FOCUSED_TEST,
        *requirements[role],
        *extras,
    }
    return sorted(required)


def build_identity_report(
    role: str,
    *,
    stage: str,
    passed: bool,
    checks: dict[str, Any],
    blockers: Sequence[str] = (),
    extras: Iterable[str] = (),
    qualification_evidence: bool = False,
    execution: dict[str, Any] | None = None,
) -> dict[str, Any]:
    blockers = sorted(set(blockers))
    if qualification_evidence and (not passed or blockers):
        raise ValueError(
            "qualification evidence requires a passing report with no blockers"
        )
    document = manifest()
    spec = qualification_spec()
    sources = source_descriptors(role_sources(role, extras))
    report: dict[str, Any] = {
        "schema_version": 2,
        "report_kind": "quickpls_htmt_qualification_identity_report",
        "role": role,
        "stage": stage,
        "passed": bool(passed),
        "qualification_evidence": bool(qualification_evidence),
        "qualification_ready": bool(qualification_evidence and passed),
        "feature_id": EXPECTED_FEATURE["id"],
        "qualification_id": EXPECTED_QUALIFICATION_ID,
        "capability_id": EXPECTED_CAPABILITY["capability_id"],
        "cell_id": EXPECTED_CAPABILITY["cell_id"],
        "method_version": EXPECTED_FEATURE["method_version"],
        "catalogue_snapshot_date": EXPECTED_FEATURE["catalogue_snapshot_date"],
        "spec_frozen_at_utc": spec["identity"]["spec_frozen_at_utc"],
        "generated_at_utc": datetime.now(timezone.utc)
        .isoformat()
        .replace("+00:00", "Z"),
        "manifest_sha256": sha256_file(MANIFEST_PATH),
        "qualification_spec_sha256": sha256_file(QUALIFICATION_SPEC_PATH),
        "scenario_set_sha256": canonical_sha256(spec["scenario_contract"]),
        "source_set_sha256": source_set_sha256(sources),
        "source_artifacts": sources,
        "checks": checks,
        "blockers": blockers,
        "admission_policy": (
            "identity_report_requires_separate_manifest_admission_and_review"
        ),
        "declared_manifest_state": document["qualification"]["declared_state"],
    }
    if execution is not None:
        report["execution"] = execution
    return report


def write_identity_report(
    role: str,
    *,
    stage: str,
    passed: bool,
    checks: dict[str, Any],
    blockers: Sequence[str] = (),
    extras: Iterable[str] = (),
    qualification_evidence: bool = False,
    execution: dict[str, Any] | None = None,
) -> Path:
    report = build_identity_report(
        role,
        stage=stage,
        passed=passed,
        checks=checks,
        blockers=blockers,
        extras=extras,
        qualification_evidence=qualification_evidence,
        execution=execution,
    )
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    suffix = "identity" if qualification_evidence else "source_audit"
    path = REPORT_ROOT / f"{role}.{suffix}.json"
    path.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return path


def run_command(
    command: Sequence[str], *, timeout: int = 300
) -> tuple[subprocess.CompletedProcess[str], dict[str, Any]]:
    started = time.monotonic()
    completed = subprocess.run(
        list(command),
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    return completed, {
        "command": list(command),
        "returncode": completed.returncode,
        "duration_seconds": round(time.monotonic() - started, 3),
        "stdout_tail": completed.stdout[-4000:],
        "stderr_tail": completed.stderr[-4000:],
    }


def current_candidate_status(
    candidate: Path,
    *,
    source_paths: Iterable[str],
    build_receipt: Path | None = None,
) -> dict[str, Any]:
    """Check whether a candidate is even eligible for qualification execution.

    Timestamp freshness is only a guard.  Qualification additionally requires
    a build receipt binding the candidate hash to the exact source-set hash.
    """

    sources = source_descriptors(source_paths)
    status: dict[str, Any] = {
        "path": str(candidate),
        "exists": candidate.is_file(),
        "source_set_sha256": source_set_sha256(sources),
        "source_artifacts": sources,
        "timestamp_current": False,
        "build_receipt_verified": False,
        "eligible": False,
        "blockers": [],
    }
    if not candidate.is_file():
        status["blockers"].append("candidate_executable_missing")
        return status
    status["sha256"] = sha256_file(candidate)
    status["size_bytes"] = candidate.stat().st_size
    latest_source = max((ROOT / row["path"]).stat().st_mtime_ns for row in sources)
    status["timestamp_current"] = candidate.stat().st_mtime_ns >= latest_source
    if not status["timestamp_current"]:
        status["blockers"].append("candidate_older_than_bound_sources")
    if build_receipt is None or not build_receipt.is_file():
        status["blockers"].append("source_bound_build_receipt_missing")
    else:
        receipt = strict_load_json(build_receipt)
        status["build_receipt_verified"] = (
            receipt.get("passed") is True
            and receipt.get("candidate_sha256") == status["sha256"]
            and receipt.get("source_set_sha256") == status["source_set_sha256"]
        )
        if not status["build_receipt_verified"]:
            status["blockers"].append("source_bound_build_receipt_mismatch")
    status["eligible"] = not status["blockers"]
    return status


def require_exact_case_ids(
    document: dict[str, Any], required: Iterable[str]
) -> dict[str, Any]:
    rows = document.get("cases")
    if not isinstance(rows, list):
        return {
            "passed": False,
            "required_case_ids": sorted(set(required)),
            "observed_case_ids": [],
            "missing_case_ids": sorted(set(required)),
            "unexpected_case_ids": [],
            "duplicate_case_ids": [],
        }
    observed = [row.get("id") for row in rows if isinstance(row, dict)]
    observed_ids = [value for value in observed if isinstance(value, str)]
    duplicates = sorted(
        {value for value in observed_ids if observed_ids.count(value) > 1}
    )
    required_ids = set(required)
    actual_ids = set(observed_ids)
    all_passed = all(
        isinstance(row, dict) and row.get("passed") is True for row in rows
    )
    return {
        "passed": (
            not duplicates
            and actual_ids == required_ids
            and len(observed_ids) == len(rows)
            and all_passed
        ),
        "required_case_ids": sorted(required_ids),
        "observed_case_ids": sorted(actual_ids),
        "missing_case_ids": sorted(required_ids - actual_ids),
        "unexpected_case_ids": sorted(actual_ids - required_ids),
        "duplicate_case_ids": duplicates,
        "all_cases_passed": all_passed,
    }
