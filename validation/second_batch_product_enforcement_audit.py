#!/usr/bin/env python3
"""Verify v1.2.1 promoted scopes are enforced in product-facing code."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "validation" / "results" / "second_batch_product_enforcement_audit.json"


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def check(name: str, passed: bool, detail: str) -> dict:
    return {"name": name, "passed": bool(passed), "detail": detail}


def main() -> int:
    sample = text("src/data/sample.ts")
    methods = text("crates/qpls-core/src/methods.rs")
    results = text("src/domain/resultTables.ts")
    engine = text("crates/qpls-estimation/src/pls.rs")
    compat = text("docs/METHOD_COMPATIBILITY.md")
    checks = [
        check("desktop_catalog_promotes_second_batch", all(
            f'id: "{method}"' in sample and 'status: "validated"' in sample.split(f'id: "{method}"', 1)[1].split("}", 1)[0]
            for method in ["plsc", "wpls", "predict", "ipma", "nca"]
        ), "Selectable second-batch methods are validated in the desktop method catalog."),
        check("core_registry_promotes_second_batch", all(
            f'id: "{method}"' in methods and "status: MethodStatus::Validated" in methods.split(f'id: "{method}"', 1)[1].split("}", 1)[0]
            for method in ["pls_mediation", "pls_two_stage_moderation", "plsc", "wpls", "predict", "ipma", "nca"]
        ), "Core method registry promotes second-batch method ids."),
        check("result_tables_promote_second_batch", all(snippet in results for snippet in [
            'id: "plsc_reliability",',
            'id: "wpls_weights",',
            'id: "plspredict_holdout",',
            'id: "cvpat",',
            'id: "nca_ceilings",',
            'id: "ipma_constructs",',
            'status: "validated",',
            'result.method_version === "plspredict_holdout_v1"',
            'result.method_version === "nca_v1"',
        ]), "Report/export tables mark promoted payloads validated and route run provenance through scope detection."),
        check("engine_warnings_promoted_scope", all(phrase in engine for phrase in [
            "PLS mediation effect decomposition is validated for the documented QuickPLS v1.2.1 scope",
            "Two-stage moderation is validated for the documented QuickPLS v1.2.1 single-interaction scope",
            "PLSc is validated for the documented QuickPLS v1.2.1 reflective path/factor-weighting scope",
            "WPLS is validated for the documented QuickPLS v1.2.1 positive case-weighted reflective path/factor-weighting scope",
            "PLSpredict holdout v1 is validated for the documented QuickPLS v1.2.1",
            "IPMA is validated for the documented QuickPLS v1.2.1 supported scope",
            "NCA v1 is validated for the documented QuickPLS v1.2.1 numeric CE-FDH/CR-FDH scope",
        ]), "Newly generated result warnings use bounded validated-scope language."),
        check("later_methods_stay_experimental", all(phrase in compat for phrase in [
            "| PLS-SEM | Higher-order constructs | Experimental",
            "| PLS-SEM | CCA | Experimental",
            "| PLS-SEM | CTA-PLS | Experimental",
            "| PLS-SEM | Moderated mediation | Experimental",
            "| Groups | FIMIX-PLS | Experimental",
            "| CB-SEM | CFA and maximum-likelihood SEM | Experimental",
            "| Components | GSCA | Experimental",
            "| Regression | Logistic and PROCESS | Experimental",
        ]), "Unpromoted methods remain experimental in compatibility docs."),
    ]
    passed = all(item["passed"] for item in checks)
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps({
        "schema_version": 1,
        "target": "v1_2_1_second_batch_method_promotion",
        "passed": passed,
        "checks": checks,
    }, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
