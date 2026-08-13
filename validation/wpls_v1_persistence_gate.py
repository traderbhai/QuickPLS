"""Fresh WPLS v1 archive round-trip and fail-closed tamper evidence."""

from __future__ import annotations

import hashlib
import json
import zipfile
from pathlib import Path
from typing import Any, Callable

from wpls_v1_factory_common import (
    CLI,
    REPORT_ROOT,
    ROOT,
    WORK_ROOT,
    construct,
    repository_path,
    run_command,
    run_model,
    sha256_file,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/wpls_v1_persistence_gate.py"
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


def _wpls_result(project: dict[str, Any]) -> dict[str, Any]:
    matches = [
        result
        for result in project.get("results", [])
        if result.get("provenance", {}).get("method") == "wpls"
    ]
    if len(matches) != 1:
        raise ValueError(f"factory archive must contain exactly one WPLS result, found {len(matches)}")
    return matches[0]


def _mutate_feature_identity(project: dict[str, Any]) -> None:
    _wpls_result(project)["provenance"]["method"] = "pls_pm"


def _mutate_method_version(project: dict[str, Any]) -> None:
    estimation = _wpls_result(project)["payload"]["estimation"]
    estimation["method_version"] = "wpls_case_weighted_v999"
    estimation["wpls"]["method_version"] = "wpls_case_weighted_v999"


def _mutate_dataset_fingerprint(project: dict[str, Any]) -> None:
    _wpls_result(project)["provenance"]["dataset_fingerprint"] = "v2:" + "0" * 64


def _mutate_malformed_payload(project: dict[str, Any]) -> None:
    _wpls_result(project)["payload"]["estimation"]["wpls"]["weight_sum"] = "not-a-number"


def _mutate_legacy_identity(project: dict[str, Any]) -> None:
    estimation = _wpls_result(project)["payload"]["estimation"]
    estimation["method_version"] = "wpls_case_weighted_preview_v0"
    estimation["wpls"]["method_version"] = "wpls_case_weighted_preview_v0"


def _mutate_weight_metadata(project: dict[str, Any]) -> None:
    _wpls_result(project)["payload"]["estimation"]["wpls"]["case_weight_column"] = "other_weight"


def create_factory_archive() -> tuple[Path, dict[str, Any]]:
    WORK_ROOT.mkdir(parents=True, exist_ok=True)
    data = WORK_ROOT / "persistence_wpls.csv"
    rows = [
        [1.0, 1.2, 2.0, 2.1, 0.5],
        [2.0, 1.8, 3.0, 2.8, 1.0],
        [3.0, 3.2, 4.1, 4.0, 1.5],
        [4.0, 3.9, 5.0, 5.2, 2.0],
        [5.0, 5.1, 6.2, 6.0, 2.5],
        [6.0, 5.8, 6.9, 7.1, 3.0],
        [7.0, 7.2, 8.0, 8.2, 3.5],
        [8.0, 7.9, 9.1, 8.9, 4.0],
    ]
    from wpls_v1_factory_common import write_csv

    write_csv(data, ["x1", "x2", "y1", "y2", "case_wt"], rows)
    run = run_model(
        name="factory_persistence_wpls",
        csv_path=data,
        constructs=[construct("x", ["x1", "x2"]), construct("y", ["y1", "y2"])],
        paths=[{"source": "x", "target": "y"}],
    )
    imported = ROOT / run["fingerprint_execution"]["project"]
    entries, archive_manifest, project = _read_archive(imported)
    recipe = strict_load_json(ROOT / run["recipe"])
    result = strict_load_json(ROOT / run["output"])
    estimation = result.get("payload", {}).get("estimation", {})
    if (
        result.get("payload", {}).get("kind") != "pls_pm_v1"
        or estimation.get("method_version") != "wpls_case_weighted_v1"
        or estimation.get("wpls", {}).get("method_version") != "wpls_case_weighted_v1"
    ):
        raise ValueError("fresh persistence fixture did not produce exact WPLS v1 identity")
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
    archive = WORK_ROOT / "wpls_v1_factory_archive.qpls"
    archive.unlink(missing_ok=True)
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as output:
        for name, content in entries.items():
            output.writestr(name, content)
    return archive, {
        "passed": run["passed"],
        "recipe": run["recipe"],
        "recipe_sha256": run["recipe_sha256"],
        "raw_result": run["output"],
        "raw_result_sha256": run["output_sha256"],
        "execution": run["execution"],
        "fingerprint_execution": run["fingerprint_execution"],
    }


def _inspect_rejection(path: Path, diagnostic: str | None = None) -> dict[str, Any]:
    completed, execution = run_command([str(CLI), "inspect", repository_path(path), "--json"])
    output = (completed.stdout + completed.stderr).lower()
    return {
        "passed": completed.returncode != 0 and (diagnostic is None or diagnostic in output),
        "archive": repository_path(path),
        "archive_sha256": sha256_file(path),
        "diagnostic": output[-1600:],
        "execution": execution,
    }


def archive_checksum_probe() -> dict[str, Any]:
    archive, creation = create_factory_archive()
    destination = WORK_ROOT / "wpls_v1_tampered_checksum.qpls"
    _write_archive(archive, destination, None, update_checksum=False)
    return {**_inspect_rejection(destination, "checksum"), "creation": creation}


def archive_weight_metadata_probe() -> dict[str, Any]:
    archive, creation = create_factory_archive()
    destination = WORK_ROOT / "wpls_v1_tampered_weight_metadata.qpls"
    _write_archive(archive, destination, _mutate_weight_metadata, update_checksum=True)
    return {**_inspect_rejection(destination), "creation": creation}


def run_persistence_gate() -> dict[str, Any]:
    archive, creation = create_factory_archive()
    entries, archive_manifest, project = _read_archive(archive)
    result = _wpls_result(project)
    estimation = result["payload"]["estimation"]
    identity = {
        "payload_kind": result["payload"]["kind"] == "pls_pm_v1",
        "estimation_method_version": estimation["method_version"] == "wpls_case_weighted_v1",
        "wpls_method_version": estimation["wpls"]["method_version"] == "wpls_case_weighted_v1",
        "provenance_method": result["provenance"]["method"] == "wpls",
        "case_weight_column_bound": (
            estimation["wpls"]["case_weight_column"]
            == result["provenance"]["settings"]["case_weight_column"]
            == project["recipes"][0]["settings"]["case_weight_column"]
        ),
        "dataset_fingerprint_bound": (
            result["provenance"]["dataset_fingerprint"] == project["datasets"][0]["fingerprint"]
        ),
        "archive_checksum_matches": (
            archive_manifest["checksums"][PROJECT_ENTRY]
            == hashlib.sha256(entries[PROJECT_ENTRY]).hexdigest()
        ),
    }
    reopened, reopen_execution = run_command(
        [str(CLI), "inspect", repository_path(archive), "--json"], timeout=120
    )
    summary = json.loads(reopened.stdout) if reopened.returncode == 0 else None
    round_trip = {
        "passed": (
            creation["passed"]
            and reopened.returncode == 0
            and isinstance(summary, dict)
            and summary.get("models") == 1
            and summary.get("recipes") == 1
            and summary.get("results") == 1
            and all(identity.values())
        ),
        "identity": identity,
        "archive": repository_path(archive),
        "archive_sha256": sha256_file(archive),
        "reopen_summary": summary,
        "reopen_execution": reopen_execution,
        "creation": creation,
    }
    mutations: dict[str, tuple[Callable[[dict[str, Any]], None] | None, bool, str | None]] = {
        "feature_identity": (_mutate_feature_identity, True, None),
        "method_version": (_mutate_method_version, True, None),
        "dataset_fingerprint": (_mutate_dataset_fingerprint, True, None),
        "checksum": (None, False, "checksum"),
        "malformed_payload": (_mutate_malformed_payload, True, None),
        "legacy_reinterpretation": (_mutate_legacy_identity, True, None),
    }
    tamper: dict[str, Any] = {}
    for category, (mutation, update_checksum, diagnostic) in mutations.items():
        destination = WORK_ROOT / f"wpls_v1_tampered_{category}.qpls"
        _write_archive(archive, destination, mutation, update_checksum=update_checksum)
        tamper[category] = _inspect_rejection(destination, diagnostic)
    weight_metadata = archive_weight_metadata_probe()
    expected = set(mutations)
    return {
        "passed": (
            round_trip["passed"]
            and set(tamper) == expected
            and all(row["passed"] for row in tamper.values())
            and weight_metadata["passed"]
        ),
        "round_trip": round_trip,
        "tamper_categories": tamper,
        "weight_metadata_tamper": weight_metadata,
    }


def main() -> int:
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    detail = run_persistence_gate()
    path = write_identity_report(
        "persistence_report",
        passed=detail["passed"],
        checks=detail,
        extras=[SOURCE, PROJECT_SOURCE, ARCHIVE_SOURCE, "crates/qpls-cli/src/main.rs"],
    )
    print(f"wrote {path} | passed={detail['passed']}")
    return 0 if detail["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
