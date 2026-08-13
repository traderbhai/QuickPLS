"""Build, run, and independently verify focused Windows PCA v1 acceptance.

The existing UI runner remains the source of user-path evidence.  This wrapper
adds method-factory identity binding, opens the saved archive and XLSX itself,
cross-checks their exact PCA values, and executes fail-closed archive mutations.
"""

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

from diagnostic_bundle_source_manifest import (
    SourceManifestFailure,
    validate_build_receipt,
)
from method_promotion_manifest import _verify_artifact
from pca_v1_factory_common import (
    REPORT_ROOT,
    ROOT,
    manifest,
    repository_path,
    run_command,
    sha256_file,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/pca_v1_packaged_acceptance.py"
RAW_REPORT = ROOT / "validation" / "results" / "v247_tauri_native_acceptance_pca.json"
FACTORY_RAW_REPORT = REPORT_ROOT / "pca_v1_packaged_raw.json"
FACTORY_ARCHIVE = REPORT_ROOT / "pca_v1_packaged.qpls"
FACTORY_XLSX = REPORT_ROOT / "pca_v1_packaged.xlsx"
BUILD_RECEIPT = ROOT / "validation" / "results" / "diagnostic_bundle_build_receipt.json"
DESKTOP = ROOT / "target" / "release" / "quickpls-desktop.exe"
RELEASE_CLI = ROOT / "target" / "release" / "qpls.exe"
PROJECT_ENTRY = "project.json"
MANIFEST_ENTRY = "manifest.json"
GATE_SOURCES = {
    SOURCE,
    "validation/diagnostic_bundle_source_manifest.py",
    "validation/method_promotion_manifest.py",
    "validation/methods/method_promotion_manifest.schema.json",
    "validation/methods/pca_v1.manifest.json",
    "validation/pca_v1_factory_audit.py",
    "validation/pca_v1_factory_common.py",
    "validation/run_v247_pca_native_acceptance.ps1",
    "validation/test_pca_v1_packaged_adapter.py",
    "validation/v247_tauri_native_acceptance.mjs",
}


def cli_source_paths() -> list[str]:
    """Return the exact local Rust source closure consumed by release qpls.exe."""
    paths = {"Cargo.lock", "Cargo.toml", "crates/qpls-cli/Cargo.toml"}
    paths.update(
        repository_path(path)
        for path in (ROOT / "crates" / "qpls-cli" / "src").rglob("*.rs")
        if path.is_file()
    )
    for crate in (
        "qpls-assessment",
        "qpls-core",
        "qpls-data",
        "qpls-estimation",
        "qpls-project",
        "qpls-resampling",
        "qpls-runner",
    ):
        crate_root = ROOT / "crates" / crate
        paths.add(repository_path(crate_root / "Cargo.toml"))
        paths.update(
            repository_path(path)
            for path in (crate_root / "src").rglob("*.rs")
            if path.is_file()
        )
    return sorted(paths)


def _binary_descriptor(path: Path) -> dict[str, Any]:
    return {
        "path": path.resolve().relative_to(ROOT.resolve()).as_posix(),
        "size": path.stat().st_size,
        "sha256": sha256_file(path),
        "mtime_ns": path.stat().st_mtime_ns,
    }


def source_freshness() -> dict[str, Any]:
    """Bind the desktop to the frozen receipt and the CLI to build sources only.

    PCA gate files, including the shared v247 runner, are recorded as evidence
    but intentionally do not make a frozen product binary stale.
    """
    try:
        if not BUILD_RECEIPT.is_file() or not DESKTOP.is_file() or not RELEASE_CLI.is_file():
            raise FileNotFoundError("frozen build receipt, release desktop, or release CLI is missing")
        receipt = strict_load_json(BUILD_RECEIPT)
        validate_build_receipt(receipt, ROOT)
        desktop = _binary_descriptor(DESKTOP)
        release_cli = _binary_descriptor(RELEASE_CLI)
        cli_rows = [
            {
                "path": relative,
                "mtime_ns": (ROOT / relative).stat().st_mtime_ns,
                "size": (ROOT / relative).stat().st_size,
                "sha256": sha256_file(ROOT / relative),
            }
            for relative in cli_source_paths()
        ]
        cli_newer = [row["path"] for row in cli_rows if row["mtime_ns"] > release_cli["mtime_ns"]]
        gate_rows = [
            {
                "path": relative,
                "mtime_ns": (ROOT / relative).stat().st_mtime_ns,
                "size": (ROOT / relative).stat().st_size,
                "sha256": sha256_file(ROOT / relative),
            }
            for relative in sorted(GATE_SOURCES)
        ]
        return {
            "passed": not cli_newer,
            "desktop_receipt_exact": True,
            "desktop": desktop,
            "release_cli": release_cli,
            "release_cli_newer_build_sources": cli_newer,
            "release_cli_build_sources": cli_rows,
            "gate_sources_excluded_from_binary_freshness": gate_rows,
            "build_receipt": _binary_descriptor(BUILD_RECEIPT),
            "build_receipt_schema_version": receipt.get("schema_version"),
        }
    except (FileNotFoundError, OSError, ValueError, SourceManifestFailure) as error:
        return {
            "passed": False,
            "desktop_receipt_exact": False,
            "error": str(error),
        }


def verify_prior_factory_stages() -> dict[str, Any]:
    document = manifest()
    feature = document["feature"]
    expected_identity = {
        "passed": True,
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
    }
    expected_roles = {
        "engine_only": {"method_spec", "independent_reference", "simulation_report", "boundary_report"},
        "archive_qualified": {"persistence_report"},
        "native_qualified": {"frontend_report", "export_report"},
    }
    checks: list[dict[str, Any]] = []
    for stage, roles in expected_roles.items():
        observed: set[str] = set()
        for artifact in document["qualification"]["evidence"][stage]:
            observed.update(artifact["roles"])
            passed, errors = _verify_artifact(artifact, document, ROOT, expected_identity)
            checks.append(
                {
                    "stage": stage,
                    "path": artifact["path"],
                    "roles": artifact["roles"],
                    "passed": passed,
                    "errors": errors,
                }
            )
        if observed != roles:
            checks.append(
                {
                    "stage": stage,
                    "passed": False,
                    "errors": [f"expected roles {sorted(roles)}, found {sorted(observed)}"],
                }
            )
    return {"passed": all(row["passed"] for row in checks), "artifacts": checks}


def read_archive(path: Path) -> tuple[dict[str, Any], dict[str, Any], dict[str, bytes]]:
    with zipfile.ZipFile(path) as archive:
        names = [entry.filename for entry in archive.infolist()]
        if len(names) != len(set(names)):
            raise ValueError("PCA project archive contains duplicate entries")
        if any(
            name.startswith(("/", "\\"))
            or "\\" in name
            or ".." in Path(name).parts
            for name in names
        ):
            raise ValueError("PCA project archive contains an unsafe entry path")
        entries = {name: archive.read(name) for name in names}
    project = json.loads(entries[PROJECT_ENTRY].decode("utf-8"))
    archive_manifest = json.loads(entries[MANIFEST_ENTRY].decode("utf-8"))
    checksums = archive_manifest.get("checksums", {})
    expected_names = set(entries) - {MANIFEST_ENTRY}
    if set(checksums) != expected_names:
        raise ValueError("PCA project manifest checksum membership is not exact")
    for name, expected in checksums.items():
        actual = hashlib.sha256(entries[name]).hexdigest()
        if actual != expected:
            raise ValueError(f"PCA project checksum mismatch for {name}")
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
        shared = [
            "".join(node.itertext())
            for node in shared_root.findall(f"{{{main_ns}}}si")
        ]
        rel_root = ElementTree.fromstring(workbook.read("xl/_rels/workbook.xml.rels"))
        relationships = {
            node.attrib["Id"]: node.attrib["Target"]
            for node in rel_root.findall(f"{{{package_rel_ns}}}Relationship")
        }
        workbook_root = ElementTree.fromstring(workbook.read("xl/workbook.xml"))
        tables: dict[str, list[list[str]]] = {}
        for sheet in workbook_root.findall(f".//{{{main_ns}}}sheet"):
            name = sheet.attrib["name"]
            relationship = sheet.attrib[f"{{{rel_ns}}}id"]
            target = relationships[relationship].lstrip("/")
            entry = target if target.startswith("xl/") else f"xl/{target}"
            sheet_root = ElementTree.fromstring(workbook.read(entry))
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
                    width = max(cells) + 1
                    rows.append([cells.get(index, "") for index in range(width)])
            tables[name] = rows
    return tables


def rows_after_header(rows: list[list[str]], first_cell: str) -> list[list[str]]:
    for index, row in enumerate(rows):
        if row and row[0] == first_cell:
            return rows[index:]
    raise ValueError(f"XLSX header {first_cell!r} not found")


def verify_exact_values(report: dict[str, Any], project: dict[str, Any], tables: dict[str, list[list[str]]]) -> dict[str, Any]:
    result_view = report["checks"]["pcaResult"]
    reopen = report["checks"]["pcaSaveReopen"]
    result_id = result_view["runId"]
    project_results = [row for row in project["results"] if row["id"] == result_id]
    if len(project_results) != 1:
        raise ValueError("focused PCA run does not map to exactly one archived result")
    archived = project_results[0]
    estimation = archived["payload"]["estimation"]
    pca = estimation["pca"]
    identity = {
        "same_run_restored": reopen["sameRunRestored"] is True
        and reopen["expectedRunId"] == result_id
        and reopen["selectedRunId"] == result_id,
        "provenance_method": archived["provenance"]["method"] == "pca",
        "provenance_method_version": archived["provenance"]["method_version"] == "pca_v1",
        "payload_kind": archived["payload"]["kind"] == "pls_pm_v1",
        "estimation_method_version": estimation["method_version"] == "pca_v1",
        "pca_method_version": pca["method_version"] == "pca_v1",
        "standalone_null_model": all(
            value in (None, 0)
            for value in (
                reopen["archive"]["models"],
                reopen["archive"]["activeModelId"],
                reopen["archive"]["runModelId"],
                reopen["archive"]["runModelSnapshot"],
            )
        ),
    }

    expected_summary = [
        [
            component["component"],
            f"{component['eigenvalue']:.4f}",
            f"{component['explained_variance'] * 100:.2f}%",
            f"{component['cumulative_variance'] * 100:.2f}%",
        ]
        for component in pca["components"]
    ]
    ui_summary = result_view["summary"]["values"]
    workbook_summary = rows_after_header(tables["Component summary"], "Component")
    summary_header = workbook_summary[0]
    workbook_summary_values = workbook_summary[1:]

    expected_loadings = [
        [
            row["variable"],
            row["component"],
            f"{row['loading']:.4f}",
            f"{row['weight']:.4f}",
        ]
        for row in pca["loadings"]
    ]
    ui_loadings = result_view["loadings"]["values"]
    workbook_loadings = rows_after_header(tables["Component loadings and weights"], "Variable")
    loading_header = workbook_loadings[0]
    workbook_loading_values = workbook_loadings[1:]

    scores_by_observation: dict[int, dict[str, float]] = {}
    for row in pca["scores"]:
        scores_by_observation.setdefault(row["observation"], {})[row["component"]] = row["score"]
    component_ids = [row["component"] for row in pca["components"]]
    expected_scores = [
        [str(observation + 1)]
        + [f"{values[component]:.6f}" for component in component_ids]
        for observation, values in sorted(scores_by_observation.items())
    ]
    workbook_scores = rows_after_header(tables["Component scores"], "Complete-case observation")
    score_header = workbook_scores[0]
    workbook_score_values = workbook_scores[1:]

    provenance_rows = rows_after_header(tables["Run provenance"], "Field")
    provenance = {
        row[0]: row[1]
        for row in provenance_rows[1:]
        if len(row) >= 2
    }
    exact = {
        "archive_identity": all(identity.values()),
        "identity_checks": identity,
        "component_summary_ui_matches_archive": ui_summary == expected_summary,
        "component_summary_xlsx_matches_archive": workbook_summary_values == expected_summary,
        "component_summary_header": summary_header,
        "loadings_ui_matches_archive": ui_loadings == expected_loadings,
        "loadings_xlsx_matches_archive": workbook_loading_values == expected_loadings,
        "loading_header": loading_header,
        "scores_xlsx_matches_archive": workbook_score_values == expected_scores,
        "score_header": score_header,
        "provenance_xlsx_binds_same_run": (
            provenance.get("Method version") == "pca_v1"
            and provenance.get("Dataset fingerprint")
            == archived["provenance"]["dataset_fingerprint"]
            and provenance.get("Recipe") == archived["provenance"]["recipe_id"]
        ),
        "component_count": len(expected_summary),
        "loading_count": len(expected_loadings),
        "score_count": sum(len(values) for values in scores_by_observation.values()),
    }
    exact["passed"] = all(
        value
        for key, value in exact.items()
        if key.endswith("_archive")
        or key.startswith(("archive_identity", "component_summary_", "loadings_", "scores_", "provenance_"))
        and isinstance(value, bool)
    )
    return exact


def write_mutated_archive(
    original: Path,
    destination: Path,
    mutation: Callable[[dict[str, Any]], None] | None,
    *,
    update_checksum: bool,
) -> None:
    with zipfile.ZipFile(original) as archive:
        entries = {entry.filename: archive.read(entry) for entry in archive.infolist()}
    if mutation is None:
        entries[PROJECT_ENTRY] = entries[PROJECT_ENTRY] + b"\n"
    else:
        project = json.loads(entries[PROJECT_ENTRY].decode("utf-8"))
        mutation(project)
        entries[PROJECT_ENTRY] = (
            json.dumps(project, indent=2, sort_keys=True, allow_nan=False) + "\n"
        ).encode("utf-8")
    if update_checksum:
        archive_manifest = json.loads(entries[MANIFEST_ENTRY].decode("utf-8"))
        archive_manifest["checksums"][PROJECT_ENTRY] = hashlib.sha256(
            entries[PROJECT_ENTRY]
        ).hexdigest()
        entries[MANIFEST_ENTRY] = (
            json.dumps(archive_manifest, indent=2, sort_keys=True, allow_nan=False) + "\n"
        ).encode("utf-8")
    destination.unlink(missing_ok=True)
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, content in entries.items():
            archive.writestr(name, content)


def fail_closed_mutations(archive: Path) -> dict[str, Any]:
    mutations: dict[str, tuple[Callable[[dict[str, Any]], None] | None, bool]] = {
        "feature_identity": (
            lambda doc: doc["results"][0]["provenance"].__setitem__("method", "regression"),
            True,
        ),
        "method_version": (
            lambda doc: doc["results"][0]["payload"]["estimation"]["pca"].__setitem__("method_version", "pca_v0"),
            True,
        ),
        "dataset_fingerprint": (
            lambda doc: doc["results"][0]["provenance"].__setitem__("dataset_fingerprint", "v2:tampered"),
            True,
        ),
        "checksum": (None, False),
        "malformed_payload": (
            lambda doc: doc["results"][0]["payload"]["estimation"]["pca"]["components"][0].__setitem__("eigenvalue", "not-a-number"),
            True,
        ),
        "legacy_reinterpretation": (
            lambda doc: doc["recipes"][0]["settings"].__setitem__("method", "pls_pm"),
            True,
        ),
    }
    results: dict[str, Any] = {}
    for category, (mutation, update_checksum) in mutations.items():
        path = REPORT_ROOT / f"mutation_{category}.qpls"
        write_mutated_archive(
            archive,
            path,
            mutation,
            update_checksum=update_checksum,
        )
        completed, execution = run_command(
            [str(RELEASE_CLI), "inspect", repository_path(path), "--json"],
            timeout=120,
        )
        results[category] = {
            "passed": completed.returncode != 0,
            "archive": repository_path(path),
            "archive_sha256": sha256_file(path),
            "execution": execution,
        }
    return {"passed": all(row["passed"] for row in results.values()), "categories": results}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args()
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)

    prior = verify_prior_factory_stages()
    if not prior["passed"]:
        print(json.dumps(prior, indent=2))
        raise SystemExit("PCA packaged acceptance is blocked: lightweight factory stages are not current and passing")

    build_executions: list[dict[str, Any]] = []
    if not args.skip_build:
        for command, timeout in [
            (["cargo", "build", "--release", "-p", "qpls-cli"], 1800),
            (
                [
                    "python",
                    "validation/diagnostic_bundle_source_manifest.py",
                    "build",
                    "--receipt",
                    "validation/results/diagnostic_bundle_build_receipt.json",
                ],
                1800,
            ),
        ]:
            completed, execution = run_command(command, timeout=timeout)
            build_executions.append(execution)
            if completed.returncode != 0:
                print(json.dumps(execution, indent=2))
                return 1

    freshness = source_freshness()
    if not freshness["passed"]:
        print(json.dumps(freshness, indent=2))
        return 1

    busy = []
    for process_name in ("quickpls-desktop", "qpls", "cargo", "rustc"):
        probe, _ = run_command(
            [
                "powershell",
                "-NoProfile",
                "-Command",
                f"@(Get-Process -Name '{process_name}' -ErrorAction SilentlyContinue).Count",
            ],
            timeout=30,
        )
        if probe.returncode != 0 or probe.stdout.strip() != "0":
            busy.append({"process": process_name, "observed": probe.stdout.strip()})
    if busy:
        print(json.dumps({"passed": False, "phase": "clean_process_boundary", "busy": busy}, indent=2))
        return 1

    FACTORY_XLSX.unlink(missing_ok=True)
    started = datetime.now(timezone.utc)
    completed, runner_execution = run_command(
        [
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "validation/run_v247_pca_native_acceptance.ps1",
            "-ExportPath",
            str(FACTORY_XLSX),
        ],
        timeout=1800,
    )
    lingering_probe, lingering_execution = run_command(
        [
            "powershell",
            "-NoProfile",
            "-Command",
            "@(Get-Process -Name 'quickpls-desktop' -ErrorAction SilentlyContinue).Count",
        ],
        timeout=30,
    )
    lingering = lingering_probe.returncode != 0 or lingering_probe.stdout.strip() != "0"
    if lingering:
        cleanup, cleanup_execution = run_command(
            [
                "powershell",
                "-NoProfile",
                "-Command",
                "$p=@(Get-Process -Name 'quickpls-desktop' -ErrorAction SilentlyContinue | Where-Object {$_.Path -eq [System.IO.Path]::GetFullPath('target/release/quickpls-desktop.exe')}); if($p.Count -gt 1){throw 'Refusing ambiguous cleanup'}; if($p.Count -eq 1){Stop-Process -Id $p[0].Id -Force; $p[0].WaitForExit(10000)}; if(Get-Process -Name 'quickpls-desktop' -ErrorAction SilentlyContinue){exit 1}",
            ],
            timeout=30,
        )
        runner_execution["post_failure_cleanup"] = cleanup_execution
        lingering = cleanup.returncode != 0
    runner_execution["initial_cleanup_probe"] = lingering_execution
    runner_execution["clean_after_wrapper"] = not lingering
    if (
        completed.returncode != 0
        or lingering
        or not RAW_REPORT.is_file()
        or not FACTORY_XLSX.is_file()
    ):
        print(json.dumps(runner_execution, indent=2))
        return 1

    raw = strict_load_json(RAW_REPORT)
    generated = datetime.fromisoformat(raw["generatedAt"].replace("Z", "+00:00"))
    focused = raw.get("focusedRun", {})
    project_path = Path(raw["checks"]["pcaFixture"]["projectPath"])
    if not project_path.is_file():
        raise FileNotFoundError(project_path)
    shutil.copy2(project_path, FACTORY_ARCHIVE)
    shutil.copy2(RAW_REPORT, FACTORY_RAW_REPORT)

    project, archive_manifest, _ = read_archive(FACTORY_ARCHIVE)
    tables = read_xlsx_tables(FACTORY_XLSX)
    exact_values = verify_exact_values(raw, project, tables)
    mutations = fail_closed_mutations(FACTORY_ARCHIVE)
    required_sheets = {
        "Component summary",
        "Component loadings and weights",
        "Calculation scope",
        "Component scores",
        "Run provenance",
    }
    packaged_checks = {
        "passed": (
            raw.get("passed") is True
            and focused.get("scope") == "pca"
            and generated >= started
            and not raw.get("failures")
            and not raw.get("consoleErrors")
            and set(tables) == required_sheets
            and exact_values["passed"]
            and mutations["passed"]
            and freshness["passed"]
        ),
        "focused_scope": focused,
        "fresh_after_invocation_start": generated >= started,
        "raw_report_passed": raw.get("passed") is True,
        "failures": raw.get("failures"),
        "console_errors": raw.get("consoleErrors"),
        "builds": build_executions,
        "source_freshness": freshness,
        "runner": runner_execution,
        "archive": {
            "path": repository_path(FACTORY_ARCHIVE),
            "sha256": sha256_file(FACTORY_ARCHIVE),
            "schema_version": archive_manifest.get("schema_version"),
            "checksums_verified": True,
        },
        "xlsx": {
            "path": repository_path(FACTORY_XLSX),
            "sha256": sha256_file(FACTORY_XLSX),
            "sheets": sorted(tables),
        },
        "exact_values": exact_values,
        "fail_closed_mutations": mutations,
    }
    packaged_report = write_identity_report(
        "packaged_acceptance",
        passed=packaged_checks["passed"],
        checks=packaged_checks,
        extras=[
            SOURCE,
            "validation/diagnostic_bundle_source_manifest.py",
            "validation/test_pca_v1_packaged_adapter.py",
            "validation/v247_tauri_native_acceptance.mjs",
            repository_path(BUILD_RECEIPT),
            repository_path(DESKTOP),
            repository_path(RELEASE_CLI),
            "Cargo.lock",
            "package.json",
            "src-tauri/Cargo.toml",
            "src-tauri/src/lib.rs",
            "src-tauri/tauri.conf.json",
            "crates/qpls-estimation/src/pls.rs",
            "crates/qpls-project/src/lib.rs",
            "src/native/nativePca.ts",
            "src/native/nativeResults.ts",
            "src/native/nativeExportTables.ts",
            repository_path(FACTORY_RAW_REPORT),
            repository_path(FACTORY_ARCHIVE),
            repository_path(FACTORY_XLSX),
        ],
        execution=runner_execution,
    )
    print(f"wrote {packaged_report} | passed={packaged_checks['passed']}")
    if not packaged_checks["passed"]:
        return 1
    audit, audit_execution = run_command(
        ["python", "validation/pca_v1_factory_audit.py"],
        timeout=600,
    )
    if audit.returncode != 0:
        print(json.dumps(audit_execution, indent=2))
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
