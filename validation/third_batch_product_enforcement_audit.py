#!/usr/bin/env python3
"""Verify v1.2.2 promoted group/prediction/regression scopes are enforced in product-facing code."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "validation" / "results" / "third_batch_product_enforcement_audit.json"


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def check(name: str, passed: bool, detail: str) -> dict:
    return {"name": name, "passed": bool(passed), "detail": detail}


def main() -> int:
    sample = text("src/data/sample.ts")
    methods = text("crates/qpls-core/src/methods.rs")
    status = text("src/domain/methodStatus.ts")
    results = text("src/domain/resultTables.ts")
    groups = text("src/components/GroupsWorkspace.tsx")
    engine = text("crates/qpls-estimation/src/pls.rs")
    compat = text("docs/METHOD_COMPATIBILITY.md")
    checks = [
        check("desktop_catalog_promotes_group_and_regression_methods", all(
            f'id: "{method}"' in sample and 'status: "validated"' in sample.split(f'id: "{method}"', 1)[1].split("}", 1)[0]
            for method in ["mga", "predict", "regression"]
        ), "Desktop catalog marks MGA, prediction, and regression as validated entry points with setting-aware scope text."),
        check("core_registry_promotes_entry_points", all(
            f'id: "{method}"' in methods and "status: MethodStatus::Validated" in methods.split(f'id: "{method}"', 1)[1].split("}", 1)[0]
            for method in ["mga", "predict", "regression"]
        ), "Core registry exposes the promoted entry points as validated while validation guards unsupported settings."),
        check("setting_aware_boundaries", all(snippet in status for snippet in [
            'item === "micom" || item === "mga_permutation"',
            'regressionType === "ols" || regressionType === "logistic"',
            '(settings?.processModel ?? "mediation") !== "moderated_mediation"',
        ]), "UI method status is setting-aware for MGA and regression PROCESS moderated-mediation exclusion."),
        check("result_tables_promote_third_batch", all(snippet in results for snippet in [
            'result.segmentation.method_version === "pls_pos_v1" ? "validated" : "experimental"',
            'status: "validated"',
            'id: "micom_constructs"',
            'id: "mga_permutation"',
            'id: "fimix_summary"',
            'regression.regression_type === "ols" || regression.regression_type === "logistic"',
            'regression.regression_type === "process" && regression.process?.model !== "moderated_mediation"',
        ]), "Report/export tables promote only v1.2.2 scoped group, segmentation, logistic, and bounded PROCESS payloads."),
        check("groups_workspace_uses_validated_scope_language", all(snippet in groups for snippet in [
            "Validated group and segmentation payloads are limited to the documented QuickPLS v1.2.2 scopes",
            'className="status-text validated"',
        ]), "Groups workspace labels promoted v1.2.2 payloads as validated scope."),
        check("engine_warnings_promoted_scope", all(phrase in engine for phrase in [
            "Logistic regression v1 is validated for the documented QuickPLS v1.2.2 binary numeric complete-case scope",
            "PROCESS-style regression v1 is validated for the documented QuickPLS v1.2.2 bounded mediation/moderation workflow scope",
            "PLS-POS v1 is validated for the documented QuickPLS v1.2.2 deterministic 2-5 segment",
            "Two-group MGA v1 is validated for the documented QuickPLS v1.2.2",
            "MICOM v1 is validated for the documented QuickPLS v1.2.2",
            "Permutation MGA v1 is validated for the documented QuickPLS v1.2.2",
            "FIMIX-PLS v1 is validated for the documented QuickPLS v1.2.2",
            "moderated mediation remains experimental",
        ]), "Newly generated result warnings use bounded v1.2.2 validated-scope language."),
        check("later_methods_remain_experimental", all(phrase in compat for phrase in [
            "| PLS-SEM | Higher-order constructs | Experimental",
            "| PLS-SEM | CCA | Experimental",
            "| PLS-SEM | CTA-PLS | Experimental",
            "| PLS-SEM | Moderated mediation | Experimental",
            "| CB-SEM | CFA and maximum-likelihood SEM | Experimental",
            "| Components | GSCA | Experimental",
        ]), "Unpromoted later methods remain experimental in compatibility docs."),
    ]
    passed = all(item["passed"] for item in checks)
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps({
        "schema_version": 1,
        "target": "v1_2_2_group_prediction_regression_promotion",
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
