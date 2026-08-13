"""Fresh bootstrap-v4 archive round-trip and fail-closed tamper evidence."""

from __future__ import annotations

import hashlib
import json
import zipfile
from pathlib import Path
from typing import Any, Callable

from pls_bootstrap_v4_factory_common import (
    CLI,
    REPORT_ROOT,
    ROOT,
    WORK_ROOT,
    construct,
    repository_path,
    run_bootstrap,
    run_command,
    sha256_file,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/pls_bootstrap_release_persistence_gate.py"
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


def _bootstrap_result(project: dict[str, Any]) -> dict[str, Any]:
    matches = [
        result
        for result in project.get("results", [])
        if isinstance(result.get("payload", {}).get("bootstrap"), dict)
    ]
    if len(matches) != 1:
        raise ValueError(f"factory archive must contain one bootstrap result, found {len(matches)}")
    return matches[0]


def _mutate_feature_identity(project: dict[str, Any]) -> None:
    _bootstrap_result(project)["provenance"]["method"] = "regression"


def _mutate_method_version(project: dict[str, Any]) -> None:
    _bootstrap_result(project)["payload"]["bootstrap"]["method_version"] = (
        "indexed_resampling_v999"
    )


def _mutate_dataset_fingerprint(project: dict[str, Any]) -> None:
    _bootstrap_result(project)["provenance"]["dataset_fingerprint"] = "v2:" + "0" * 64


def _mutate_malformed_payload(project: dict[str, Any]) -> None:
    _bootstrap_result(project)["payload"]["bootstrap"]["percentile"]["parameters"][0][
        "lower"
    ] = "not-a-number"


def create_factory_archive() -> tuple[Path, dict[str, Any]]:
    run = run_bootstrap(
        name="factory_persistence_bootstrap_v4",
        csv_path=ROOT / "validation" / "fixtures" / "simple_reflective.csv",
        constructs=[construct("x", ["x1", "x2"]), construct("y", ["y1", "y2"])],
        paths=[{"source": "x", "target": "y"}],
        bootstrap_samples=999,
        seed=20_260_831,
        workers=4,
    )
    imported = ROOT / run["fingerprint_execution"]["project"]
    entries, archive_manifest, project = _read_archive(imported)
    recipe = strict_load_json(ROOT / run["recipe"])
    result = strict_load_json(ROOT / run["output"])
    bootstrap = result.get("payload", {}).get("bootstrap", {})
    if (
        result.get("payload", {}).get("kind") != "pls_pm_v2"
        or bootstrap.get("method_version") != "indexed_resampling_v4"
        or bootstrap.get("plan", {}).get("replicates") != 999
    ):
        raise ValueError("fresh persistence fixture did not produce exact bootstrap v4 identity")
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
    archive = WORK_ROOT / "pls_bootstrap_v4_factory_round_trip.qpls"
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


def _legacy_contract_test() -> dict[str, Any]:
    test = "tests::bootstrap_pls_payload_round_trips_with_recipe_provenance"
    completed, execution = run_command(
        ["cargo", "test", "-p", "qpls-project", test, "--", "--exact"], timeout=1800
    )
    output = completed.stdout + completed.stderr
    return {
        "passed": completed.returncode == 0 and "1 passed" in output and "0 failed" in output,
        "test": test,
        "meaning": (
            "The current contract accepts historical v1-v3 shapes under their own identities "
            "and rejects a v4 studentized artifact relabeled as v3."
        ),
        "execution": execution,
    }


def run_persistence_gate() -> tuple[dict[str, Any], list[str]]:
    archive, creation = create_factory_archive()
    entries, archive_manifest, project = _read_archive(archive)
    result = _bootstrap_result(project)
    bootstrap = result["payload"]["bootstrap"]
    identity = {
        "payload_kind": result["payload"]["kind"] == "pls_pm_v2",
        "bootstrap_method_version": bootstrap["method_version"] == "indexed_resampling_v4",
        "provenance_method": result["provenance"]["method"] == "pls_pm",
        "provenance_contains_method_version": "indexed_resampling_v4"
        in result["provenance"]["method_version"],
        "dataset_fingerprint_bound": result["provenance"]["dataset_fingerprint"]
        == project["datasets"][0]["fingerprint"],
        "recipe_fingerprint_bound": project["recipes"][0]["dataset_fingerprint"]
        == project["datasets"][0]["fingerprint"],
        "requested_count_bound": bootstrap["plan"]["replicates"]
        == result["provenance"]["settings"]["bootstrap_samples"],
        "failure_denominator_exact": bootstrap["usable_replicates"]
        + len(bootstrap["failed_replicates"])
        == bootstrap["plan"]["replicates"],
        "archive_checksum_matches": archive_manifest["checksums"][PROJECT_ENTRY]
        == hashlib.sha256(entries[PROJECT_ENTRY]).hexdigest(),
    }
    reopened, reopen_execution = run_command(
        [str(CLI), "inspect", repository_path(archive), "--json"], timeout=600
    )
    summary = json.loads(reopened.stdout) if reopened.returncode == 0 else None
    round_trip = {
        "passed": creation["passed"]
        and reopened.returncode == 0
        and isinstance(summary, dict)
        and summary.get("models") == 1
        and summary.get("recipes") == 1
        and summary.get("results") == 1
        and all(identity.values()),
        "identity": identity,
        "archive": repository_path(archive),
        "archive_sha256": sha256_file(archive),
        "creation": creation,
        "reopen_summary": summary,
        "reopen_execution": reopen_execution,
    }
    mutations: dict[str, tuple[Callable[[dict[str, Any]], None] | None, bool, str | None]] = {
        "feature_identity": (_mutate_feature_identity, True, None),
        "method_version": (_mutate_method_version, True, None),
        "dataset_fingerprint": (_mutate_dataset_fingerprint, True, None),
        "checksum": (None, False, "checksum"),
        "malformed_payload": (_mutate_malformed_payload, True, None),
    }
    mutation_results: dict[str, Any] = {}
    generated = [repository_path(archive), creation["recipe"], creation["raw_result"]]
    for category, (mutation, update_checksum, diagnostic) in mutations.items():
        destination = WORK_ROOT / f"pls_bootstrap_v4_tampered_{category}.qpls"
        _write_archive(archive, destination, mutation, update_checksum=update_checksum)
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
        generated.append(repository_path(destination))
    mutation_results["legacy_reinterpretation"] = _legacy_contract_test()
    required = {
        "feature_identity",
        "method_version",
        "dataset_fingerprint",
        "checksum",
        "malformed_payload",
        "legacy_reinterpretation",
    }
    checks = {
        "passed": round_trip["passed"]
        and set(mutation_results) == required
        and all(row["passed"] for row in mutation_results.values()),
        "round_trip": round_trip,
        "tamper_categories": mutation_results,
        "legacy_policy": (
            "v1-v3 remain readable only under their historical fields and identities; "
            "v4-only studentized data cannot be inherited by relabeling."
        ),
    }
    return checks, generated


def main() -> int:
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    checks, generated = run_persistence_gate()
    report = write_identity_report(
        "persistence_report",
        passed=checks["passed"],
        checks=checks,
        extras=[
            SOURCE,
            PROJECT_SOURCE,
            ARCHIVE_SOURCE,
            "crates/qpls-resampling/src/lib.rs",
            "crates/qpls-cli/src/main.rs",
            *generated,
        ],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
