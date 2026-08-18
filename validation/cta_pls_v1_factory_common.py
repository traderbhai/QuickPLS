"""Method-scoped evidence utilities for the bounded CTA-PLS v1 factory.

Every identity report produced here is bound to the frozen CTA-PLS manifest
and to the exact repository bytes exercised by the check. Historical CTA
preview reports are never accepted as current factory evidence.
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
MANIFEST_PATH = VALIDATION / "methods" / "cta_pls_v1.manifest.json"
REPORT_ROOT = VALIDATION / "results" / "method_factory" / "cta_pls_v1"
WORK_ROOT = REPORT_ROOT / "work"
CLI = ROOT / "target" / "debug" / "qpls.exe"
COMMON_SOURCE = "validation/cta_pls_v1_factory_common.py"
FOCUSED_FACTORY_TEST = "validation/test_cta_pls_v1_factory_evidence.py"
_CLI_READY: dict[str, Any] | None = None


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
            raise FileNotFoundError(
                f"required CTA-PLS v1 evidence source is missing: {relative}"
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
        "id": "qpls3.assessment.cta_pls",
        "method_version": "cta_pls_tetrad_v1",
        "catalogue_snapshot_date": "2026-08-12",
    }
    for key, value in expected.items():
        if feature.get(key) != value:
            raise ValueError(
                f"CTA-PLS v1 manifest identity mismatch for {key}: "
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
        "stdout_tail": completed.stdout[-4000:],
        "stderr_tail": completed.stderr[-4000:],
    }
    return completed, record


def ensure_cli() -> dict[str, Any]:
    global _CLI_READY
    if _CLI_READY is not None:
        return _CLI_READY
    completed, execution = run_command(["cargo", "build", "-p", "qpls-cli"], timeout=1200)
    if completed.returncode != 0 or not CLI.is_file():
        raise RuntimeError(f"qpls CLI debug build failed: {execution}")
    _CLI_READY = {
        "passed": True,
        "path": repository_path(CLI),
        "sha256": sha256_file(CLI),
        "built": True,
        "execution": execution,
    }
    return _CLI_READY


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
        raise RuntimeError(f"CTA-PLS fixture import failed for {name}: {execution}")
    inspected, inspect_execution = run_command(
        [str(CLI), "inspect", repository_path(project), "--json"]
    )
    if inspected.returncode != 0:
        raise RuntimeError(f"CTA-PLS fixture inspect failed for {name}: {inspect_execution}")
    payload = json.loads(inspected.stdout)
    return payload["datasets"][0]["fingerprint"], {
        "import": execution,
        "inspect": inspect_execution,
        "project": repository_path(project),
        "project_sha256": sha256_file(project),
    }


def construct(
    construct_id: str,
    indicators: Sequence[str],
    *,
    mode: str = "reflective",
) -> dict[str, Any]:
    return {
        "id": construct_id,
        "name": construct_id.upper(),
        "short_name": construct_id.upper(),
        "mode": mode,
        "indicators": list(indicators),
    }


def run_cta_pls(
    *,
    name: str,
    csv_path: Path,
    constructs: Sequence[dict[str, Any]],
    paths: Sequence[dict[str, str]],
    weighting_scheme: str = "path",
    preprocessing: str = "standardized",
    tolerance: float = 1e-10,
    max_iterations: int = 10_000,
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
        "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"quickpls:cta-pls-v1:{name}")),
        "created_at": "2026-08-13T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"quickpls:cta-pls-v1:model:{name}")),
            "name": "CTA-PLS v1 factory fixture",
            "constructs": list(constructs),
            "paths": list(paths),
            "controls": [],
            "higher_order_constructs": [],
            "interactions": [],
        },
        "settings": {
            "method": "cta_pls",
            "weighting_scheme": weighting_scheme,
            "tolerance": tolerance,
            "max_iterations": max_iterations,
            "bootstrap_samples": 0,
            "studentized_inner_samples": 0,
            "permutation_samples": 0,
            "seed": 20_260_813,
            "workers": 1,
            "confidence_level": 0.95,
            "preprocessing": preprocessing,
            "missing_data": "listwise_deletion",
            "case_weight_column": None,
        },
        "method_config": {"kind": "cta_pls"},
        "metadata": {"status": "validated_cta_pls_v1_bounded_descriptive_scope"},
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
    if not expect_success:
        return {
            "passed": completed.returncode != 0 and not output.exists(),
            "execution": execution,
            "recipe": repository_path(recipe),
            "recipe_sha256": sha256_file(recipe),
            "fingerprint_execution": fingerprint_execution,
        }
    if completed.returncode != 0 or not output.is_file():
        raise RuntimeError(f"CTA-PLS execution failed for {name}: {execution}")
    document = strict_load_json(output)
    estimation = document["payload"]["estimation"]
    cta = estimation["cta_pls"]
    identity_passed = (
        document["payload"]["kind"] == "pls_pm_v1"
        and estimation["method_version"] == "cta_pls_tetrad_v1"
        and cta["method_version"] == "cta_pls_tetrad_v1"
        and cta["covariance"] == "sample_covariance_of_preprocessed_indicators_v1"
        and document["provenance"]["method"] == "cta_pls"
        and str(document["provenance"]["method_version"]).startswith("pls_pm_v1+cta_pls_tetrad_v1")
        and document["provenance"]["dataset_fingerprint"] == fingerprint
        and document["provenance"]["settings"]["weighting_scheme"] == weighting_scheme
        and document["provenance"]["settings"]["preprocessing"] == preprocessing
        and document["provenance"]["settings"]["bootstrap_samples"] == 0
        and document["provenance"]["settings"]["permutation_samples"] == 0
    )
    return {
        "passed": completed.returncode == 0 and identity_passed,
        "result": document,
        "estimation": estimation,
        "cta": cta,
        "execution": execution,
        "recipe": repository_path(recipe),
        "recipe_sha256": sha256_file(recipe),
        "output": repository_path(output),
        "output_sha256": sha256_file(output),
        "fingerprint": fingerprint,
        "fingerprint_execution": fingerprint_execution,
        "identity_passed": identity_passed,
    }


def analytic_payload(run: dict[str, Any]) -> dict[str, Any]:
    """Return deterministic CTA values and complete-case counts only."""

    return {
        "used_observations": run["estimation"]["used_observations"],
        "omitted_observations": run["estimation"]["omitted_observations"],
        "cta_pls": run["cta"],
    }
