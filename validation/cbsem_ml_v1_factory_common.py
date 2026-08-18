"""Method-scoped evidence helpers for the bounded CB-SEM ML v1 workflow.

The helpers deliberately expose only the frozen single-group, reflective,
raw-data ML CFA/recursive-SEM contract.  Every identity report binds the exact
repository bytes used to produce its claim.
"""

from __future__ import annotations

import csv
import hashlib
import json
import os
import subprocess
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Sequence


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
MANIFEST_PATH = VALIDATION / "methods" / "cbsem_ml_v1.manifest.json"
REPORT_ROOT = VALIDATION / "results" / "method_factory" / "cbsem_ml_v1"
WORK_ROOT = REPORT_ROOT / "work"
CLI = ROOT / "target" / "debug" / "qpls.exe"
COMMON_SOURCE = "validation/cbsem_ml_v1_factory_common.py"
FOCUSED_FACTORY_TEST = "validation/test_cbsem_ml_v1_factory.py"
EXPECTED_PROVENANCE_VERSION = (
    "pls_pm_v1+cbsem_ml_v1+cbsem_fit_v1+"
    "cbsem_modification_indices_v1+pls_mediation_v1+pls_assessment_v8"
)


class DuplicateKeyError(ValueError):
    """Raised when JSON contains an ambiguous duplicate object key."""


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
            raise FileNotFoundError(
                f"required CB-SEM ML v1 evidence source is missing: {relative}"
            )
        descriptors.append(
            {
                "path": relative,
                "size": path.stat().st_size,
                "sha256": sha256_file(path),
            }
        )
    return descriptors


def manifest() -> dict[str, Any]:
    document = strict_load_json(MANIFEST_PATH)
    feature = document["feature"]
    expected = {
        "id": "qpls3.cbsem.ml",
        "method_version": "cbsem_ml_v1",
        "catalogue_snapshot_date": "2026-08-12",
    }
    for key, value in expected.items():
        if feature.get(key) != value:
            raise ValueError(
                f"CB-SEM ML v1 manifest identity mismatch for {key}: "
                f"expected {value!r}, found {feature.get(key)!r}"
            )
    return document


def role_sources(role: str, extras: Iterable[str] = ()) -> list[str]:
    document = manifest()
    governance = document["governance"]
    required = {
        governance["manifest_path"],
        governance["schema_path"],
        governance["validator_path"],
        governance["focused_test_path"],
        COMMON_SOURCE,
        FOCUSED_FACTORY_TEST,
        *document["qualification"]["source_requirements"][role],
        *extras,
    }
    return sorted(required)


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
    command: Sequence[str],
    *,
    timeout: int = 300,
    env: dict[str, str] | None = None,
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
    record = {
        "command": list(command),
        "returncode": completed.returncode,
        "duration_seconds": round(time.monotonic() - started, 3),
        "stdout_tail": completed.stdout[-6000:],
        "stderr_tail": completed.stderr[-6000:],
    }
    return completed, record


def ensure_cli() -> dict[str, Any]:
    if not CLI.is_file():
        raise FileNotFoundError(
            "the coordinated source-frozen debug qpls.exe is missing; rebuild it "
            "before starting evidence generation"
        )
    return {
        "passed": True,
        "path": repository_path(CLI),
        "sha256": sha256_file(CLI),
        "built": False,
    }


def engine_source_paths() -> list[str]:
    """Return the exact local Rust source closure exercised by CB-SEM runs."""

    paths = {"Cargo.lock", "Cargo.toml", "crates/qpls-cli/Cargo.toml"}
    for crate in (
        "qpls-assessment",
        "qpls-core",
        "qpls-data",
        "qpls-estimation",
        "qpls-project",
        "qpls-resampling",
        "qpls-runner",
    ):
        crate_root = ROOT / "crates" / crate
        paths.add(repository_path(crate_root / "Cargo.toml"))
        paths.update(
            repository_path(path)
            for path in (crate_root / "src").rglob("*.rs")
            if path.is_file()
        )
    paths.update(
        repository_path(path)
        for path in (ROOT / "crates" / "qpls-cli" / "src").rglob("*.rs")
        if path.is_file()
    )
    return sorted(paths)


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
            writer.writerow(
                ["" if value is None else f"{float(value):.15g}" for value in row]
            )


