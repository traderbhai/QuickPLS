#!/usr/bin/env python3
"""Current-schema adapter for the independent GSCA ALS v2 reference.

The published independent optimizer remains in ``gsca_als_v2_reference``.
This adapter migrates its immutable historical recipe to a distinct schema-v3
work file so current execution never relies on permissive legacy migration.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

import gsca_als_v2_reference as reference


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "validation/results/method_factory/gsca_als_v2"
CLI = ROOT / "target/debug/qpls.exe"


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    migrated = OUTPUT / "gsca_als_v2_current.recipe.json"
    result = OUTPUT / "gsca_als_v2_current.quickpls.json"
    report = OUTPUT / "independent_reference.json"
    migrated.unlink(missing_ok=True)
    result.unlink(missing_ok=True)
    report.unlink(missing_ok=True)
    subprocess.run(
        [str(CLI), "migrate-recipe", str(reference.RECIPE), str(migrated)],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    reference.RECIPE = migrated
    reference.QUICKPLS = result
    reference.REPORT = report
    run_current = reference.run_quickpls

    def run_without_legacy_empty_sections():
        payload = run_current()
        estimation = payload["payload"]["estimation"]
        # The current standalone GSCA envelope omits irrelevant PLS mediation
        # instead of serializing the historical empty section.  Normalize only
        # that semantically equivalent absence for the legacy oracle check.
        estimation.setdefault("mediation", {"estimates": []})
        return payload

    reference.run_quickpls = run_without_legacy_empty_sections
    reference.main()


if __name__ == "__main__":
    main()
