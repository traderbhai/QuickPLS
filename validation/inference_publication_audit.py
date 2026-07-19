"""Publication audit for v0.4 inference and resampling procedures."""

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "inference_publication_audit.json"
MATRIX = RESULTS / "inference_publication_matrix.json"


def run(command, timeout=300):
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    return {
        "command": command,
        "returncode": proc.returncode,
        "passed": proc.returncode == 0,
        "stdout_tail": proc.stdout[-3000:],
        "stderr_tail": proc.stderr[-3000:],
    }


def load(path):
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def passed(path):
    value = load(path)
    return (
        value.get("passed") is True
        or value.get("qualification_passed") is True
        or value.get("qualification", {}).get("passed") is True
        or value.get("status") == "passed"
        or value.get("usable") is True
    )


PROCEDURES = [
    ("bootstrap_percentile", "indexed bootstrap percentile intervals", "validation/results/pls_bootstrap_external_reference.json", "validation/results/studentized_worker_matrix.json", "validation/results/monte_carlo_qualification.json", "validation/results/studentized_release_stress.json", "fixed complete-case sample; unsupported failed fits are reported"),
    ("bca", "BCa bootstrap intervals", "validation/results/studentized_supplied_reference.json", "validation/results/studentized_worker_matrix.json", "validation/results/monte_carlo_qualification.json", "validation/results/studentized_release_stress.json", "requires jackknife availability; degenerate acceleration is explicit"),
    ("studentized_bootstrap_t", "nested studentized/bootstrap-t intervals", "validation/results/studentized_supplied_reference.json", "validation/results/studentized_worker_matrix.json", "validation/results/monte_carlo_studentized_qualification.json", "validation/results/studentized_release_stress.json", "nested failure is explicit without corrupting percentile results"),
    ("jackknife", "jackknife support for BCa and diagnostics", "validation/results/studentized_supplied_reference.json", "validation/results/studentized_worker_matrix.json", "validation/results/monte_carlo_studentized_qualification.json", "validation/results/v04_inference_qualification_quick.json", "invalid plans and cancellation are explicit"),
    ("freedman_lane_permutation", "Freedman-Lane path permutation", "validation/results/v04_inference_qualification_quick.json", "validation/results/studentized_worker_matrix.json", "validation/results/monte_carlo_qualification.json", "validation/results/v04_inference_qualification_quick.json", "linear nuisance model assumptions documented"),
]


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    required = [
        "validation/results/monte_carlo_qualification.json",
        "validation/results/monte_carlo_studentized_qualification.json",
        "validation/results/studentized_worker_matrix.json",
        "validation/results/studentized_release_stress.json",
        "validation/results/studentized_supplied_reference.json",
        "validation/results/pls_bootstrap_external_reference.json",
        "validation/results/pls_bootstrap_corporate_csem_reference.json",
        "validation/results/pls_bootstrap_plspm_external_reference.json",
        "validation/results/v04_inference_qualification_quick.json",
    ]
    evidence = [{"path": path, "present": (ROOT / path).exists(), "passed": (ROOT / path).exists() and passed(path)} for path in required]
    matrix_rows = [
        {
            "procedure": procedure,
            "contract": contract,
            "reference_evidence": reference,
            "worker_invariance_evidence": worker,
            "coverage_type_i_evidence": coverage,
            "performance_evidence": performance,
            "unsupported_cases": unsupported,
            "complete": all((ROOT / path).exists() for path in [reference, worker, coverage, performance]) and bool(unsupported),
        }
        for procedure, contract, reference, worker, coverage, performance, unsupported in PROCEDURES
    ]
    MATRIX.write_text(json.dumps({"schema_version": 1, "rows": matrix_rows, "passed": all(row["complete"] for row in matrix_rows)}, indent=2), encoding="utf-8")
    cargo = run(["cargo", "test", "-p", "qpls-resampling"])
    qualify = run(["cargo", "run", "-p", "qpls-cli", "--", "qualify", "v04-inference"], timeout=360)
    quick = load("validation/results/v04_inference_qualification_quick.json")
    cancellation_checks = {
        "bootstrap_cancellation_latency": any(check.get("id") == "bootstrap_cancellation_latency" and check.get("status") == "passed" for check in quick.get("checks", [])),
        "studentized_cancellation_latency_999x99": any(check.get("id") == "studentized_cancellation_latency_999x99" and check.get("status") == "passed" for check in quick.get("checks", [])),
    }
    passed_all = all(item["passed"] for item in evidence) and all(row["complete"] for row in matrix_rows) and cargo["passed"] and qualify["passed"] and all(cancellation_checks.values())
    report = {
        "schema_version": 1,
        "target": "v0.4 inference publication audit",
        "passed": passed_all,
        "evidence": evidence,
        "inference_matrix": {"path": str(MATRIX.relative_to(ROOT)), "rows": len(matrix_rows), "passed": all(row["complete"] for row in matrix_rows)},
        "cancellation_checks": cancellation_checks,
        "commands": [cargo, qualify],
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed_all:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
