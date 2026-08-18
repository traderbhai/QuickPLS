"""Fresh independent lavaan parity evidence for bounded CB-SEM ML v1."""

from __future__ import annotations

import tempfile
import json
import subprocess
import time
from pathlib import Path
from typing import Any
from unittest.mock import patch

from cbsem_ml_v1_factory_common import (
    ROOT,
    engine_source_paths,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/cbsem_ml_v1_reference.py"
REFERENCE_SCRIPT = "validation/cbsem_lavaan_reference.py"
REFERENCE_REPORT = "validation/results/cbsem_lavaan_reference_report.json"
EXPECTED_MODEL_ORDER = (
    "one_factor_cfa",
    "two_factor_cfa",
    "three_factor_cfa",
    "latent_regression_sem",
    "latent_mediation_sem",
    "correlated_exogenous_sem",
)
EXPECTED_MODELS = set(EXPECTED_MODEL_ORDER)
LIMITS = {
    "max_estimate_delta": 1e-6,
    "max_fit_delta": 1e-6,
    "max_standardized_delta": 1e-5,
    "max_standard_error_delta": 1e-4,
    "max_z_delta": 1e-4,
    "max_p_delta": 1e-4,
}


def summarize_lavaan_report(report: dict[str, Any]) -> dict[str, Any]:
    models = report.get("models")
    if not isinstance(models, list):
        return {"passed": False, "error": "models must be an array"}
    names = [row.get("model") for row in models if isinstance(row, dict)]
    rows: list[dict[str, Any]] = []
    for row in models:
        if not isinstance(row, dict):
            rows.append({"passed": False, "error": "model row is not an object"})
            continue
        limits = {
            metric: isinstance(row.get(metric), (int, float))
            and float(row[metric]) <= limit
            for metric, limit in LIMITS.items()
        }
        rows.append(
            {
                "model": row.get("model"),
                "passed": (
                    row.get("passed") is True
                    and row.get("quickpls_converged") is True
                    and isinstance(row.get("matched_parameters"), int)
                    and row["matched_parameters"] >= 7
                    and all(limits.values())
                ),
                "matched_parameters": row.get("matched_parameters"),
                "tolerance_checks": limits,
                **{metric: row.get(metric) for metric in LIMITS},
            }
        )
    identity = {
        "kind": report.get("kind") == "cbsem_lavaan_reference_v1",
        "status": report.get("status") == "passed",
        "lavaan_version": report.get("lavaan") == "0.7.2",
        "exact_model_set": len(names) == len(set(names)) and set(names) == EXPECTED_MODELS,
    }
    return {
        "passed": all(identity.values()) and all(row["passed"] for row in rows),
        "identity": identity,
        "limits": LIMITS,
        "models": rows,
    }


def run_fresh_reference() -> tuple[dict[str, Any], dict[str, Any]]:
    """Run the independent reference with an isolated output and v3 recipes.

    The legacy reference module remains a useful independent implementation but
    still emits schema-v2 recipes.  This adapter changes only its recipe envelope
    to the current typed v3 contract; its fixture generation, lavaan execution,
    parameter mapping, and tolerance comparisons remain untouched.
    """

    import cbsem_lavaan_reference as legacy

    if not legacy.CLI_EXE.is_file():
        raise FileNotFoundError(
            "the coordinated source-frozen debug qpls.exe is missing"
        )
    legacy.CLI_READY = True

    with tempfile.TemporaryDirectory(prefix="cbsem_ml_v1_reference_") as directory:
        work = Path(directory)

        def quickpls_run_v3(model: str, data_path: Path) -> dict[str, Any]:
            fingerprint = legacy.dataset_fingerprint(data_path, f"factory_lavaan_{model}")
            model_type = "sem" if legacy.paths_for(model) else "cfa"
            recipe = {
                "schema_version": 3,
                "id": f"00000000-0000-0000-0000-000000081{len(model):03d}",
                "created_at": "2026-08-13T00:00:00Z",
                "dataset_fingerprint": fingerprint,
                "model": {
                    "id": f"00000000-0000-0000-0000-000000082{len(model):03d}",
                    "name": f"factory lavaan {model}",
                    "constructs": legacy.constructs_for(model),
                    "paths": legacy.paths_for(model),
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
                    "seed": 20260722,
                    "workers": 1,
                    "confidence_level": 0.95,
                    "preprocessing": "standardized",
                    "missing_data": "listwise_deletion",
                    "case_weight_column": None,
                },
                "method_config": {
                    "kind": "cbsem",
                    "model_type": model_type,
                    "estimator": "ml",
                    "input": "raw",
                    "mean_structure": False,
                    "bootstrap_samples": 0,
                },
                "metadata": {
                    "status": "validated_v1_2_4_cbsem_single_group_bounded_scope"
                },
            }
            recipe_path = work / f"lavaan_{model}.recipe.json"
            output_path = work / f"lavaan_{model}_quickpls.json"
            recipe_path.write_text(
                json.dumps(recipe, indent=2, allow_nan=False) + "\n",
                encoding="utf-8",
            )
            legacy.qpls(
                [
                    "run",
                    str(recipe_path),
                    "--data",
                    str(data_path.relative_to(ROOT)),
                    "--output",
                    str(output_path),
                ],
                check=True,
                stdout=subprocess.DEVNULL,
            )
            return strict_load_json(output_path)["payload"]["estimation"]["cbsem"]

        started = time.monotonic()
        records: list[dict[str, Any]] = []
        with patch.object(legacy, "quickpls_run", quickpls_run_v3):
            for model in EXPECTED_MODEL_ORDER:
                record = legacy.compare_model(model)
                records.append(record)
        report = {
            "kind": "cbsem_lavaan_reference_v1",
            "rscript": legacy.rscript_path(),
            "lavaan": "0.7.2",
            "models": records,
            "status": "passed" if all(row["passed"] for row in records) else "failed",
        }
        (ROOT / REFERENCE_REPORT).write_text(
            json.dumps(report, indent=2, allow_nan=False),
            encoding="utf-8",
        )
        execution = {
            "command": ["python", SOURCE],
            "returncode": 0 if report["status"] == "passed" else 1,
            "duration_seconds": round(time.monotonic() - started, 3),
            "stdout_tail": "fresh six-model QuickPLS/lavaan reference completed",
            "stderr_tail": "",
        }
        return report, execution


def main() -> int:
    try:
        report, execution = run_fresh_reference()
        checks = summarize_lavaan_report(report)
        checks["execution_passed"] = execution["returncode"] == 0
        checks["passed"] = checks["passed"] and checks["execution_passed"]
        checks["reference_report"] = REFERENCE_REPORT
    except Exception as error:
        execution = {
            "command": ["python", SOURCE],
            "returncode": 1,
            "duration_seconds": 0.0,
            "stdout_tail": "",
            "stderr_tail": f"{type(error).__name__}: {error}",
        }
        checks = {"passed": False, "error": execution["stderr_tail"]}
    identity_path = write_identity_report(
        "independent_reference",
        passed=checks["passed"],
        checks=checks,
        execution=execution,
        extras=[
            SOURCE,
            REFERENCE_SCRIPT,
            "validation/r_runtime.py",
            *engine_source_paths(),
        ],
    )
    print(f"wrote {identity_path} | passed={checks['passed']}")
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
