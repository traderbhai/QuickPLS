#!/usr/bin/env python3
"""Lightweight one-warm-up/five-run measurement harness for contract V2.

The caller selects one capability, hardware profile, complexity case, and an
explicit command. The harness measures the complete process tree and writes a
new receipt without overwriting an existing file. It does not select or launch
weekly maximum workloads on its own. Missing progress, cancellation,
repeated-run, baseline, or result observations leave the receipt incomplete;
the fail-closed validator will not accept it.
"""

from __future__ import annotations

import argparse
import ctypes
import json
import os
import platform
import subprocess
import tempfile
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Mapping, Sequence

try:
    from validation.capability_registry_v2 import canonical_sha256, load_json
    from validation.complexity_performance_v2 import (
        CONTRACT_ID,
        DEFAULT_MANIFEST_PATH,
        DEFAULT_MEASUREMENT_SCHEMA_PATH,
        DEFAULT_REGISTRY_PATH,
        DEFAULT_SCHEMA_PATH,
        aggregate_runs,
        load_contract,
        qualification_link_identity,
        validate_contract_documents,
    )
except ModuleNotFoundError:  # Direct `python validation/...py` execution.
    from capability_registry_v2 import canonical_sha256, load_json
    from complexity_performance_v2 import (
        CONTRACT_ID,
        DEFAULT_MANIFEST_PATH,
        DEFAULT_MEASUREMENT_SCHEMA_PATH,
        DEFAULT_REGISTRY_PATH,
        DEFAULT_SCHEMA_PATH,
        aggregate_runs,
        load_contract,
        qualification_link_identity,
        validate_contract_documents,
    )


POLL_SECONDS = 0.02


def _windows_process_snapshot() -> dict[int, int]:
    from ctypes import wintypes

    TH32CS_SNAPPROCESS = 0x00000002
    INVALID_HANDLE_VALUE = ctypes.c_void_p(-1).value

    class PROCESSENTRY32W(ctypes.Structure):
        _fields_ = [
            ("dwSize", wintypes.DWORD),
            ("cntUsage", wintypes.DWORD),
            ("th32ProcessID", wintypes.DWORD),
            ("th32DefaultHeapID", ctypes.c_size_t),
            ("th32ModuleID", wintypes.DWORD),
            ("cntThreads", wintypes.DWORD),
            ("th32ParentProcessID", wintypes.DWORD),
            ("pcPriClassBase", wintypes.LONG),
            ("dwFlags", wintypes.DWORD),
            ("szExeFile", wintypes.WCHAR * 260),
        ]

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    snapshot = kernel32.CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
    if snapshot == INVALID_HANDLE_VALUE:
        return {}
    entry = PROCESSENTRY32W()
    entry.dwSize = ctypes.sizeof(entry)
    parents: dict[int, int] = {}
    try:
        if kernel32.Process32FirstW(snapshot, ctypes.byref(entry)):
            while True:
                parents[int(entry.th32ProcessID)] = int(entry.th32ParentProcessID)
                if not kernel32.Process32NextW(snapshot, ctypes.byref(entry)):
                    break
    finally:
        kernel32.CloseHandle(snapshot)
    return parents


def _windows_working_set(pid: int) -> int:
    from ctypes import wintypes

    PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
    PROCESS_VM_READ = 0x0010

    class PROCESS_MEMORY_COUNTERS(ctypes.Structure):
        _fields_ = [
            ("cb", wintypes.DWORD),
            ("PageFaultCount", wintypes.DWORD),
            ("PeakWorkingSetSize", ctypes.c_size_t),
            ("WorkingSetSize", ctypes.c_size_t),
            ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
            ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
            ("PagefileUsage", ctypes.c_size_t),
            ("PeakPagefileUsage", ctypes.c_size_t),
        ]

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    psapi = ctypes.WinDLL("psapi", use_last_error=True)
    handle = kernel32.OpenProcess(
        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, False, pid
    )
    if not handle:
        return 0
    counters = PROCESS_MEMORY_COUNTERS()
    counters.cb = ctypes.sizeof(counters)
    try:
        if not psapi.GetProcessMemoryInfo(
            handle, ctypes.byref(counters), counters.cb
        ):
            return 0
        return int(counters.WorkingSetSize)
    finally:
        kernel32.CloseHandle(handle)


