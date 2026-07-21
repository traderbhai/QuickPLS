#!/usr/bin/env python3
"""Verify v1.2.4 CB-SEM/CFA and GSCA scopes are enforced in product-facing code."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "validation" / "results" / "fifth_batch_product_enforcement_audit.json"


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def check(name: str, passed: bool, detail: str) -> dict:
    return {"name": name, "passed": bool(passed), "detail": detail}


def main() -> int:
    sample = text("src/data/sample.ts")
    methods = text("crates/qpls-core/src/methods.rs")
    tables = text("src/domain/resultTables.ts")
    run_history = text("src/components/RunHistory.tsx")
    engine = text("crates/qpls-estimation/src/pls.rs")
    compat = text("docs/METHOD_COMPATIBILITY.md")
    checks = [
        check("catalog_promotes_cbsem_and_gsca", all(
            f'id: "{method}"' in sample and 'status: "validated"' in sample.split(f'id: "{method}"', 1)[1].split("}", 1)[0]
            for method in ["cbsem", "gsca"]
        ), "Desktop catalog marks CB-SEM/CFA and GSCA as validated entry points for documented scopes."),
        check("core_registry_promotes_cbsem_and_gsca", all(
            f'id: "{method}"' in methods and "status: MethodStatus::Validated" in methods.split(f'id: "{method}"', 1)[1].split("}", 1)[0]
            for method in ["cbsem", "gsca"]
        ), "Core method registry marks CB-SEM/CFA and GSCA as validated."),
        check("result_tables_promote_base_cbsem_and_gsca", all(snippet in tables for snippet in [
            'id: "cbsem_fit"',
            'id: "cbsem_parameters"',
            'id: "cbsem_standardized"',
            'id: "cbsem_modification_indices"',
            'id: "gsca_fit"',
            'id: "gsca_paths"',
            'result.cbsem.bootstrap || result.cbsem.multigroup',
        ]), "Result/export tables validate base CB-SEM/CFA and GSCA payloads while keeping CB-SEM bootstrap/multigroup experimental."),
        check("run_history_uses_validated_cbsem_title", "CB-SEM / CFA ML" in run_history and "beta" not in run_history, "Saved-run display no longer labels the bounded CB-SEM ML scope as beta."),
        check("engine_warnings_use_bounded_validated_language", all(phrase in engine for phrase in [
            "CB-SEM/CFA ML v1 is validated for the documented QuickPLS v1.2.4 raw-data single-group reflective ML scope",
            "GSCA v1 is validated for the documented QuickPLS v1.2.4 bounded deterministic component-model scope",
        ]), "Newly generated warnings use bounded v1.2.4 validated-scope language."),
        check("compatibility_docs_match_fifth_batch", all(phrase in compat for phrase in [
            "| CB-SEM | CFA and maximum-likelihood SEM | Validated",
            "| Components | GSCA | Validated",
        ]), "Compatibility matrix marks CB-SEM/CFA and GSCA validated only for documented bounded scopes."),
    ]
    passed = all(item["passed"] for item in checks)
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps({
        "schema_version": 1,
        "target": "v1_2_4_cbsem_gsca_promotion",
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
