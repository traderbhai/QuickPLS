from __future__ import annotations

import argparse
import hashlib
import json
import os
from copy import deepcopy
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
CONTRACT_PATH = ROOT / "validation/capabilities/packaged_windows_acceptance_v2.manifest.json"
RESULTS = ROOT / "validation/results"
FULL_REPORT_PATH = RESULTS / "v247_tauri_native_acceptance_full.json"
OUTPUT_PATH = RESULTS / "v247_tauri_native_acceptance.json"


class AssemblyError(RuntimeError):
    pass


def read_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8-sig"))
    except (OSError, json.JSONDecodeError) as error:
        raise AssemblyError(f"Could not read valid JSON from {path}: {error}") from error
    if not isinstance(value, dict):
        raise AssemblyError(f"Expected a JSON object in {path}")
    return value


def parse_utc(value: str, label: str) -> datetime:
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except (AttributeError, ValueError) as error:
        raise AssemblyError(f"{label} is not an ISO-8601 timestamp") from error
    if parsed.tzinfo is None:
        raise AssemblyError(f"{label} has no timezone")
    return parsed.astimezone(timezone.utc)


def descriptor(path: Path) -> dict[str, Any]:
    data = path.read_bytes()
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "size": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
    }


def validate_contract(contract: dict[str, Any]) -> list[dict[str, Any]]:
    if contract.get("schema_version") != 2 or contract.get("contract_id") != "quickpls.packaged_windows_acceptance.v2":
        raise AssemblyError("Packaged acceptance contract identity is invalid")
    check_sets = contract.get("ordered_check_sets")
    if not isinstance(check_sets, list) or not check_sets:
        raise AssemblyError("Packaged acceptance contract has no ordered check sets")
    seen: set[str] = set()
    scopes: list[str] = []
    for check_set in check_sets:
        if not isinstance(check_set, dict):
            raise AssemblyError("Packaged acceptance contract contains a non-object check set")
        scope = check_set.get("scope")
        check_ids = check_set.get("required_check_ids")
        if not isinstance(scope, str) or not scope or scope in scopes:
            raise AssemblyError(f"Packaged acceptance contract contains an invalid or duplicate scope: {scope!r}")
        if not isinstance(check_ids, list) or not check_ids:
            raise AssemblyError(f"Packaged acceptance scope {scope} has no required check IDs")
        scopes.append(scope)
        for check_id in check_ids:
            if not isinstance(check_id, str) or not check_id or check_id in seen:
                raise AssemblyError(f"Packaged acceptance contract contains an invalid or duplicate check ID: {check_id!r}")
            seen.add(check_id)
    if contract.get("final_scope") != scopes[-1]:
        raise AssemblyError("Packaged acceptance final scope does not equal the last ordered scope")
    return check_sets


def source_report_path(scope: str, results: Path) -> Path:
    if scope == "full":
        return results / FULL_REPORT_PATH.name
    return results / f"v247_tauri_native_acceptance_{scope}.json"


def validate_source_report(
    report: dict[str, Any],
    path: Path,
    scope: str,
    required_check_ids: list[str],
    not_before: datetime,
) -> None:
    if report.get("passed") is not True or report.get("failures") != [] or report.get("consoleErrors") != []:
        raise AssemblyError(f"Acceptance source is not clean: {path}")
    if parse_utc(report.get("generatedAt"), f"{path} generatedAt") < not_before:
        raise AssemblyError(f"Acceptance source predates the cumulative supervisor: {path}")
    focused = report.get("focusedRun")
    if scope == "full":
        if focused is not None:
            raise AssemblyError(f"Full acceptance source unexpectedly has a focused scope: {path}")
    elif not isinstance(focused, dict) or focused.get("scope") != scope:
        raise AssemblyError(f"Acceptance source has the wrong focused scope for {scope}: {path}")
    checks = report.get("checks")
    if not isinstance(checks, dict):
        raise AssemblyError(f"Acceptance source has no checks object: {path}")
    missing = [check_id for check_id in required_check_ids if check_id not in checks]
    if missing:
        raise AssemblyError(f"Acceptance source {path} is missing required checks: {', '.join(missing)}")


def assemble(
    contract_path: Path = CONTRACT_PATH,
    results: Path = RESULTS,
    output_path: Path = OUTPUT_PATH,
    not_before: datetime | None = None,
) -> dict[str, Any]:
    boundary = not_before or datetime.min.replace(tzinfo=timezone.utc)
    contract = read_json(contract_path)
    check_sets = validate_contract(contract)
    checks: dict[str, Any] = {}
    screenshots: list[str] = []
    screenshot_artifacts: list[dict[str, Any]] = []
    seen_screenshots: set[str] = set()
    seen_artifacts: set[str] = set()
    sources: list[dict[str, Any]] = []
    supplemental_check_ids: dict[str, list[str]] = {}
    full_report: dict[str, Any] | None = None
    final_report: dict[str, Any] | None = None

    for check_set in check_sets:
        scope = check_set["scope"]
        required_check_ids = check_set["required_check_ids"]
        path = source_report_path(scope, results)
        report = read_json(path)
        validate_source_report(report, path, scope, required_check_ids, boundary)
        if scope == "full":
            full_report = report
        if scope == contract["final_scope"]:
            final_report = report
        source_checks = report["checks"]
        for check_id in required_check_ids:
            checks[check_id] = deepcopy(source_checks[check_id])
        supplemental_check_ids[scope] = sorted(set(source_checks) - set(required_check_ids))
        for screenshot in report.get("screenshots", []):
            if isinstance(screenshot, str) and screenshot not in seen_screenshots:
                screenshots.append(screenshot)
                seen_screenshots.add(screenshot)
        for artifact in report.get("screenshotArtifacts", []):
            if isinstance(artifact, dict) and isinstance(artifact.get("path"), str) and artifact["path"] not in seen_artifacts:
                screenshot_artifacts.append(deepcopy(artifact))
                seen_artifacts.add(artifact["path"])
        source = descriptor(path)
        source.update({"scope": scope, "included_check_ids": list(required_check_ids)})
        sources.append(source)

    if full_report is None or final_report is None:
        raise AssemblyError("Cumulative acceptance sources did not include the full and final scopes")
    generated_at = datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z")
    cumulative = deepcopy(full_report)
    cumulative.update(
        {
            "passed": True,
            "generatedAt": generated_at,
            "focusedRun": {
                **deepcopy(final_report.get("focusedRun", {})),
                "scope": contract["final_scope"],
                "completedAt": generated_at,
                "cumulativeAssembly": True,
            },
            "checks": checks,
            "screenshots": screenshots,
            "screenshotArtifacts": screenshot_artifacts,
            "consoleErrors": [],
            "failures": [],
            "cumulativeAssembly": {
                "schemaVersion": 1,
                "contractId": contract["contract_id"],
                "contractVersion": contract["contract_version"],
                "requiredCheckCount": len(checks),
                "sources": sources,
                "supplementalCheckIds": supplemental_check_ids,
            },
        }
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)
    temporary = output_path.with_name(f".{output_path.name}.{os.getpid()}.tmp")
    temporary.write_text(json.dumps(cumulative, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    os.replace(temporary, output_path)
    return {"passed": True, "report": descriptor(output_path), "checks": len(checks), "sources": sources}


def main() -> int:
    parser = argparse.ArgumentParser(description="Assemble the immutable QuickPLS cumulative packaged acceptance report")
    parser.add_argument("--not-before-utc", required=True)
    arguments = parser.parse_args()
    result = assemble(not_before=parse_utc(arguments.not_before_utc, "--not-before-utc"))
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