def _posix_process_snapshot() -> dict[int, int]:
    parents: dict[int, int] = {}
    proc = Path("/proc")
    if not proc.is_dir():
        return parents
    for entry in proc.iterdir():
        if not entry.name.isdigit():
            continue
        try:
            fields = (entry / "stat").read_text(encoding="utf-8").split()
            parents[int(entry.name)] = int(fields[3])
        except (OSError, UnicodeError, ValueError, IndexError):
            continue
    return parents


def _posix_working_set(pid: int) -> int:
    try:
        for line in Path(f"/proc/{pid}/status").read_text(encoding="utf-8").splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1]) * 1024
    except (OSError, UnicodeError, ValueError, IndexError):
        pass
    return 0


def process_tree_sample(root_pid: int) -> tuple[set[int], int]:
    """Return current descendant IDs and summed resident working set."""

    if os.name == "nt":
        parents = _windows_process_snapshot()
        working_set = _windows_working_set
    else:
        parents = _posix_process_snapshot()
        working_set = _posix_working_set
    descendants = {root_pid}
    changed = True
    while changed:
        changed = False
        for pid, parent in parents.items():
            if parent in descendants and pid not in descendants:
                descendants.add(pid)
                changed = True
    return descendants, sum(working_set(pid) for pid in descendants)


def process_exists(pid: int) -> bool:
    if os.name == "nt":
        PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        handle = kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, pid)
        if not handle:
            return False
        kernel32.CloseHandle(handle)
        return True
    return Path(f"/proc/{pid}").exists()


def _read_progress(path: Path) -> list[float]:
    if not path.is_file():
        return []
    values: list[float] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        value = json.loads(line)
        progress = value.get("progress") if isinstance(value, Mapping) else None
        if not isinstance(progress, (int, float)) or isinstance(progress, bool):
            raise ValueError("progress events must contain a numeric progress field")
        progress = float(progress)
        if not 0 <= progress <= 1:
            raise ValueError("progress must be within [0, 1]")
        values.append(progress)
    return values


def measure_command_once(
    argv: Sequence[str],
    *,
    cwd: Path,
    result_path: Path,
    phase: str,
    index: int,
    remove_prior_result: bool,
) -> dict[str, Any]:
    progress_handle = tempfile.NamedTemporaryFile(
        prefix="qpls-perf-progress-", suffix=".jsonl", delete=False
    )
    progress_path = Path(progress_handle.name)
    progress_handle.close()
    progress_path.unlink(missing_ok=True)
    environment = os.environ.copy()
    environment.update(
        {
            "QPLS_PERFORMANCE_PHASE": phase,
            "QPLS_PERFORMANCE_RUN_INDEX": str(index),
            "QPLS_PERFORMANCE_PROGRESS_PATH": str(progress_path),
            "QPLS_PERFORMANCE_RESULT_PATH": str(result_path),
        }
    )
    if remove_prior_result:
        result_path.unlink(missing_ok=True)
    elif result_path.exists():
        raise FileExistsError(
            f"measurement result path must be new before the first run: {result_path}"
        )
    started = time.perf_counter()
    process = subprocess.Popen(
        list(argv),
        cwd=cwd,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        shell=False,
    )
    peak = 0
    observed_pids: set[int] = set()
    while process.poll() is None:
        pids, working_set = process_tree_sample(process.pid)
        observed_pids.update(pids)
        peak = max(peak, working_set)
        time.sleep(POLL_SECONDS)
    pids, working_set = process_tree_sample(process.pid)
    observed_pids.update(pids)
    peak = max(peak, working_set)
    elapsed = time.perf_counter() - started
    time.sleep(POLL_SECONDS)
    orphan_processes = sum(
        1 for pid in observed_pids if pid != process.pid and process_exists(pid)
    )
    try:
        progress = _read_progress(progress_path)
    finally:
        progress_path.unlink(missing_ok=True)
    result_bytes = result_path.stat().st_size if result_path.is_file() else 0
    return {
        "phase": phase,
        "index": index,
        "exit_code": int(process.returncode),
        "elapsed_seconds": elapsed,
        "peak_working_set_bytes": peak,
        "result_bytes": result_bytes,
        "progress_values": progress,
        "orphan_processes": orphan_processes,
    }


