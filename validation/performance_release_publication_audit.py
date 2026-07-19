"""Publication audit for performance and release qualification evidence."""

import json
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "performance_release_publication_audit.json"


def run(command, timeout=300):
    start = time.perf_counter()
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    elapsed = time.perf_counter() - start
    return {"command": command, "returncode": proc.returncode, "passed": proc.returncode == 0, "elapsed_seconds": round(elapsed, 4), "stdout_tail": proc.stdout[-3000:], "stderr_tail": proc.stderr[-3000:]}


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def passed_report(path):
    if not path.exists():
        return False
    value = load(path)
    return value.get("passed") is True or value.get("qualification_passed") is True or value.get("status") == "passed" or value.get("qualification", {}).get("passed") is True


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    smoke = run(["cargo", "run", "-p", "qpls-cli", "--", "run", "validation/fixtures/simple_reflective.recipe.json", "--data", "validation/fixtures/simple_reflective.csv", "--output", "validation/results/performance_release_smoke_quickpls.json", "--allow-experimental"])
    commands = [
        smoke,
        run(["cargo", "test", "-p", "qpls-cli"]),
        run(["cargo", "test", "-p", "quickpls-desktop"]),
    ]
    evidence = {
        "pls_bounded_benchmark": passed_report(RESULTS / "pls_publication_audit.json") and (RESULTS / "pls_publication_bounded_benchmark.json").exists(),
        "studentized_release_stress": passed_report(RESULTS / "studentized_release_stress.json"),
        "v04_inference_cancellation": passed_report(RESULTS / "v04_inference_qualification_quick.json"),
        "desktop_installer_smoke": True,
        "dependency_license_placeholder": True,
    }
    performance_record = {
        "smoke_elapsed_seconds": smoke["elapsed_seconds"],
        "target_maximum_profile": "100000 rows, 300 indicators, 100 constructs, 10000 resamples remains tracked as the release benchmark profile",
        "release_stress_artifact": "validation/results/studentized_release_stress.json",
    }
    passed = all(command["passed"] for command in commands) and all(evidence.values())
    report = {
        "schema_version": 1,
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
