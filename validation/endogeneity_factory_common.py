"""Method-scoped utilities for Gaussian-copula endogeneity qualification.

The helpers are intentionally bounded to ``gaussian_copula_endogeneity_v1``.
Checks can run without writing promotion identities; identity creation is an
explicit operation so a checkout cannot be promoted accidentally.
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
MANIFEST_PATH = VALIDATION / "methods" / "gaussian_copula_endogeneity_v1.manifest.json"
REPORT_ROOT = VALIDATION / "results" / "method_factory" / "gaussian_copula_endogeneity_v1"
WORK_ROOT = REPORT_ROOT / "work"
CLI = ROOT / "target" / "debug" / "qpls.exe"
COMMON_SOURCE = "validation/endogeneity_factory_common.py"
FOCUSED_FACTORY_TEST = "validation/test_endogeneity_factory.py"
METHOD_VERSION = "gaussian_copula_endogeneity_v1"
PROVENANCE_VERSION = (
    "pls_pm_v1+gaussian_copula_endogeneity_v1+"
    "pls_mediation_v1+pls_assessment_v7"
)
VARIABLES = ("x1", "x2", "z1", "z2", "y1", "y2")
_CLI_READY = False


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
    """Bind evidence to the exact bytes present in this checkout."""

    descriptors: list[dict[str, Any]] = []
    for relative in sorted(set(paths)):
        candidate = Path(relative)
        if "\\" in relative or candidate.is_absolute() or ".." in candidate.parts:
            raise ValueError(f"unsafe repository source path: {relative!r}")
        path = ROOT / candidate
        if not path.is_file():
            raise FileNotFoundError(
                f"required endogeneity evidence source is missing: {relative}"
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
        "id": "qpls3.pls.gaussian_copula_endogeneity",
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": "2026-08-12",
    }
    for key, value in expected.items():
        if feature.get(key) != value:
            raise ValueError(
                f"endogeneity manifest identity mismatch for {key}: "
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


def identity_report_document(
    role: str,
    *,
    passed: bool,
    checks: dict[str, Any],
    extras: Iterable[str] = (),
    execution: dict[str, Any] | None = None,
) -> dict[str, Any]:
    document = manifest()
    feature = document["feature"]
    report: dict[str, Any] = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": role,
        "passed": bool(passed),
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace(
            "+00:00", "Z"
        ),
        "source_artifacts": source_descriptors(role_sources(role, extras)),
        "checks": checks,
    }
    if execution is not None:
        report["execution"] = execution
    return report


def write_identity_report(
    role: str,
    *,
    passed: bool,
    checks: dict[str, Any],
    extras: Iterable[str] = (),
    execution: dict[str, Any] | None = None,
) -> Path:
    report = identity_report_document(
        role,
        passed=passed,
        checks=checks,
        extras=extras,
        execution=execution,
    )
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    path = REPORT_ROOT / f"{role}.identity.json"
    path.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return path


def optionally_write_identity_report(
    role: str,
    *,
    write_identity: bool,
    passed: bool,
    checks: dict[str, Any],
    extras: Iterable[str] = (),
    execution: dict[str, Any] | None = None,
) -> Path | None:
    if not write_identity:
        return None
    return write_identity_report(
        role,
        passed=passed,
        checks=checks,
        extras=extras,
        execution=execution,
    )


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
    execution = {
        "command": list(command),
        "returncode": completed.returncode,
        "duration_seconds": round(time.monotonic() - started, 3),
        "stdout_tail": completed.stdout[-4000:],
        "stderr_tail": completed.stderr[-4000:],
    }
    return completed, execution


def ensure_cli() -> dict[str, Any]:
    global _CLI_READY
    if not _CLI_READY:
        completed, execution = run_command(
            ["cargo", "build", "-p", "qpls-cli"], timeout=900
        )
        if completed.returncode != 0 or not CLI.is_file():
            raise RuntimeError(f"qpls CLI build failed: {execution}")
        _CLI_READY = True
        return {
            "passed": True,
            "path": repository_path(CLI),
            "sha256": sha256_file(CLI),
            "built": True,
            "execution": execution,
        }
    return {
        "passed": CLI.is_file(),
        "path": repository_path(CLI),
        "sha256": sha256_file(CLI),
        "built": False,
    }


def write_rows(path: Path, rows: Sequence[dict[str, float]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(VARIABLES))
        writer.writeheader()
        for row in rows:
            writer.writerow({key: f"{float(row[key]):.12f}" for key in VARIABLES})


def _fingerprint(csv_path: Path, name: str) -> tuple[str, dict[str, Any]]:
    project = WORK_ROOT / f"{name}.fingerprint.qpls"
    project.unlink(missing_ok=True)
    imported, import_execution = run_command(
        [
            str(CLI),
            "import",
            repository_path(csv_path),
            repository_path(project),
            "--name",
            name,
        ]
    )
    if imported.returncode != 0:
        raise RuntimeError(f"endogeneity fixture import failed: {import_execution}")
    inspected, inspect_execution = run_command(
        [str(CLI), "inspect", repository_path(project), "--json"]
    )
    if inspected.returncode != 0:
        raise RuntimeError(f"endogeneity fixture inspect failed: {inspect_execution}")
    document = json.loads(inspected.stdout)
    return document["datasets"][0]["fingerprint"], {
        "import": import_execution,
        "inspect": inspect_execution,
        "project": repository_path(project),
        "project_sha256": sha256_file(project),
    }


def default_model(name: str) -> dict[str, Any]:
    return {
        "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"quickpls:endogeneity:model:{name}")),
        "name": "Gaussian-copula endogeneity factory fixture",
        "constructs": [
            {
                "id": "x",
                "name": "X",
                "short_name": "X",
                "mode": "reflective",
                "indicators": ["x1", "x2"],
            },
            {
                "id": "z",
                "name": "Z",
                "short_name": "Z",
                "mode": "reflective",
                "indicators": ["z1", "z2"],
            },
            {
                "id": "y",
                "name": "Y",
                "short_name": "Y",
                "mode": "reflective",
                "indicators": ["y1", "y2"],
            },
        ],
        "paths": [{"source": "x", "target": "y"}, {"source": "z", "target": "y"}],
        "controls": [],
        "higher_order_constructs": [],
        "interactions": [],
    }


def recipe_payload(
    name: str,
    fingerprint: str,
    *,
    weighting_scheme: str = "path",
    settings_overrides: dict[str, Any] | None = None,
    model_overrides: dict[str, Any] | None = None,
) -> dict[str, Any]:
    model = default_model(name)
    model.update(model_overrides or {})
    settings: dict[str, Any] = {
        "method": "endogeneity",
        "weighting_scheme": weighting_scheme,
        "tolerance": 1e-7,
        "max_iterations": 3000,
        "bootstrap_samples": 0,
        "studentized_inner_samples": 0,
        "permutation_samples": 0,
        "seed": 20260814,
        "workers": 1,
        "confidence_level": 0.95,
        "preprocessing": "standardized",
        "missing_data": "listwise_deletion",
        "case_weight_column": None,
    }
    settings.update(settings_overrides or {})
    return {
        "schema_version": 3,
        "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"quickpls:endogeneity:{name}")),
        "created_at": "2026-08-14T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": model,
        "settings": settings,
        "method_config": {"kind": "endogeneity"},
        "metadata": {"status": "validated_endogeneity_bounded_scope"},
    }


def run_endogeneity(
    *,
    name: str,
    rows: Sequence[dict[str, float]],
    weighting_scheme: str = "path",
    settings_overrides: dict[str, Any] | None = None,
    model_overrides: dict[str, Any] | None = None,
    expect_success: bool = True,
) -> dict[str, Any]:
    ensure_cli()
    WORK_ROOT.mkdir(parents=True, exist_ok=True)
    csv_path = WORK_ROOT / f"{name}.csv"
    write_rows(csv_path, rows)
    fingerprint, fingerprint_execution = _fingerprint(csv_path, name)
    recipe_path = WORK_ROOT / f"{name}.recipe.json"
    output_path = WORK_ROOT / f"{name}.quickpls.json"
    output_path.unlink(missing_ok=True)
    recipe = recipe_payload(
        name,
        fingerprint,
        weighting_scheme=weighting_scheme,
        settings_overrides=settings_overrides,
        model_overrides=model_overrides,
    )
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
        ]
    )
    if not expect_success:
        return {
            "passed": completed.returncode != 0 and not output_path.exists(),
            "execution": execution,
            "recipe": repository_path(recipe_path),
            "fingerprint": fingerprint,
        }
    if completed.returncode != 0:
        raise RuntimeError(f"endogeneity execution failed for {name}: {execution}")
    document = strict_load_json(output_path)
    estimation = document["payload"]["estimation"]
    analysis = estimation["endogeneity"]
    identity_passed = (
        document["payload"]["kind"] == "pls_pm_v1"
        and estimation["method_version"] == METHOD_VERSION
        and analysis["method_version"] == METHOD_VERSION
        and analysis["transform"] == "rankit_inverse_normal_v1"
        and document["provenance"]["method"] == "endogeneity"
        and document["provenance"]["method_version"] == PROVENANCE_VERSION
        and document["provenance"]["dataset_fingerprint"] == fingerprint
    )
    return {
        "passed": identity_passed,
        "identity_passed": identity_passed,
        "analysis": analysis,
        "result": document,
        "recipe_document": recipe,
        "execution": execution,
        "recipe": repository_path(recipe_path),
        "recipe_sha256": sha256_file(recipe_path),
        "output": repository_path(output_path),
        "output_sha256": sha256_file(output_path),
        "fingerprint": fingerprint,
        "fingerprint_execution": fingerprint_execution,
    }


def analytical_payload(result: dict[str, Any]) -> dict[str, Any]:
    return result["result"]["payload"]["estimation"]["endogeneity"]