def detected_total_memory_bytes() -> int:
    if os.name == "nt":
        class MEMORYSTATUSEX(ctypes.Structure):
            _fields_ = [
                ("dwLength", ctypes.c_ulong),
                ("dwMemoryLoad", ctypes.c_ulong),
                ("ullTotalPhys", ctypes.c_ulonglong),
                ("ullAvailPhys", ctypes.c_ulonglong),
                ("ullTotalPageFile", ctypes.c_ulonglong),
                ("ullAvailPageFile", ctypes.c_ulonglong),
                ("ullTotalVirtual", ctypes.c_ulonglong),
                ("ullAvailVirtual", ctypes.c_ulonglong),
                ("ullAvailExtendedVirtual", ctypes.c_ulonglong),
            ]

        status = MEMORYSTATUSEX()
        status.dwLength = ctypes.sizeof(status)
        if ctypes.WinDLL("kernel32").GlobalMemoryStatusEx(ctypes.byref(status)):
            return int(status.ullTotalPhys)
    if hasattr(os, "sysconf"):
        try:
            return int(os.sysconf("SC_PAGE_SIZE") * os.sysconf("SC_PHYS_PAGES"))
        except (OSError, ValueError):
            pass
    return 0


def _hardware_fingerprint(args: argparse.Namespace) -> dict[str, Any]:
    system = platform.system()
    if system != "Windows":
        raise ValueError("QuickPLS performance hardware profiles require Windows 11")
    architecture = platform.machine().lower()
    if architecture in {"amd64", "x86_64"}:
        architecture = "x86_64"
    return {
        "os": "windows_11",
        "architecture": architecture,
        "cpu": args.cpu or platform.processor() or "unknown",
        "physical_cores": args.physical_cores,
        "logical_cores": os.cpu_count() or args.physical_cores,
        "memory_bytes": args.memory_bytes or detected_total_memory_bytes(),
    }


def _resolve_context(args: argparse.Namespace) -> tuple[dict[str, Any], dict[str, Any]]:
    manifest, registry, schema, measurement_schema = load_contract(
        args.manifest, args.registry, args.schema, args.measurement_schema
    )
    context = validate_contract_documents(manifest, registry, schema, measurement_schema)
    if not context["contract_valid"]:
        raise ValueError("performance contract is invalid: " + "; ".join(context["errors"]))
    reference = {
        "registry_schema_version": 2,
        "capability_id": args.capability_id,
        "cell_id": args.cell_id,
        "capability_version": args.capability_version,
    }
    identity = qualification_link_identity(reference)
    if identity not in context["resolved_classes"]:
        raise ValueError("the capability reference is not an active Registry V2 option cell")
    if (args.profile, args.case) not in context["expected_cases"]:
        raise ValueError("the complexity profile/case pair is not in the contract")
    if args.hardware_profile not in context["hardware"]:
        raise ValueError("the hardware profile is not in the contract")
    return context, reference


def _load_observations(path: Path | None) -> dict[str, Any]:
    if path is None:
        return {}
    value = load_json(path)
    allowed = {"cancellation_observation", "memory_growth_observation"}
    if not set(value) <= allowed:
        raise ValueError("observation file contains unsupported fields")
    return value


