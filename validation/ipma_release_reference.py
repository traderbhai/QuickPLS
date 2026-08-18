#!/usr/bin/env python3
"""Current-schema adapter for the independent IPMA v1 reference."""

from __future__ import annotations

import subprocess
from pathlib import Path

import ipma_reference as reference


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "validation/results/method_factory/ipma_v1"
CLI = ROOT / "target/debug/qpls.exe"


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    reference.RESULTS = OUTPUT
    reference.DATA = OUTPUT / "ipma_reference.csv"
    reference.RECIPE = OUTPUT / "ipma_reference_legacy.recipe.json"
    reference.QUICKPLS = OUTPUT / "ipma_reference.quickpls.json"
    reference.OUTPUT = OUTPUT / "independent_reference.json"
    original_write = reference.write_recipe

    def write_current_schema(fingerprint: str) -> None:
        original_write(fingerprint)
        legacy = reference.RECIPE
        current = OUTPUT / "ipma_reference_current.recipe.json"
        current.unlink(missing_ok=True)
        subprocess.run(
            [str(CLI), "migrate-recipe", str(legacy), str(current)],
            cwd=ROOT,
            check=True,
            stdout=subprocess.DEVNULL,
        )
        reference.RECIPE = current

    reference.write_recipe = write_current_schema
    reference.main()


if __name__ == "__main__":
    main()
