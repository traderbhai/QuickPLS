"""Fresh PLSc v2 archive round-trip and fail-closed tamper evidence."""

from __future__ import annotations

import hashlib
import json
import subprocess
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable

from plsc_v2_factory_common import (
    CLI,
    REPORT_ROOT,
    ROOT,
    WORK_ROOT,
    construct,
    repository_path,
    run_command,
    run_plsc,
    sha256_file,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/plsc_v2_persistence_gate.py"
PROJECT_SOURCE = "crates/qpls-project/src/lib.rs"
ARCHIVE_SOURCE = "crates/qpls-project/src/archive_integrity.rs"
MANIFEST_ENTRY = "manifest.json"
PROJECT_ENTRY = "project.json"


def _read_archive(path: Path) -> tuple[dict[str, bytes], dict[str, Any], dict[str, Any]]:
    with zipfile.ZipFile(path) as archive:
        entries = {entry.filename: archive.read(entry) for entry in archive.infolist()}
    return (
        entries,
        json.loads(entries[MANIFEST_ENTRY].decode("utf-8")),
        json.loads(entries[PROJECT_ENTRY].decode("utf-8")),
    )


def _write_archive(
    source: Path,
    destination: Path,
    mutation: Callable[[dict[str, Any]], None] | None,
    *,
    update_checksum: bool,
) -> None:
    entries, manifest, project = _read_archive(source)
    if mutation is None:
        entries[PROJECT_ENTRY] += b"\n"
    else:
        mutation(project)
        entries[PROJECT_ENTRY] = (
            json.dumps(project, indent=2, sort_keys=True, allow_nan=False) + "\n"
        ).encode("utf-8")
    if update_checksum:
        manifest["checksums"][PROJECT_ENTRY] = hashlib.sha256(entries[PROJECT_ENTRY]).hexdigest()
        entries[MANIFEST_ENTRY] = (
            json.dumps(manifest, indent=2, sort_keys=True, allow_nan=False) + "\n"
        ).encode("utf-8")
    destination.unlink(missing_ok=True)
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, content in entries.items():
            archive.writestr(name, content)


def _plsc_result(project: dict[str, Any]) -> dict[str, Any]:
    matches = [
        result
        for result in project.get("results", [])
        if result.get("provenance", {}).get("method") == "plsc"
    ]
    if len(matches) != 1:
        raise ValueError(f"factory archive must contain exactly one PLSc result, found {len(matches)}")
    return matches[0]


def _mutate_feature_identity(project: dict[str, Any]) -> None:
    _plsc_result(project)["provenance"]["method"] = "pls_pm"


def _mutate_method_version(project: dict[str, Any]) -> None:
    result = _plsc_result(project)
    result["payload"]["estimation"]["method_version"] = "plsc_v999"
    result["payload"]["estimation"]["plsc"]["method_version"] = "plsc_v999"
    result["provenance"]["method_version"] = "pls_pm_v1+plsc_v999"


def _mutate_dataset_fingerprint(project: dict[str, Any]) -> None:
    _plsc_result(project)["provenance"]["dataset_fingerprint"] = "v2:" + "0" * 64


def _mutate_malformed_payload(project: dict[str, Any]) -> None:
    result = _plsc_result(project)
    result["payload"]["estimation"]["plsc"]["reliabilities"][0]["rho_a"] = "not-a-number"


def _create_archive() -> tuple[Path, dict[str, Any]]:
    csv_path = WORK_ROOT / "persistence_plsc.csv"
    # Reuse the deterministic independently validated data bytes without
    # importing the reference script's global output paths.
    csv_path.write_bytes(
        (ROOT / "validation" / "results" / "plsc_reference.csv").read_bytes()
    )
    run = run_plsc(
        name="factory_persistence_plsc_v2",
        csv_path=csv_path,
        constructs=[
            construct("x", ["x1", "x2"]),
            construct("z", ["z1", "z2"]),
            construct("y", ["y1", "y2"]),
        ],
        paths=[{"source": "x", "target": "y"}, {"source": "z", "target": "y"}],
    )
    archive = WORK_ROOT / "plsc_v2_factory_round_trip.qpls"
    archive.unlink(missing_ok=True)
    imported = ROOT / run["fingerprint_execution"]["project"]
    entries, archive_manifest, project = _read_archive(imported)
    recipe = strict_load_json(ROOT / run["recipe"])
    result = strict_load_json(ROOT / run["output"])
    if (
        result.get("payload", {}).get("kind") != "pls_pm_v1"
        or result.get("payload", {}).get("estimation", {}).get("method_version") != "plsc_v2"
        or result.get("payload", {}).get("estimation", {}).get("plsc", {}).get("method_version")
        != "plsc_v2"
    ):
        raise ValueError("fresh persistence fixture did not produce exact PLSc v2 identity")
    project["models"] = [recipe["model"]]
    project["recipes"] = [recipe]
    project["results"] = [result]
    entries[PROJECT_ENTRY] = (
        json.dumps(project, indent=2, sort_keys=True, allow_nan=False) + "\n"
    ).encode("utf-8")
    archive_manifest["checksums"][PROJECT_ENTRY] = hashlib.sha256(entries[PROJECT_ENTRY]).hexdigest()
    entries[MANIFEST_ENTRY] = (
        json.dumps(archive_manifest, indent=2, sort_keys=True, allow_nan=False) + "\n"
    ).encode("utf-8")
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as output:
        for name, content in entries.items():
            output.writestr(name, content)
    return archive, {
        "passed": run["passed"],
        "dataset": repository_path(csv_path),
        "recipe": run["recipe"],
        "recipe_sha256": run["recipe_sha256"],
        "raw_result": run["output"],
        "raw_result_sha256": run["output_sha256"],
        "execution": run["execution"],
        "fingerprint_execution": run["fingerprint_execution"],
    }


def _prebuilt_project_test(test_name: str) -> dict[str, Any]:
    candidates = sorted(
        (ROOT / "target" / "debug" / "deps").glob("qpls_project-*.exe"),
        key=lambda path: path.stat().st_mtime_ns,
        reverse=True,
    )
    selected: Path | None = None
    for candidate in candidates:
        listed = subprocess.run(
            [str(candidate), "--list"], cwd=ROOT, capture_output=True, text=True, timeout=120
        )
        if listed.returncode == 0 and f"{test_name}: test" in listed.stdout:
            selected = candidate
            break
    if selected is None:
        return {"passed": False, "error": "no prebuilt qpls-project test binary contains the test"}
    completed, execution = run_command([str(selected), test_name, "--exact", "--nocapture"], timeout=900)
    output = completed.stdout + completed.stderr
    relevant_sources = [ROOT / PROJECT_SOURCE, ROOT / ARCHIVE_SOURCE]
    source_freshness = {
        repository_path(path): path.stat().st_mtime_ns <= selected.stat().st_mtime_ns
        for path in relevant_sources
    }
    return {
        "passed": completed.returncode == 0
        and "1 passed" in output
        and "0 failed" in output
        and all(source_freshness.values()),
        "test": test_name,
        "binary": repository_path(selected),
        "binary_sha256": sha256_file(selected),
        "binary_last_write_utc": datetime.fromtimestamp(
            selected.stat().st_mtime, tz=timezone.utc
        ).isoformat().replace("+00:00", "Z"),
        "source_not_newer_than_binary": source_freshness,
        "factory_built_binary": False,
        "execution": execution,
    }


def main() -> int:
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    archive, create_execution = _create_archive()
    entries, archive_manifest, project = _read_archive(archive)
    result = _plsc_result(project)
    identity = {
        "payload_kind": result["payload"]["kind"] == "pls_pm_v1",
        "estimation_method_version": result["payload"]["estimation"]["method_version"]
        == "plsc_v2",
        "nested_method_version": result["payload"]["estimation"]["plsc"]["method_version"]
        == "plsc_v2",
        "provenance_method": result["provenance"]["method"] == "plsc",
        "dataset_fingerprint_bound": result["provenance"]["dataset_fingerprint"]
        == project["datasets"][0]["fingerprint"],
        "archive_checksum_matches": archive_manifest["checksums"][PROJECT_ENTRY]
        == hashlib.sha256(entries[PROJECT_ENTRY]).hexdigest(),
    }
    reopened, reopen_execution = run_command(
        [str(CLI), "inspect", repository_path(archive), "--json"], timeout=600
    )
    summary = json.loads(reopened.stdout) if reopened.returncode == 0 else None
    round_trip = {
        "passed": create_execution["passed"]
        and reopened.returncode == 0
        and isinstance(summary, dict)
        and summary.get("models") == 1
        and summary.get("recipes") == 1
        and summary.get("results") == 1
        and all(identity.values()),
        "identity": identity,
        "archive": repository_path(archive),
        "archive_sha256": sha256_file(archive),
        "create_execution": create_execution,
        "reopen_summary": summary,
        "reopen_execution": reopen_execution,
    }

    mutations: dict[str, tuple[Callable[[dict[str, Any]], None] | None, bool]] = {
        "feature_identity": (_mutate_feature_identity, True),
        "method_version": (_mutate_method_version, True),
        "dataset_fingerprint": (_mutate_dataset_fingerprint, True),
        "checksum": (None, False),
        "malformed_payload": (_mutate_malformed_payload, True),
    }
    mutation_results: dict[str, Any] = {}
    generated_archives = [repository_path(archive)]
    for category, (mutation, update_checksum) in mutations.items():
        destination = WORK_ROOT / f"plsc_v2_tampered_{category}.qpls"
        _write_archive(archive, destination, mutation, update_checksum=update_checksum)
        completed, execution = run_command(
            [str(CLI), "inspect", repository_path(destination), "--json"], timeout=120
        )
        mutation_results[category] = {
            "passed": completed.returncode != 0,
            "archive": repository_path(destination),
            "archive_sha256": sha256_file(destination),
            "execution": execution,
        }
        generated_archives.append(repository_path(destination))

    current_contract = _prebuilt_project_test(
        "tests::plsc_and_wpls_payloads_round_trip_and_reject_contract_tampering"
    )
    legacy = _prebuilt_project_test(
        "tests::legacy_plsc_v1_remains_readable_and_is_marked_noncurrent"
    )
    checksum_test = _prebuilt_project_test(
        "tests::changed_payload_is_rejected_by_its_manifest_checksum"
    )
    mutation_results["legacy_reinterpretation"] = legacy
    categories = {
        "feature_identity",
        "method_version",
        "dataset_fingerprint",
        "checksum",
        "malformed_payload",
        "legacy_reinterpretation",
    }
    checks = {
        "passed": round_trip["passed"]
        and current_contract["passed"]
        and checksum_test["passed"]
        and set(mutation_results) == categories
        and all(row["passed"] for row in mutation_results.values()),
        "round_trip": round_trip,
        "tamper_categories": mutation_results,
        "prebuilt_current_contract_test": current_contract,
        "prebuilt_checksum_test": checksum_test,
        "no_build_performed": True,
    }
    report = write_identity_report(
        "persistence_report",
        passed=checks["passed"],
        checks=checks,
        extras=[
            SOURCE,
            PROJECT_SOURCE,
            ARCHIVE_SOURCE,
            "crates/qpls-cli/src/main.rs",
            "validation/results/plsc_reference.csv",
            create_execution["dataset"],
            create_execution["recipe"],
            create_execution["raw_result"],
            *generated_archives,
        ],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