def _fingerprint(csv_path: Path, name: str) -> tuple[str, dict[str, Any]]:
    project = WORK_ROOT / f"{name}.fingerprint.qpls"
    project.unlink(missing_ok=True)
    completed, execution = run_command(
        [
            str(CLI),
            "import",
            repository_path(csv_path),
            repository_path(project),
            "--name",
            name,
        ]
    )
    if completed.returncode != 0:
        raise RuntimeError(f"CB-SEM fixture import failed for {name}: {execution}")
    inspected, inspect_execution = run_command(
        [str(CLI), "inspect", repository_path(project), "--json"]
    )
    if inspected.returncode != 0:
        raise RuntimeError(f"CB-SEM fixture inspect failed for {name}: {inspect_execution}")
    payload = json.loads(inspected.stdout)
    return payload["datasets"][0]["fingerprint"], {
        "import": execution,
        "inspect": inspect_execution,
        "project": repository_path(project),
        "project_sha256": sha256_file(project),
    }


def _construct(identifier: str, indicators: Sequence[str]) -> dict[str, Any]:
    return {
        "id": identifier,
        "name": identifier.upper(),
        "short_name": identifier.upper(),
        "mode": "reflective",
        "indicators": list(indicators),
    }


def run_cbsem(
    *,
    name: str,
    csv_path: Path,
    constructs: Sequence[tuple[str, Sequence[str]]],
    paths: Sequence[tuple[str, str]],
    model_type: str = "sem",
    estimator: str = "ml",
    input_kind: str = "raw",
    mean_structure: bool = False,
    bootstrap_samples: int = 0,
    expect_success: bool = True,
) -> dict[str, Any]:
    ensure_cli()
    WORK_ROOT.mkdir(parents=True, exist_ok=True)
    fingerprint, fingerprint_execution = _fingerprint(csv_path, name)
    recipe = WORK_ROOT / f"{name}.recipe.json"
    output = WORK_ROOT / f"{name}.quickpls.json"
    output.unlink(missing_ok=True)
    recipe_payload = {
        "schema_version": 3,
        "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"quickpls:cbsem-ml-v1:{name}")),
        "created_at": "2026-08-13T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": str(
                uuid.uuid5(uuid.NAMESPACE_URL, f"quickpls:cbsem-ml-v1:model:{name}")
            ),
            "name": "CB-SEM ML v1 factory fixture",
            "constructs": [_construct(identifier, indicators) for identifier, indicators in constructs],
            "paths": [
                {"source": source, "target": target} for source, target in paths
            ],
            "controls": [],
            "higher_order_constructs": [],
            "interactions": [],
        },
        "settings": {
            "method": "cbsem",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "studentized_inner_samples": 0,
            "permutation_samples": 0,
            "seed": 20260813,
            "workers": 1,
            "confidence_level": 0.95,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
            "case_weight_column": None,
        },
        "method_config": {
            "kind": "cbsem",
            "model_type": model_type,
            "estimator": estimator,
            "input": input_kind,
            "mean_structure": mean_structure,
            "bootstrap_samples": bootstrap_samples,
        },
        "metadata": {
            "status": "validated_v1_2_4_cbsem_single_group_bounded_scope"
        },
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
        ],
        timeout=600,
    )
    if expect_success and completed.returncode != 0:
        raise RuntimeError(f"CB-SEM execution failed for {name}: {execution}")
    if not expect_success:
        return {
            "passed": completed.returncode != 0 and not output.exists(),
            "execution": execution,
            "recipe": repository_path(recipe),
            "recipe_sha256": sha256_file(recipe),
            "fingerprint_execution": fingerprint_execution,
        }

    document = strict_load_json(output)
    estimation = document["payload"]["estimation"]
    cbsem = estimation["cbsem"]
    expected_method = "cfa_ml_v1" if model_type == "cfa" else "cbsem_ml_v1"
    expected_provenance = (
        EXPECTED_PROVENANCE_VERSION
        if model_type == "sem"
        else EXPECTED_PROVENANCE_VERSION.replace("cbsem_ml_v1", "cfa_ml_v1")
    )
    identity_passed = (
        document["payload"]["kind"] == "pls_pm_v1"
        and estimation["method_version"] == expected_method
        and cbsem["method_version"] == expected_method
        and cbsem["model_type"] == model_type
        and cbsem["estimator"] == "ml"
        and cbsem["input"] == "raw"
        and cbsem["mean_structure"] is False
        and document["provenance"]["method"] == "cbsem"
        and document["provenance"]["method_version"] == expected_provenance
        and document["provenance"]["dataset_fingerprint"] == fingerprint
    )
    return {
        "passed": completed.returncode == 0 and identity_passed,
        "cbsem": cbsem,
        "result": document,
        "execution": execution,
        "recipe": repository_path(recipe),
        "recipe_sha256": sha256_file(recipe),
        "output": repository_path(output),
        "output_sha256": sha256_file(output),
        "fingerprint": fingerprint,
        "fingerprint_execution": fingerprint_execution,
        "identity_passed": identity_passed,
    }


def cbsem_analytic_payload(result: dict[str, Any]) -> dict[str, Any]:
    return result["payload"]["estimation"]["cbsem"]
