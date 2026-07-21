#!/usr/bin/env python3
"""Verify v1.2.3 extended PLS diagnostic scopes are enforced in product-facing code."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "validation" / "results" / "fourth_batch_product_enforcement_audit.json"


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def check(name: str, passed: bool, detail: str) -> dict:
    return {"name": name, "passed": bool(passed), "detail": detail}


def main() -> int:
    sample = text("src/data/sample.ts")
    methods = text("crates/qpls-core/src/methods.rs")
    tables = text("src/domain/resultTables.ts")
    engine = text("crates/qpls-estimation/src/pls.rs")
    compat = text("docs/METHOD_COMPATIBILITY.md")
    checks = [
        check("catalog_promotes_extended_pls_diagnostics", all(
            f'id: "{method}"' in sample and 'status: "validated"' in sample.split(f'id: "{method}"', 1)[1].split("}", 1)[0]
            for method in ["cca", "cta_pls", "endogeneity", "nonlinear_effects", "moderated_mediation"]
        ), "Desktop catalog marks promoted extended PLS diagnostics as validated entry points."),
        check("core_registry_promotes_extended_pls_diagnostics", all(
            f'id: "{method}"' in methods and "status: MethodStatus::Validated" in methods.split(f'id: "{method}"', 1)[1].split("}", 1)[0]
            for method in ["cca", "cta_pls", "endogeneity", "nonlinear_effects", "moderated_mediation"]
        ), "Core method registry marks promoted extended PLS diagnostics as validated."),
        check("result_tables_promote_extended_pls_payloads", all(snippet in tables for snippet in [
            'id: "cca_residuals"',
            'id: "cta_pls_summary"',
            'id: "endogeneity_copula"',
            'id: "nonlinear_effects"',
            'id: "moderated_mediation"',
            'result.cca ||',
            'result.cta_pls ||',
            'result.endogeneity ||',
            'result.nonlinear_effects ||',
            'result.moderated_mediation ||',
        ]) is False and all(snippet in tables for snippet in [
            'id: "cca_residuals"',
            'status: "validated"',
            'id: "cta_pls_summary"',
            'id: "endogeneity_copula"',
            'id: "nonlinear_effects"',
            'id: "moderated_mediation"',
        ]), "Result/export tables expose the promoted diagnostic payloads as validated and no longer force the run envelope experimental for them."),
        check("engine_warnings_use_bounded_validated_language", all(phrase in engine for phrase in [
            "Gaussian-copula endogeneity diagnostics are validated for the documented QuickPLS v1.2.3 diagnostic scope",
            "Nonlinear effects are validated for the documented QuickPLS v1.2.3 fixed-score quadratic diagnostic scope",
            "CTA-PLS tetrad diagnostics are validated for the documented QuickPLS v1.2.3 descriptive tetrad scope",
            "CCA is validated for the documented QuickPLS v1.2.3 descriptive composite residual scope",
            "Moderated mediation is validated for the documented QuickPLS v1.2.3 two-stage conditional indirect-effect diagnostic scope",
            "higher-order constructs are validated for the documented QuickPLS v1.2.3 bounded repeated-indicator, two-stage, and hybrid scopes",
        ]), "Newly generated warnings use bounded v1.2.3 validated-scope language."),
        check("compatibility_docs_match_fourth_batch", all(phrase in compat for phrase in [
            "| PLS-SEM | Higher-order constructs | Validated",
            "| PLS-SEM | CCA | Validated",
            "| PLS-SEM | CTA-PLS | Validated",
            "| PLS-SEM | Endogeneity analysis | Validated",
            "| PLS-SEM | Nonlinear effects | Validated",
            "| PLS-SEM | Moderated mediation | Validated",
        ]), "Compatibility matrix marks only the documented bounded scopes as validated."),
    ]
    passed = all(item["passed"] for item in checks)
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps({
        "schema_version": 1,
        "target": "v1_2_3_extended_pls_diagnostics_promotion",
        "passed": passed,
        "checks": checks,
    }, indent=2) + "\n", encoding="utf-8")
    if not passed:
        for item in checks:
            if not item["passed"]:
                print(f"FAIL {item['name']}: {item['detail']}")
        return 1
    print(f"wrote {OUTPUT} | passed=True")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
