#!/usr/bin/env python3
"""Promotion audit for bounded indicator-level PLSpredict/CVPAT v2."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "plspredict_method_promotion_audit.json"


def read_text(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def read_json(relative: str) -> dict:
    source = ROOT / relative
    return json.loads(source.read_text(encoding="utf-8")) if source.exists() else {}


def check(name: str, passed: bool, detail: str) -> dict:
    return {"name": name, "passed": bool(passed), "detail": detail}


def main() -> int:
    reference = read_json("validation/results/plspredict_indicator_reference_report.json")
    packaged = read_json("validation/results/v247_tauri_native_acceptance_prediction.json")
    engine = read_text("crates/qpls-estimation/src/pls.rs")
    core_validation = read_text("crates/qpls-core/src/validation.rs")
    project = read_text("crates/qpls-project/src/lib.rs")
    native_mode = read_text("src/native/nativeCalculationMode.ts")
    native_results = read_text("src/native/nativeResults.ts")
    native_export = read_text("src/native/nativeExportTables.ts")
    native_harness = read_text("validation/v247_tauri_native_acceptance.mjs")
    spec = read_text("docs/methods/PLSPREDICT_INDICATOR_V2.md")
    legacy_spec = read_text("docs/methods/PLSPREDICT_HOLDOUT_V1.md")

    reference_checks = reference.get("checks", {})
    packaged_checks = packaged.get("checks", {})
    packaged_dialog = packaged_checks.get("predictionV2Dialog", {})
    packaged_progress = packaged_checks.get("predictionV2Progress", {})
    packaged_result = packaged_checks.get("predictionV2Result", {})
    packaged_export = packaged_checks.get("predictionV2Export", {})
    packaged_reopen = packaged_checks.get("predictionV2SaveReopen", {})

    checks = [
        check(
            "independent_indicator_and_cvpat_reference",
            reference.get("passed") is True
            and all(reference_checks.get(name) is True for name in [
                "current_method_versions",
                "seeded_balanced_plan",
                "indicator_prediction_matches_independent_oracle",
                "recursive_construct_prediction_matches_independent_oracle",
                "cvpat_matches_independent_paired_test",
                "legacy_rows_are_not_relabelled",
                "bounded_scope_is_explicit",
                "deterministic_repeat",
            ]),
            "The independent Python oracle reproduces the seeded SHA-256 10x10 plan, train-only recursive indicator prediction, IA/LM metrics, and one-sided paired CVPAT statistics.",
        ),
        check(
            "current_engine_and_validation_contract",
            all(token in engine for token in [
                'PLS_PREDICT_METHOD_VERSION: &str = "plspredict_indicator_v2"',
                'PLS_PREDICT_REPEATED_KFOLD_METHOD_VERSION: &str =',
                '"plspredict_repeated_kfold_indicator_v2"',
                'CVPAT_INDICATOR_BENCHMARK_METHOD_VERSION: &str = "cvpat_indicator_benchmarks_v2"',
                'seeded_sha256_source_row_order_round_robin_10_v1',
                'does not compare separately saved models',
            ])
            and all(token in core_validation for token in [
                "predict.reflective_endogenous_required",
                "predict.listwise_required",
                "predict.external_resampling_unsupported",
                "predict.higher_order_unsupported",
                "predict.interactions_unsupported",
            ]),
            "The executable v2 engine uses the exact versioned fold/CVPAT contract and blocks unsupported endogenous, missing-data, resampling, interaction, HOC, and case-weight shapes.",
        ),
        check(
            "strict_archive_and_legacy_boundary",
            all(token in project for token in [
                "validate_prediction_payload_contract",
                "runner_generated_prediction_round_trips_and_rejects_contract_tampering",
                "legacy_prediction_v1_reopens_with_warning_but_cannot_be_appended_as_new_evidence",
                "plspredict_holdout_v1 contract",
            ])
            and "PLS_PREDICT_METHOD_VERSION_V1" in project,
            "Current v2 results append/save/reopen only after strict payload validation; v1 construct-score archives remain readable and visibly legacy without being accepted as new v2 evidence.",
        ),
        check(
            "native_results_and_export_are_indicator_first",
            'NATIVE_PREDICTION_METHOD_LABEL = "PLSpredict / CVPAT"' in native_mode
            and 'NATIVE_LEGACY_PREDICTION_METHOD_LABEL = "Legacy construct-score prediction (v1)"' in native_mode
            and all(token in native_results for token in [
                '"plspredict_indicator_summary"',
                '"cvpat_benchmark_assessment"',
                '"plspredict_validation_plan"',
                '"plspredict_holdout_indicator_summary"',
                '"CVPAT benchmark assessment (single model)"',
                "validPredictionAssignmentDigest",
            ])
            and all(token in native_export for token in [
                '["Assignment digest", prediction.repeated.assignment_digest ?? ""]',
                '["CVPAT alternative", "PLS-SEM loss < benchmark (one-sided)"]',
                '["Prediction scope", "Endogenous indicators; construct scores supplementary"]',
            ])
            and '"N/A"' not in native_results,
            "Native Results default to indicator prediction, show explicit IA/LM availability and single-model CVPAT, preserve blank unavailable numerics with reasons, and export the exact fold digest/provenance.",
        ),
        check(
            "genuine_packaged_native_workflow",
            packaged.get("passed") is True
            and packaged.get("runtime") == "tauri-webview2-cdp"
            and packaged.get("focusedRun", {}).get("scope") == "prediction"
            and packaged_dialog.get("selectedMethod") == "PLSpredict / CVPAT"
            and packaged_dialog.get("startEnabled") is True
            and packaged_progress.get("status") in {"queued", "validating", "running"}
            and packaged_result.get("initialSelection") == "plspredict_indicator_summary"
            and packaged_result.get("indicator", {}).get("rows") == 2
            and packaged_result.get("cvpat", {}).get("rows") == 2
            and packaged_result.get("noPlaceholderOrLegacyClaim") is True
            and packaged_export.get("nativeXlsx", {}).get("file", {}).get("isFile") is True
            and packaged_export.get("nativeXlsx", {}).get("file", {}).get("size", 0) > 0
            and packaged_reopen.get("sameRunRestored") is True
            and packaged_reopen.get("archive", {}).get("exactVersions") is True
            and packaged_reopen.get("archive", {}).get("exactRepeatedPlan") is True
            and packaged_reopen.get("archive", {}).get("exactIndicatorRows") is True
            and packaged_reopen.get("archive", {}).get("exactCvpatRows") is True
            and "QUICKPLS_PREDICTION_NATIVE_EXPORT_PATH" in native_harness,
            "A genuine packaged Tauri run must visibly author the model, run v2, capture active progress, inspect indicator/CVPAT tables, create a native XLSX, and reopen the exact saved result.",
        ),
        check(
            "bounded_scope_and_legacy_docs",
            all(token in spec for token in [
                "plspredict_indicator_v2",
                "plspredict_repeated_kfold_indicator_v2",
                "cvpat_indicator_benchmarks_v2",
                "fixed 10-fold",
                "repeated 10 times",
                "not a comparison of saved models",
            ])
            and "archive-compatible" in legacy_spec
            and "not current indicator-level PLSpredict/CVPAT evidence" in legacy_spec,
            "The current method note documents the exact fixed indicator/CVPAT scope and the v1 note remains archive-only rather than being relabeled as current evidence.",
        ),
    ]

    report = {
        "kind": "plspredict_indicator_v2_method_promotion_audit",
        "passed": all(item["passed"] for item in checks),
        "promotion_status": "packaged_native_validated_bounded_scope" if all(item["passed"] for item in checks) else "packaged_native_acceptance_pending",
        "checks": checks,
        "limitations": [
            "The fixed seeded 10x10 plan is independently implemented and does not reproduce SmartPLS random fold assignments.",
            "CVPAT compares one fitted PLS model with IA and LM benchmarks; separately saved-model comparison remains outside this workflow.",
            "Formative endogenous constructs, HOCs, interactions, case weights, non-listwise missing-data handling, and external inference mixing remain unsupported.",
        ],
    }
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
