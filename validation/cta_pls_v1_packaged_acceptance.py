"""Fail-closed release qualification for the bounded CTA-PLS v1 Windows workflow."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import shutil
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable
from xml.etree import ElementTree

from jsonschema import Draft202012Validator, FormatChecker

from cta_pls_v1_factory_common import (
    CLI,
    REPORT_ROOT,
    ROOT,
    manifest,
    repository_path,
    run_command,
    sha256_file,
    strict_load_json,
    write_identity_report,
)
from method_promotion_manifest import _verify_artifact, validate_manifest


SOURCE = "validation/cta_pls_v1_packaged_acceptance.py"
SCHEMA = ROOT / "validation" / "cta_pls_v1_packaged_acceptance.schema.json"
RAW_REPORT = ROOT / "validation" / "results" / "v247_tauri_native_acceptance_cta_pls.json"
FACTORY_RAW_REPORT = REPORT_ROOT / "cta_pls_v1_packaged_raw.json"
FACTORY_ARCHIVE = REPORT_ROOT / "cta_pls_v1_packaged.qpls"
FACTORY_XLSX = REPORT_ROOT / "cta_pls_v1_packaged.xlsx"
FACTORY_NETWORK = REPORT_ROOT / "cta_pls_v1_network_samples.jsonl"
RUNTIME_XLSX = ROOT / "validation" / "results" / "cta_pls_v1_packaged_runtime.xlsx"
RUNTIME_NETWORK = ROOT / "validation" / "results" / "cta_pls_v1_network_runtime.jsonl"
DESKTOP = ROOT / "target" / "release" / "quickpls-desktop.exe"
PROJECT_ENTRY = "project.json"
MANIFEST_ENTRY = "manifest.json"
EXPECTED_PAIRINGS = [
    "ab_cd_minus_ac_bd",
    "ac_bd_minus_ad_bc",
    "ad_bc_minus_ab_cd",
]
EXPECTED_SHEET_ORDER = [
    "Path coefficients",
    "Outer loadings",
    "Outer weights",
    "R-square",
    "Total effects",
    "CTA-PLS tetrad summary",
    "CTA-PLS tetrads",
    "CTA-PLS scope and exclusions",
    "Construct reliability and valid",
    "Cross loadings",
    "Fornell-Larcker criterion",
    "HTMT+",
    "Original HTMT",
    "Structural model",
    "Inner VIF values",
    "f-square effect sizes",
    "Model fit",
    "Construct cross-validated redun",
    "Run provenance",
]
INTERNAL_APP_ORIGINS = [
    "http://ipc.localhost",
    "http://tauri.localhost",
]
REQUIRED_BUILD_SOURCES = {
    "Cargo.lock",
    "package.json",
    "src-tauri/Cargo.toml",
    "src-tauri/src/lib.rs",
    "src-tauri/tauri.conf.json",
    "crates/qpls-core/src/validation.rs",
    "crates/qpls-estimation/src/pls.rs",
    "crates/qpls-project/src/lib.rs",
    "crates/qpls-runner/src/lib.rs",
    "src/native/NativeCalculationDialog.tsx",
    "src/native/NativeDesktopController.tsx",
    "src/native/nativeAnalysisCatalog.ts",
    "src/native/nativeAnalysisRecipe.ts",
    "src/native/nativeCtaPls.ts",
    "src/native/nativeExportTables.ts",
    "src/native/nativeResults.ts",
}
GATE_SOURCES = {
    SOURCE,
    "validation/cta_pls_v1_packaged_acceptance.schema.json",
    "validation/cta_pls_v1_factory_audit.py",
    "validation/cta_pls_v1_factory_common.py",
    "validation/method_promotion_manifest.py",
    "validation/methods/cta_pls_v1.manifest.json",
    "validation/methods/method_promotion_manifest.schema.json",
    "validation/monitor_quickpls_network.ps1",
    "validation/run_cta_pls_native_acceptance.ps1",
    "validation/test_cta_pls_v1_factory_evidence.py",
    "validation/test_cta_pls_v1_packaged_acceptance.py",
    "validation/v247_tauri_native_acceptance.mjs",
}


def build_source_paths() -> list[str]:
    paths = set(REQUIRED_BUILD_SOURCES)
    for base, patterns in (
        (ROOT / "src", ("*.ts", "*.tsx", "*.css")),
        (ROOT / "src-tauri", ("*.rs", "*.toml", "*.json")),
        (ROOT / "crates", ("*.rs", "Cargo.toml")),
    ):
        for pattern in patterns:
            paths.update(repository_path(path) for path in base.rglob(pattern) if path.is_file())
    for relative in ("index.html", "package-lock.json", "tsconfig.json", "tsconfig.node.json", "vite.config.ts"):
        if (ROOT / relative).is_file():
            paths.add(relative)
    return sorted(paths)


def cli_source_paths() -> list[str]:
    """Return the exact local Rust source closure consumed by qpls.exe."""
    paths = {"Cargo.lock", "Cargo.toml", "crates/qpls-cli/Cargo.toml"}
    paths.update(
        repository_path(path)
        for path in (ROOT / "crates" / "qpls-cli" / "src").rglob("*.rs")
        if path.is_file()
    )
    for crate in (
        "qpls-assessment", "qpls-core", "qpls-data", "qpls-estimation",
        "qpls-project", "qpls-resampling", "qpls-runner",
    ):
        crate_root = ROOT / "crates" / crate
        paths.add(repository_path(crate_root / "Cargo.toml"))
        paths.update(
            repository_path(path)
            for path in (crate_root / "src").rglob("*.rs")
            if path.is_file()
        )
    return sorted(paths)


def _parse_utc(value: str) -> datetime:
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        raise ValueError("UTC timestamp must contain an offset")
    return parsed.astimezone(timezone.utc)


def verify_prior_factory_stages() -> dict[str, Any]:
    document = manifest()
    feature = document["feature"]
    identity = {
        "passed": True,
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
    }
    required = {
        "engine_only": {"method_spec", "independent_reference", "simulation_report", "boundary_report"},
        "archive_qualified": {"persistence_report"},
        "native_qualified": {"frontend_report", "export_report"},
    }
    rows: list[dict[str, Any]] = []
    for stage, roles in required.items():
        observed: set[str] = set()
        for artifact in document["qualification"]["evidence"][stage]:
            observed.update(artifact["roles"])
            passed, errors = _verify_artifact(artifact, document, ROOT, identity)
            rows.append({"stage": stage, "roles": artifact["roles"], "path": artifact["path"], "passed": passed, "errors": errors})
        if observed != roles:
            rows.append({"stage": stage, "passed": False, "roles": sorted(observed), "errors": [f"expected {sorted(roles)}"]})
    return {"passed": all(row["passed"] for row in rows), "artifacts": rows}


def read_archive(path: Path) -> tuple[dict[str, Any], dict[str, Any], dict[str, bytes]]:
    with zipfile.ZipFile(path) as archive:
        names = [entry.filename for entry in archive.infolist()]
        if len(names) != len(set(names)):
            raise ValueError("CTA-PLS archive contains duplicate entries")
        if any(name.startswith(("/", "\\")) or "\\" in name or ".." in Path(name).parts for name in names):
            raise ValueError("CTA-PLS archive contains an unsafe entry path")
        entries = {name: archive.read(name) for name in names}
    project = json.loads(entries[PROJECT_ENTRY].decode("utf-8"))
    archive_manifest = json.loads(entries[MANIFEST_ENTRY].decode("utf-8"))
    checksums = archive_manifest.get("checksums", {})
    if set(checksums) != set(entries) - {MANIFEST_ENTRY}:
        raise ValueError("CTA-PLS manifest checksum membership is not exact")
    for name, expected in checksums.items():
        if hashlib.sha256(entries[name]).hexdigest() != expected:
            raise ValueError(f"CTA-PLS archive checksum mismatch for {name}")
    return project, archive_manifest, entries


def _column_index(reference: str) -> int:
    letters = re.match(r"[A-Z]+", reference)
    if not letters:
        raise ValueError(f"invalid XLSX cell reference: {reference}")
    value = 0
    for letter in letters.group(0):
        value = value * 26 + ord(letter) - ord("A") + 1
    return value - 1


def read_xlsx_tables(path: Path) -> dict[str, list[list[str]]]:
    main_ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
    rel_ns = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
    package_rel_ns = "http://schemas.openxmlformats.org/package/2006/relationships"
    with zipfile.ZipFile(path) as workbook:
        shared_root = ElementTree.fromstring(workbook.read("xl/sharedStrings.xml"))
        shared = ["".join(node.itertext()) for node in shared_root.findall(f"{{{main_ns}}}si")]
        rel_root = ElementTree.fromstring(workbook.read("xl/_rels/workbook.xml.rels"))
        relationships = {node.attrib["Id"]: node.attrib["Target"] for node in rel_root.findall(f"{{{package_rel_ns}}}Relationship")}
        workbook_root = ElementTree.fromstring(workbook.read("xl/workbook.xml"))
        tables: dict[str, list[list[str]]] = {}
        for sheet in workbook_root.findall(f".//{{{main_ns}}}sheet"):
            target = relationships[sheet.attrib[f"{{{rel_ns}}}id"]].lstrip("/")
            sheet_root = ElementTree.fromstring(workbook.read(target if target.startswith("xl/") else f"xl/{target}"))
            rows: list[list[str]] = []
            for row in sheet_root.findall(f".//{{{main_ns}}}row"):
                cells: dict[int, str] = {}
                for cell in row.findall(f"{{{main_ns}}}c"):
                    value_node = cell.find(f"{{{main_ns}}}v")
                    if value_node is None:
                        inline = cell.find(f"{{{main_ns}}}is")
                        value = "" if inline is None else "".join(inline.itertext())
                    else:
                        raw = value_node.text or ""
                        value = shared[int(raw)] if cell.attrib.get("t") == "s" else raw
                    cells[_column_index(cell.attrib["r"])] = value
                if cells:
                    rows.append([cells.get(index, "") for index in range(max(cells) + 1)])
            tables[sheet.attrib["name"]] = rows
    return tables


def rows_after_header(rows: list[list[str]], first_cell: str) -> list[list[str]]:
    for index, row in enumerate(rows):
        if row and row[0] == first_cell:
            return rows[index:]
    raise ValueError(f"XLSX header {first_cell!r} not found")


def verify_exact_values(raw: dict[str, Any], project: dict[str, Any], tables: dict[str, list[list[str]]]) -> dict[str, Any]:
    result_view = raw["checks"]["ctaPlsResult"]
    reopen = raw["checks"]["ctaPlsSaveReopen"]
    run_id = result_view["runId"]
    results = [row for row in project["results"] if row["id"] == run_id]
    if len(results) != 1:
        raise ValueError("CTA-PLS run does not map to exactly one archived result")
    archived = results[0]
    cta = archived["payload"]["estimation"]["cta_pls"]
    archive_by_pairing = {row["pairing"]: row for row in cta["estimates"]}
    if list(archive_by_pairing) != EXPECTED_PAIRINGS:
        raise ValueError("CTA-PLS archived pairing order drifted")
    ui_rows = result_view["tetrads"]["values"]
    xlsx_rows = rows_after_header(tables["CTA-PLS tetrads"], "Construct")[1:]
    expected_rows = [
        [
            "Predictor", row["indicator_a"], row["indicator_b"], row["indicator_c"], row["indicator_d"],
            {
                "ab_cd_minus_ac_bd": "Ab cd minus ac bd",
                "ac_bd_minus_ad_bc": "Ac bd minus ad bc",
                "ad_bc_minus_ab_cd": "Ad bc minus ab cd",
            }[row["pairing"]],
            f"{row['tetrad']:.4f}", f"{row['absolute_tetrad']:.4f}",
        ]
        for row in cta["estimates"]
    ]
    provenance_rows = rows_after_header(tables["Run provenance"], "Field")[1:]
    provenance = {row[0]: row[1] for row in provenance_rows if len(row) >= 2}
    checks = {
        "same_run_restored": reopen["sameRunRestored"] is True and reopen["expectedRunId"] == run_id and reopen["selectedRunId"] == run_id,
        "archive_identity": archived["provenance"]["method"] == "cta_pls"
        and archived["provenance"]["method_version"] == "pls_pm_v1+cta_pls_tetrad_v1+pls_mediation_v1+pls_assessment_v7"
        and archived["payload"]["estimation"]["method_version"] == "cta_pls_tetrad_v1"
        and cta["method_version"] == "cta_pls_tetrad_v1",
        "ui_matches_archive": ui_rows == expected_rows,
        "xlsx_matches_archive": xlsx_rows == expected_rows,
        "provenance_binds_same_run": provenance.get("Recipe") == archived["provenance"]["recipe_id"]
        and provenance.get("Dataset fingerprint") == archived["provenance"]["dataset_fingerprint"]
        and provenance.get("Method version") == archived["provenance"]["method_version"],
        "exact_sheet_order": list(tables) == EXPECTED_SHEET_ORDER,
        "pairing_count": len(expected_rows),
    }
    checks["passed"] = all(value for value in checks.values() if isinstance(value, bool))
    return checks


def write_mutated_archive(original: Path, destination: Path, mutation: Callable[[dict[str, Any]], None] | None, *, update_checksum: bool) -> None:
    with zipfile.ZipFile(original) as archive:
        entries = {entry.filename: archive.read(entry) for entry in archive.infolist()}
    if mutation is None:
        entries[PROJECT_ENTRY] += b"\n"
    else:
        project = json.loads(entries[PROJECT_ENTRY].decode("utf-8"))
        mutation(project)
        entries[PROJECT_ENTRY] = (json.dumps(project, indent=2, sort_keys=True, allow_nan=False) + "\n").encode()
    if update_checksum:
        archive_manifest = json.loads(entries[MANIFEST_ENTRY].decode("utf-8"))
        archive_manifest["checksums"][PROJECT_ENTRY] = hashlib.sha256(entries[PROJECT_ENTRY]).hexdigest()
        entries[MANIFEST_ENTRY] = (json.dumps(archive_manifest, indent=2, sort_keys=True, allow_nan=False) + "\n").encode()
    destination.unlink(missing_ok=True)
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, content in entries.items():
            archive.writestr(name, content)


def _first_result(project: dict[str, Any]) -> dict[str, Any]:
    return project["results"][0]


def fail_closed_mutations(archive: Path) -> dict[str, Any]:
    mutations: dict[str, tuple[Callable[[dict[str, Any]], None] | None, bool]] = {
        "feature_identity": (lambda doc: _first_result(doc)["provenance"].__setitem__("method", "cca"), True),
        "method_version": (lambda doc: _first_result(doc)["payload"]["estimation"]["cta_pls"].__setitem__("method_version", "cta_pls_v0"), True),
        "dataset_fingerprint": (lambda doc: _first_result(doc)["provenance"].__setitem__("dataset_fingerprint", "v2:tampered"), True),
        "checksum": (None, False),
        "malformed_payload": (lambda doc: _first_result(doc)["payload"]["estimation"]["cta_pls"]["estimates"][0].__setitem__("absolute_tetrad", -1), True),
        "legacy_reinterpretation": (lambda doc: doc["recipes"][0]["settings"].__setitem__("method", "pls_pm"), True),
    }
    results: dict[str, Any] = {}
    for category, (mutation, update_checksum) in mutations.items():
        path = REPORT_ROOT / f"mutation_{category}.qpls"
        write_mutated_archive(archive, path, mutation, update_checksum=update_checksum)
        completed, execution = run_command([str(CLI), "inspect", repository_path(path), "--json"], timeout=120)
        results[category] = {"passed": completed.returncode != 0, "archive": repository_path(path), "sha256": sha256_file(path), "execution": execution}
    return {"passed": all(row["passed"] for row in results.values()), "categories": results}


def evaluate_method_functional_offline(
    browser: dict[str, Any],
    *,
    analysis_export_save_reopen_succeeded: bool,
) -> dict[str, Any]:
    """Classify application traffic independently from platform background egress."""
    observed_origins = browser.get("origins")
    external_requests = browser.get("externalRequests")
    observed_request_count = browser.get("observedRequestCount")
    external_request_count = browser.get("externalRequestCount")
    valid_origins = (
        isinstance(observed_origins, list)
        and observed_origins == INTERNAL_APP_ORIGINS
        and all(isinstance(origin, str) for origin in observed_origins)
    )
    valid_external_requests = isinstance(external_requests, list) and not external_requests
    valid_counts = (
        isinstance(observed_request_count, int)
        and not isinstance(observed_request_count, bool)
        and observed_request_count > 0
        and isinstance(external_request_count, int)
        and not isinstance(external_request_count, bool)
        and external_request_count == 0
    )
    no_external_app_requests = (
        browser.get("passed") is True
        and valid_origins
        and valid_external_requests
        and valid_counts
    )
    return {
        "passed": analysis_export_save_reopen_succeeded and no_external_app_requests,
        "analysis_export_save_reopen_succeeded": analysis_export_save_reopen_succeeded,
        "no_external_app_requests": no_external_app_requests,
        "runtime_network_dependency": not no_external_app_requests,
        "allowed_origins": INTERNAL_APP_ORIGINS,
        "observed_origins": observed_origins if isinstance(observed_origins, list) else [],
        "observed_request_count": observed_request_count,
        "external_request_count": external_request_count,
        "external_requests": external_requests if isinstance(external_requests, list) else [],
        "browser": browser,
    }


def read_network_observation(path: Path) -> dict[str, Any]:
    samples = [json.loads(line) for line in path.read_text(encoding="utf-8-sig").splitlines() if line.strip()]
    valid_samples = all(isinstance(sample, dict) for sample in samples)
    roots_present = valid_samples and all(sample.get("root_present") is True for sample in samples)
    observation_kind_is_exact = valid_samples and all(
        sample.get("observation") == "sampled_exact_process_tree_tcp_v1"
        for sample in samples
    )
    remote_lists_valid = valid_samples and all(
        isinstance(sample.get("remote_connections"), list)
        for sample in samples
    )
    remote = [
        connection
        for sample in samples
        for connection in sample.get("remote_connections", [])
    ] if remote_lists_valid else []
    observation_complete = bool(samples) and roots_present and observation_kind_is_exact and remote_lists_valid
    egress_observed = bool(remote)
    return {
        # This pass means the process-tree observation is complete, not that it
        # met the separate commercial zero-egress release criterion.
        "passed": observation_complete,
        "observation_kind": "sampled_exact_process_tree_tcp_v1",
        "sample_count": len(samples),
        "root_present_every_sample": roots_present,
        "platform_background_egress_observed": egress_observed,
        "commercial_zero_egress_passed": observation_complete and not egress_observed,
        "remote_connections": remote,
    }


def screenshot_integrity(raw: dict[str, Any]) -> dict[str, Any]:
    observed = raw.get("screenshots", [])
    rows: list[dict[str, Any]] = []
    safe_paths: list[str] = []
    for value in observed:
        path = Path(value).resolve() if isinstance(value, str) else Path()
        safe = isinstance(value, str) and ROOT.resolve() in path.parents and path.is_file() and path.stat().st_size > 0
        relative = repository_path(path) if safe else None
        if safe and relative is not None:
            safe_paths.append(relative)
        rows.append({
            "path": relative,
            "passed": safe,
            "size": path.stat().st_size if safe else None,
            "sha256": sha256_file(path) if safe else None,
        })
    names = {Path(path).name for path in safe_paths}
    required = {
        f"207-tauri-native-cta-pls-reopened-{viewport}.png"
        for viewport in ("1024x700", "1280x720", "1440x900")
    }
    return {
        "passed": len(observed) == len(set(observed)) and all(row["passed"] for row in rows) and required <= names,
        "required_responsive_screenshots": sorted(required),
        "artifacts": rows,
        "paths": sorted(safe_paths),
    }


def source_freshness() -> dict[str, Any]:
    if not DESKTOP.is_file() or not CLI.is_file():
        return {"passed": False, "reason": "release binaries missing"}
    desktop_time = DESKTOP.stat().st_mtime_ns
    cli_time = CLI.stat().st_mtime_ns
    desktop_sources = build_source_paths()
    cli_sources = cli_source_paths()
    desktop_rows = [
        {"path": relative, "mtime_ns": (ROOT / relative).stat().st_mtime_ns}
        for relative in desktop_sources
    ]
    cli_rows = [
        {"path": relative, "mtime_ns": (ROOT / relative).stat().st_mtime_ns}
        for relative in cli_sources
    ]
    desktop_newer = [row["path"] for row in desktop_rows if row["mtime_ns"] > desktop_time]
    cli_newer = [row["path"] for row in cli_rows if row["mtime_ns"] > cli_time]
    gate_rows = [
        {"path": relative, "mtime_ns": (ROOT / relative).stat().st_mtime_ns}
        for relative in sorted(GATE_SOURCES)
    ]
    return {
        "passed": not desktop_newer and not cli_newer,
        "desktop_mtime_ns": desktop_time,
        "cli_mtime_ns": cli_time,
        "desktop_newer_sources": desktop_newer,
        "cli_newer_sources": cli_newer,
        "desktop_sources": desktop_rows,
        "cli_sources": cli_rows,
        "gate_sources": gate_rows,
    }


def validate_packaged_report(report: dict[str, Any]) -> list[str]:
    schema = strict_load_json(SCHEMA)
    Draft202012Validator.check_schema(schema)
    return [error.message for error in sorted(Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(report), key=lambda row: list(row.path))]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args()
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    prior = verify_prior_factory_stages()
    if not prior["passed"]:
        print(json.dumps(prior, indent=2))
        return 1
    builds: list[dict[str, Any]] = []
    if not args.skip_build:
        for command, timeout in [
            (["cargo", "build", "--release", "-p", "qpls-cli"], 1800),
            (["npm.cmd", "run", "tauri", "--", "build", "--no-bundle"], 1800),
        ]:
            completed, execution = run_command(command, timeout=timeout)
            builds.append(execution)
            if completed.returncode != 0:
                print(json.dumps(execution, indent=2))
                return 1
    freshness = source_freshness()
    if not freshness["passed"]:
        print(json.dumps(freshness, indent=2))
        return 1
    for artifact in (
        FACTORY_RAW_REPORT, FACTORY_ARCHIVE, FACTORY_XLSX, FACTORY_NETWORK,
        RUNTIME_XLSX, RUNTIME_NETWORK,
    ):
        artifact.unlink(missing_ok=True)
    started = datetime.now(timezone.utc)
    completed, execution = run_command([
        "powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
        "validation/run_cta_pls_native_acceptance.ps1",
        "-ExportPath", str(RUNTIME_XLSX),
        "-NetworkSamplesPath", str(RUNTIME_NETWORK),
    ], timeout=1800)
    cleanup_probe, cleanup_execution = run_command([
        "powershell", "-NoProfile", "-Command", "@(Get-Process -Name 'quickpls-desktop' -ErrorAction SilentlyContinue).Count",
    ], timeout=30)
    clean_after = cleanup_probe.returncode == 0 and cleanup_probe.stdout.strip() == "0"
    execution["clean_after_wrapper"] = clean_after
    execution["cleanup_probe"] = cleanup_execution
    required = [RAW_REPORT, RUNTIME_XLSX, RUNTIME_NETWORK]
    if completed.returncode != 0 or not clean_after or any(not path.is_file() for path in required):
        print(json.dumps(execution, indent=2))
        return 1

    raw = strict_load_json(RAW_REPORT)
    generated = _parse_utc(raw["generatedAt"])
    project_path = Path(raw["checks"]["ctaPlsFixture"]["projectPath"])
    shutil.copy2(project_path, FACTORY_ARCHIVE)
    shutil.copy2(RAW_REPORT, FACTORY_RAW_REPORT)
    shutil.copy2(RUNTIME_XLSX, FACTORY_XLSX)
    shutil.copy2(RUNTIME_NETWORK, FACTORY_NETWORK)
    project, archive_manifest, _ = read_archive(FACTORY_ARCHIVE)
    tables = read_xlsx_tables(FACTORY_XLSX)
    exact = verify_exact_values(raw, project, tables)
    offline = read_network_observation(FACTORY_NETWORK)
    screenshots = screenshot_integrity(raw)
    mutations = fail_closed_mutations(FACTORY_ARCHIVE)
    invalid = raw["checks"]["ctaPlsInvalidSetup"]
    reopen = raw["checks"]["ctaPlsSaveReopen"]
    responsive = raw["checks"]["ctaPlsResponsiveViewports"]
    focused_report = {
        "passed": raw.get("passed") is True
        and raw.get("focusedRun", {}).get("scope") == "cta_pls"
        and generated >= started
        and not raw.get("failures") and not raw.get("consoleErrors"),
        "path": repository_path(FACTORY_RAW_REPORT),
        "sha256": sha256_file(FACTORY_RAW_REPORT),
        "generated_at_utc": raw["generatedAt"],
        "focused_run": raw.get("focusedRun"),
    }
    invalid_setup = {
        "passed": invalid.get("attempted") is True
        and invalid.get("startEnabled") is False
        and invalid.get("runStateUnchanged") is True
        and invalid.get("resultCreated") is False,
        "evidence": invalid,
    }
    same_run = {
        "passed": reopen.get("sameRunRestored") is True
        and reopen.get("sameVisibleValuesRestored") is True
        and exact["same_run_restored"],
        "run_id": reopen.get("expectedRunId"),
    }
    exact_archive = {
        "passed": exact["archive_identity"],
        "path": repository_path(FACTORY_ARCHIVE),
        "sha256": sha256_file(FACTORY_ARCHIVE),
        "schema_version": archive_manifest.get("schema_version"),
    }
    exact_xlsx = {
        "passed": exact["passed"],
        "path": repository_path(FACTORY_XLSX),
        "sha256": sha256_file(FACTORY_XLSX),
        "sheets": sorted(tables),
        "exact_values": exact,
    }
    workflow_succeeded = all(
        check["passed"]
        for check in (focused_report, same_run, exact_archive, exact_xlsx)
    )
    method_functional_offline = evaluate_method_functional_offline(
        raw["checks"].get("ctaPlsBrowserNetwork", {}),
        analysis_export_save_reopen_succeeded=workflow_succeeded,
    )
    checks = {
        "passed": False,
        "prior_factory_stages": prior,
        "focused_report": focused_report,
        "tested_binary": {"passed": True, "desktop": repository_path(DESKTOP), "desktop_sha256": sha256_file(DESKTOP), "cli": repository_path(CLI), "cli_sha256": sha256_file(CLI)},
        "source_freshness": freshness,
        "invalid_setup": invalid_setup,
        "same_run": same_run,
        "exact_archive": exact_archive,
        "exact_xlsx": exact_xlsx,
        "responsive_viewports": {"passed": responsive.get("passed") is True and len(responsive.get("exactViewports", [])) == 3 and screenshots["passed"], "evidence": responsive, "screenshot_integrity": screenshots},
        "method_functional_offline": method_functional_offline,
        "platform_background_egress_observation": offline,
        "cleanup": {"passed": clean_after, "probe": cleanup_execution},
        "fail_closed_mutations": mutations,
    }
    checks["passed"] = all(value["passed"] for key, value in checks.items() if key != "passed")
    report = write_identity_report(
        "packaged_acceptance",
        passed=checks["passed"],
        checks=checks,
        extras=[
            *sorted(GATE_SOURCES),
            *build_source_paths(), repository_path(FACTORY_RAW_REPORT),
            repository_path(FACTORY_ARCHIVE), repository_path(FACTORY_XLSX), repository_path(FACTORY_NETWORK),
            *screenshots["paths"],
        ],
        execution=execution,
    )
    errors = validate_packaged_report(strict_load_json(report))
    if errors:
        print(json.dumps({"schema_errors": errors}, indent=2))
        return 1
    audit, audit_execution = run_command(["python", "validation/cta_pls_v1_factory_audit.py"], timeout=600)
    if audit.returncode != 0:
        print(json.dumps(audit_execution, indent=2))
        return 1
    final_manifest = validate_manifest(ROOT / "validation" / "methods" / "cta_pls_v1.manifest.json", ROOT)
    if not final_manifest["passed"] or final_manifest["derived_state"] != "release_qualified":
        print(json.dumps(final_manifest, indent=2))
        return 1
    print(f"wrote {report} | passed={checks['passed']} | builds={len(builds)}")
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
