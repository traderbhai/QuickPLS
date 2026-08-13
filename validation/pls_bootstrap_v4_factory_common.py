"""Fail-closed utilities for the PLS bootstrap v4 method factory.

The factory never builds or packages QuickPLS.  It accepts only a current,
prebuilt debug CLI and binds every identity report to the exact repository
bytes exercised by the check.
"""

from __future__ import annotations

import csv
import hashlib
import json
import math
import os
import subprocess
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Sequence


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
MANIFEST_PATH = VALIDATION / "methods" / "pls_bootstrap_v4.manifest.json"
REPORT_ROOT = VALIDATION / "results" / "method_factory" / "pls_bootstrap_v4"
WORK_ROOT = REPORT_ROOT / "work"
CLI = ROOT / "target" / "debug" / "qpls.exe"
COMMON_SOURCE = "validation/pls_bootstrap_v4_factory_common.py"
FOCUSED_FACTORY_TEST = "validation/test_pls_bootstrap_v4_factory_evidence.py"
EXECUTION_SOURCES = (
    "crates/qpls-core/src/contract.rs",
    "crates/qpls-core/src/validation.rs",
    "crates/qpls-estimation/src/pls.rs",
    "crates/qpls-resampling/src/lib.rs",
    "crates/qpls-runner/src/lib.rs",
    "crates/qpls-project/src/lib.rs",
    "crates/qpls-cli/src/main.rs",
)


class DuplicateKeyError(ValueError):
    """Raised for ambiguous JSON evidence."""


def _strict_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateKeyError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def strict_load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8-sig") as handle:
        return json.load(
            handle,
            object_pairs_hook=_strict_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                ValueError(f"non-finite JSON number: {value}")
            ),
        )


def repository_path(path: Path) -> str:
    return path.resolve().relative_to(ROOT.resolve()).as_posix()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def source_descriptors(paths: Iterable[str]) -> list[dict[str, Any]]:
    descriptors: list[dict[str, Any]] = []
    for relative in sorted(set(paths)):
        candidate = Path(relative)
        if "\\" in relative or candidate.is_absolute() or ".." in candidate.parts:
            raise ValueError(f"unsafe repository source path: {relative!r}")
        path = ROOT / relative
        if not path.is_file():
            raise FileNotFoundError(f"required bootstrap v4 source is missing: {relative}")
        descriptors.append(
            {"path": relative, "size": path.stat().st_size, "sha256": sha256_file(path)}
        )
    return descriptors


def manifest() -> dict[str, Any]:
    document = strict_load_json(MANIFEST_PATH)
    expected = {
        "id": "qpls3.inference.bootstrap",
        "method_version": "indexed_resampling_v4",
        "catalogue_snapshot_date": "2026-08-12",
    }
    feature = document["feature"]
    for key, value in expected.items():
        if feature.get(key) != value:
            raise ValueError(
                f"bootstrap v4 manifest identity mismatch for {key}: "
                f"expected {value!r}, found {feature.get(key)!r}"
            )
    return document


def role_sources(role: str, extras: Iterable[str] = ()) -> list[str]:
    document = manifest()
    governance = document["governance"]
    return sorted(
        {
            governance["manifest_path"],
            governance["schema_path"],
            governance["validator_path"],
            governance["focused_test_path"],
            COMMON_SOURCE,
            FOCUSED_FACTORY_TEST,
            *document["qualification"]["source_requirements"][role],
            *extras,
        }
    )


def write_identity_report(
    role: str,
    *,
    passed: bool,
    checks: dict[str, Any],
    extras: Iterable[str] = (),
    execution: dict[str, Any] | None = None,
) -> Path:
    document = manifest()
    feature = document["feature"]
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    path = REPORT_ROOT / f"{role}.identity.json"
    report: dict[str, Any] = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": role,
        "passed": bool(passed),
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "source_artifacts": source_descriptors(role_sources(role, extras)),
        "checks": checks,
    }
    if execution is not None:
        report["execution"] = execution
    path.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return path


def run_command(
    command: Sequence[str], *, timeout: int = 600, env: dict[str, str] | None = None
) -> tuple[subprocess.CompletedProcess[str], dict[str, Any]]:
    started = time.monotonic()
    completed = subprocess.run(
        list(command),
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        env={**os.environ, "CARGO_BUILD_JOBS": "1", **(env or {})},
    )
    return completed, {
        "command": list(command),
        "returncode": completed.returncode,
        "duration_seconds": round(time.monotonic() - started, 3),
        "stdout_tail": completed.stdout[-4000:],
        "stderr_tail": completed.stderr[-4000:],
    }


def require_current_cli() -> dict[str, Any]:
    if not CLI.is_file():
        raise FileNotFoundError(
            "target/debug/qpls.exe is required; the bootstrap factory does not build the product"
        )
    source_times = {
        relative: (ROOT / relative).stat().st_mtime_ns for relative in EXECUTION_SOURCES
    }
    current = CLI.stat().st_mtime_ns >= max(source_times.values())
    if not current:
        raise RuntimeError(
            "target/debug/qpls.exe predates a bound bootstrap execution source; "
            "run the coordinated repository build before regenerating evidence"
        )
    return {
        "passed": True,
        "path": repository_path(CLI),
        "sha256": sha256_file(CLI),
        "prebuilt": True,
        "built_by_factory": False,
        "newer_than_bound_execution_sources": current,
        "source_mtime_ns": source_times,
    }


