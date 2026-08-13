"""Fresh PLS-PM v1 archive round-trip and fail-closed tamper evidence."""

from __future__ import annotations

import hashlib
import json
import zipfile
from pathlib import Path
from typing import Any, Callable

from pls_algorithm_v1_factory_common import (
    CLI,
    REPORT_ROOT,
    ROOT,
    WORK_ROOT,
    repository_path,
    run_command,
    run_pls,
    sha256_file,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/pls_core_persistence_gate.py"
PROJECT_SOURCE = "crates/qpls-project/src/lib.rs"
ARCHIVE_SOURCE = "crates/qpls-project/src/archive_integrity.rs"
MANIFEST_ENTRY = "manifest.json"
PROJECT_ENTRY = "project.json"


def _read_archive(path: Path) -> tuple[dict[str, bytes], dict[str, Any], dict[str, Any]]:
    with zipfile.ZipFile(path) as archive:
        entries = {entry.filename: archive.read(entry) for entry in archive.infolist()}
    manifest = json.loads(entries[MANIFEST_ENTRY].decode("utf-8"))
    project = json.loads(entries[PROJECT_ENTRY].decode("utf-8"))
    return entries, manifest, project


def _write_archive(
    source: Path,
    destination: Path,
    mutation: Callable[[dict[str, Any]], None] | None,
    *,
    update_checksum: bool,
) -> None:
    entries, manifest, project = _read_archive(source)
    if mutation is None:
        entries[PROJECT_ENTRY] = entries[PROJECT_ENTRY] + b"\n"
    else:
        mutation(project)
        entries[PROJECT_ENTRY] = (
            json.dumps(project, indent=2, sort_keys=True, allow_nan=False) + "\n"
        ).encode("utf-8")
    if update_checksum:
        manifest["checksums"][PROJECT_ENTRY] = hashlib.sha256(
            entries[PROJECT_ENTRY]
        ).hexdigest()
        entries[MANIFEST_ENTRY] = (
            json.dumps(manifest, indent=2, sort_keys=True, allow_nan=False) + "\n"
        ).encode("utf-8")
    destination.unlink(missing_ok=True)
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, content in entries.items():
            archive.writestr(name, content)


def _pls_result(project: dict[str, Any]) -> dict[str, Any]:
    matches = [
        result
        for result in project.get("results", [])
        if result.get("provenance", {}).get("method") == "pls_pm"
    ]
    if len(matches) != 1:
        raise ValueError(
            f"factory demo archive must contain exactly one PLS result, found {len(matches)}"
        )
    return matches[0]


def _mutate_feature_identity(project: dict[str, Any]) -> None:
    _pls_result(project)["provenance"]["method"] = "regression"


def _mutate_method_version(project: dict[str, Any]) -> None:
    _pls_result(project)["payload"]["estimation"]["method_version"] = "pls_pm_v999"


def _mutate_dataset_fingerprint(project: dict[str, Any]) -> None:
    _pls_result(project)["provenance"]["dataset_fingerprint"] = "v2:" + "0" * 64


def _mutate_malformed_payload(project: dict[str, Any]) -> None:
    result = _pls_result(project)
    result["payload"]["estimation"]["paths"][0]["coefficient"] = "not-a-number"


def create_factory_archive() -> tuple[Path, dict[str, Any]]:
    """Create a genuine point-estimate-only PLS archive from fresh CLI output.

    The generic ``qpls demo create`` command intentionally includes bootstrap
    and permutation inference, so its current payload is ``pls_pm_v3``. It is
    not valid persistence evidence for this bounded ``pls_pm_v1`` algorithm
    contract. This helper starts with a newly imported project, runs an exact
    zero-resampling recipe, and persists that recipe/model/result together.
    """

    WORK_ROOT.mkdir(parents=True, exist_ok=True)
    archive = WORK_ROOT / "pls_algorithm_v1_factory_demo.qpls"
    archive.unlink(missing_ok=True)
    run = run_pls(
        name="factory_persistence_point_estimate",
        csv_path=ROOT / "validation" / "fixtures" / "simple_reflective.csv",
        constructs=[
            {
                "id": "x",
                "name": "X",
                "short_name": "X",
                "mode": "reflective",
                "indicators": ["x1", "x2"],
            },
            {
                "id": "y",
                "name": "Y",
                "short_name": "Y",
                "mode": "reflective",
                "indicators": ["y1", "y2"],
            },
        ],
        paths=[{"source": "x", "target": "y"}],
        weighting_scheme="path",
        tolerance=1e-10,
        max_iterations=10_000,
    )
    imported = ROOT / run["fingerprint_execution"]["project"]
    entries, archive_manifest, project = _read_archive(imported)
    recipe = strict_load_json(ROOT / run["recipe"])
    result = strict_load_json(ROOT / run["output"])
    if (
        result.get("payload", {}).get("kind") != "pls_pm_v1"
        or result.get("payload", {}).get("estimation", {}).get("method_version")
        != "pls_pm_v1"
    ):
        raise ValueError("fresh persistence fixture did not produce an exact pls_pm_v1 payload")
    project["models"] = [recipe["model"]]
    project["recipes"] = [recipe]
    project["results"] = [result]
    entries[PROJECT_ENTRY] = (
        json.dumps(project, indent=2, sort_keys=True, allow_nan=False) + "\n"
    ).encode("utf-8")
    archive_manifest["checksums"][PROJECT_ENTRY] = hashlib.sha256(
        entries[PROJECT_ENTRY]
    ).hexdigest()
    entries[MANIFEST_ENTRY] = (
        json.dumps(archive_manifest, indent=2, sort_keys=True, allow_nan=False) + "\n"
    ).encode("utf-8")
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


def archive_checksum_probe() -> dict[str, Any]:
    archive, create_execution = create_factory_archive()
    destination = WORK_ROOT / "pls_algorithm_v1_checksum_tampered.qpls"
    _write_archive(archive, destination, None, update_checksum=False)
    completed, execution = run_command(
        [str(CLI), "inspect", repository_path(destination), "--json"], timeout=120
    )
    output = (completed.stdout + completed.stderr).lower()
    return {
        "passed": completed.returncode != 0 and "checksum" in output,
        "archive": repository_path(destination),
        "archive_sha256": sha256_file(destination),
        "diagnostic_mentions_checksum": "checksum" in output,
        "create_execution": create_execution,
        "execution": execution,
    }


def run_persistence_gate() -> dict[str, Any]:
    archive, create_execution = create_factory_archive()
    entries, archive_manifest, project = _read_archive(archive)
    result = _pls_result(project)
    identity = {
        "payload_kind": result.get("payload", {}).get("kind") == "pls_pm_v1",
        "estimation_method_version": result.get("payload", {})
        .get("estimation", {})
        .get("method_version")
        == "pls_pm_v1",
        "provenance_method": result.get("provenance", {}).get("method") == "pls_pm",
        "provenance_method_version": str(
            result.get("provenance", {}).get("method_version", "")
        ).startswith("pls_pm_v1"),
        "dataset_fingerprint_bound": result.get("provenance", {}).get(
            "dataset_fingerprint"
        )
        == project["datasets"][0]["fingerprint"],
        "archive_checksum_matches": archive_manifest["checksums"][PROJECT_ENTRY]
        == hashlib.sha256(entries[PROJECT_ENTRY]).hexdigest(),
    }
    reopened, reopen_execution = run_command(
        [str(CLI), "inspect", repository_path(archive), "--json"],
        timeout=600,
    )
    reopened_summary = json.loads(reopened.stdout) if reopened.returncode == 0 else None
    round_trip = {
        "passed": (
            create_execution["passed"]
            and reopened.returncode == 0
            and isinstance(reopened_summary, dict)
            and reopened_summary.get("models") == 1
            and reopened_summary.get("recipes") == 1
            and reopened_summary.get("results") == 1
            and all(identity.values())
        ),
        "identity": identity,
        "archive": repository_path(archive),
        "archive_sha256": sha256_file(archive),
        "create_execution": create_execution,
        "reopen_summary": reopened_summary,
        "reopen_execution": reopen_execution,
    }

    mutations: dict[
        str, tuple[Callable[[dict[str, Any]], None] | None, bool, str | None]
    ] = {
        "feature_identity": (_mutate_feature_identity, True, None),
        "method_version": (_mutate_method_version, True, None),
        "dataset_fingerprint": (_mutate_dataset_fingerprint, True, None),
        "checksum": (None, False, "checksum"),
        "malformed_payload": (_mutate_malformed_payload, True, None),
    }
    mutation_results: dict[str, Any] = {}
    for category, (mutation, update_checksum, diagnostic) in mutations.items():
        destination = WORK_ROOT / f"pls_algorithm_v1_tampered_{category}.qpls"
        _write_archive(
            archive,
            destination,
            mutation,
            update_checksum=update_checksum,
        )
        completed, execution = run_command(
            [str(CLI), "inspect", repository_path(destination), "--json"], timeout=120
        )
        output = (completed.stdout + completed.stderr).lower()
        mutation_results[category] = {
            "passed": completed.returncode != 0
            and (diagnostic is None or diagnostic in output),
            "archive": repository_path(destination),
            "archive_sha256": sha256_file(destination),
            "execution": execution,
        }

    legacy, legacy_execution = run_command(
        [
            "cargo",
            "test",
            "-p",
            "qpls-project",
            "tests::version_three_pls_payload_migrates_to_the_tagged_contract",
            "--",
        ],
        timeout=900,
    )
    legacy_text = legacy.stdout + legacy.stderr
    mutation_results["legacy_reinterpretation"] = {
        "passed": legacy.returncode == 0
        and "1 passed" in legacy_text
        and "0 failed" in legacy_text,
        "test": "tests::version_three_pls_payload_migrates_to_the_tagged_contract",
        "meaning": (
            "The supported legacy payload migrates under its historical tagged identity; "
            "it is not relabeled as fresh pls_pm_v1 factory evidence."
        ),
        "execution": legacy_execution,
    }
    categories = {
        "feature_identity",
        "method_version",
        "dataset_fingerprint",
        "checksum",
        "malformed_payload",
        "legacy_reinterpretation",
    }
    return {
        "passed": round_trip["passed"]
        and set(mutation_results) == categories
        and all(row["passed"] for row in mutation_results.values()),
        "round_trip": round_trip,
        "tamper_categories": mutation_results,
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
