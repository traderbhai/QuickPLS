#!/usr/bin/env python3
"""Generate exact Rust boundary evidence for structural randomization v1.

The gate performs release-profile no-run builds with one Cargo worker, discovers
the exact produced lib-test executables from Cargo JSON, lists each executable,
and runs only the six frozen test symbols with ``--exact``. Promotion re-hashes
the same executables and independently checks their source freshness; source
grep or a broad crate-level pass cannot substitute for these executions.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
REPORT = RESULTS / "structural_path_randomization_boundary_test_report.json"
FEATURE_ID = "qpls3.inference.structural_path_randomization"
METHOD_VERSION = "freedman_lane_permutation_v1"
TARGET = "structural_path_randomization_v1_focused_rust_boundary_tests"

SUITES: dict[str, dict[str, Any]] = {
    "qpls_resampling": {
        "package": "qpls-resampling",
        "tests": frozenset(
            {
                "indexed_permutations_are_bijections_and_replicate_specific",
                "freedman_lane_reference_seam_matches_orthogonal_multi_predictor_fixture_and_rejects_invalid_indices",
                "pls_freedman_lane_multi_path_is_seeded_worker_invariant_and_progressive",
                "pls_freedman_lane_multi_path_cancellation_discards_partial_output",
                "permutation_wire_contract_rejects_unknown_fields",
            }
        ),
        "source_crates": (
            "qpls-core",
            "qpls-data",
            "qpls-estimation",
            "qpls-resampling",
        ),
    },
    "qpls_project": {
        "package": "qpls-project",
        "tests": frozenset(
            {
                "permutation_pls_multi_path_appends_round_trips_and_rejects_manifest_tampering",
            }
        ),
        "source_crates": (
            "qpls-core",
            "qpls-data",
            "qpls-estimation",
            "qpls-assessment",
            "qpls-resampling",
            "qpls-runner",
            "qpls-project",
        ),
    },
}


def suite_source_paths(root: Path, crate_names: tuple[str, ...]) -> tuple[Path, ...]:
    paths: list[Path] = [root / "Cargo.toml", root / "Cargo.lock"]
    for crate_name in crate_names:
        crate = root / "crates" / crate_name
        paths.append(crate / "Cargo.toml")
        paths.extend(sorted(crate.rglob("*.rs"), key=lambda path: path.as_posix()))
    unique = tuple(dict.fromkeys(path.resolve() for path in paths))
    missing = [path for path in unique if not path.is_file()]
    if missing:
        raise RuntimeError(f"boundary source set is incomplete: {missing}")
    return unique


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def artifact(root: Path, path: Path) -> dict[str, object]:
    return {
        "path": path.resolve().relative_to(root.resolve()).as_posix(),
        "size": path.stat().st_size,
        "sha256": sha256_file(path),
        "modified_at_ns": path.stat().st_mtime_ns,
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
        raise RuntimeError(f"expected one {target_name} lib-test executable; observed {unique}")
    executable = unique[0]
    expected_parent = (ROOT / "target" / "release" / "deps").resolve()
    if executable.parent != expected_parent or not executable.name.startswith(f"{target_name}-"):
        raise RuntimeError(f"Cargo reported an unexpected test executable: {executable}")
    return executable


def listed_test_names(executable: Path) -> set[str]:
    listed = run([str(executable), "--list"], timeout=120)
    if listed.returncode:
        raise RuntimeError(
            f"Rust test listing failed for {executable.name}: {listed.stderr[-4_000:]}"
        )
    return {
        line.rsplit(": test", 1)[0].strip()
        for line in listed.stdout.splitlines()
        if line.rstrip().endswith(": test")
    }


def source_snapshot(paths: tuple[Path, ...]) -> list[dict[str, object]]:
    return [artifact(ROOT, path) for path in paths]


def main() -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    source_paths = {
        target: suite_source_paths(ROOT, suite["source_crates"])
        for target, suite in SUITES.items()
    }
    before_sources = {target: source_snapshot(paths) for target, paths in source_paths.items()}
    build_commands: dict[str, list[str]] = {}
    executables: dict[str, Path] = {}
    listed_names: dict[str, set[str]] = {}
    for target_name, suite in SUITES.items():
        command = [
            "cargo",
            "test",
            "--release",
            "-p",
            suite["package"],
            "--lib",
            "--no-run",
            "--message-format=json",
        ]
        build_commands[target_name] = command
        built = run(command)
        if built.returncode:
            raise SystemExit(
                f"Cargo test build failed for {target_name} ({built.returncode}):\n"
                f"{built.stderr[-4_000:]}"
            )
        executables[target_name] = discover_test_executable(built.stdout, target_name)
        listed_names[target_name] = listed_test_names(executables[target_name])

    selected: dict[str, tuple[str, str]] = {}
    for target_name, suite in SUITES.items():
        for expected in suite["tests"]:
            matches = [
                name
                for name in listed_names[target_name]
                if name == expected or name.endswith(f"::{expected}")
            ]
            if len(matches) != 1:
                raise SystemExit(
                    f"expected exactly one {target_name} test ending in {expected!r}; "
                    f"observed {matches}"
                )
            selected[expected] = (target_name, matches[0])

    checks: dict[str, dict[str, bool]] = {target: {} for target in SUITES}
    executions: dict[str, dict[str, object]] = {}
    for short_name in sorted(selected):
        target_name, full_name = selected[short_name]
        completed = run(
            [
                str(executables[target_name]),
                full_name,
                "--exact",
                "--test-threads=1",
            ],
            timeout=300,
        )
        passed = completed.returncode == 0 and "1 passed; 0 failed" in completed.stdout
        checks[target_name][short_name] = passed
        executions[short_name] = {
            "target": target_name,
            "full_name": full_name,
            "exit_code": completed.returncode,
            "passed": passed,
            "stdout_tail": completed.stdout[-2_000:],
            "stderr_tail": completed.stderr[-2_000:],
        }

    after_sources = {target: source_snapshot(paths) for target, paths in source_paths.items()}
    source_stable = before_sources == after_sources
    executable_artifacts = {
        target: artifact(ROOT, executable) for target, executable in executables.items()
    }
    executable_freshness = {
        target: all(
            executable.stat().st_mtime_ns >= source.stat().st_mtime_ns
            for source in source_paths[target]
        )
        for target, executable in executables.items()
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
        "environment": {"CARGO_BUILD_JOBS": "1", "test_threads": 1},
        "build_commands": build_commands,
        "expected_tests": {
            target: sorted(suite["tests"]) for target, suite in SUITES.items()
        },
        "checks": checks,
        "executions": executions,
        "test_executables": executable_artifacts,
        "source_artifacts": after_sources,
        "source_stable_during_gate": source_stable,
        "test_executables_not_older_than_sources": executable_freshness,
        "passed": (
            exact_sets
            and len(selected) == 6
            and all(value for suite in checks.values() for value in suite.values())
            and source_stable
            and all(executable_freshness.values())
        ),
    }
    REPORT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {REPORT} | passed={report['passed']}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
