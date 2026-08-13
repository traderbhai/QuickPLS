#!/usr/bin/env python3
"""Preserve one focused workflow from the cumulative v247 native report.

Focused Tauri runs historically accumulated method checks in one shared JSON,
so a later run could replace only the top-level focused-run marker. This helper
creates an explicit, immutable method slice from those already-recorded checks;
it does not fabricate or rerun native evidence.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
SOURCE = RESULTS / "v247_tauri_native_acceptance.json"

PROFILES = {
    "prediction": {
        "checks": [
            "fixtureProvisioning",
            "predictionFixture",
            "predictionInitialModel",
            "predictionModel",
            "predictionV2Dialog",
            "predictionV2Progress",
            "predictionV2Result",
            "predictionV2Export",
            "predictionV2SaveReopen",
        ],
        "screenshot": re.compile(r"\\9[0-7]-tauri-native-prediction-", re.IGNORECASE),
    },
}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("scope", choices=sorted(PROFILES))
    args = parser.parse_args()

    source = json.loads(SOURCE.read_text(encoding="utf-8"))
    profile = PROFILES[args.scope]
    source_checks = source.get("checks", {})
    missing = [name for name in profile["checks"] if name not in source_checks]
    if missing:
        raise SystemExit(f"Cannot preserve {args.scope}: missing checks {missing}")

    checks = {name: source_checks[name] for name in profile["checks"]}
    screenshots = [
        value for value in source.get("screenshots", [])
        if isinstance(value, str) and profile["screenshot"].search(value)
    ]
    if not screenshots:
        raise SystemExit(f"Cannot preserve {args.scope}: no matching screenshots")

    evidence = {
        "schemaVersion": 1,
        "kind": "v247_scoped_tauri_native_acceptance",
        "passed": source.get("passed") is True
        and not source.get("consoleErrors")
        and not source.get("failures"),
        "runtime": source.get("runtime"),
        "endpoint": source.get("endpoint"),
        "focusedRun": {
            "scope": args.scope,
            "evidenceOrigin": "preserved_from_cumulative_v247_report",
            "sourceGeneratedAt": source.get("generatedAt"),
            "sourceFocusedScope": source.get("focusedRun", {}).get("scope"),
        },
        "checks": checks,
        "screenshots": screenshots,
        "consoleErrors": source.get("consoleErrors", []),
        "failures": source.get("failures", []),
        "sourceReport": str(SOURCE.relative_to(ROOT)).replace("\\", "/"),
        "note": "This is a lossless method-specific slice of genuine checks accumulated by the shared v247 packaged-Tauri report before scoped-report preservation was added.",
    }
    output = RESULTS / f"v247_tauri_native_acceptance_{args.scope}.json"
    output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {output} | passed={evidence['passed']} | checks={len(checks)} | screenshots={len(screenshots)}")
    return 0 if evidence["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
