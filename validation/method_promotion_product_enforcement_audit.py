#!/usr/bin/env python3
"""Audit product surfaces for v1.2 method-promotion scope enforcement."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULT = ROOT / "validation" / "results" / "method_promotion_product_enforcement_audit.json"


def contains(path: str, needle: str) -> bool:
    return needle in (ROOT / path).read_text(encoding="utf-8")


def check(name: str, passed: bool, detail: str) -> dict:
    return {"name": name, "passed": bool(passed), "detail": detail}


def main() -> int:
    checks = [
        check(
            "catalog_promotes_pca",
            contains("src/data/sample.ts", '{ id: "pca", family: "Components", name: "Principal component analysis", status: "validated" }'),
            "Desktop method catalog marks standalone PCA as validated.",
        ),
        check(
            "catalog_promotes_pls_inference",
            contains("src/data/sample.ts", '{ id: "bootstrap", family: "PLS-SEM", name: "Bootstrapping", status: "validated" }')
            and contains("src/data/sample.ts", '{ id: "permutation", family: "PLS-SEM", name: "Freedman-Lane permutation", status: "validated" }'),
            "Bootstrap and Freedman-Lane permutation entries are validated for documented PLS inference scope.",
        ),
        check(
            "regression_status_is_setting_aware",
            contains("src/domain/methodStatus.ts", 'method.id === "regression"')
            and contains("src/domain/methodStatus.ts", 'regressionType === "ols" || regressionType === "logistic"')
            and contains("src/domain/methodStatus.ts", '(settings?.processModel ?? "mediation") !== "moderated_mediation"'),
            "Regression status is setting-aware: OLS/logistic/bounded PROCESS are validated while PROCESS moderated mediation remains experimental.",
        ),
        check(
            "analysis_readiness_uses_effective_status",
            contains("src/domain/analysisReadiness.ts", "effectiveMethodStatus(method, settings)"),
            "Run readiness uses setting-aware method status.",
        ),
        check(
            "topbar_uses_effective_status",
            contains("src/components/TopBar.tsx", "effectiveMethodStatus(selectedMethod, analysisSettings)"),
            "Top bar method badge uses setting-aware method status.",
        ),
        check(
            "result_tables_promote_validated_batches_only",
            contains("src/domain/resultTables.ts", 'result.method_version.startsWith("pls_pm_v1")')
            and contains("src/domain/resultTables.ts", 'result.method_version === "pca_v1"')
            and contains("src/domain/resultTables.ts", 'result.method_version === "plsc_v1"')
            and contains("src/domain/resultTables.ts", 'result.method_version === "wpls_case_weighted_v1"')
            and contains("src/domain/resultTables.ts", 'result.method_version === "plspredict_holdout_v1"')
            and contains("src/domain/resultTables.ts", 'result.method_version === "ipma_v1"')
            and contains("src/domain/resultTables.ts", 'result.method_version === "nca_v1"')
            and contains("src/domain/resultTables.ts", 'result.method_version === "regression_logistic_v1"')
            and contains("src/domain/resultTables.ts", 'result.method_version === "regression_process_v1"')
            and contains("src/domain/resultTables.ts", 'regression.regression_type === "ols" || regression.regression_type === "logistic"')
            and contains("src/domain/resultTables.ts", 'regression.process?.model !== "moderated_mediation"'),
            "Export table status promotes first-, second-, and eligible third-batch validated scopes while leaving PROCESS moderated mediation experimental.",
        ),
        check(
            "engine_warnings_do_not_overclaim_regression_nca",
            contains("crates/qpls-estimation/src/pls.rs", "Logistic regression v1 is validated for the documented QuickPLS v1.2.2 binary numeric complete-case scope")
            and contains("crates/qpls-estimation/src/pls.rs", "PROCESS-style regression v1 is validated for the documented QuickPLS v1.2.2 bounded mediation/moderation workflow scope")
            and contains("crates/qpls-estimation/src/pls.rs", "NCA v1 is validated for the documented QuickPLS v1.2.1 numeric CE-FDH/CR-FDH scope"),
            "Newly generated run warnings promote logistic and bounded PROCESS only for documented scopes while treating NCA as validated only for the documented v1.2.1 numeric scope.",
        ),
        check(
            "core_validation_does_not_warn_pca_experimental",
            not contains("crates/qpls-core/src/validation.rs", "pca.experimental")
            and contains("crates/qpls-core/src/methods.rs", 'id: "pca"') 
            and contains("crates/qpls-core/src/methods.rs", "status: MethodStatus::Validated"),
            "Core registry treats PCA as validated and no longer emits a PCA experimental warning.",
        ),
        check(
            "compatibility_docs_match_promoted_scope",
            contains("docs/METHOD_COMPATIBILITY.md", "| Components | Standalone PCA | Validated")
            and contains("docs/METHOD_COMPATIBILITY.md", "| Regression | OLS regression | Validated")
            and contains("docs/METHOD_COMPATIBILITY.md", "| Regression | Logistic regression | Validated")
            and contains("docs/METHOD_COMPATIBILITY.md", "| Regression | PROCESS-style workflows | Validated"),
            "Compatibility matrix separates validated PCA/OLS from later-batch regression scopes.",
        ),
    ]
    passed = all(item["passed"] for item in checks)
    RESULT.parent.mkdir(parents=True, exist_ok=True)
    RESULT.write_text(
        json.dumps(
            {
                "audit": "method_promotion_product_enforcement",
                "target": "v1_2_method_promotion_program",
                "passed": passed,
                "checks": checks,
                "note": "Validated product surfaces are constrained to the first, second, and eligible third promoted method batches; unpromoted methods remain experimental or watermarked.",
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    if not passed:
        for item in checks:
            if not item["passed"]:
                print(f"FAIL {item['name']}: {item['detail']}")
        return 1
    print(f"method promotion product enforcement passed: {RESULT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
