#!/usr/bin/env python3
"""Method-isolated independent NCA v2 reference runner."""

from __future__ import annotations

import json
from pathlib import Path

import v08_extended_methods_reference as reference


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "validation/results/method_factory/nca_v2"


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    reference.RESULTS = OUTPUT
    reference.DATA = OUTPUT / "nca_reference.csv"
    reference.METHOD_OUTPUT_NAMES["nca"] = "independent_reference.json"

    def run_current_nca(name: str, method: str, metadata: dict, model=None):
        if method != "nca":
            raise ValueError("the isolated NCA runner only accepts method='nca'")
        fingerprint = reference.dataset_fingerprint(name)
        recipe = OUTPUT / f"{name}.recipe.json"
        result = OUTPUT / f"{name}_quickpls.json"
        payload = {
            "schema_version": 3,
            "id": f"00000000-0000-0000-0000-00000008{sum(ord(ch) for ch in name) % 10000:04d}",
            "created_at": "2026-07-19T00:00:00Z",
            "dataset_fingerprint": fingerprint,
            "model": model or reference.empty_model(),
            "settings": reference.base_settings(method),
            "method_config": {
                "kind": "nca",
                "condition": metadata["nca_x"],
                "outcome": metadata["nca_y"],
                "ceiling": metadata.get("nca_ceiling", "both"),
                "permutation_samples": int(metadata.get("nca_permutation_samples", "999")),
            },
            "metadata": {"fixture": "nca_v2_release_reference"},
        }
        recipe.write_text(json.dumps(payload, indent=2), encoding="utf-8")
        reference.qpls(
            [
                "run", str(recipe.relative_to(ROOT)), "--data", str(reference.DATA.relative_to(ROOT)),
                "--output", str(result.relative_to(ROOT)),
                "--allow-internal-qualification",
            ],
            check=True,
            stdout=reference.subprocess.DEVNULL,
        )
        return json.loads(result.read_text(encoding="utf-8"))["payload"]["estimation"]

    reference.run_recipe = run_current_nca
    reference.write_dataset()
    checks = {"nca": reference.check_nca()}
    report = reference.build_report("nca", checks)
    reference.write_scoped_reports("nca", checks, results=OUTPUT)
    print(json.dumps(report, indent=2, sort_keys=True))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
