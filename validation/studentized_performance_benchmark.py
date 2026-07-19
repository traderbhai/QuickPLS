#!/usr/bin/env python3
"""Benchmark bounded studentized-bootstrap plans through the QuickPLS CLI."""

from __future__ import annotations

import argparse
import ctypes
import json
import os
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any


PROCESS_QUERY_INFORMATION = 0x0400
PROCESS_VM_READ = 0x0010


class ProcessMemoryCounters(ctypes.Structure):
    _fields_ = [
        ("cb", ctypes.c_ulong),
        ("PageFaultCount", ctypes.c_ulong),
        ("PeakWorkingSetSize", ctypes.c_size_t),
        ("WorkingSetSize", ctypes.c_size_t),
        ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
        ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
        ("PagefileUsage", ctypes.c_size_t),
        ("PeakPagefileUsage", ctypes.c_size_t),
    ]


@dataclass(frozen=True)
class BenchmarkPlan:
    name: str
    bootstrap_samples: int
    studentized_inner_samples: int
    recipe: str = "validation/fixtures/simple_reflective.recipe.json"
    data: str = "validation/fixtures/simple_reflective.csv"
    fixture_label: str = "compact_reflective"


BOUNDED_PLANS = (
    BenchmarkPlan("minimum_999x99", 999, 99),
    BenchmarkPlan("default_inner_999x199", 999, 199),
    BenchmarkPlan("outer_stress_1999x99", 1999, 99),
    BenchmarkPlan("maximum_inner_999x999", 999, 999),
    BenchmarkPlan(
        "broader_corporate_999x99",
        999,
        99,
        "validation/fixtures/corporate_reputation.recipe.json",
        "validation/fixtures/corporate_reputation.csv",
        "corporate_reputation_4_constructs_9_indicators_3_paths",
    ),
)

RELEASE_STRESS_PLANS = (
    BenchmarkPlan("maximum_outer_inner_1999x999", 1999, 999),
    BenchmarkPlan(
        "broader_corporate_999x199",
        999,
        199,
        "validation/fixtures/corporate_reputation.recipe.json",
        "validation/fixtures/corporate_reputation.csv",
        "corporate_reputation_4_constructs_9_indicators_3_paths",
    ),
)


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def detected_max_workers() -> int:
    return max(1, os.cpu_count() or 1)


def qpls_executable(root: Path) -> Path:
    executable = root / "target" / "debug" / "qpls.exe"
    if executable.exists():
        return executable
    raise FileNotFoundError(
        f"{executable} is missing; build the CLI first with `cargo build -p qpls-cli`"
    )


def peak_working_set_bytes(pid: int) -> int | None:
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    psapi = ctypes.WinDLL("psapi", use_last_error=True)
    handle = kernel32.OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, False, pid)
    if not handle:
        return None
    try:
        counters = ProcessMemoryCounters()
        counters.cb = ctypes.sizeof(ProcessMemoryCounters)
        ok = psapi.GetProcessMemoryInfo(
            handle, ctypes.byref(counters), ctypes.sizeof(ProcessMemoryCounters)
        )
        if not ok:
            return None
        return int(counters.PeakWorkingSetSize)
    finally:
        kernel32.CloseHandle(handle)