def write_csv(path: Path, variables: Sequence[str], rows: Sequence[Sequence[float]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(variables)
        writer.writerows([f"{float(value):.15g}" for value in row] for row in rows)


def construct(construct_id: str, indicators: Sequence[str], *, mode: str = "reflective") -> dict[str, Any]:
    return {
        "id": construct_id,
        "name": construct_id.upper(),
        "short_name": construct_id.upper(),
        "mode": mode,
        "indicators": list(indicators),
    }


def _fingerprint(csv_path: Path, name: str) -> tuple[str, dict[str, Any]]:
    project = WORK_ROOT / f"{name}.fingerprint.qpls"
    project.unlink(missing_ok=True)
    imported, import_execution = run_command(
        [str(CLI), "import", repository_path(csv_path), repository_path(project), "--name", name]
    )
    if imported.returncode != 0:
        raise RuntimeError(f"bootstrap fixture import failed: {import_execution}")
    inspected, inspect_execution = run_command(
        [str(CLI), "inspect", repository_path(project), "--json"]
    )
    if inspected.returncode != 0:
        raise RuntimeError(f"bootstrap fixture inspect failed: {inspect_execution}")
    document = json.loads(inspected.stdout)
    return document["datasets"][0]["fingerprint"], {
        "import": import_execution,
        "inspect": inspect_execution,
        "project": repository_path(project),
        "project_sha256": sha256_file(project),
    }


def run_bootstrap(
    *,
    name: str,
    csv_path: Path,
    constructs: Sequence[dict[str, Any]],
    paths: Sequence[dict[str, str]],
    bootstrap_samples: int = 199,
    studentized_inner_samples: int = 0,
    workers: int = 1,
    seed: int = 20_260_813,
    confidence_level: float = 0.95,
    expect_success: bool = True,
) -> dict[str, Any]:
    cli_identity = require_current_cli()
    WORK_ROOT.mkdir(parents=True, exist_ok=True)
    fingerprint, fingerprint_execution = _fingerprint(csv_path, name)
    recipe_path = WORK_ROOT / f"{name}.recipe.json"
    output_path = WORK_ROOT / f"{name}.quickpls.json"
    output_path.unlink(missing_ok=True)
    recipe = {
        "schema_version": 3,
        "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"quickpls:bootstrap-v4:{name}")),
        "created_at": "2026-08-13T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"quickpls:bootstrap-v4:model:{name}")),
            "name": "PLS bootstrap v4 factory fixture",
            "constructs": list(constructs),
            "paths": list(paths),
            "controls": [],
            "higher_order_constructs": [],
            "interactions": [],
        },
        "settings": {
            "method": "pls_pm",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": bootstrap_samples,
            "studentized_inner_samples": studentized_inner_samples,
            "permutation_samples": 0,
            "seed": seed,
            "workers": workers,
            "confidence_level": confidence_level,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
            "case_weight_column": None,
        },
        "method_config": {"kind": "pls_bootstrap"},
        "metadata": {"status": "factory_bootstrap_v4_bounded_scope"},
    }
    recipe_path.write_text(
        json.dumps(recipe, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    completed, execution = run_command(
        [
            str(CLI),
            "run",
            repository_path(recipe_path),
            "--data",
            repository_path(csv_path),
            "--output",
            repository_path(output_path),
            "--allow-experimental",
        ],
        timeout=1800,
    )
    base = {
        "execution": execution,
        "recipe": repository_path(recipe_path),
        "recipe_sha256": sha256_file(recipe_path),
        "fingerprint_execution": fingerprint_execution,
        "cli_identity": cli_identity,
    }
    if not expect_success:
        return {**base, "passed": completed.returncode != 0 and not output_path.exists()}
    if completed.returncode != 0 or not output_path.is_file():
        raise RuntimeError(f"bootstrap execution failed: {execution}")
    document = strict_load_json(output_path)
    bootstrap = document.get("payload", {}).get("bootstrap")
    identity = (
        document.get("status") == "completed"
        and document.get("payload", {}).get("kind") in {"pls_pm_v2", "pls_pm_v3"}
        and isinstance(bootstrap, dict)
        and bootstrap.get("method_version") == "indexed_resampling_v4"
        and bootstrap.get("plan", {}).get("replicates") == bootstrap_samples
        and bootstrap.get("plan", {}).get("master_seed") == seed
        and document.get("provenance", {}).get("settings", {}).get("workers") == workers
        and document.get("provenance", {}).get("dataset_fingerprint") == fingerprint
    )
    return {
        **base,
        "passed": bool(identity),
        "identity_passed": bool(identity),
        "result": document,
        "bootstrap": bootstrap,
        "output": repository_path(output_path),
        "output_sha256": sha256_file(output_path),
    }


def parameter_row(bootstrap: dict[str, Any], family: str, kind: str, *parts: str) -> dict[str, Any]:
    key = json.dumps([kind, list(parts)], separators=(",", ":"))
    rows = bootstrap[family]["parameters"]
    matches = [row for row in rows if row.get("parameter") == key]
    if len(matches) != 1:
        raise ValueError(f"expected one {family} parameter {key}, found {len(matches)}")
    return matches[0]


def finite_interval(row: dict[str, Any]) -> bool:
    return all(math.isfinite(float(row[key])) for key in ("lower", "upper")) and row["lower"] <= row["upper"]
