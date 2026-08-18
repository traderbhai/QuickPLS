#!/usr/bin/env python3
"""Same-run CSV/XLSX and fail-closed export gate for PLS power v1."""

from __future__ import annotations

import csv
import json
import re
import zipfile
from copy import deepcopy
from pathlib import Path
from typing import Any
from xml.etree import ElementTree

from pls_sample_size_power_simulation import (
    CLI,
    EXECUTION_SOURCES,
    FEATURE_ID,
    METHOD_VERSION,
    WORK_ROOT,
    canonical_recipe,
    ensure_fixture_dataset,
    repository_path,
    require_current_cli,
    require_stable_execution_sources,
    run_command,
    run_product_power,
    sha256_file,
    write_identity_report,
)


SOURCE = "validation/pls_sample_size_power_export_gate.py"
CELL_REFERENCE = re.compile(r"([A-Z]+)[0-9]+")
MAIN_NS = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"


def _column_index(reference: str) -> int:
    match = CELL_REFERENCE.fullmatch(reference)
    if match is None:
        raise ValueError(f"invalid XLSX cell reference: {reference!r}")
    value = 0
    for character in match.group(1):
        value = value * 26 + ord(character) - ord("A") + 1
    return value - 1


def read_xlsx_rows(path: Path) -> list[list[str]]:
    """Read the first worksheet using only the standard library."""

    with zipfile.ZipFile(path) as archive:
        names = set(archive.namelist())
        shared: list[str] = []
        if "xl/sharedStrings.xml" in names:
            root = ElementTree.fromstring(archive.read("xl/sharedStrings.xml"))
            for item in root.findall(f"{{{MAIN_NS}}}si"):
                shared.append(
                    "".join(node.text or "" for node in item.iter(f"{{{MAIN_NS}}}t"))
                )
        workbook = ElementTree.fromstring(archive.read("xl/workbook.xml"))
        relationships = ElementTree.fromstring(
            archive.read("xl/_rels/workbook.xml.rels")
        )
        relation_targets = {
            row.attrib["Id"]: row.attrib["Target"] for row in relationships
        }
        sheet = workbook.find(f"{{{MAIN_NS}}}sheets/{{{MAIN_NS}}}sheet")
        if sheet is None:
            raise ValueError("XLSX contains no worksheet")
        relationship_id = sheet.attrib[
            "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}id"
        ]
        target = relation_targets[relationship_id].lstrip("/")
        worksheet_name = target if target.startswith("xl/") else f"xl/{target}"
        worksheet = ElementTree.fromstring(archive.read(worksheet_name))
        rows: list[list[str]] = []
        for row in worksheet.findall(f".//{{{MAIN_NS}}}row"):
            cells: dict[int, str] = {}
            for cell in row.findall(f"{{{MAIN_NS}}}c"):
                index = _column_index(cell.attrib["r"])
                cell_type = cell.attrib.get("t")
                if cell_type == "inlineStr":
                    value = "".join(
                        node.text or "" for node in cell.iter(f"{{{MAIN_NS}}}t")
                    )
                else:
                    node = cell.find(f"{{{MAIN_NS}}}v")
                    value = "" if node is None or node.text is None else node.text
                    if cell_type == "s":
                        value = shared[int(value)]
                cells[index] = value
            if cells:
                rows.append([cells.get(index, "") for index in range(max(cells) + 1)])
        return rows


def read_csv_rows(path: Path) -> list[list[str]]:
    with path.open("r", encoding="utf-8-sig", newline="") as handle:
        return list(csv.reader(handle))


def _normalize_rows(rows: list[list[str]]) -> list[list[str]]:
    normalized = []
    for row in rows:
        current = list(row)
        while current and current[-1] == "":
            current.pop()
        normalized.append(current)
    return normalized


def _metadata(rows: list[list[str]]) -> dict[str, str]:
    header = rows[0]
    positions = {name: index for index, name in enumerate(header)}
    values: dict[str, str] = {}
    for row in rows[1:]:
        padded = [*row, *([""] * max(0, len(header) - len(row)))]
        if padded[positions["section"]] == "metadata":
            values[padded[positions["metric"]]] = padded[positions["value"]]
    return values


def _write_json(path: Path, value: Any) -> None:
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )


def _tamper_export(
    *, name: str, document: dict[str, Any], output: Path
) -> tuple[dict[str, Any], dict[str, Any]]:
    result_path = WORK_ROOT / f"{name}.tampered.result.json"
    _write_json(result_path, document)
    if output.exists():
        output.unlink()
    completed, execution = run_command(
        [
            str(CLI),
            "export",
            str(result_path),
            "--format",
            "csv",
            "--output",
            str(output),
        ],
        timeout=120,
    )
    combined = (completed.stdout + "\n" + completed.stderr).lower()
    check = {
        "passed": completed.returncode != 0 and not output.exists(),
        "nonzero_exit": completed.returncode != 0,
        "no_export_created": not output.exists(),
        "error_mentions_typed_power": "typed pls sample-size/power" in combined
        or "sample-size/power" in combined,
    }
    check["passed"] = check["passed"] and check["error_mentions_typed_power"]
    return check, execution