def run_plan(root: Path, executable: Path, artifact_dir: Path, plan: BenchmarkPlan) -> dict[str, Any]:
    workers = detected_max_workers()
    output = artifact_dir / f"{plan.name}.json"
    command = [
        str(executable),
        "run",
        plan.recipe,
        "--data",
        plan.data,
        "--output",
        str(output),
        "--allow-experimental",
        "--bootstrap-samples",
        str(plan.bootstrap_samples),
        "--studentized-inner-samples",
        str(plan.studentized_inner_samples),
        "--workers",
        str(workers),
    ]
    started = time.perf_counter()
    process = subprocess.Popen(command, cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    peak_memory = 0
    while process.poll() is None:
        sample = peak_working_set_bytes(process.pid)
        if sample is not None:
            peak_memory = max(peak_memory, sample)
        time.sleep(0.05)
    stdout, stderr = process.communicate()
    elapsed = time.perf_counter() - started
    sample = peak_working_set_bytes(process.pid)
    if sample is not None:
        peak_memory = max(peak_memory, sample)
    if process.returncode != 0:
        raise RuntimeError(
            f"benchmark plan {plan.name} failed\nstdout:\n{stdout}\n\nstderr:\n{stderr}"
        )
    with output.open("r", encoding="utf-8") as handle:
        envelope = json.load(handle)
    payload = envelope.get("payload", {})
    bootstrap = payload.get("bootstrap", {}) if isinstance(payload, dict) else {}
    studentized = bootstrap.get("studentized", {}) if isinstance(bootstrap, dict) else {}
    parameters = studentized.get("parameters") if isinstance(studentized, dict) else []
    unavailable = [
        row
        for row in parameters or []
        if isinstance(row, dict) and row.get("unavailable_reason") is not None
    ]
    requested_inner_fits = plan.bootstrap_samples * plan.studentized_inner_samples
    passed = (
        envelope.get("status") == "completed"
        and bootstrap.get("usable_replicates", 0) >= int(plan.bootstrap_samples * 0.9)
        and studentized.get("inner_replicates") == plan.studentized_inner_samples
        and studentized.get("failure") is None
        and len(parameters or []) > len(unavailable)
    )
    return {
        "name": plan.name,
        "fixture_label": plan.fixture_label,
        "recipe": plan.recipe,
        "data": plan.data,
        "passed": passed,
        "output": str(output),
        "status": envelope.get("status"),
        "bootstrap_samples": plan.bootstrap_samples,
        "studentized_inner_samples": plan.studentized_inner_samples,
        "requested_inner_fits": requested_inner_fits,
        "workers": workers,
        "elapsed_seconds": elapsed,
        "inner_fits_per_second": requested_inner_fits / elapsed if elapsed else None,
        "peak_working_set_bytes": peak_memory or None,
        "result_file_bytes": output.stat().st_size,
        "usable_replicates": bootstrap.get("usable_replicates"),
        "studentized_failure": studentized.get("failure"),
        "studentized_parameter_count": len(parameters or []),
        "studentized_available_parameter_count": len(parameters or []) - len(unavailable),
        "studentized_unavailable_parameter_count": len(unavailable),
    }


def plans_for_profile(profile: str) -> tuple[BenchmarkPlan, ...]:
    if profile == "bounded":
        return BOUNDED_PLANS
    if profile == "release-stress":
        return RELEASE_STRESS_PLANS
    if profile == "all":
        return BOUNDED_PLANS + RELEASE_STRESS_PLANS
    raise ValueError(f"unsupported profile: {profile}")


def build_report(root: Path, output: Path, profile: str) -> dict[str, Any]:
    executable = qpls_executable(root)
    artifact_dir = root / "validation" / "results" / "studentized_performance"
    artifact_dir.mkdir(parents=True, exist_ok=True)
    runs = [run_plan(root, executable, artifact_dir, plan) for plan in plans_for_profile(profile)]
    report = {
        "schema_version": 1,
        "kind": "studentized_performance_benchmark_v1",
        "profile": profile,
        "passed": all(run["passed"] for run in runs),
        "fixture": {
            "primary_recipe": "validation/fixtures/simple_reflective.recipe.json",
            "primary_data": "validation/fixtures/simple_reflective.csv",
            "detected_max_workers": detected_max_workers(),
        },
        "plans": runs,
        "artifact_directory": str(artifact_dir),
        "note": "Nested studentized bootstrap performance evidence. The bounded profile is consumed by qpls qualify v04-inference; the release-stress profile records maximum outer-plus-inner and broader corporate model-shape stress evidence on documented local hardware.",
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2)
        handle.write("\n")
    return report


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        default=repo_root() / "validation" / "results" / "studentized_performance.json",
    )
    parser.add_argument(
        "--profile",
        choices=("bounded", "release-stress", "all"),
        default="bounded",
        help="Benchmark plan group to execute.",
    )
    args = parser.parse_args()
    report = build_report(repo_root(), args.output, args.profile)
    slowest = max(plan["elapsed_seconds"] for plan in report["plans"])
    print(f"wrote {args.output} | passed={report['passed']} | slowest_seconds={slowest:.3f}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
