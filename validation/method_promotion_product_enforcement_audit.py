#!/usr/bin/env python3
"""Audit product surfaces for current bounded method-promotion truth."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULT = ROOT / "validation" / "results" / "method_promotion_product_enforcement_audit.json"


PRODUCT_CONTRACT_PATHS = (
    "src/data/sample.ts",
    "src/domain/methodStatus.ts",
    "src/domain/analysisReadiness.ts",
    "src/components/TopBar.tsx",
    "src/domain/resultTables.ts",
    "crates/qpls-estimation/src/pls.rs",
    "crates/qpls-core/src/validation.rs",
    "crates/qpls-core/src/methods.rs",
    "docs/METHOD_COMPATIBILITY.md",
)


def check(name: str, passed: bool, detail: str) -> dict:
    return {"name": name, "passed": bool(passed), "detail": detail}


def load_product_contract_sources(root: Path = ROOT) -> dict[str, str]:
    return {
        path: (root / path).read_text(encoding="utf-8")
        for path in PRODUCT_CONTRACT_PATHS
    }


def product_contract_checks(sources: dict[str, str]) -> list[dict]:
    def contains(path: str, needle: str) -> bool:
        return needle in sources[path]

    permutation_catalog_row = next(
        (
            line
            for line in sources["src/data/sample.ts"].splitlines()
            if 'name: "Freedman-Lane permutation"' in line
        ),
        "",
    )

    return [
        check(
            "catalog_promotes_pca",
            contains("src/data/sample.ts", '{ id: "pca", family: "Components", name: "Principal component analysis", status: "validated" }'),
            "Desktop method catalog marks standalone PCA as validated.",
        ),
        check(
            "catalog_separates_bootstrap_from_structural_randomization",
            contains("src/data/sample.ts", '{ id: "bootstrap", family: "PLS-SEM", name: "Bootstrapping", status: "validated" }')
            and contains("src/data/sample.ts", '{ id: "permutation", family: "PLS-SEM", name: "Freedman-Lane permutation", status: "experimental" }')
            and 'status: "validated"' not in permutation_catalog_row,
            "Bootstrap retains its validated scope while Structural Path Randomization keeps a conservative experimental product label independent of dedicated qualification evidence.",
        ),
        check(
            "regression_status_is_setting_aware",
            contains("src/domain/methodStatus.ts", 'method.id === "regression"')
            and contains("src/domain/methodStatus.ts", 'if (regressionType === "ols" || regressionType === "logistic") return "validated";')
            and contains("src/domain/methodStatus.ts", 'if (regressionType === "process") return "experimental";')
            and contains("src/domain/methodStatus.ts", "Graph-defined PROCESS v2 is an implemented bounded candidate pending current promotion evidence; historical PROCESS v1 is archive-only."),
            "Regression status is setting-aware: OLS/logistic remain validated while current PROCESS v2 and historical PROCESS v1 fail closed in generic product status.",
        ),
        check(
            "structural_randomization_status_discloses_bounded_candidate_scope",
            contains("src/domain/methodStatus.ts", "Candidate fixed-score path inference assumes exchangeable reduced-model residuals")
            and contains("src/domain/methodStatus.ts", "raw unadjusted pathwise plus-one p values")
            and contains("src/domain/methodStatus.ts", "current calibration covers homoscedastic Gaussian errors only."),
            "Structural Path Randomization product guidance discloses the fixed-score, exchangeability, unadjusted-probability, and calibration boundaries.",
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
            "result_tables_fail_closed_by_exact_method_identity",
            contains("src/domain/resultTables.ts", 'return regression.method_version === "regression_logistic_v2" ? "validated" : "experimental";')
            and contains("src/domain/resultTables.ts", 'if (resultMethodVersion === "regression_process_v1"')
            and contains("src/domain/resultTables.ts", 'process?.method_version === "regression_process_v1"')
            and contains("src/domain/resultTables.ts", 'const runStatus = structuralPathRandomization ? "experimental" : resultScopeStatus(run.result);')
            and contains("src/domain/resultTables.ts", '["Qualification status", "Internal candidate/experimental product label; method-specific qualification evidence is tracked separately"]')
            and contains("src/domain/resultTables.ts", 'result.method_version === "regression_logistic_v2"')
            and not contains("src/domain/resultTables.ts", 'result.method_version === "regression_process_v2"'),
            "Result and export tables validate only exact current identities, keep PROCESS archives fail-closed in generic status, and keep Structural Path Randomization visibly candidate-labelled.",
        ),
        check(
            "engine_warnings_match_current_versions_and_bounds",
            contains("crates/qpls-estimation/src/pls.rs", "Logistic regression v2 is validated for the documented QuickPLS binary numeric complete-case scope")
            and contains("crates/qpls-estimation/src/pls.rs", "PROCESS v2 is an independently implemented graph-defined observed-variable path-analysis workflow; it does not execute copied numbered templates.")
            and contains("crates/qpls-estimation/src/pls.rs", "NCA v2 is limited to the documented numeric X/Y CE-FDH and CR-FDH scope with observed-range bottlenecks")
            and not contains("crates/qpls-estimation/src/pls.rs", "NCA v1 is validated"),
            "Generated warnings bind current logistic v2 and PROCESS v2 identities and preserve bounded NCA v2 exclusions without presenting legacy identities as current evidence.",
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
            and contains("docs/METHOD_COMPATIBILITY.md", "| Regression | Binary logistic regression | Current `regression_logistic_v2`")
            and contains("docs/METHOD_COMPATIBILITY.md", "| Regression | Graph-defined Path Analysis and PROCESS | Current `regression_process_v2`")
            and contains("docs/METHOD_COMPATIBILITY.md", "Structural Path Randomization v1 is separately release-qualified")
            and contains("docs/METHOD_COMPATIBILITY.md", "release-qualified bounded v1 evidence with an explicit conditional/approximate interpretation warning"),
            "Compatibility documentation records current logistic/PROCESS identities and the separately qualified bounded Structural Path Randomization scope without changing conservative product labels.",
        ),
    ]


def main() -> int:
    checks = product_contract_checks(load_product_contract_sources())
    passed = all(item["passed"] for item in checks)
    RESULT.parent.mkdir(parents=True, exist_ok=True)
    RESULT.write_text(
        json.dumps(
            {
                "audit": "method_promotion_product_enforcement",
                "target": "v1_2_method_promotion_program",
                "passed": passed,
                "checks": checks,
                "note": (
                    "Current product surfaces validate exact supported method identities. Structural Path "
                    "Randomization and PROCESS retain conservative candidate/experimental product labels "
                    "while their separately scoped 2.46 qualification evidence is tracked without broader claims."
                ),
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