def main() -> int:
    cli_identity = require_current_cli()
    before = cli_identity["execution_sources"]
    dataset, fingerprint, fixture_execution = ensure_fixture_dataset()
    recipe = canonical_recipe(
        fingerprint,
        name="export_same_run",
        population_path=0.45,
        sample_size_grid=(60, 120),
        monte_carlo_replicates=100,
        bootstrap_replicates=99,
        target_power=0.70,
        seed=20_260_819,
        workers=2,
    )
    run = run_product_power(name="export_same_run", recipe=recipe, dataset=dataset)
    result_path = WORK_ROOT / "export_same_run.result.json"
    csv_path = WORK_ROOT / "export_same_run.csv"
    xlsx_path = WORK_ROOT / "export_same_run.xlsx"
    for path in (csv_path, xlsx_path):
        if path.exists():
            path.unlink()

    csv_completed, csv_execution = run_command(
        [
            str(CLI),
            "export",
            str(result_path),
            "--format",
            "csv",
            "--output",
            str(csv_path),
        ],
        timeout=300,
    )
    xlsx_completed, xlsx_execution = run_command(
        [
            str(CLI),
            "export",
            str(result_path),
            "--format",
            "xlsx",
            "--output",
            str(xlsx_path),
        ],
        timeout=300,
    )
    csv_rows = read_csv_rows(csv_path) if csv_path.is_file() else []
    xlsx_rows = read_xlsx_rows(xlsx_path) if xlsx_path.is_file() else []
    normalized_csv = _normalize_rows(csv_rows)
    normalized_xlsx = _normalize_rows(xlsx_rows)
    metadata = _metadata(csv_rows) if csv_rows else {}
    sections = {row[0] for row in csv_rows[1:] if row}
    same_table = normalized_csv == normalized_xlsx

    native_test, native_test_execution = run_command(
        [
            "npx.cmd",
            "vitest",
            "run",
            "src/native/nativePlsSampleSizePower.test.ts",
            "--reporter=verbose",
        ],
        timeout=600,
    )
    native_output = native_test.stdout + "\n" + native_test.stderr
    native_source = (WORK_ROOT.parents[3] / "src" / "native" / "nativePlsSampleSizePower.ts").read_text(
        encoding="utf-8"
    )
    native_table_contract = all(
        token in native_source
        for token in (
            'name: "Power by sample size"',
            'name: "Simulation failures"',
            'name: "Design assumptions"',
            'name: "Run provenance"',
        )
    ) and "exports all four contract tables from the same validated result" in native_output

    changed_row = deepcopy(run["document"])
    changed_row["payload"]["analysis"]["rows"][0]["rejections"] += 1
    row_tamper, row_tamper_execution = _tamper_export(
        name="export_changed_row",
        document=changed_row,
        output=WORK_ROOT / "export_changed_row.must_not_exist.csv",
    )
    changed_identity = deepcopy(run["document"])
    changed_identity["payload"]["analysis"]["capability_id"] = "qpls3.pls.power_forgery"
    identity_tamper, identity_tamper_execution = _tamper_export(
        name="export_changed_identity",
        document=changed_identity,
        output=WORK_ROOT / "export_changed_identity.must_not_exist.csv",
    )

    source_stability = require_stable_execution_sources(before)
    checks = {
        "typed_run_completed": run["passed"],
        "csv_export_completed": csv_completed.returncode == 0
        and csv_path.is_file()
        and csv_path.stat().st_size > 0,
        "xlsx_export_completed": xlsx_completed.returncode == 0
        and xlsx_path.is_file()
        and xlsx_path.stat().st_size > 0,
        "csv_xlsx_tables_exact": same_table,
        "same_result_id": metadata.get("result_id") == run["document"]["id"],
        "same_recipe_id": metadata.get("recipe_id") == run["document"]["provenance"]["recipe_id"],
        "same_dataset_fingerprint": metadata.get("dataset_fingerprint") == fingerprint,
        "same_method_version": metadata.get("method_version") == METHOD_VERSION,
        "typed_identity_exported": any(
            len(row) >= 7
            and row[0] == "pls_power_provenance"
            and row[5] == "capability_id"
            and row[6] == FEATURE_ID
            for row in csv_rows
        ),
        "required_tables_present": {
            "pls_power_by_sample_size",
            "pls_power_replicate_ledger",
            "pls_power_provenance",
        }.issubset(sections),
        "native_four_table_contract_executed": native_test.returncode == 0
        and native_table_contract,
        "changed_row_rejected": row_tamper["passed"],
        "changed_identity_rejected": identity_tamper["passed"],
        "source_stable_during_gate": source_stability["passed"],
    }
    passed = all(checks.values())
    report = {
        "passed": passed,
        "checks": checks,
        "same_run": {
            "result": repository_path(result_path),
            "result_sha256": sha256_file(result_path),
            "csv": repository_path(csv_path) if csv_path.exists() else None,
            "csv_sha256": sha256_file(csv_path) if csv_path.exists() else None,
            "xlsx": repository_path(xlsx_path) if xlsx_path.exists() else None,
            "xlsx_sha256": sha256_file(xlsx_path) if xlsx_path.exists() else None,
            "row_count": len(csv_rows),
            "sections": sorted(sections),
            "metadata": metadata,
        },
        "tamper_checks": {
            "changed_row": row_tamper,
            "changed_identity": identity_tamper,
        },
        "standalone_integrity_limit": (
            "The CLI export path validates typed shape, identities, accounting, rows, ledger order, "
            "and decision. Full recipe/outcome digest recomputation is covered by the project "
            "persistence gate because a standalone result does not carry its full recipe."
        ),
        "source_stability": source_stability,
    }
    path = write_identity_report(
        "export_report",
        passed=passed,
        checks=report,
        execution=[
            fixture_execution,
            run["execution"],
            csv_execution,
            xlsx_execution,
            native_test_execution,
            row_tamper_execution,
            identity_tamper_execution,
        ],
        extras=[
            SOURCE,
            "src/native/nativePlsSampleSizePower.ts",
            "src/native/nativePlsSampleSizePower.test.ts",
            *EXECUTION_SOURCES,
        ],
    )
    print(json.dumps({"passed": passed, "output": str(path)}, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
