#!/usr/bin/env python3
"""Generate machine-readable focused Rust evidence for PROCESS v2 promotion.

The runner performs serial release-profile test builds, discovers the exact
lib-test executables from Cargo JSON, lists each executable, and invokes every
frozen test by its full exact name.  Promotion independently re-hashes these
executables and checks source freshness; source-name greps never pass the gate.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
REPORT = RESULTS / "process_v2_boundary_test_report.json"
FEATURE_ID = "qpls3.standalone.process"
METHOD_VERSION = "regression_process_v2"
TARGET = "process_v2_focused_rust_boundary_tests"

SUITES = {
    "qpls_core": {
        "package": "qpls-core",
        "tests": frozenset(
            {
                "process_graph_v2_accepts_typed_dag_and_rejects_cycles_role_and_order_drift",
                "process_graph_v2_bootstrap_is_typed_and_stage_bounded",
            }
        ),
    },
    "qpls_estimation": {
        "package": "qpls-estimation",
        "tests": frozenset(
            {
                "process_graph_v2_parallel_serial_moderation_and_effect_arithmetic",
                "process_graph_v2_hc3_simple_slopes_and_johnson_neyman",
                "process_graph_v2_frozen_raw_probes_survive_resample_recentering",
                "process_graph_v2_rejects_high_leverage_hc3_instability_without_clamping",
                "process_graph_v2_rejects_nonpositive_hc3_variance_without_absolute_value",
                "process_graph_v2_rejects_degenerate_simple_slope_variance",
                "process_graph_v2_point_progress_completes_and_cancellation_returns_no_result",
                "process_graph_v2_point_is_row_irrelevant_column_and_recipe_order_invariant",
                "process_graph_v2_scale_aware_svd_is_affine_unit_invariant_and_rejects_relative_collinearity",
                "process_graph_v2_jn_root_solver_is_affine_stable_and_deduplicates_near_double_roots",
                "process_graph_v2_jn_nonpositive_contrast_variance_is_tagged_unavailable",
                "process_graph_v2_rejects_exact_binary_endogenous_outcomes_in_original_sample",
                "process_graph_v2_semantic_probe_grid_rejects_collapsed_f64_levels",
            }
        ),
    },
    "qpls_resampling": {
        "package": "qpls-resampling",
        "tests": frozenset(
            {
                "process_graph_v2_unavailable_inference_uses_process_specific_tokens",
                "process_graph_v2_case_bootstrap_is_worker_invariant_and_bca_conditional",
                "process_graph_v2_case_bootstrap_cancellation_returns_no_result",
                "process_graph_v2_bootstrap_maps_high_leverage_hc3_failure",
                "process_graph_v2_bootstrap_maps_invalid_hc3_covariance_failure",
                "process_graph_v2_bootstrap_maps_degenerate_simple_slope_failure",
            }
        ),
    },
    "qpls_project": {
        "package": "qpls-project",
        "tests": frozenset(
            {
                "process_graph_v2_unavailable_bootstrap_roundtrip_and_tamper_contract",
                "process_graph_v2_append_save_reopen_and_tamper_are_atomic",
                "process_v1_remains_archive_only",
            }
        ),
    },
    "qpls_runner": {
        "package": "qpls-runner",
        "tests": frozenset(
            {
                "process_v2_point_progress_completes_and_base_fit_cancellation_returns_no_result",
                "process_v2_runner_rejects_exact_binary_endogenous_original_profiles",
            }
        ),
    },
}


def run(command: list[str], *, timeout: int = 1_200) -> subprocess.CompletedProcess[str]:
    environment = dict(os.environ)
    environment["CARGO_BUILD_JOBS"] = "1"
    return subprocess.run(
        command,
        cwd=ROOT,
        env=environment,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )


def artifact(path: Path) -> dict[str, object]:
    contents = path.read_bytes()
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "size": len(contents),
        "sha256": hashlib.sha256(contents).hexdigest(),
    }


def discover_test_executable(output: str, target_name: str) -> Path:
    candidates: list[Path] = []
    for line in output.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        target = message.get("target")
        profile = message.get("profile")
        executable = message.get("executable")
        if (
            message.get("reason") == "compiler-artifact"
            and isinstance(target, dict)
            and target.get("name") == target_name
            and isinstance(profile, dict)
            and profile.get("test") is True
            and isinstance(executable, str)
        ):
            candidates.append(Path(executable).resolve())
    unique = list(dict.fromkeys(candidates))
    if len(unique) != 1 or not unique[0].is_file():
        raise RuntimeError(f"Expected one {target_name} lib-test executable; observed {unique}")
    executable = unique[0]
    expected_parent = (ROOT / "target" / "release" / "deps").resolve()
    if executable.parent != expected_parent or not executable.name.startswith(f"{target_name}-"):
        raise RuntimeError(f"Cargo reported an unexpected test executable: {executable}")
    return executable


def main() -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    build_commands: dict[str, list[str]] = {}
    executables: dict[str, Path] = {}
    listed_names: dict[str, set[str]] = {}
    for target_name, suite in SUITES.items():
        command = [
            "cargo", "test", "--release", "-p", str(suite["package"]),
            "--lib", "--no-run", "--message-format=json",
        ]
        build_commands[target_name] = command
        built = run(command)
        if built.returncode:
            raise SystemExit(
                f"Cargo test build failed for {target_name} ({built.returncode}):\n{built.stderr[-4000:]}"
            )
        executable = discover_test_executable(built.stdout, target_name)
        executables[target_name] = executable
        listed = run([str(executable), "--list"], timeout=120)
        if listed.returncode:
            raise SystemExit(f"Rust test listing failed for {target_name}:\n{listed.stderr[-4000:]}")
        listed_names[target_name] = {
            line.rsplit(": test", 1)[0].strip()
            for line in listed.stdout.splitlines()
            if line.rstrip().endswith(": test")
        }

    selected: dict[str, tuple[str, str]] = {}
    for target_name, suite in SUITES.items():
        for expected in suite["tests"]:
            matches = [
                name for name in listed_names[target_name]
                if name == expected or name.endswith(f"::{expected}")
            ]
            if len(matches) != 1:
                raise SystemExit(
                    f"Expected exactly one {target_name} test ending in {expected!r}; observed {matches}"
                )
            selected[expected] = (target_name, matches[0])

    checks: dict[str, dict[str, bool]] = {target: {} for target in SUITES}
    executions: dict[str, dict[str, object]] = {}
    for short_name in sorted(selected):
        target_name, full_name = selected[short_name]
        completed = run(
            [str(executables[target_name]), full_name, "--exact", "--test-threads=1"],
            timeout=300,
        )
        passed = completed.returncode == 0 and "1 passed; 0 failed" in completed.stdout
        checks[target_name][short_name] = passed
        executions[short_name] = {
            "target": target_name,
            "full_name": full_name,
            "exit_code": completed.returncode,
            "passed": passed,
            "stdout_tail": completed.stdout[-2000:],
            "stderr_tail": completed.stderr[-2000:],
        }

    exact_sets = all(
        frozenset(checks[target]) == suite["tests"]
        for target, suite in SUITES.items()
    )
    report = {
        "schema_version": 1,
        "target": TARGET,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "passed": exact_sets and all(value for suite in checks.values() for value in suite.values()),
        "checks": checks,
        "environment": {"CARGO_BUILD_JOBS": "1"},
        "build_commands": build_commands,
        "test_executables": {
            target: artifact(executable) for target, executable in executables.items()
        },
        "executions": executions,
    }
    REPORT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {REPORT} | passed={report['passed']}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
