#!/usr/bin/env python3
"""Current-schema adapter for the independent CCA residual reference."""

from __future__ import annotations

import subprocess
from pathlib import Path

import cca_reference as reference


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "validation/results/method_factory/cca_residuals_v1"
CLI = ROOT / "target/debug/qpls.exe"


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    reference.RESULTS = OUTPUT
    reference.DATA = OUTPUT / "cca_reference.csv"
    reference.RECIPE = OUTPUT / "cca_reference_legacy.recipe.json"
    reference.QUICKPLS = OUTPUT / "cca_reference.quickpls.json"
    reference.OUTPUT = OUTPUT / "independent_reference.json"
    reference.GUARD_RECIPE = OUTPUT / "cca_invalid.recipe.json"
    original_run = reference.run_quickpls

    def run_current_schema():
        current = OUTPUT / "cca_reference_current.recipe.json"
        current.unlink(missing_ok=True)
        subprocess.run(
            [str(CLI), "migrate-recipe", str(reference.RECIPE), str(current)],
            cwd=ROOT,
            check=True,
            stdout=subprocess.DEVNULL,
        )
        reference.RECIPE = current
        return original_run()

    reference.run_quickpls = run_current_schema
    reference.main()


if __name__ == "__main__":
    main()