def build_receipt(args: argparse.Namespace) -> dict[str, Any]:
    context, reference = _resolve_context(args)
    command = list(args.command)
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        raise ValueError("an explicit command is required after --")
    cwd = args.cwd.resolve()
    result_path = args.result_path.resolve()
    policy = load_json(args.manifest)["measurement_policy"]
    warmup_runs = []
    measured_runs = []
    prior_result_is_harness_output = False
    for index in range(policy["warmup_runs"]):
        warmup_runs.append(
            measure_command_once(
                command,
                cwd=cwd,
                result_path=result_path,
                phase="warmup",
                index=index,
                remove_prior_result=prior_result_is_harness_output,
            )
        )
        prior_result_is_harness_output = result_path.exists()
    for index in range(policy["measured_runs"]):
        measured_runs.append(
            measure_command_once(
                command,
                cwd=cwd,
                result_path=result_path,
                phase="measured",
                index=index,
                remove_prior_result=prior_result_is_harness_output,
            )
        )
        prior_result_is_harness_output = result_path.exists()
    aggregates = aggregate_runs(measured_runs)
    all_progress = {
        float(value) for run in measured_runs for value in run["progress_values"]
    }
    exceeded = any(
        run["elapsed_seconds"]
        > load_json(args.manifest)["operation_requirements"]["progress"][
            "elapsed_threshold_seconds"
        ]
        for run in measured_runs
    )
    progress_observation = {
        "operation_exceeded_threshold": exceeded,
        "real_progress_shown": bool(all_progress),
        "distinct_progress_values": len(all_progress),
        "monotonic": all(
            run["progress_values"] == sorted(run["progress_values"])
            for run in measured_runs
        ),
    }
    observations = _load_observations(args.observations)
    baseline_reference = None
    if args.measurement_role == "current":
        if args.baseline is not None:
            baseline = load_json(args.baseline)
            baseline_reference = {
                "measurement_id": baseline.get("measurement_id"),
                "receipt_sha256": canonical_sha256(baseline),
            }
    elif args.baseline is not None:
        raise ValueError("accepted baselines cannot reference another baseline")
    profile = context["profiles"][args.profile]
    cancellation_observation = observations.get("cancellation_observation")
    memory_growth_observation = observations.get("memory_growth_observation")
    runs_complete = all(
        run["exit_code"] == 0
        and run["result_bytes"] > 0
        and run["peak_working_set_bytes"] > 0
        and run["orphan_processes"] == 0
        for run in [*warmup_runs, *measured_runs]
    )
    progress_complete = not exceeded or (
        progress_observation["real_progress_shown"]
        and progress_observation["distinct_progress_values"] >= 2
        and progress_observation["monotonic"]
    )
    receipt_complete = (
        runs_complete
        and progress_complete
        and (not profile["potentially_long"] or cancellation_observation is not None)
        and (not profile["repeat_memory_gate"] or memory_growth_observation is not None)
        and (args.measurement_role != "current" or baseline_reference is not None)
    )
    return {
        "schema_version": 2,
        "document_kind": "capability_performance_measurement",
        "measurement_id": f"qpls-perf-{uuid.uuid4()}",
        "measurement_role": args.measurement_role,
        "captured_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "contract_id": CONTRACT_ID,
        "contract_sha256": context["contract_sha256"],
        "capability_reference": reference,
        "hardware_profile_id": args.hardware_profile,
        "hardware_fingerprint": _hardware_fingerprint(args),
        "budget_class_id": context["resolved_classes"][qualification_link_identity(reference)],
        "profile_id": args.profile,
        "case_id": args.case,
        "applicability": "measured",
        "not_applicable_reason": None,
        "predicate_references": [],
        "command": {
            "argv": command,
            "working_directory": str(cwd),
            "build_fingerprint": args.build_fingerprint,
            "workload_fingerprint": args.workload_fingerprint,
            "process_tree_measured": True,
        },
        "warmup_runs": warmup_runs,
        "measured_runs": measured_runs,
        "aggregates": aggregates,
        "progress_observation": progress_observation,
        "cancellation_observation": cancellation_observation,
        "memory_growth_observation": memory_growth_observation,
        "baseline_reference": baseline_reference,
        "receipt_complete": receipt_complete,
    }


def write_new_receipt(path: Path, receipt: Mapping[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(receipt, indent=2, sort_keys=True, allow_nan=False) + "\n"
    with path.open("x", encoding="utf-8", newline="\n") as handle:
        handle.write(payload)
        handle.flush()
        os.fsync(handle.fileno())


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST_PATH)
    parser.add_argument("--registry", type=Path, default=DEFAULT_REGISTRY_PATH)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA_PATH)
    parser.add_argument(
        "--measurement-schema", type=Path, default=DEFAULT_MEASUREMENT_SCHEMA_PATH
    )
    parser.add_argument("--capability-id", required=True)
    parser.add_argument("--cell-id", required=True)
    parser.add_argument("--capability-version", required=True)
    parser.add_argument("--hardware-profile", required=True)
    parser.add_argument("--physical-cores", type=int, required=True)
    parser.add_argument("--cpu")
    parser.add_argument("--memory-bytes", type=int)
    parser.add_argument("--profile", required=True)
    parser.add_argument("--case", required=True)
    parser.add_argument(
        "--measurement-role", choices=("accepted_baseline", "current"), required=True
    )
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--build-fingerprint", required=True)
    parser.add_argument("--workload-fingerprint", required=True)
    parser.add_argument("--cwd", type=Path, default=Path.cwd())
    parser.add_argument("--result-path", type=Path, required=True)
    parser.add_argument("--observations", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    receipt = build_receipt(args)
    write_new_receipt(args.output, receipt)
    print(
        json.dumps(
            {
                "output": str(args.output),
                "measurement_id": receipt["measurement_id"],
                "receipt_complete": receipt["receipt_complete"],
                "note": "A written receipt is not a product-finalization pass; run the complete V2 validator with every required current and accepted-baseline receipt.",
            },
            indent=2,
        )
    )
    return 0 if receipt["receipt_complete"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
