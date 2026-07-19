"""Publication audit for v0.3 PLS core stable run envelope."""

import json
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "pls_publication_audit.json"
CLI = ROOT / "target" / "debug" / "qpls.exe"


def run(command, timeout=240):
    start = time.perf_counter()
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    return {
        "command": command,
        "returncode": proc.returncode,
        "passed": proc.returncode == 0,
        "elapsed_seconds": round(time.perf_counter() - start, 4),
        "stdout_tail": proc.stdout[-3000:],
        "stderr_tail": proc.stderr[-3000:],
    }


def load(path):
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def status_ok(path):
    value = load(path)
    return value.get("status") == "passed" or value.get("passed") is True


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    build = run(["cargo", "build", "-p", "qpls-cli"])
    references = {
        "csem": {"path": "validation/results/pls_csem_comparison.json", "passed": status_ok("validation/results/pls_csem_comparison.json")},
        "plspm": {"path": "validation/results/pls_plspm_comparison.json", "passed": status_ok("validation/results/pls_plspm_comparison.json")},
        "numpy_pca": {"path": "validation/results/pls_pca_numpy_comparison.json", "passed": status_ok("validation/results/pls_pca_numpy_comparison.json")},
        "published_csem": {"path": "validation/results/pls_csem_threecommonfactors_comparison.json", "passed": status_ok("validation/results/pls_csem_threecommonfactors_comparison.json")},
    }
    export_path = RESULTS / "pls_publication_stable_export.csv"
    export = run([str(CLI), "export", "validation/results/pls_quickpls_path_mode_a.json", "--format", "csv", "--output", str(export_path.relative_to(ROOT))])
    export_text = export_path.read_text(encoding="utf-8") if export_path.exists() else ""
    export_check = {
        "path": str(export_path.relative_to(ROOT)),
        "exists": export_path.exists(),
        "contains_scope_warning": "QuickPLS v1.0.0 supported scope" in export_text,
        "contains_stable_scope_note": "v0.3 validated estimator only" in export_text,
        "contains_assessment_rows": "assessment" in export_text.lower(),
    }
    benchmark_output = RESULTS / "pls_publication_bounded_benchmark.json"
    benchmark_start = time.perf_counter()
    quick = run([str(CLI), "run", "validation/fixtures/simple_reflective.recipe.json", "--data", "validation/fixtures/simple_reflective.csv", "--output", "validation/results/pls_publication_benchmark_quickpls.json", "--allow-experimental"], timeout=120)
    benchmark = {
        "profile": "bounded_smoke",
        "elapsed_seconds": round(time.perf_counter() - benchmark_start, 4),
        "command_passed": quick["passed"],
        "future_maximum_profile": "100000 rows, 300 indicators, 100 constructs, 10000 resamples remains in the release performance blocker",
    }
    benchmark_output.write_text(json.dumps(benchmark, indent=2), encoding="utf-8")
    known_differences = ROOT / "docs" / "KNOWN_DIFFERENCES.md"
    passed = (
        build["passed"]
        and all(item["passed"] for item in references.values())
        and export["passed"]
        and export_check["exists"]
        and export_check["contains_stable_scope_note"]
        and quick["passed"]
        and known_differences.exists()
    )
    report = {
        "schema_version": 1,
        "target": "v0.3 PLS publication audit",
        "passed": passed,
        "references": references,
        "stable_export": export_check,
        "benchmark": benchmark,
        "known_differences": {"path": "docs/KNOWN_DIFFERENCES.md", "present": known_differences.exists()},
        "commands": [build, export, quick],
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
