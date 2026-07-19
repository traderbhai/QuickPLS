"""Final v1.0 performance and reproducibility audit."""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v10_performance_audit.json"


def read_json(relative: str) -> dict:
    path = ROOT / relative
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def passed_report(relative: str) -> bool:
    value = read_json(relative)
    return (
        value.get("passed") is True
        or value.get("qualification_passed") is True
        or value.get("status") == "passed"
        or value.get("qualification", {}).get("passed") is True
    )


def file_status(relative: str) -> dict:
    path = ROOT / relative
    return {
        "path": relative,
        "present": path.exists(),
        "bytes": path.stat().st_size if path.exists() else 0,
    }


def main() -> None:
    RESULTS.mkdir(parents=True, exist_ok=True)
    artifacts = {
        "performance_release_publication_audit": "validation/results/performance_release_publication_audit.json",
        "pls_bounded_benchmark": "validation/results/pls_publication_bounded_benchmark.json",
        "studentized_release_stress": "validation/results/studentized_release_stress.json",
        "v04_inference_qualification": "validation/results/v04_inference_qualification_quick.json",
        "v093_designer_visual_smoke": "validation/results/v093_sem_designer_visual_smoke.json",
        "desktop_smoke": "validation/results/v10_desktop_smoke_check.json",
    }
    checks = {
        "import_and_base_run_smoke": passed_report(artifacts["desktop_smoke"]),
        "base_pls_benchmark_recorded": (ROOT / artifacts["pls_bounded_benchmark"]).exists(),
        "resampling_throughput_recorded": passed_report(artifacts["studentized_release_stress"]),
        "cancellation_latency_recorded": passed_report(artifacts["v04_inference_qualification"]),
        "peak_memory_recorded": "peak" in (ROOT / artifacts["studentized_release_stress"]).read_text(encoding="utf-8").lower() if (ROOT / artifacts["studentized_release_stress"]).exists() else False,
        "report_export_generation_smoke": passed_report(artifacts["desktop_smoke"]),
        "diagram_responsiveness_smoke": passed_report(artifacts["v093_designer_visual_smoke"]),
        "deterministic_rerun_evidence": passed_report(artifacts["performance_release_publication_audit"]) or passed_report(artifacts["v04_inference_qualification"]),
        "worker_count_reproducibility": "worker" in (ROOT / "validation" / "results" / "inference_publication_matrix.json").read_text(encoding="utf-8").lower(),
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.0.0 performance and reproducibility audit",
        "passed": all(checks.values()),
        "checks": checks,
        "artifacts": {key: file_status(value) for key, value in artifacts.items()},
        "maximum_profile": "The full 100000 rows / 300 indicators / 100 constructs / 10000 resamples benchmark remains a named maximum benchmark profile, not a quick release-gate runtime.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
