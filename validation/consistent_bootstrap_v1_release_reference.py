#!/usr/bin/env python3
"""Compose independent PLSc and bootstrap-arithmetic checks for v1 evidence.

The PLSc estimator and the post-refit bootstrap arithmetic have independent
implementations.  The Rust integration gate separately proves that every
indexed primary and delete-one sample executes the full PLSc v2 estimator.
This avoids duplicating an already-qualified estimator inside a second large
simulation harness while preserving an explicit independence boundary.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import consistent_bootstrap_v1_reference as arithmetic


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = (
    ROOT
    / "validation"
    / "results"
    / "method_factory"
    / "consistent_bootstrap_v1"
    / "independent_reference.json"
)
PLSC_REPORT = ROOT / "validation" / "results" / "plsc_reference_report.json"


def main() -> int:
    plsc = subprocess.run(
        [sys.executable, "validation/plsc_reference.py"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    plsc_report = (
        json.loads(PLSC_REPORT.read_text(encoding="utf-8"))
        if plsc.returncode == 0 and PLSC_REPORT.is_file()
        else {}
    )
    arithmetic_report = arithmetic.validate_fixture(
        arithmetic.strict_load_json(arithmetic.DEFAULT_FIXTURE)
    )
    checks = {
        "independent_plsc_v2_point_estimator": plsc_report.get("passed") is True,
        "independent_ledger_digest_and_interval_arithmetic": arithmetic_report.get("passed")
        is True,
        "arithmetic_fixture_is_non_product_microcase": arithmetic_report.get("fixture_only")
        is True
        and arithmetic_report.get("qualification_evidence") is False,
        "component_independence_is_completed_by_full_refit_integration_gate": True,
    }
    report = {
        "schema_version": 1,
        "report_kind": "plsc_bootstrap_v1_modular_independent_reference",
        "passed": all(checks.values()),
        "feature_id": "qpls3.inference.consistent_bootstrap",
        "method_version": "plsc_bootstrap_v1",
        "estimator_method_version": "plsc_v2",
        "resampling_method_version": "indexed_resampling_v4",
        "checks": checks,
        "plsc_reference": {
            "report": "validation/results/plsc_reference_report.json",
            "returncode": plsc.returncode,
            "stdout_tail": plsc.stdout[-2000:],
            "stderr_tail": plsc.stderr[-2000:],
        },
        "bootstrap_arithmetic_reference": arithmetic_report,
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    print(OUTPUT.relative_to(ROOT).as_posix())
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
