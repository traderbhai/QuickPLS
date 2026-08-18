#!/usr/bin/env python3
"""Strict release adapter for the focused packaged OLS v1 workflow.

The adapter never launches the desktop. It converts an already completed,
current scoped Tauri run into the two release-stage identity reports only when
the frozen desktop receipt, CLI closure, exact OLS archive/XLSX/screenshots,
three-viewport visual setup, same-run reopen contract, and clean cumulative
process receipt all verify byte-for-byte.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

try:
    from validation.diagnostic_bundle_source_manifest import (
        SourceManifestFailure,
        validate_build_receipt,
    )
    from validation.method_promotion_manifest import validate_manifest
    from validation.ols_v1_factory import cli_source_paths
    from validation.packaged_windows_acceptance_v2 import (
        CONTRACT as PACKAGED_ACCEPTANCE_CONTRACT,
        CONTRACT_FILE_SHA256,
        EXPECTED_CHECK_COUNT,
        receipt_binds_packaged_acceptance_contract,
        validate_required_report_checks,
    )
except ModuleNotFoundError:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from validation.diagnostic_bundle_source_manifest import (  # type: ignore[no-redef]
        SourceManifestFailure,
        validate_build_receipt,
    )
    from validation.method_promotion_manifest import validate_manifest  # type: ignore[no-redef]
    from validation.ols_v1_factory import cli_source_paths  # type: ignore[no-redef]
    from validation.packaged_windows_acceptance_v2 import (  # type: ignore[no-redef]
        CONTRACT as PACKAGED_ACCEPTANCE_CONTRACT,
        CONTRACT_FILE_SHA256,
        EXPECTED_CHECK_COUNT,
        receipt_binds_packaged_acceptance_contract,
        validate_required_report_checks,
    )


ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "validation/methods/ols_v1.manifest.json"
BUILD_RECEIPT = ROOT / "validation/results/diagnostic_bundle_build_receipt.json"
CUMULATIVE_RECEIPT = ROOT / "validation/results/v247_cumulative_native_acceptance_receipt.json"
EXPECTED_CUMULATIVE_CHECKS = EXPECTED_CHECK_COUNT
SCOPED_REPORT = ROOT / "validation/results/v247_tauri_native_acceptance_ols.json"
VISUAL_REPORT = ROOT / "validation/results/v247_native_desktop_visual_acceptance.json"
OUTPUT = ROOT / "validation/results/method_factory/ols_v1/packaged_acceptance.identity.json"
AUDIT_OUTPUT = ROOT / "validation/results/method_factory/ols_v1/method_audit.identity.json"
FEATURE_ID = "qpls3.standalone.ols"
METHOD_VERSION = "regression_ols_v1"
OLS_VALIDATED_SCOPE = (
    "Raw numeric ordinary least squares with an intercept, listwise deletion, HC3 robust "
    "standard errors, and fixed two-sided 95% confidence intervals. Optional regression "
    "case-resampling reports percentile-primary and conditional BCa inference. Categorical "
    "encoding, weights, clusters, generic PLS resampling, logistic regression, and PROCESS "
    "models are not included."
)
CATALOGUE_DATE = "2026-08-12"
NATIVE_CATALOG_SOURCE = ROOT / "src/native/nativeAnalysisCatalog.ts"


class AdapterError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AdapterError(message)


def canonical_native_catalogue_count() -> int | None:
    try:
        source = NATIVE_CATALOG_SOURCE.read_text(encoding="utf-8")
    except OSError:
        return None
    match = re.search(r"const CATALOG_DRAFTS[^=]*= \[([\s\S]*?)\n\] as const;", source)
    if match is None:
        return None
    kinds = re.findall(r'^[ \t]{4}kind:\s*"([a-z_]+)",\r?$', match.group(1), re.MULTILINE)
    return len(kinds) if kinds and len(kinds) == len(set(kinds)) else None


def strict_json(path: Path) -> dict[str, Any]:
    def object_hook(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        value: dict[str, Any] = {}
        for key, item in pairs:
            require(key not in value, f"duplicate JSON key in {path}: {key}")
            value[key] = item
        return value

    value = json.loads(
        path.read_text(encoding="utf-8-sig"),
        object_pairs_hook=object_hook,
        parse_constant=lambda token: (_ for _ in ()).throw(
            AdapterError(f"non-finite JSON value in {path}: {token}")
        ),
    )
    require(isinstance(value, dict), f"{path} must be a JSON object")
    return value


def repository_path(path: Path) -> str:
    resolved = path.resolve()
    require(resolved.is_relative_to(ROOT.resolve()), f"path escapes repository: {path}")
    return resolved.relative_to(ROOT.resolve()).as_posix()


def descriptor(path: Path) -> dict[str, Any]:
    require(path.is_file() and not path.is_symlink(), f"missing or linked artifact: {path}")
    payload = path.read_bytes()
    require(payload, f"empty artifact: {path}")
    return {
        "path": repository_path(path),
        "size": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }


def parse_utc(value: object, label: str) -> datetime:
    require(isinstance(value, str), f"{label} must be an ISO timestamp")
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    require(parsed.tzinfo is not None, f"{label} must include an offset")
    return parsed.astimezone(timezone.utc)


def source_freshness() -> tuple[dict[str, Any], list[dict[str, Any]]]:
    receipt = strict_json(BUILD_RECEIPT)
    validate_build_receipt(receipt, ROOT)
    desktop = descriptor(ROOT / "target/release/quickpls-desktop.exe")
    tested_desktop = receipt.get("tested_desktop")
    require(isinstance(tested_desktop, dict), "frozen build receipt has no tested desktop descriptor")
    require(
        desktop
        == {
            "path": tested_desktop.get("path"),
            "size": tested_desktop.get("size"),
            "sha256": tested_desktop.get("sha256"),
        },
        "release desktop differs from frozen build receipt",
    )
    cli = ROOT / "target/release/qpls.exe"
    cli_row = descriptor(cli)
    cli_sources = [ROOT / relative for relative in cli_source_paths()]
    newer = [repository_path(path) for path in cli_sources if path.stat().st_mtime_ns > cli.stat().st_mtime_ns]
    require(not newer, "release CLI predates current Rust inputs: " + ", ".join(newer))
    return {
        "passed": True,
        "desktop_receipt_exact": True,
        "release_cli_newer_sources": newer,
        "build_finished_at_utc": receipt.get("build_finished_at_utc"),
    }, [descriptor(BUILD_RECEIPT), desktop, cli_row, *[descriptor(path) for path in cli_sources]]


def visual_contract(not_before: datetime) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    report = strict_json(VISUAL_REPORT)
    require(report.get("passed") is True, "visual acceptance did not pass")
    require(report.get("failures") == [] and report.get("consoleErrors") == [], "visual acceptance contains failures")
    require(
        parse_utc(report.get("generatedAt"), "visual generatedAt")
        >= not_before - timedelta(seconds=2),
        "visual acceptance predates the requested evidence floor",
    )
    checks = report.get("checks", {}).get("ols")
    require(isinstance(checks, list) and len(checks) == 3, "visual OLS matrix must contain three viewports")
    observed = {row.get("viewport") for row in checks if isinstance(row, dict)}
    require(observed == {"1024x700", "1280x720", "1440x900"}, "visual OLS viewports are incomplete")
    require(
        all(
            row.get("selectedMethod") == "Regression"
            and row.get("startCommandDisabled") is True
            and row.get("runtimeBlockerVisible") is True
            and row.get("noModelBlocker") is True
            and row.get("linkage", {}).get("linkage") is True
            and row.get("truthAndOverflow", {}).get("noFabricatedRunState") is True
            and row.get("truthAndOverflow", {}).get("noHorizontalOverflow") is True
            and row.get("closeFocus", {}).get("dialogClosed") is True
            and row.get("closeFocus", {}).get("focusRestored") is True
            for row in checks
        ),
        "visual OLS setup contract failed",
    )
    screenshots: list[dict[str, Any]] = []
    for viewport in sorted(observed):
        path = ROOT / f"validation/results/screens/v247-native-desktop-visual/17-ols-standalone-dialog-{viewport}.png"
        screenshots.append(descriptor(path))
    return {"passed": True, "viewports": sorted(observed)}, [descriptor(VISUAL_REPORT), *screenshots]


def cumulative_cleanup(not_before: datetime) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    receipt = strict_json(CUMULATIVE_RECEIPT)
    started = parse_utc(receipt.get("supervisor_started_at_utc"), "supervisor_started_at_utc")
    completed = parse_utc(receipt.get("completed_at_utc"), "completed_at_utc")
    require(started >= not_before - timedelta(seconds=2), "cumulative receipt predates the OLS evidence floor")
    require(completed >= started, "cumulative receipt completion predates its start")
    require(receipt.get("passed") is True and receipt.get("failures") == 0 and receipt.get("console_errors") == 0, "cumulative receipt is not clean")
    require(receipt_binds_packaged_acceptance_contract(receipt), "cumulative receipt does not bind the current packaged acceptance manifest")
    require(receipt.get("final_scope") == PACKAGED_ACCEPTANCE_CONTRACT["final_scope"], "cumulative receipt final scope is wrong")
    require(receipt.get("graceful_process_cleanup_verified") is True, "cumulative cleanup is not verified")
    require(receipt.get("kind") == "quickpls_v247_cumulative_native_acceptance_receipt", "cumulative receipt identity is wrong")
    report_path = ROOT / str(receipt.get("report"))
    actual = descriptor(report_path)
    require(actual["size"] == receipt.get("report_size") and actual["sha256"] == receipt.get("report_sha256"), "cumulative report bytes differ from receipt")
    report = strict_json(report_path)
    require(validate_required_report_checks(PACKAGED_ACCEPTANCE_CONTRACT, report.get("checks"))["passed"], "cumulative report check IDs differ from the packaged acceptance manifest")
    ols_exports = [row for row in receipt.get("exports", []) if isinstance(row, dict) and row.get("role") == "ols"]
    require(len(ols_exports) == 1, "cumulative receipt must contain one OLS export")
    export_path = ROOT / str(ols_exports[0].get("path"))
    actual_export = descriptor(export_path)
    require(
        actual_export["path"] == ols_exports[0].get("path")
        and actual_export["size"] == ols_exports[0].get("size")
        and actual_export["sha256"] == ols_exports[0].get("sha256"),
        "cumulative OLS export differs from receipt",
    )
    return {"passed": True, "started_at_utc": started.isoformat(), "completed_at_utc": completed.isoformat(), "ols_export": ols_exports[0]}, [descriptor(CUMULATIVE_RECEIPT), actual, actual_export]


def packaged_contract(report: dict[str, Any]) -> tuple[dict[str, Any], list[dict[str, Any]], datetime]:
    require(report.get("passed") is True, "focused OLS report did not pass")
    require(report.get("failures") == [] and report.get("consoleErrors") == [], "focused OLS report contains failures")
    focused = report.get("focusedRun")
    require(isinstance(focused, dict) and focused.get("scope") == "ols", "focused report scope is not OLS")
    generated = parse_utc(report.get("generatedAt"), "generatedAt")
    completed = parse_utc(focused.get("completedAt"), "focusedRun.completedAt")
    require(completed >= generated, "focused OLS completion predates report generation")
    checks = report.get("checks")
    require(isinstance(checks, dict), "focused OLS checks are missing")
    dialog = checks.get("olsDialog", {})
    result = checks.get("olsResult", {})
    export = checks.get("olsExport", {})
    reopen = checks.get("olsSaveReopen", {})
    require(dialog.get("catalogCount") == canonical_native_catalogue_count() and dialog.get("selectedMethod") == "Regression", "OLS dialog catalogue contract failed")
    require(dialog.get("outcome") == "y" and dialog.get("selectedPredictors") == ["x", "m"] and dialog.get("selectedControls") == ["z"], "OLS role contract failed")
    require(dialog.get("startEnabled") is True and dialog.get("unsupportedControls") == 0 and dialog.get("blockers") == [], "OLS setup is not executable and bounded")
    progress = checks.get("olsProgress", {})
    require(progress.get("captured") is True or progress.get("completedBeforeCapture") is True, "OLS lifecycle evidence is missing")
    require(result.get("runId") and result.get("initialSelectedTable") == "ols_coefficients", "OLS result identity is missing")
    require(result.get("coefficients", {}).get("rows") == 4 and result.get("fit", {}).get("rows") == 1 and result.get("scope", {}).get("rows") == 12, "OLS result tables are incomplete")
    scope_values = result.get("scopeValues", {})
    require(scope_values.get("Method version") == METHOD_VERSION, "OLS result method version differs")
    require(scope_values.get("Validated scope") == OLS_VALIDATED_SCOPE, "OLS validated scope differs")
    native_xlsx = export.get("nativeXlsx", {})
    xlsx_path = Path(str(native_xlsx.get("targetPath", "")))
    require(native_xlsx.get("attempted") is True and native_xlsx.get("file", {}).get("isFile") is True, "native OLS XLSX was not written")
    expected_sheets = ["Coefficients", "Model fit", "Calculation scope", "Fitted values and residuals", "Run provenance"]
    require(native_xlsx.get("workbookSheets") == expected_sheets, "OLS XLSX sheets differ")
    completion = native_xlsx.get("helper", {}).get("completion", {})
    workbook = completion.get("workbook", {})
    require(completion.get("passed") is True and workbook.get("sheetNames") == expected_sheets, "OLS XLSX read-back differs")
    actual_xlsx = descriptor(xlsx_path)
    require(workbook.get("size") == actual_xlsx["size"] and workbook.get("sha256") == actual_xlsx["sha256"], "OLS XLSX descriptor differs from read-back")
    require(reopen.get("sameRunRestored") is True and reopen.get("expectedRunId") == result.get("runId") and reopen.get("selectedRunId") == result.get("runId"), "same OLS run was not reopened")
    archive = reopen.get("archive", {})
    require(archive.get("provenanceMethodVersion") == METHOD_VERSION and archive.get("regressionMethodVersion") == METHOD_VERSION and archive.get("payloadKind") == "pls_pm_v1", "OLS archive identity differs")
    require(archive.get("coefficientContract") is True and archive.get("predictionContract") is True and archive.get("fitContract") is True, "OLS archive contract failed")
    require(
        archive.get("recipe", {}).get("status") == "validated_regression_ols_v1_bounded_scope"
        and archive.get("recipe", {}).get("robustSe") == "hc3"
        and archive.get("recipe", {}).get("bootstrapSamples") == 0,
        "OLS persisted recipe differs from the bounded contract",
    )
    functional_offline = checks.get("olsFunctionalOffline", {})
    observed_request_count = functional_offline.get("observedRequestCount")
    external_request_count = functional_offline.get("externalRequestCount")
    require(
        functional_offline.get("passed") is True
        and functional_offline.get("analyticalWorkflowRequiresInternet") is False
        and functional_offline.get("strictZeroProcessEgressClaimed") is False
        and functional_offline.get("platformBackgroundEgressOutsidePageRequestScope") is True
        and isinstance(observed_request_count, int)
        and not isinstance(observed_request_count, bool)
        and observed_request_count >= 0
        and isinstance(external_request_count, int)
        and not isinstance(external_request_count, bool)
        and external_request_count == 0
        and functional_offline.get("externalRequests") == []
        and isinstance(functional_offline.get("origins"), list)
        and set(functional_offline.get("origins", [])).issubset(
            {"http://ipc.localhost", "http://tauri.localhost"}
        ),
        "functional-offline browser request boundary failed",
    )
    project_path = Path(str(checks.get("olsFixture", {}).get("projectPath", "")))
    screenshots = [
        ROOT / f"validation/results/screens/v247-native-desktop-acceptance/{number}-tauri-native-ols-{state}-1536x794.png"
        for number, state in ((120, "fixture-data"), (121, "dialog"), (122, "running"), (123, "results"), (124, "export"), (125, "reopened"))
    ]
    screenshot_rows = report.get("screenshots", [])
    require(all(str(path.resolve()) in screenshot_rows for path in screenshots), "OLS screenshot list is incomplete")
    artifacts = [descriptor(SCOPED_REPORT), actual_xlsx, descriptor(project_path), *[descriptor(path) for path in screenshots]]
    return {
        "passed": True,
        "run_id": result["runId"],
        "same_run_reopened": True,
        "xlsx_sheets": native_xlsx["workbookSheets"],
        "screenshot_count": len(screenshots),
        "functional_offline": True,
        "strict_zero_process_egress_claimed": False,
    }, artifacts, completed


def write_identity(path: Path, role: str, checks: dict[str, Any], source_paths: list[Path]) -> None:
    sources = [descriptor(source) for source in source_paths]
    unique = {row["path"]: row for row in sources}
    payload = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "passed": True,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_DATE,
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "role": role,
        "checks": checks,
        "source_artifacts": [unique[key] for key in sorted(unique)],
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8", newline="\r\n")


def role_sources(role: str, extras: list[Path]) -> list[Path]:
    manifest = strict_json(MANIFEST_PATH)
    governance = manifest["governance"]
    relative = {
        governance["manifest_path"],
        governance["schema_path"],
        governance["validator_path"],
        governance["focused_test_path"],
        *manifest["qualification"]["source_requirements"][role],
    }
    paths = [ROOT / value for value in sorted(relative)]
    paths.extend(extras)
    unique = {repository_path(path): path for path in paths}
    return [unique[key] for key in sorted(unique)]


def run(not_before: datetime) -> dict[str, Any]:
    prior = validate_manifest(MANIFEST_PATH, ROOT)
    require(prior.get("derived_state") == "native_qualified", "OLS prior factory stages are not native-qualified")
    fresh, freshness_sources = source_freshness()
    report = strict_json(SCOPED_REPORT)
    packaged, packaged_sources, completed = packaged_contract(report)
    require(
        completed >= not_before - timedelta(seconds=2),
        "focused OLS report predates the requested evidence floor",
    )
    visual, visual_sources = visual_contract(not_before)
    cleanup, cleanup_sources = cumulative_cleanup(not_before)
    checks = {"passed": True, "prior_factory": prior["derived_state"], "source_freshness": fresh, "packaged": packaged, "visual": visual, "cleanup": cleanup}
    packaged_extra = [Path(row["path"]) for row in [*freshness_sources, *packaged_sources, *visual_sources, *cleanup_sources]]
    write_identity(OUTPUT, "packaged_acceptance", checks, role_sources("packaged_acceptance", [ROOT / path for path in packaged_extra]))
    audit = subprocess.run(
        [sys.executable, "validation/ols_v1_factory_audit.py"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
        timeout=600,
    )
    require(
        audit.returncode == 0,
        "independent OLS final audit failed: " + (audit.stdout + "\n" + audit.stderr)[-3000:],
    )
    final_fresh, final_sources = source_freshness()
    require(final_fresh == fresh, "product or build source changed during the OLS release gate")
    require(
        [descriptor(Path(row["path"])) for row in final_sources]
        == [descriptor(Path(row["path"])) for row in freshness_sources],
        "source descriptors changed during the OLS release gate",
    )
    final = validate_manifest(MANIFEST_PATH, ROOT)
    require(final.get("passed") is True and final.get("derived_state") == "release_qualified", "OLS final manifest did not derive release-qualified")
    return {"passed": True, "derived_state": final["derived_state"], "packaged_identity": descriptor(OUTPUT), "method_audit_identity": descriptor(AUDIT_OUTPUT)}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--not-before-utc", required=True)
    args = parser.parse_args()
    try:
        report = run(parse_utc(args.not_before_utc, "--not-before-utc"))
    except (AdapterError, OSError, KeyError, ValueError, SourceManifestFailure) as error:
        print(json.dumps({"passed": False, "error": f"{type(error).__name__}: {error}"}, indent=2))
        return 1
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
