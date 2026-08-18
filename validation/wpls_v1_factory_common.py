"""Method-scoped evidence utilities for the bounded WPLS v1 factory.

The helpers deliberately use an already-built local CLI and fail closed when
that executable predates any WPLS execution source.  They never build or
package the product, and every identity report binds the frozen manifest plus
the exact repository bytes exercised by the check.
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
MANIFEST_PATH = VALIDATION / "methods" / "wpls_v1.manifest.json"
REPORT_ROOT = VALIDATION / "results" / "method_factory" / "wpls_v1"
WORK_ROOT = REPORT_ROOT / "work"
CLI = ROOT / "target" / "debug" / "qpls.exe"
COMMON_SOURCE = "validation/wpls_v1_factory_common.py"
FOCUSED_FACTORY_TEST = "validation/test_wpls_v1_factory_evidence.py"
EXECUTION_SOURCES = (
    "crates/qpls-core/src/contract.rs",
    "crates/qpls-core/src/validation.rs",
    "crates/qpls-estimation/src/pls.rs",
    "crates/qpls-assessment/src/lib.rs",
    "crates/qpls-runner/src/lib.rs",
    "crates/qpls-cli/src/main.rs",
)


class DuplicateKeyError(ValueError):
    """Raised when a JSON object contains an ambiguous duplicate key."""


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
        if "\\" in relative or Path(relative).is_absolute() or ".." in Path(relative).parts:
            raise ValueError(f"unsafe repository source path: {relative!r}")
        path = ROOT / relative
        if not path.is_file():
            raise FileNotFoundError(f"required WPLS v1 evidence source is missing: {relative}")
        descriptors.append(
            {"path": relative, "size": path.stat().st_size, "sha256": sha256_file(path)}
        )
    return descriptors


def manifest() -> dict[str, Any]:
    document = strict_load_json(MANIFEST_PATH)
    feature = document["feature"]
    expected = {
        "id": "qpls3.pls.weighted",
        "method_version": "wpls_case_weighted_v1",
        "catalogue_snapshot_date": "2026-08-12",
    }
    for key, value in expected.items():
        if feature.get(key) != value:
            raise ValueError(
                f"WPLS v1 manifest identity mismatch for {key}: "
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
    report = {
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
    command: Sequence[str], *, timeout: int = 300, env: dict[str, str] | None = None
) -> tuple[subprocess.CompletedProcess[str], dict[str, Any]]:
    started = time.monotonic()
    completed = subprocess.run(
        list(command),
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        env={**os.environ, **(env or {})},
    )
    record = {
        "command": list(command),
        "returncode": completed.returncode,
        "duration_seconds": round(time.monotonic() - started, 3),
        "stdout_tail": completed.stdout[-4000:],
        "stderr_tail": completed.stderr[-4000:],
    }
    return completed, record


def require_current_cli() -> dict[str, Any]:
    if not CLI.is_file():
        raise FileNotFoundError(
            "target/debug/qpls.exe is required for the light WPLS factory lane; "
            "this lane does not build the product"
        )
    source_times = {relative: (ROOT / relative).stat().st_mtime_ns for relative in EXECUTION_SOURCES}
    newest_source = max(source_times.values())
    current = CLI.stat().st_mtime_ns >= newest_source
    if not current:
        raise RuntimeError(
            "target/debug/qpls.exe predates a WPLS execution source; run the coordinated "
            "repository build before regenerating factory evidence"
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


def write_csv(
    path: Path,
    variables: Sequence[str],
    rows: Sequence[Sequence[float | None]],
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(variables)
        for row in rows:
            writer.writerow(["" if value is None else f"{float(value):.15g}" for value in row])


def _fingerprint(csv_path: Path, name: str) -> tuple[str, dict[str, Any]]:
    project = WORK_ROOT / f"{name}.fingerprint.qpls"
    project.unlink(missing_ok=True)
    imported, import_execution = run_command(
        [str(CLI), "import", repository_path(csv_path), repository_path(project), "--name", name]
    )
    if imported.returncode != 0:
        raise RuntimeError(f"WPLS fixture import failed for {name}: {import_execution}")
    inspected, inspect_execution = run_command(
        [str(CLI), "inspect", repository_path(project), "--json"]
    )
    if inspected.returncode != 0:
        raise RuntimeError(f"WPLS fixture inspect failed for {name}: {inspect_execution}")
    payload = json.loads(inspected.stdout)
    return payload["datasets"][0]["fingerprint"], {
        "import": import_execution,
        "inspect": inspect_execution,
        "project": repository_path(project),
        "project_sha256": sha256_file(project),
    }


def construct(construct_id: str, indicators: Sequence[str], *, mode: str = "reflective") -> dict[str, Any]:
    return {
        "id": construct_id,
        "name": construct_id.upper(),
        "short_name": construct_id.upper(),
        "mode": mode,
        "indicators": list(indicators),
    }


def run_model(
    *,
    name: str,
    csv_path: Path,
    constructs: Sequence[dict[str, Any]],
    paths: Sequence[dict[str, str]],
    method: str = "wpls",
    case_weight_column: str | None = "case_wt",
    weighting_scheme: str = "path",
    preprocessing: str = "standardized",
    bootstrap_samples: int = 0,
    permutation_samples: int = 0,
    tolerance: float = 1e-7,
    max_iterations: int = 3_000,
    interactions: Sequence[dict[str, Any]] = (),
    higher_order_constructs: Sequence[dict[str, Any]] = (),
    expect_success: bool = True,
) -> dict[str, Any]:
    cli_identity = require_current_cli()
    WORK_ROOT.mkdir(parents=True, exist_ok=True)
    fingerprint, fingerprint_execution = _fingerprint(csv_path, name)
    recipe = WORK_ROOT / f"{name}.recipe.json"
    output = WORK_ROOT / f"{name}.quickpls.json"
    output.unlink(missing_ok=True)
    recipe_payload = {
        "schema_version": 3,
        "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"quickpls:wpls-v1:{name}")),
        "created_at": "2026-08-13T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"quickpls:wpls-v1:model:{name}")),
            "name": "WPLS v1 factory fixture",
            "constructs": list(constructs),
            "paths": list(paths),
            "controls": [],
            "higher_order_constructs": list(higher_order_constructs),
            "interactions": list(interactions),
        },
        "settings": {
            "method": method,
            "weighting_scheme": weighting_scheme,
            "tolerance": tolerance,
            "max_iterations": max_iterations,
            "bootstrap_samples": bootstrap_samples,
            "studentized_inner_samples": 0,
            "permutation_samples": permutation_samples,
            "seed": 20_260_813,
            "workers": 1,
            "confidence_level": 0.95,
            "preprocessing": preprocessing,
            "missing_data": "listwise_deletion",
            "case_weight_column": case_weight_column,
        },
        "method_config": {"kind": "wpls" if method == "wpls" else "pls_algorithm"},
        "metadata": {"status": "validated_wpls_case_weighted_v1_bounded_scope"},
    }
    recipe.write_text(
        json.dumps(recipe_payload, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    completed, execution = run_command(
        [
            str(CLI),
            "run",
            repository_path(recipe),
            "--data",
            repository_path(csv_path),
            "--output",
            repository_path(output),
            "--allow-experimental",
            "--allow-internal-qualification",
        ],
        timeout=600,
    )
    base = {
        "execution": execution,
        "recipe": repository_path(recipe),
        "recipe_sha256": sha256_file(recipe),
        "fingerprint": fingerprint,
        "fingerprint_execution": fingerprint_execution,
        "cli_identity": cli_identity,
    }
    if not expect_success:
        return {**base, "passed": completed.returncode != 0 and not output.exists()}
    if completed.returncode != 0 or not output.is_file():
        raise RuntimeError(f"WPLS execution failed for {name}: {execution}")
    document = strict_load_json(output)
    estimation = document["payload"]["estimation"]
    if method == "wpls":
        wpls = estimation.get("wpls")
        identity_passed = (
            document["payload"]["kind"] == "pls_pm_v1"
            and estimation.get("method_version") == "wpls_case_weighted_v1"
            and isinstance(wpls, dict)
            and wpls.get("method_version") == "wpls_case_weighted_v1"
            and wpls.get("case_weight_column") == case_weight_column
            and wpls.get("covariance") == "positive_case_weighted_unbiased_covariance_v1"
            and math.isfinite(wpls.get("weight_sum", float("nan")))
            and wpls.get("weight_sum", 0) > 0
            and math.isfinite(wpls.get("effective_sample_size", float("nan")))
            and wpls.get("effective_sample_size", 0) > 0
            and document["provenance"]["method"] == "wpls"
            and document["provenance"]["dataset_fingerprint"] == fingerprint
            and document["provenance"]["settings"]["case_weight_column"] == case_weight_column
        )
    else:
        identity_passed = (
            document["payload"]["kind"] == "pls_pm_v1"
            and estimation.get("method_version") == "pls_pm_v1"
            and estimation.get("wpls") is None
            and document["provenance"]["method"] == "pls_pm"
            and document["provenance"]["dataset_fingerprint"] == fingerprint
        )
    return {
        **base,
        "passed": completed.returncode == 0 and identity_passed,
        "result": document,
        "estimation": estimation,
        "wpls": estimation.get("wpls"),
        "output": repository_path(output),
        "output_sha256": sha256_file(output),
        "identity_passed": identity_passed,
    }


def analytic_payload(run: dict[str, Any], *, include_scores: bool = True) -> dict[str, Any]:
    estimation = run["estimation"]
    payload = {
        "used_observations": estimation["used_observations"],
        "omitted_observations": estimation["omitted_observations"],
        "iterations": estimation["iterations"],
        "paths": estimation["paths"],
        "outer_estimates": estimation["outer_estimates"],
        "r_squared": estimation["r_squared"],
        "effects": estimation["effects"],
        "wpls": estimation.get("wpls"),
    }
    if include_scores:
        payload["construct_scores"] = estimation["construct_scores"]
    return payload
