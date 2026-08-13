"""Publication audit for performance and release qualification evidence."""

import json
import subprocess
import time
from pathlib import Path

from promotion_audit_integrity import evaluate_document, evaluate_report, sha256_file


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "performance_release_publication_audit.json"


def run(command, timeout=300):
    start = time.perf_counter()
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    elapsed = time.perf_counter() - start
    return {"command": command, "returncode": proc.returncode, "passed": proc.returncode == 0, "elapsed_seconds": round(elapsed, 4), "stdout_tail": proc.stdout[-3000:], "stderr_tail": proc.stderr[-3000:]}


def load(path):
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"expected a JSON object in {path}")
    return value


def verify_release_artifacts(manifest_path, release_directory, expected_version, *, root=ROOT, results=RESULTS):
    manifest_evidence = evaluate_report(
        root,
        results,
        {
            "name": manifest_path.name,
            "required_values": {"version": expected_version},
            "required_nonempty": ["timestamp_utc", "artifacts"],
            "max_age_days": 30,
        },
    )
    if not manifest_path.is_file():
        return {"passed": False, "manifest": manifest_evidence, "artifacts": [], "roles": {}}
    try:
        manifest = load(manifest_path)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        return {
            "passed": False,
            "manifest": manifest_evidence,
            "artifacts": [],
            "roles": {},
            "error": f"{type(error).__name__}: {error}",
        }

    artifacts = []
    for entry in manifest.get("artifacts", []):
        declared = Path(entry.get("path", ""))
        candidates = [root / declared, release_directory / declared.name]
        actual = next((candidate for candidate in candidates if candidate.is_file()), None)
        expected_sha256 = str(entry.get("sha256", "")).lower()
        expected_bytes = entry.get("bytes")
        actual_sha256 = sha256_file(actual) if actual else None
        actual_bytes = actual.stat().st_size if actual else None
        artifacts.append({
            "declared_path": str(declared),
            "resolved_path": str(actual.relative_to(root)) if actual else None,
            "present": actual is not None,
            "expected_bytes": expected_bytes,
            "actual_bytes": actual_bytes,
            "expected_sha256": expected_sha256 or None,
            "actual_sha256": actual_sha256,
            "passed": (
                actual is not None
                and isinstance(expected_bytes, int)
                and expected_bytes > 0
                and actual_bytes == expected_bytes
                and len(expected_sha256) == 64
                and actual_sha256 == expected_sha256
            ),
        })

    by_suffix = {
        suffix: next((item for item in artifacts if item["declared_path"].lower().endswith(suffix)), None)
        for suffix in ("_portable.exe", "_setup.exe", "_checksums.txt")
    }
    checksum_record = by_suffix["_checksums.txt"]
    checksum_contents_passed = False
    if checksum_record and checksum_record["resolved_path"]:
        checksum_path = root / checksum_record["resolved_path"]
        contents = checksum_path.read_text(encoding="utf-8").casefold()
        binary_records = [by_suffix["_portable.exe"], by_suffix["_setup.exe"]]
        checksum_contents_passed = all(
            item is not None
            and item["actual_sha256"] in contents
            and Path(item["declared_path"]).name.casefold() in contents
            for item in binary_records
        )
    roles = {
        "portable": by_suffix["_portable.exe"] is not None and by_suffix["_portable.exe"]["passed"],
        "setup": by_suffix["_setup.exe"] is not None and by_suffix["_setup.exe"]["passed"],
        "checksums": checksum_record is not None and checksum_record["passed"] and checksum_contents_passed,
    }
    return {
        "passed": manifest_evidence["passed"] and bool(artifacts) and all(item["passed"] for item in artifacts) and all(roles.values()),
        "manifest": manifest_evidence,
        "artifacts": artifacts,
        "roles": roles,
        "checksum_contents_passed": checksum_contents_passed,
    }


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    package = load(ROOT / "package.json")
    expected_version = package.get("version")
    smoke = run(["cargo", "run", "-p", "qpls-cli", "--", "run", "validation/fixtures/simple_reflective.recipe.json", "--data", "validation/fixtures/simple_reflective.csv", "--output", "validation/results/performance_release_smoke_quickpls.json", "--allow-experimental"])
    commands = [
        smoke,
        run(["cargo", "test", "-p", "qpls-cli"]),
        run(["cargo", "test", "-p", "quickpls-desktop"]),
    ]
    evidence = {
        "pls_bounded_benchmark": evaluate_report(
            ROOT,
            RESULTS,
            {
                "name": "pls_publication_audit.json",
                "source_paths": ["validation/pls_publication_audit.py"],
                "companions": [{
                    "path": "validation/results/pls_publication_bounded_benchmark.json",
                    "required_true": ["command_passed"],
                    "required_nonempty": ["profile", "future_maximum_profile"],
                }],
            },
        ),
        "studentized_release_stress": evaluate_report(
            ROOT,
            RESULTS,
            {
                "name": "studentized_release_stress.json",
                "required_values": {"kind": "studentized_performance_benchmark_v1"},
                "required_nonempty": ["profile", "plans"],
            },
        ),
        "v04_inference_cancellation": evaluate_report(
            ROOT,
            RESULTS,
            {
                "name": "v04_inference_qualification_quick.json",
                "pass_paths": ["qualification_passed"],
                "required_list_items": [
                    {
                        "path": "checks",
                        "where": {"id": "bootstrap_cancellation_latency"},
                        "required_values": {"status": "passed"},
                        "required_true": ["evidence.passed", "evidence.cancelled_result"],
                    },
                    {
                        "path": "checks",
                        "where": {"id": "studentized_cancellation_latency_999x99"},
                        "required_values": {"status": "passed"},
                        "required_true": ["evidence.passed", "evidence.cancelled_result"],
                    },
                ],
            },
        ),
        "desktop_installer_artifacts": verify_release_artifacts(
            RESULTS / "release_artifacts.json",
            ROOT / "release",
            expected_version,
        ),
        "desktop_packaged_smoke": evaluate_report(
            ROOT,
            RESULTS,
            {
                "name": "v247_tauri_native_acceptance.json",
                "required_values": {
                    "runtime": "tauri-webview2-cdp",
                    "checks.runtime.tauriRuntime": True,
                    "failures": [],
                    "consoleErrors": [],
                },
                "required_nonempty": ["generatedAt", "focusedRun.completedAt"],
                "max_age_days": 30,
            },
        ),
        "dependency_notices": {
            "documents": [
                evaluate_document(ROOT, {
                    "path": "docs/DEPENDENCY_NOTICES.md",
                    "required_phrases": ["Validation-Only Tools", "GPL code must not be linked"],
                }),
                evaluate_document(ROOT, {
                    "path": "THIRD_PARTY_NOTICES.md",
                    "required_phrases": ["Third-party dependencies retain their original licenses", "MIT License"],
                }),
                evaluate_document(ROOT, {
                    "path": "NOTICE.md",
                    "required_phrases": ["independent proprietary research application", "does not reverse-engineer"],
                }),
            ],
        },
    }
    evidence["dependency_notices"]["passed"] = all(
        item["passed"] for item in evidence["dependency_notices"]["documents"]
    )
    performance_record = {
        "smoke_elapsed_seconds": smoke["elapsed_seconds"],
        "target_maximum_profile": "100000 rows, 300 indicators, 100 constructs, 10000 resamples remains tracked as the release benchmark profile",
        "release_stress_artifact": "validation/results/studentized_release_stress.json",
    }
    passed = all(command["passed"] for command in commands) and all(item["passed"] for item in evidence.values())
    report = {
        "schema_version": 2,
        "integrity_contract": "explicit_pass_state_and_bound_evidence_v1",
        "target": "performance and release publication audit",
        "passed": passed,
        "evidence": evidence,
        "performance_record": performance_record,
        "commands": commands,
        "note": "This audit records the current release qualification evidence and keeps the maximum benchmark profile named explicitly.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
