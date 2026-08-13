#!/usr/bin/env python3
"""Fail-closed release promotion for graph-defined PROCESS v2."""

from __future__ import annotations

import hashlib
import json
import math
import re
import zipfile
import xml.etree.ElementTree as ET
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from promotion_audit_integrity import ROOT, RESULTS, sha256_file


FEATURE_ID = "qpls3.standalone.process"
METHOD_VERSION = "regression_process_v2"
BOOTSTRAP_METHOD_VERSION = "regression_process_bootstrap_v1"
WITNESS_VERSION = "regression_process_bootstrap_validation_witness_v1"
CATALOGUE_SNAPSHOT_DATE = "2026-08-12"
REFERENCE_CONDITION = (
    "Continuous moderators are evaluated at their original complete-sample raw means "
    "(coded 0); binary moderators are evaluated at 0."
)
JN_INVALID_COVARIANCE_MESSAGE = (
    "Johnson-Neyman conditional-effect variance must be finite and strictly positive "
    "across the tested moderator range."
)
PROCESS_PROMOTION_PENDING_WARNING = (
    "Implemented bounded PROCESS v2 candidate; release qualification remains pending until "
    "the current independent-reference, native, packaged, and promotion evidence gates pass."
)
PROCESS_CURVE_WARNING_DISCLOSURE = (
    "Exact engine-persisted Johnson-Neyman curve points; bootstrap validation witnesses "
    "are never exported."
)
RESOURCE_ROLE_NAMES = (
    "desktop_root", "webview_browser", "webview_renderer", "webview_gpu",
    "webview_utility", "webview_other", "other_descendant",
)
TARGET = "quickpls3_process_v2_promotion"
REFERENCE_REPORT = "process_v2_reference_report.json"
PACKAGED_REPORT = "process_v2_packaged_acceptance.json"
VISUAL_REPORT = "v247_native_desktop_visual_acceptance.json"
BOUNDARY_REPORT = "process_v2_boundary_test_report.json"
FRONTEND_REPORT = "process_v2_frontend_gate_report.json"
PACKAGED_SOURCE_REPORT = "validation/results/v247_tauri_native_acceptance_process_v2.json"
PROCESS_CLEANUP_REPORT = "v247_process_v2_process_cleanup.json"
OUTPUT = "process_v2_method_promotion_audit.json"
PACKAGED_TAURI_ORIGIN = "http://tauri.localhost"
DOCUMENTATION_REFERENCE_IDENTIFIERS = (
    "10.1016/0304-4076(85)90158-7",
    "10.1093/biomet/6.1.1",
    "10.3102/10769986031004437",
    "10.1080/00273171.2014.962683",
    "10.1214/aos/1176344552",
    "10.1080/01621459.1987.10478410",
    "10.1080/00031305.1996.10473566",
    "https://smartpls.com/documentation/algorithms-and-techniques/",
    "2026-08-12",
)

REFERENCE_CHECKS = frozenset(
    {
        "typed_recipe_and_method_identity", "global_listwise_sample_and_profiles",
        "equation_order_terms_hc3_fit", "hc3_high_leverage_fail_closed",
        "hc3_covariance_diagonal_fail_closed", "simple_slope_variance_fail_closed",
        "scale_aware_solver_affine_unit_invariance_and_relative_collinearity",
        "johnson_neyman_root_solver_affine_stability",
        "johnson_neyman_invalid_covariance_fail_closed",
        "binary_endogenous_outcome_rejected_in_original_sample",
        "semantic_probe_grid_rejects_collapsed_f64_levels",
        "reference_effect_condition_policy",
        "point_metamorphic_invariance",
        "parallel_serial_effect_arithmetic",
        "continuous_binary_three_way_slopes", "first_second_stage_moderated_mediation",
        "johnson_neyman_roots_regions_curves", "conditional_plot_arithmetic",
        "bootstrap_witness_type7_bca_ratio", "bootstrap_index_partitions",
        "independent_python_resampling_distribution",
        "independent_r_equations_effects_resampling", "seed_worker_invariance",
        "dedicated_graph_result_not_generic_regression",
        "legacy_v1_not_current_evidence",
    }
)
PACKAGED_CHECKS = frozenset(
    {
        "runtime_preflight", "workflow", "setup", "results", "export",
        "save_reopen", "cancellation", "cancelled_retry_setup", "witness_boundary",
        "resource_reset", "resources",
    }
)
PROCESS_SOURCE_CHECKS = frozenset(
    {
        "runtimePreflight", "processV2ReferenceFixture", "processV2FixtureProvisioning",
        "runtime", "processV2Workflow", "processV2Fixture", "processV2Cancellation",
        "processV2CancelledRetrySetup",
        "processV2Setup", "processV2Export", "processV2Results",
        "processV2WitnessBoundary", "processV2RepeatedCompletion",
        "processV2SaveReopen", "processV2ResourceResetClone", "recentProjectsRestored",
    }
)
VISUAL_VIEWPORTS = frozenset({"1024x700", "1280x720", "1440x900"})
BOUNDARY_SUITES = {
    "qpls_core": frozenset(
        {
            "process_graph_v2_accepts_typed_dag_and_rejects_cycles_role_and_order_drift",
            "process_graph_v2_bootstrap_is_typed_and_stage_bounded",
        }
    ),
    "qpls_estimation": frozenset(
        {
            "process_graph_v2_parallel_serial_moderation_and_effect_arithmetic",
            "process_graph_v2_hc3_simple_slopes_and_johnson_neyman",
            "process_graph_v2_frozen_raw_probes_survive_resample_recentering",
            "process_graph_v2_rejects_high_leverage_hc3_instability_without_clamping",
            "process_graph_v2_rejects_nonpositive_hc3_variance_without_absolute_value",
            "process_graph_v2_rejects_degenerate_simple_slope_variance",
            "process_graph_v2_point_progress_completes_and_cancellation_returns_no_result",
            "process_graph_v2_point_is_row_irrelevant_column_and_recipe_order_invariant",
            "process_graph_v2_scale_aware_svd_is_affine_unit_invariant_and_rejects_relative_collinearity",
            "process_graph_v2_jn_root_solver_is_affine_stable_and_deduplicates_near_double_roots",
            "process_graph_v2_jn_nonpositive_contrast_variance_is_tagged_unavailable",
            "process_graph_v2_rejects_exact_binary_endogenous_outcomes_in_original_sample",
            "process_graph_v2_semantic_probe_grid_rejects_collapsed_f64_levels",
        }
    ),
    "qpls_resampling": frozenset(
        {
            "process_graph_v2_unavailable_inference_uses_process_specific_tokens",
            "process_graph_v2_case_bootstrap_is_worker_invariant_and_bca_conditional",
            "process_graph_v2_bootstrap_maps_high_leverage_hc3_failure",
            "process_graph_v2_bootstrap_maps_invalid_hc3_covariance_failure",
            "process_graph_v2_bootstrap_maps_degenerate_simple_slope_failure",
        }
    ),
    "qpls_project": frozenset(
        {
            "process_graph_v2_unavailable_bootstrap_roundtrip_and_tamper_contract",
            "process_graph_v2_append_save_reopen_and_tamper_are_atomic",
            "process_v1_remains_archive_only",
        }
    ),
    "qpls_runner": frozenset(
        {
            "process_v2_point_progress_completes_and_base_fit_cancellation_returns_no_result",
            "process_v2_runner_rejects_exact_binary_endogenous_original_profiles",
        }
    ),
}

RUST_WORKSPACE_MANIFESTS = ["Cargo.toml", "Cargo.lock"]
CORE_BUILD_INPUTS = [
    *RUST_WORKSPACE_MANIFESTS, "crates/qpls-core/Cargo.toml",
    "crates/qpls-core/src/contract.rs", "crates/qpls-core/src/lib.rs",
    "crates/qpls-core/src/methods.rs", "crates/qpls-core/src/roadmap.rs",
    "crates/qpls-core/src/statistics.rs", "crates/qpls-core/src/validation.rs",
]
DATA_BUILD_INPUTS = [
    *RUST_WORKSPACE_MANIFESTS, "crates/qpls-data/Cargo.toml", "crates/qpls-data/src/lib.rs",
]
ESTIMATION_BUILD_INPUTS = [
    *CORE_BUILD_INPUTS, *DATA_BUILD_INPUTS,
    "crates/qpls-estimation/Cargo.toml", "crates/qpls-estimation/src/lib.rs",
    "crates/qpls-estimation/src/pls.rs",
]
ASSESSMENT_BUILD_INPUTS = [
    *CORE_BUILD_INPUTS, *DATA_BUILD_INPUTS, *ESTIMATION_BUILD_INPUTS,
    "crates/qpls-assessment/Cargo.toml", "crates/qpls-assessment/src/lib.rs",
]
RESAMPLING_BUILD_INPUTS = [
    *ESTIMATION_BUILD_INPUTS, "crates/qpls-resampling/Cargo.toml",
    "crates/qpls-resampling/src/lib.rs",
]
PROJECT_BUILD_INPUTS = [
    *ASSESSMENT_BUILD_INPUTS, *RESAMPLING_BUILD_INPUTS,
    "crates/qpls-project/Cargo.toml", "crates/qpls-project/src/archive_integrity.rs",
    "crates/qpls-project/src/lib.rs", "crates/qpls-runner/Cargo.toml",
    "crates/qpls-runner/src/lib.rs",
]
RUNNER_BUILD_INPUTS = [
    *ASSESSMENT_BUILD_INPUTS, *RESAMPLING_BUILD_INPUTS,
    "crates/qpls-runner/Cargo.toml", "crates/qpls-runner/src/lib.rs",
]
BOUNDARY_PRODUCT_SOURCES = {
    "qpls_core": list(dict.fromkeys(CORE_BUILD_INPUTS)),
    "qpls_estimation": list(dict.fromkeys(ESTIMATION_BUILD_INPUTS)),
    "qpls_resampling": list(dict.fromkeys(RESAMPLING_BUILD_INPUTS)),
    "qpls_project": list(dict.fromkeys(PROJECT_BUILD_INPUTS)),
    "qpls_runner": list(dict.fromkeys(RUNNER_BUILD_INPUTS)),
}
REFERENCE_PRODUCT_SOURCES = list(dict.fromkeys([
    *PROJECT_BUILD_INPUTS, "crates/qpls-cli/Cargo.toml", "crates/qpls-cli/src/main.rs",
]))
PACKAGED_PRODUCT_SOURCES = [
    *REFERENCE_PRODUCT_SOURCES,
    "package.json", "package-lock.json", "tsconfig.json", "tsconfig.app.json",
    "tsconfig.node.json", "vite.config.ts", "src/main.tsx",
    "src/native/NativeCalculationDialog.tsx", "src/native/nativeAnalysisCatalog.ts",
    "src/native/nativeAnalysisRecipe.ts", "src/native/nativeCalculationRequest.ts",
    "src/native/NativeProcessSetup.tsx", "src/native/nativeProcess.ts",
    "src/native/nativeProcessResults.ts", "src/native/nativeCanonicalProject.ts",
    "src/native/NativeDesktopApp.tsx", "src/native/NativeDesktopController.tsx",
    "src/native/NativeExportDialog.tsx", "src/native/NativeResultsSurface.tsx",
    "src/native/nativeResults.ts", "src/native/nativeExportTables.ts",
    "src/domain/resultTables.ts", "src/components/ReportsWorkspace.tsx",
    "src/native/nativeDesktop.css", "src/services/projectService.ts", "src/types.ts",
    "src-tauri/Cargo.toml", "src-tauri/build.rs", "src-tauri/src/lib.rs",
    "src-tauri/src/main.rs", "src-tauri/tauri.conf.json",
    "src-tauri/capabilities/default.json",
]
FRONTEND_TEST_FILES = (
    "src/native/nativeProcess.test.ts",
    "src/native/nativeProcessAuthoring.test.tsx",
    "src/native/nativeProcessResults.test.ts",
    "src/native/NativeProcessSurface.test.tsx",
    "src/native/NativeCalculationDialog.test.ts",
    "src/native/nativeAnalysisCatalog.test.ts",
    "src/native/nativeAnalysisRecipe.test.ts",
    "src/native/nativeCalculationRequest.test.ts",
    "src/native/nativeCanonicalProject.test.ts",
    "src/native/nativeControllerContracts.test.ts",
    "src/native/nativePlsReadiness.test.ts",
    "src/native/nativeResults.test.ts",
    "src/native/NativeExportDialog.test.ts",
    "src/native/nativeExportTables.test.ts",
    "src/domain/resultTables.test.ts",
    "src/domain/resultInterpretation.test.ts",
    "src/native/NativeDesktopApp.test.ts",
    "src/components/ReportsWorkspace.test.ts",
    "src/components/accessibilityContracts.test.ts",
)
FRONTEND_TYPESCRIPT_SOURCES = tuple(sorted(
    path.relative_to(ROOT).as_posix()
    for suffix in ("*.ts", "*.tsx")
    for path in (ROOT / "src").rglob(suffix)
    if path.is_file()
))
FRONTEND_GATE_SOURCES = list(dict.fromkeys([
    "package.json", "package-lock.json", "tsconfig.json", "tsconfig.app.json",
    "tsconfig.node.json", "vite.config.ts", *FRONTEND_TYPESCRIPT_SOURCES,
    "src/native/NativeProcessSetup.tsx", "src/native/nativeProcess.ts",
    "src/native/nativeProcessResults.ts", "src/native/nativeProcessTestFixture.ts",
    "src/native/NativeCalculationDialog.tsx",
    "src/native/nativeAnalysisCatalog.ts", "src/native/nativeAnalysisRecipe.ts",
    "src/native/nativeCalculationRequest.ts", "src/native/nativeCanonicalProject.ts",
    "src/native/NativeDesktopApp.tsx", "src/native/NativeDesktopController.tsx",
    "src/native/NativeResultsSurface.tsx", "src/native/nativeResults.ts",
    "src/native/NativeExportDialog.tsx", "src/native/nativeExportTables.ts",
    "src/native/nativePlsReadiness.ts", "src/domain/resultTables.ts",
    "src/domain/methodApplicability.ts", "src/domain/methodStatus.ts",
    "src/store.ts", "src/types.ts",
    "validation/process_v2_frontend_gate.py",
    "validation/process_v2_method_promotion_audit.py",
]))
REFERENCE_SOURCES = [
    "validation/process_v2_reference.py", "validation/process_v2_reference.R",
    "validation/r_runtime.py",
    *REFERENCE_PRODUCT_SOURCES,
]
VISUAL_SOURCES = [
    "validation/v247_native_desktop_visual_acceptance.mjs",
    "validation/lib/v2_ui_smoke_harness.mjs",
    "package.json", "package-lock.json", "vite.config.ts", "src/main.tsx",
    "src/native/NativeCalculationDialog.tsx", "src/native/nativeAnalysisCatalog.ts",
    "src/native/nativeAnalysisRecipe.ts", "src/native/NativeProcessSetup.tsx",
    "src/native/nativeProcess.ts", "src/native/nativeProcessResults.ts",
    "src/native/NativeDesktopApp.tsx", "src/native/NativeDesktopController.tsx",
    "src/native/nativeResults.ts", "src/native/NativeResultsSurface.tsx",
    "src/domain/resultTables.ts",
    "src/native/nativeDesktop.css", "src/types.ts",
]
PACKAGED_SOURCES = [
    "validation/v247_tauri_native_acceptance.mjs",
    "validation/process_v2_packaged_acceptance.schema.json",
    "validation/run_v247_process_v2_native_acceptance.ps1",
    "validation/monitor_quickpls_process_tree.ps1",
    "validation/windows_native_save_export.py",
    "validation/close_tauri_test_window.mjs",
    "validation/process_v2_reference.py",
    *PACKAGED_PRODUCT_SOURCES,
]
PROCESS_TEXT_SURFACES = list(dict.fromkeys([
    "crates/qpls-estimation/src/pls.rs",
    "crates/qpls-project/src/lib.rs",
    "src/native/NativeCalculationDialog.tsx",
    "src/native/NativeProcessSetup.tsx",
    "src/native/nativeProcess.ts",
    "src/native/nativeProcessResults.ts",
    "src/native/NativeResultsSurface.tsx",
    "src/components/ReportsWorkspace.tsx",
    "docs/methods/PROCESS_V2.md",
    "validation/process_v2_reference.py",
    "validation/process_v2_reference.R",
    "validation/process_v2_packaged_acceptance.schema.json",
    "validation/process_v2_method_promotion_audit.py",
    "validation/v247_native_desktop_visual_acceptance.mjs",
    "validation/v247_tauri_native_acceptance.mjs",
]))
MOJIBAKE_TOKENS = (chr(0x00E2), chr(0x00C3), chr(0xFFFD))
EXPECTED_TABLE_IDS = [
    "process_model_summary", "process_paths", "process_equation_coefficients",
    "process_equation_fit", "process_reference_effects",
    "process_conditional_indirect_effects", "process_moderated_mediation_indices",
    "process_simple_slopes", "process_conditional_plot_points", "process_johnson_neyman",
    "process_johnson_neyman_curve_points", "process_bootstrap_summary",
    "process_bootstrap_failures", "process_bootstrap_inference", "process_bootstrap_bca",
    "process_scope",
]
EXPECTED_ESTIMAND_IDS = [
    "direct:X->Y", "indirect:X->M1->M2->Y", "indirect:X->M3->Y",
    "indirect:X->M4->Y", "total_indirect:X->Y", "total:X->Y",
    "indirect:X->M3->Y@W=minus_1sd", "indirect:X->M3->Y@W=mean",
    "indirect:X->M3->Y@W=plus_1sd", "indirect:X->M4->Y@B=binary_0",
    "indirect:X->M4->Y@B=binary_1",
    "index:X->M3->Y:X->M3:W", "index:X->M4->Y:M4->Y:B",
    "slope:moderation:X->M3@W@W=minus_1sd", "slope:moderation:X->M3@W@W=mean",
    "slope:moderation:X->M3@W@W=plus_1sd",
    "slope:moderation:X->Y@W|B@W=minus_1sd,B=binary_0",
    "slope:moderation:X->Y@W|B@W=minus_1sd,B=binary_1",
    "slope:moderation:X->Y@W|B@W=mean,B=binary_0",
    "slope:moderation:X->Y@W|B@W=mean,B=binary_1",
    "slope:moderation:X->Y@W|B@W=plus_1sd,B=binary_0",
    "slope:moderation:X->Y@W|B@W=plus_1sd,B=binary_1",
    "slope:moderation:M4->Y@B@B=binary_0", "slope:moderation:M4->Y@B@B=binary_1",
]
EXPECTED_WORKBOOK_SHEETS = [
    "Model summary", "Directed paths", "Equation coefficients", "Equation fit",
    "Reference effects", "Conditional indirect effects", "Moderated-mediation indices",
    "Simple slopes and conditional p", "Conditional outcome plot data",
    "Johnson-Neyman regions", "Johnson-Neyman curve data", "Bootstrap summary",
    "Bootstrap failures", "Bootstrap inference", "Bootstrap BCa intervals",
    "Scope and provenance", "Run provenance",
]
FORBIDDEN_WITNESS_EXPORT_TOKENS = [
    "regression_process_bootstrap_validation_witness_v1",
    "validation_witness", "successful_bootstrap", "successful_jackknife",
    "failed_jackknife",
]
PROCESS_REPLICATE_FAILURE_REASON_CODES = frozenset(
    {
        "rank_deficient_equation",
        "invalid_binary_profile",
        "high_leverage_hc3_instability",
        "invalid_hc3_covariance",
        "degenerate_simple_slope_variance",
        "nonfinite_estimate",
    }
)


def load_optional(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    try:
        value = json.loads(path.read_text(encoding="utf-8-sig"))
        return value if isinstance(value, dict) else {}
    except (OSError, UnicodeError, json.JSONDecodeError):
        return {}


def timestamp(value: Any) -> datetime | None:
    if not isinstance(value, str):
        return None
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    return parsed if parsed.tzinfo else None


def bounded_process_role_window(samples: list[dict[str, Any]]) -> dict[str, Any]:
    """Independently recompute bounded, terminally stable WebView process churn."""

    def identity(process: dict[str, Any]) -> dict[str, Any]:
        return {
            "pid": int(process["pid"]),
            "parent_pid": int(process["parent_pid"]),
            "name": str(process["name"]).lower(),
            "role": str(process["role"]),
            "creation_date": str(process["creation_date"]),
        }

    def identity_key(process: dict[str, Any]) -> tuple[int, int, str, str, str]:
        row = identity(process)
        return row["pid"], row["parent_pid"], row["name"], row["role"], row["creation_date"]

    def default_result() -> dict[str, Any]:
        return {
            "policy": "modal_pid_role_identity_with_bounded_webview_churn_v1",
            "passed": False,
            "sample_count": len(samples),
            "modal_sample_count": 0,
            "minimum_modal_sample_count": 6,
            "minimum_modal_percent": 80,
            "unique_modal_identity": False,
            "modal_pid_role_identities": [],
            "modal_role_counts": None,
            "first_three_exact_modal": False,
            "last_three_exact_modal": False,
            "deviating_sample_indices": [],
            "deviating_sample_count": 0,
            "maximum_deviating_sample_count": math.floor(len(samples) * 0.20),
            "maximum_deviating_percent": 20,
            "longest_deviation_streak": 0,
            "maximum_deviation_streak": 2,
            "root_identity_every_sample": False,
            "other_descendant_zero_every_sample": False,
            "reported_role_counts_match_processes_every_sample": False,
            "baseline_identities_never_removed_or_replaced": False,
            "persistent_browser_pid": None,
            "transient_identity_count": 0,
            "allowed_transient_roles": ["webview_gpu", "webview_utility", "webview_other"],
            "transient_processes": [],
            "transient_processes_allowed": False,
            "transients_absent_terminal_three": False,
        }

    try:
        signatures = [
            tuple(sorted(identity_key(process) for process in sample["processes"]))
            for sample in samples
        ]
        counts: dict[tuple[tuple[int, int, str, str, str], ...], int] = {}
        for signature in signatures:
            counts[signature] = counts.get(signature, 0) + 1
        groups = sorted(counts.items(), key=lambda row: (-row[1], row[0]))
        if not groups:
            return default_result()
        modal_signature, modal_sample_count = groups[0]
        unique_modal = len(groups) == 1 or modal_sample_count > groups[1][1]
        modal_index = signatures.index(modal_signature)
        modal_processes = sorted(samples[modal_index]["processes"], key=lambda row: int(row["pid"]))
        modal_identities = [identity(process) for process in modal_processes]
        modal_keys = {identity_key(process) for process in modal_processes}
        modal_role_counts = {
            role: sum(1 for process in modal_processes if str(process["role"]) == role)
            for role in RESOURCE_ROLE_NAMES
        }
        deviation_indices = [
            index for index, signature in enumerate(signatures) if signature != modal_signature
        ]
        longest_deviation_streak = current_streak = 0
        for signature in signatures:
            if signature != modal_signature:
                current_streak += 1
                longest_deviation_streak = max(longest_deviation_streak, current_streak)
            else:
                current_streak = 0
        first_three_exact = len(signatures) >= 6 and all(
            signature == modal_signature for signature in signatures[:3]
        )
        last_three_exact = len(signatures) >= 6 and all(
            signature == modal_signature for signature in signatures[-3:]
        )
        modal_roots = [process for process in modal_processes if process["role"] == "desktop_root"]
        modal_browsers = [process for process in modal_processes if process["role"] == "webview_browser"]
        root_key = identity_key(modal_roots[0]) if len(modal_roots) == 1 else None
        persistent_browser_pid = int(modal_browsers[0]["pid"]) if len(modal_browsers) == 1 else None
        root_identity_every_sample = len(modal_roots) == 1
        other_descendant_zero_every_sample = True
        reported_role_counts_match_processes_every_sample = True
        baseline_never_removed = bool(modal_keys)
        transient_processes_allowed = True
        transient_processes: list[dict[str, Any]] = []
        transient_keys: set[tuple[int, int, str, str, str]] = set()

        def descendant_of(process: dict[str, Any], processes: list[dict[str, Any]], ancestor: int) -> bool:
            parent_by_pid = {int(row["pid"]): int(row["parent_pid"]) for row in processes}
            parent = int(process["parent_pid"])
            visited: set[int] = set()
            while parent > 0 and parent not in visited:
                visited.add(parent)
                if parent == ancestor:
                    return True
                if parent not in parent_by_pid:
                    return False
                parent = parent_by_pid[parent]
            return False

        for index, sample in enumerate(samples):
            sample_processes = list(sample["processes"])
            sample_keys = {identity_key(process) for process in sample_processes}
            sample_roots = [process for process in sample_processes if process["role"] == "desktop_root"]
            if (
                sample.get("root_present") is not True
                or int(sample.get("root_pid", -1)) != int(modal_roots[0]["pid"] if modal_roots else -2)
                or len(sample_roots) != 1
                or identity_key(sample_roots[0]) != root_key
            ):
                root_identity_every_sample = False
            if any(process["role"] == "other_descendant" for process in sample_processes):
                other_descendant_zero_every_sample = False
            computed_role_counts = {
                role: sum(1 for process in sample_processes if str(process["role"]) == role)
                for role in RESOURCE_ROLE_NAMES
            }
            reported_role_counts = {
                role: int(sample.get("process_role_counts", {}).get(role, 0))
                for role in RESOURCE_ROLE_NAMES
            }
            if reported_role_counts != computed_role_counts:
                reported_role_counts_match_processes_every_sample = False
            if not modal_keys.issubset(sample_keys):
                baseline_never_removed = False
            extras = [process for process in sample_processes if identity_key(process) not in modal_keys]
            for extra in extras:
                extra_identity = identity(extra)
                transient_keys.add(identity_key(extra))
                descendant = persistent_browser_pid is not None and descendant_of(
                    extra, sample_processes, persistent_browser_pid
                )
                allowed = (
                    extra_identity["name"] == "msedgewebview2.exe"
                    and extra_identity["role"] in {"webview_gpu", "webview_utility", "webview_other"}
                    and descendant
                )
                transient_processes_allowed = transient_processes_allowed and allowed
                transient_processes.append({
                    "sample_index": index,
                    "recorded_at_utc": sample.get("recorded_at_utc"),
                    **extra_identity,
                    "working_set_bytes": int(extra["working_set_bytes"]),
                    "private_memory_bytes": int(extra["private_memory_bytes"]),
                    "handle_count": int(extra["handle_count"]),
                    "thread_count": int(extra["thread_count"]),
                    "descendant_of_persistent_browser": descendant,
                })
        maximum_deviating = math.floor(len(samples) * 0.20)
        transients_absent_terminal_three = last_three_exact and all(
            row["sample_index"] < len(samples) - 3 for row in transient_processes
        )
        passed = (
            len(samples) >= 6
            and unique_modal
            and modal_sample_count >= 6
            and modal_sample_count * 100 >= len(samples) * 80
            and first_three_exact
            and last_three_exact
            and len(deviation_indices) <= maximum_deviating
            and longest_deviation_streak <= 2
            and root_identity_every_sample
            and other_descendant_zero_every_sample
            and reported_role_counts_match_processes_every_sample
            and baseline_never_removed
            and transient_processes_allowed
            and transients_absent_terminal_three
        )
        return {
            "policy": "modal_pid_role_identity_with_bounded_webview_churn_v1",
            "passed": passed,
            "sample_count": len(samples),
            "modal_sample_count": modal_sample_count,
            "minimum_modal_sample_count": 6,
            "minimum_modal_percent": 80,
            "unique_modal_identity": unique_modal,
            "modal_pid_role_identities": modal_identities,
            "modal_role_counts": modal_role_counts,
            "first_three_exact_modal": first_three_exact,
            "last_three_exact_modal": last_three_exact,
            "deviating_sample_indices": deviation_indices,
            "deviating_sample_count": len(deviation_indices),
            "maximum_deviating_sample_count": maximum_deviating,
            "maximum_deviating_percent": 20,
            "longest_deviation_streak": longest_deviation_streak,
            "maximum_deviation_streak": 2,
            "root_identity_every_sample": root_identity_every_sample,
            "other_descendant_zero_every_sample": other_descendant_zero_every_sample,
            "reported_role_counts_match_processes_every_sample": reported_role_counts_match_processes_every_sample,
            "baseline_identities_never_removed_or_replaced": baseline_never_removed,
            "persistent_browser_pid": persistent_browser_pid,
            "transient_identity_count": len(transient_keys),
            "allowed_transient_roles": ["webview_gpu", "webview_utility", "webview_other"],
            "transient_processes": transient_processes,
            "transient_processes_allowed": transient_processes_allowed,
            "transients_absent_terminal_three": transients_absent_terminal_three,
        }
    except (KeyError, TypeError, ValueError):
        return default_result()


def finite(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(float(value))


def mojibake_matches(text: str) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line_number, line in enumerate(text.splitlines(), 1):
        for token in MOJIBAKE_TOKENS:
            if token in line:
                rows.append({
                    "line": line_number,
                    "codepoint": f"U+{ord(token):04X}",
                    "count": line.count(token),
                })
    return rows


def text_integrity_attestation(root: Path) -> dict[str, Any]:
    rows = []
    for relative in PROCESS_TEXT_SURFACES:
        path = root / relative
        matches: list[dict[str, Any]] = []
        read_error = None
        if path.is_file():
            try:
                matches = mojibake_matches(path.read_text(encoding="utf-8"))
            except (OSError, UnicodeError) as error:
                read_error = type(error).__name__
        rows.append({
            "path": relative,
            "present": path.is_file(),
            "read_error": read_error,
            "matches": matches,
            "passed": path.is_file() and read_error is None and matches == [],
        })
    return {"rows": rows, "passed": bool(rows) and all(row["passed"] for row in rows)}


def artifact_attestation(root: Path, descriptor: Any, *, allow_zero: bool = False) -> dict[str, Any]:
    result = {"reported": descriptor, "path": None, "present": False, "inside_repository": False, "actual_size": None, "actual_sha256": None, "passed": False}
    if not isinstance(descriptor, dict):
        return result
    relative, size, digest = descriptor.get("path"), descriptor.get("size"), descriptor.get("sha256")
    if not isinstance(relative, str) or not isinstance(size, int) or isinstance(size, bool) or size < (0 if allow_zero else 1) or not isinstance(digest, str) or re.fullmatch(r"[0-9a-f]{64}", digest) is None:
        return result
    repository = root.resolve()
    path = (repository / relative).resolve()
    try:
        path.relative_to(repository)
    except ValueError:
        return result
    result["path"] = relative
    result["inside_repository"] = True
    if not path.is_file():
        return result
    result["present"] = True
    result["actual_size"] = path.stat().st_size
    result["actual_sha256"] = sha256_file(path)
    result["passed"] = result["actual_size"] == size and result["actual_sha256"] == digest
    return result


def process_resource_snapshot_attestation(
    root: Path,
    descriptor: Any,
    *,
    expected_results: int,
    expected_witnesses: int,
    expected_selected_run_id: str | None,
) -> dict[str, Any]:
    result: dict[str, Any] = {
        "artifact": {}, "computed_logical_state": {}, "errors": [], "passed": False,
    }
    expected_keys = {
        "path", "bytes", "sha256", "source_path", "source_before", "source_after",
        "source_stable_during_copy", "exclusive_atomic_copy", "application_opened",
        "logical_state",
    }
    if not isinstance(descriptor, dict) or set(descriptor) != expected_keys:
        result["errors"].append("descriptor_shape")
        return result
    artifact = artifact_attestation(root, {
        "path": descriptor.get("path"), "size": descriptor.get("bytes"),
        "sha256": descriptor.get("sha256"),
    })
    result["artifact"] = artifact
    source_before = descriptor.get("source_before")
    source_after = descriptor.get("source_after")
    source_shape = {"bytes", "sha256", "mtime_ns"}
    source_identity = (
        isinstance(source_before, dict) and set(source_before) == source_shape
        and isinstance(source_after, dict) and set(source_after) == source_shape
        and source_before == source_after
        and source_before.get("bytes") == descriptor.get("bytes")
        and source_before.get("sha256") == descriptor.get("sha256")
        and isinstance(source_before.get("mtime_ns"), str)
        and re.fullmatch(r"[0-9]+", source_before["mtime_ns"]) is not None
        and descriptor.get("source_stable_during_copy") is True
        and descriptor.get("exclusive_atomic_copy") is True
        and descriptor.get("application_opened") is False
    )
    relative = descriptor.get("path")
    source_relative = descriptor.get("source_path")
    path_identity = (
        isinstance(relative, str)
        and re.fullmatch(
            r"validation/results/process-v2-resource-snapshot-[0-9]+-[0-9]+-[a-z0-9_]+\.qpls",
            relative,
        ) is not None
        and isinstance(source_relative, str) and source_relative != relative
    )
    repository = root.resolve()
    try:
        source_path = (repository / str(source_relative)).resolve()
        source_path.relative_to(repository)
        source_inside = True
    except (OSError, ValueError):
        source_inside = False
    if not artifact.get("passed"):
        result["errors"].append("snapshot_artifact")
        return result
    snapshot_path = repository / str(relative)
    try:
        with zipfile.ZipFile(snapshot_path) as archive:
            names = [entry.filename for entry in archive.infolist()]
            if len(names) != len(set(names)) or names.count("project.json") != 1 or names.count("manifest.json") != 1:
                result["errors"].append("archive_member_identity")
                return result
            project_bytes = archive.read("project.json")
            project = json.loads(project_bytes.decode("utf-8-sig"))
            manifest = json.loads(archive.read("manifest.json").decode("utf-8-sig"))
    except (OSError, UnicodeError, RuntimeError, zipfile.BadZipFile, json.JSONDecodeError) as error:
        result["errors"].append(f"archive_read:{type(error).__name__}")
        return result
    results = project.get("results") if isinstance(project, dict) else None
    recipes = project.get("recipes") if isinstance(project, dict) else None
    workspace = project.get("layouts", {}).get("workspace", {}) if isinstance(project, dict) else {}
    runs = workspace.get("runs") if isinstance(workspace, dict) else None
    completed = [row for row in results if isinstance(row, dict) and row.get("status") == "completed"] if isinstance(results, list) else []
    completed_ids = [row.get("id") for row in completed]
    witness_ids = [
        row.get("id") for row in completed
        if row.get("payload", {}).get("estimation", {}).get("regression", {}).get("process", {})
        .get("graph_v2", {}).get("bootstrap", {}).get("validation_witness", {}).get("method_version")
        == WITNESS_VERSION
    ]
    recipe_ids = [row.get("provenance", {}).get("recipe_id") for row in completed]
    workspace_run_ids = [
        row.get("id") for row in runs
        if isinstance(row, dict) and row.get("status") == "completed"
    ] if isinstance(runs, list) else []
    selected_run_id = workspace.get("diagramOverlaySettings", {}).get("selectedRunId") if isinstance(workspace, dict) else None
    computed = {
        "manifestValid": (
            manifest.get("schema_version") == 5
            and manifest.get("checksum_algorithm") == "sha256"
            and manifest.get("checksums", {}).get("project.json")
            == hashlib.sha256(project_bytes).hexdigest()
        ),
        "completedResultCount": len(completed),
        "witnessCount": len(witness_ids),
        "completedRunIds": completed_ids,
        "witnessRunIds": witness_ids,
        "recipeIds": recipe_ids,
        "recipeCount": len(recipes) if isinstance(recipes, list) else -1,
        "workspaceRunIds": workspace_run_ids,
        "selectedRunId": selected_run_id,
    }
    result["computed_logical_state"] = computed
    result["passed"] = (
        not result["errors"] and source_identity and path_identity and source_inside
        and computed == descriptor.get("logical_state")
        and computed["manifestValid"] is True
        and computed["completedResultCount"] == expected_results
        and computed["witnessCount"] == expected_witnesses
        and computed["selectedRunId"] == expected_selected_run_id
        and len(completed_ids) == len(set(completed_ids))
        and all(isinstance(run_id, str) and bool(run_id) for run_id in completed_ids)
        and witness_ids == completed_ids
        and len(workspace_run_ids) == len(set(workspace_run_ids))
        and set(workspace_run_ids) == set(completed_ids)
        and (expected_selected_run_id is None or expected_selected_run_id in completed_ids)
    )
    return result


def binary_attestation(root: Path, descriptor: Any, expected_path: str, sources: list[str]) -> dict[str, Any]:
    artifact = artifact_attestation(root, descriptor)
    binary = root / expected_path
    freshness = [
        {
            "path": relative, "present": (root / relative).is_file(),
            "binary_not_older": binary.is_file() and (root / relative).is_file() and binary.stat().st_mtime_ns >= (root / relative).stat().st_mtime_ns,
        }
        for relative in sources
    ]
    checks = {
        "exact_path": artifact["path"] == expected_path,
        "digest_size": artifact["passed"],
        "fresh": bool(freshness) and all(row["present"] and row["binary_not_older"] for row in freshness),
    }
    return {"artifact": artifact, "source_freshness": freshness, "checks": checks, "passed": all(checks.values())}


def directory_manifest(root: Path, relative: str) -> dict[str, Any]:
    directory = root / relative
    files = sorted((path for path in directory.rglob("*") if path.is_file()), key=lambda path: path.relative_to(directory).as_posix()) if directory.is_dir() else []
    digest = hashlib.sha256()
    manifest = []
    total = 0
    for path in files:
        child = path.relative_to(directory).as_posix()
        size = path.stat().st_size
        file_digest = sha256_file(path)
        manifest.append({"path": child, "size": size, "sha256": file_digest})
        total += size
        digest.update(f"{child}\0{size}\0{file_digest}\n".encode())
    return {"path": relative, "size": total, "file_count": len(manifest), "sha256": digest.hexdigest() if manifest else None, "manifest": manifest}


def xlsx_process_table_attestation(path: Path) -> dict[str, Any]:
    result: dict[str, Any] = {
        "reference_sheet": "Reference effects",
        "reference_columns": [],
        "reference_effect_rows": 0,
        "reference_condition_values": [],
        "status_by_sheet": {},
        "warning_by_sheet": {},
        "run_provenance_warning_absent": False,
        "curve_warning_disclosure_exact": False,
        "errors": [],
        "passed": False,
    }
    if not path.is_file():
        result["errors"].append("workbook_missing")
        return result
    try:
        with zipfile.ZipFile(path) as archive:
            names = set(archive.namelist())
            required = {
                "xl/workbook.xml", "xl/_rels/workbook.xml.rels",
                "xl/sharedStrings.xml",
            }
            if not required.issubset(names):
                result["errors"].extend(f"missing:{name}" for name in sorted(required - names))
                return result
            workbook = ET.fromstring(archive.read("xl/workbook.xml"))
            relationships = ET.fromstring(archive.read("xl/_rels/workbook.xml.rels"))
            shared_root = ET.fromstring(archive.read("xl/sharedStrings.xml"))
            shared_strings = [
                "".join(node.itertext())
                for node in shared_root.iter()
                if node.tag.rsplit("}", 1)[-1] == "si"
            ]
            targets = {
                node.attrib.get("Id"): node.attrib.get("Target")
                for node in relationships.iter()
                if node.tag.rsplit("}", 1)[-1] == "Relationship"
            }
            sheets: list[tuple[str, str]] = []
            for node in workbook.iter():
                if node.tag.rsplit("}", 1)[-1] != "sheet":
                    continue
                relation_id = next(
                    (value for key, value in node.attrib.items() if key.rsplit("}", 1)[-1] == "id"),
                    None,
                )
                target = targets.get(relation_id)
                if not node.attrib.get("name") or not target:
                    result["errors"].append("sheet_relationship_missing")
                    continue
                normalized = target.replace("\\", "/").lstrip("/")
                member = normalized if normalized.startswith("xl/") else f"xl/{normalized}"
                sheets.append((node.attrib["name"], member))
            if [name for name, _ in sheets] != EXPECTED_WORKBOOK_SHEETS:
                result["errors"].append("sheet_identity")

            def cell_value(cell: ET.Element) -> str:
                cell_type = cell.attrib.get("t")
                if cell_type == "inlineStr":
                    return "".join(
                        child.text or "" for child in cell.iter()
                        if child.tag.rsplit("}", 1)[-1] == "t"
                    )
                value = next(
                    (child.text or "" for child in cell if child.tag.rsplit("}", 1)[-1] == "v"),
                    "",
                )
                if cell_type == "s":
                    try:
                        return shared_strings[int(value)]
                    except (ValueError, IndexError):
                        result["errors"].append("shared_string_index")
                        return ""
                return value

            rows_by_sheet: dict[str, dict[int, dict[str, str]]] = {}
            for sheet_name, member in sheets:
                if member not in names:
                    result["errors"].append(f"worksheet_missing:{sheet_name}")
                    continue
                worksheet = ET.fromstring(archive.read(member))
                rows: dict[int, dict[str, str]] = {}
                for cell in worksheet.iter():
                    if cell.tag.rsplit("}", 1)[-1] != "c":
                        continue
                    reference = cell.attrib.get("r", "")
                    match = re.fullmatch(r"([A-Z]+)([0-9]+)", reference)
                    if match is None:
                        result["errors"].append(f"cell_reference:{sheet_name}")
                        continue
                    rows.setdefault(int(match.group(2)), {})[match.group(1)] = cell_value(cell)
                rows_by_sheet[sheet_name] = rows
                result["status_by_sheet"][sheet_name] = rows.get(2, {}).get("B")
                result["warning_by_sheet"][sheet_name] = rows.get(3, {}).get("B")
            reference_rows = rows_by_sheet.get("Reference effects", {})
            columns = [reference_rows.get(5, {}).get(column, "") for column in ("A", "B", "C", "D", "E")]
            conditions = [reference_rows.get(row, {}).get("E", "") for row in range(6, 12)]
            result["reference_columns"] = columns
            result["reference_effect_rows"] = sum(
                bool(reference_rows.get(row, {}).get("A")) for row in range(6, 12)
            )
            result["reference_condition_values"] = conditions
            expected_process_sheets = EXPECTED_WORKBOOK_SHEETS[:-1]
            expected_curve_warning = (
                f"{PROCESS_PROMOTION_PENDING_WARNING} {PROCESS_CURVE_WARNING_DISCLOSURE}"
            )
            result["curve_warning_disclosure_exact"] = (
                result["warning_by_sheet"].get("Johnson-Neyman curve data")
                == expected_curve_warning
            )
            result["run_provenance_warning_absent"] = (
                result["warning_by_sheet"].get("Run provenance") in (None, "")
            )
            result["passed"] = (
                not result["errors"]
                and columns == ["Effect ID", "Kind", "Path", "Estimate", "Reference condition"]
                and result["reference_effect_rows"] == 6
                and conditions == [REFERENCE_CONDITION] * 6
                and set(result["status_by_sheet"]) == set(EXPECTED_WORKBOOK_SHEETS)
                and all(result["status_by_sheet"].get(name) == "experimental" for name in EXPECTED_WORKBOOK_SHEETS)
                and all(
                    (result["warning_by_sheet"].get(name) or "").startswith(
                        PROCESS_PROMOTION_PENDING_WARNING
                    )
                    for name in expected_process_sheets
                )
                and result["curve_warning_disclosure_exact"]
                and result["run_provenance_warning_absent"]
            )
    except (OSError, RuntimeError, zipfile.BadZipFile, ET.ParseError, UnicodeError) as error:
        result["errors"].append(f"read:{type(error).__name__}")
    return result


def xlsx_witness_attestation(path: Path) -> dict[str, Any]:
    result: dict[str, Any] = {
        "path": str(path), "members": [], "scanned_xml_and_rels_members": [],
        "worksheet_members": [], "worksheet_row_counts": {}, "workbook_sheets": [],
        "forbidden_matches": [], "mojibake_matches": [], "extraction_errors": [],
        "process_table_contract": {}, "passed": False,
    }
    if not path.is_file():
        result["extraction_errors"].append("workbook_missing")
        return result
    try:
        with zipfile.ZipFile(path) as archive:
            infos = archive.infolist()
            names = [info.filename for info in infos]
            result["members"] = names
            if len(names) != len(set(names)):
                result["extraction_errors"].append("duplicate_archive_member")
            required = {"xl/workbook.xml", "[Content_Types].xml"}
            for missing in sorted(required - set(names)):
                result["extraction_errors"].append(f"missing:{missing}")
            for info in infos:
                name = info.filename.replace("\\", "/")
                if name.startswith("/") or ".." in Path(name).parts:
                    result["extraction_errors"].append(f"unsafe_path:{name}")
                    continue
                if not (name.lower().endswith(".xml") or name.lower().endswith(".rels")):
                    continue
                try:
                    data = archive.read(info)
                except (OSError, RuntimeError, zipfile.BadZipFile) as error:
                    result["extraction_errors"].append(f"read:{name}:{type(error).__name__}")
                    continue
                result["scanned_xml_and_rels_members"].append(name)
                lowered = data.lower()
                for token in FORBIDDEN_WITNESS_EXPORT_TOKENS:
                    if token.encode("utf-8") in lowered:
                        result["forbidden_matches"].append({"member": name, "token": token})
                try:
                    decoded = data.decode("utf-8")
                    for match in mojibake_matches(decoded):
                        result["mojibake_matches"].append({"member": name, **match})
                except UnicodeDecodeError:
                    result["extraction_errors"].append(f"utf8:{name}")
                if re.fullmatch(r"xl/worksheets/[^/]+\.xml", name, re.IGNORECASE):
                    result["worksheet_members"].append(name)
                    result["worksheet_row_counts"][name] = len(re.findall(rb"<row(?:\s|>)", data, re.IGNORECASE))
                if name == "xl/workbook.xml":
                    try:
                        root = ET.fromstring(data)
                        result["workbook_sheets"] = [
                            node.attrib["name"] for node in root.iter()
                            if node.tag.rsplit("}", 1)[-1] == "sheet" and "name" in node.attrib
                        ]
                    except ET.ParseError:
                        result["extraction_errors"].append("invalid_workbook_xml")
    except (OSError, RuntimeError, zipfile.BadZipFile) as error:
        result["extraction_errors"].append(f"open:{type(error).__name__}")
    result["process_table_contract"] = xlsx_process_table_attestation(path)
    result["passed"] = (
        not result["extraction_errors"] and not result["forbidden_matches"]
        and not result["mojibake_matches"]
        and bool(result["scanned_xml_and_rels_members"])
        and bool(result["worksheet_members"])
        and result["workbook_sheets"] == EXPECTED_WORKBOOK_SHEETS
        and len(result["worksheet_members"]) == len(EXPECTED_WORKBOOK_SHEETS)
        and set(result["worksheet_members"]).issubset(result["scanned_xml_and_rels_members"])
        and result["process_table_contract"].get("passed") is True
    )
    return result


def archive_witness_attestation(path: Path) -> dict[str, Any]:
    result: dict[str, Any] = {
        "path": str(path), "errors": [], "result_count": 0,
        "witness_method_version": None, "estimand_ids": [],
        "bootstrap_partition": False, "jackknife_partition": False,
        "archive_integrity": False, "public_contract": False,
        "passed": False,
    }
    if not path.is_file():
        result["errors"].append("archive_missing")
        return result
    try:
        with zipfile.ZipFile(path) as archive:
            names = [info.filename for info in archive.infolist()]
            if len(names) != len(set(names)) or names.count("project.json") != 1 or names.count("manifest.json") != 1:
                result["errors"].append("archive_member_identity")
                return result
            project_bytes = archive.read("project.json")
            document = json.loads(project_bytes.decode("utf-8-sig"))
            manifest = json.loads(archive.read("manifest.json").decode("utf-8-sig"))
    except (OSError, UnicodeError, RuntimeError, zipfile.BadZipFile, json.JSONDecodeError) as error:
        result["errors"].append(f"archive_read:{type(error).__name__}")
        return result
    result["archive_integrity"] = (
        manifest.get("schema_version") == 5
        and manifest.get("checksum_algorithm") == "sha256"
        and manifest.get("checksums", {}).get("project.json") == hashlib.sha256(project_bytes).hexdigest()
        and isinstance(document, dict)
        and len(document.get("results", [])) == 1
        and len(document.get("recipes", [])) == 1
        and document.get("models") == []
    )
    rows = []
    for candidate in document.get("results", []) if isinstance(document, dict) else []:
        try:
            estimation = candidate["payload"]["estimation"]
            regression = estimation["regression"]
            process = regression["process"]
            graph = process["graph_v2"]
            bootstrap = graph["bootstrap"]
        except (KeyError, TypeError):
            continue
        if process.get("method_version") == METHOD_VERSION and bootstrap.get("method_version") == BOOTSTRAP_METHOD_VERSION:
            rows.append((candidate, estimation, regression, process, graph, bootstrap))
    result["result_count"] = len(rows)
    if len(rows) != 1:
        result["errors"].append("expected_exactly_one_process_v2_bootstrap_result")
        return result
    candidate, estimation, regression, process, graph, bootstrap = rows[0]
    recipe_id = candidate.get("provenance", {}).get("recipe_id")
    recipe = next(
        (row for row in document.get("recipes", []) if isinstance(row, dict) and row.get("id") == recipe_id),
        None,
    )
    witness = bootstrap.get("validation_witness") if isinstance(bootstrap.get("validation_witness"), dict) else {}
    expected_ids = [
        *(row.get("effect_id") for row in graph.get("reference_effects", []) if isinstance(row, dict)),
        *(row.get("effect_id") for row in graph.get("conditional_indirect_effects", []) if isinstance(row, dict)),
        *(row.get("effect_id") for row in graph.get("moderated_mediation_indices", []) if isinstance(row, dict)),
        *(row.get("effect_id") for row in graph.get("simple_slopes", []) if isinstance(row, dict)),
    ]
    observed_ids = witness.get("estimand_ids")
    result["witness_method_version"] = witness.get("method_version")
    result["estimand_ids"] = observed_ids if isinstance(observed_ids, list) else []
    successful = witness.get("successful_bootstrap") if isinstance(witness.get("successful_bootstrap"), list) else []
    failed = bootstrap.get("failed_replicates") if isinstance(bootstrap.get("failed_replicates"), list) else []
    requested = bootstrap.get("requested_replicates")
    successful_indexes = [row.get("replicate_index") for row in successful if isinstance(row, dict)]
    failed_indexes = [row.get("replicate_index") for row in failed if isinstance(row, dict)]
    result["bootstrap_partition"] = (
        requested == 10_000
        and len(successful_indexes) >= 9_000
        and len(successful_indexes) + len(failed_indexes) == requested
        and len(set(successful_indexes)) == len(successful_indexes)
        and len(set(failed_indexes)) == len(failed_indexes)
        and not set(successful_indexes) & set(failed_indexes)
        and sorted(successful_indexes + failed_indexes) == list(range(requested))
    )
    successful_jackknife = witness.get("successful_jackknife") if isinstance(witness.get("successful_jackknife"), list) else []
    failed_jackknife = witness.get("failed_jackknife") if isinstance(witness.get("failed_jackknife"), list) else []
    failures_typed = all(
        isinstance(row, dict)
        and row.get("reason_code") in PROCESS_REPLICATE_FAILURE_REASON_CODES
        and isinstance(row.get("message"), str)
        and bool(row["message"].strip())
        for row in [*failed, *failed_jackknife]
    )
    jackknife_cases = bootstrap.get("jackknife_cases")
    successful_cases = [row.get("omitted_case") for row in successful_jackknife if isinstance(row, dict)]
    failed_cases = [row.get("omitted_case") for row in failed_jackknife if isinstance(row, dict)]
    result["jackknife_partition"] = (
        isinstance(jackknife_cases, int) and not isinstance(jackknife_cases, bool) and jackknife_cases > 0
        and len(successful_cases) + len(failed_cases) == jackknife_cases
        and len(set(successful_cases)) == len(successful_cases)
        and len(set(failed_cases)) == len(failed_cases)
        and not set(successful_cases) & set(failed_cases)
        and sorted(successful_cases + failed_cases) == list(range(jackknife_cases))
    )
    estimate_widths = [len(row.get("estimates", [])) for row in [*successful, *successful_jackknife] if isinstance(row, dict)]
    identities = (
        expected_ids == EXPECTED_ESTIMAND_IDS and len(expected_ids) == len(set(expected_ids))
        and observed_ids == expected_ids
        and [row.get("effect_id") for row in bootstrap.get("estimands", []) if isinstance(row, dict)] == expected_ids
        and (not estimate_widths or all(width == 24 for width in estimate_widths))
    )
    conditional_points = sum(
        len(series.get("points", []))
        for plot in graph.get("plots", []) if isinstance(plot, dict)
        for series in plot.get("series", []) if isinstance(series, dict)
    )
    available_jn = [row for row in graph.get("johnson_neyman", []) if isinstance(row, dict) and row.get("status") == "available"]
    jn_curve_points = sum(len(row.get("curve_points", [])) for row in available_jn)
    semantic_ids = (
        all(re.fullmatch(r"indirect:.+@(?:[A-Za-z0-9_.-]+=(?:minus_1sd|mean|plus_1sd|binary_0|binary_1))(?:,[A-Za-z0-9_.-]+=(?:minus_1sd|mean|plus_1sd|binary_0|binary_1))*", row.get("effect_id", "")) for row in graph.get("conditional_indirect_effects", []))
        and all(re.fullmatch(r"slope:moderation:.+@(?:[A-Za-z0-9_.-]+=(?:minus_1sd|mean|plus_1sd|binary_0|binary_1))(?:,[A-Za-z0-9_.-]+=(?:minus_1sd|mean|plus_1sd|binary_0|binary_1))*", row.get("effect_id", "")) for row in graph.get("simple_slopes", []))
    )
    result["public_contract"] = (
        candidate.get("status") == "completed"
        and candidate.get("provenance", {}).get("method") == "regression"
        and candidate.get("provenance", {}).get("method_version") == f"{METHOD_VERSION}+{BOOTSTRAP_METHOD_VERSION}"
        and isinstance(recipe, dict)
        and recipe.get("schema_version") == 3
        and recipe.get("metadata", {}).get("status")
        == "candidate_regression_process_v2_plus_bootstrap_v1_bounded_scope"
        and "validated" not in recipe.get("metadata", {}).get("status", "")
        and estimation.get("method_version") == METHOD_VERSION
        and regression.get("method_version") == METHOD_VERSION
        and regression.get("regression_type") == "process"
        and regression.get("outcome") == "Y"
        and regression.get("predictors") == ["X", "M1", "M2", "M3", "M4", "W", "B"]
        and regression.get("controls") == ["C"] and regression.get("observations") == 175
        and regression.get("coefficients") == [] and regression.get("fit") is None
        and regression.get("predictions") == [] and "logistic" not in regression and "bootstrap" not in regression
        and "mediation" not in estimation and "moderation" not in estimation
        and process.get("model") == "graph" and process.get("effects") == [] and process.get("simple_slopes") == []
        and graph.get("policies") == {
            "centering": "equation_complete_case_mean_v1", "covariance": "hc3_v1",
            "inference_reference": "student_t_residual_df_v1", "confidence_level": 0.95,
        }
        and graph.get("complete_cases") == 175 and graph.get("omitted_cases") == 5
        and len(graph.get("paths", [])) == 8 and len(graph.get("moderations", [])) == 3
        and len(graph.get("equations", [])) == 5 and len(graph.get("reference_effects", [])) == 6
        and len(graph.get("conditional_indirect_effects", [])) == 5
        and len(graph.get("moderated_mediation_indices", [])) == 2
        and len(graph.get("simple_slopes", [])) == 11 and len(graph.get("plots", [])) == 3
        and conditional_points == 275 and len(graph.get("johnson_neyman", [])) == 4
        and len(available_jn) == 3 and jn_curve_points == 303
        and bootstrap.get("algorithm") == "indexed_case_resampling_v1"
        and bootstrap.get("interval_policy") == "percentile_primary_bca_conditional_v1"
        and bootstrap.get("test_reference") == "standard_normal_bootstrap_ratio_v1"
        and bootstrap.get("stream_token") == "process_indexed_case_stream_v1"
        and bootstrap.get("seed") == 20_260_812 and bootstrap.get("workers") == 2
        and semantic_ids
    )
    result["passed"] = (
        not result["errors"] and result["archive_integrity"] and result["public_contract"]
        and witness.get("method_version") == WITNESS_VERSION
        and identities and result["bootstrap_partition"] and result["jackknife_partition"]
        and failures_typed
    )
    return result


def report_freshness(root: Path, report_path: Path, sources: list[str], artifacts: list[Path] | None = None) -> dict[str, Any]:
    artifacts = artifacts or []
    rows = [
        {
            "path": relative,
            "present": (root / relative).is_file(),
            "report_not_older": report_path.is_file() and (root / relative).is_file()
            and report_path.stat().st_mtime_ns >= (root / relative).stat().st_mtime_ns,
        }
        for relative in sources
    ]
    rows.extend(
        {
            "path": path.relative_to(root).as_posix() if path.is_relative_to(root) else str(path),
            "present": path.is_file(),
            "report_not_older": report_path.is_file() and path.is_file()
            and report_path.stat().st_mtime_ns >= path.stat().st_mtime_ns,
        }
        for path in artifacts
    )
    return {"rows": rows, "passed": report_path.is_file() and bool(rows) and all(row["present"] and row["report_not_older"] for row in rows)}


def reference_attestation(root: Path, report: dict[str, Any], report_path: Path) -> dict[str, Any]:
    artifacts = report.get("artifacts") if isinstance(report.get("artifacts"), dict) else {}
    cli = binary_attestation(root, artifacts.get("tested_cli"), "target/release/qpls.exe", REFERENCE_PRODUCT_SOURCES)
    fixture = artifact_attestation(root, artifacts.get("fixture"))
    r_script = artifact_attestation(root, artifacts.get("r_script"))
    freshness = report_freshness(root, report_path, REFERENCE_SOURCES, [root / "target/release/qpls.exe"])
    check_names = frozenset(report.get("checks", {})) if isinstance(report.get("checks"), dict) else frozenset()
    numerical = report.get("numerical_boundaries") if isinstance(report.get("numerical_boundaries"), dict) else {}
    scale_solver = numerical.get("scale_aware_solver") if isinstance(numerical.get("scale_aware_solver"), dict) else {}
    jn_solver = numerical.get("johnson_neyman_root_solver") if isinstance(numerical.get("johnson_neyman_root_solver"), dict) else {}
    jn_covariance = numerical.get("johnson_neyman_invalid_covariance") if isinstance(numerical.get("johnson_neyman_invalid_covariance"), dict) else {}
    binary_outcome = numerical.get("binary_endogenous_outcome") if isinstance(numerical.get("binary_endogenous_outcome"), dict) else {}
    collapsed_probe = numerical.get("collapsed_probe_grid") if isinstance(numerical.get("collapsed_probe_grid"), dict) else {}
    reference_condition = report.get("reference_condition") if isinstance(report.get("reference_condition"), dict) else {}
    external_r = report.get("external_r") if isinstance(report.get("external_r"), dict) else {}
    r_numerical = external_r.get("numerical_boundaries") if isinstance(external_r.get("numerical_boundaries"), dict) else {}
    r_reference_condition = external_r.get("reference_condition") if isinstance(external_r.get("reference_condition"), dict) else {}
    checks = {
        "identity": report.get("target") == "process_v2_independent_reference" and report.get("feature_id") == FEATURE_ID and report.get("method_version") == METHOD_VERSION and report.get("bootstrap_method_version") == BOOTSTRAP_METHOD_VERSION and report.get("catalogue_snapshot_date") == CATALOGUE_SNAPSHOT_DATE,
        "utf8_text_integrity": mojibake_matches(json.dumps(report, ensure_ascii=False)) == [],
        "passed": report.get("passed") is True,
        "exact_checks": check_names == REFERENCE_CHECKS and all(report["checks"].get(name) is True for name in REFERENCE_CHECKS) if isinstance(report.get("checks"), dict) else False,
        "exact_point": finite(report.get("point_reference", {}).get("maximum_absolute_difference")) and report["point_reference"]["maximum_absolute_difference"] <= 5e-9,
        "point_metamorphic": (
            report.get("point_metamorphic", {}).get("passed") is True
            and finite(report.get("point_metamorphic", {}).get("row_order_maximum_absolute_difference"))
            and report["point_metamorphic"]["row_order_maximum_absolute_difference"] <= 5e-11
            and finite(report.get("point_metamorphic", {}).get("irrelevant_column_maximum_absolute_difference"))
            and report["point_metamorphic"]["irrelevant_column_maximum_absolute_difference"] <= 5e-11
            and report.get("point_metamorphic", {}).get("path_order_canonicalized") is True
        ),
        "scale_aware_solver_boundary": (
            scale_solver.get("passed") is True
            and scale_solver.get("normalization") == "non_intercept_welford_mean_population_rms_v1"
            and scale_solver.get("rank_rule") == "s_min_gt_s_max_times_max_n_p_times_epsilon_times_100"
            and scale_solver.get("relative_collinearity_rejected") is True
            and finite(scale_solver.get("fitted_maximum_absolute_difference"))
            and scale_solver["fitted_maximum_absolute_difference"] <= 1e-10
            and finite(scale_solver.get("slope_back_transform_absolute_difference"))
            and scale_solver["slope_back_transform_absolute_difference"] <= 1e-10
            and finite(scale_solver.get("intercept_back_transform_absolute_difference"))
            and scale_solver["intercept_back_transform_absolute_difference"] <= 1e-9
            and finite(scale_solver.get("covariance_back_transform_absolute_difference"))
            and scale_solver["covariance_back_transform_absolute_difference"] <= 1e-10
            and finite(scale_solver.get("statistic_absolute_difference"))
            and scale_solver["statistic_absolute_difference"] <= 1e-9
        ),
        "johnson_neyman_numerical_boundaries": (
            jn_solver.get("passed") is True
            and jn_solver.get("domain_normalization") == "coded_range_to_minus_one_plus_one_v1"
            and jn_solver.get("coefficient_tolerance_multiplier") == 64.0
            and jn_solver.get("root_deduplication_tolerance_multiplier") == 128.0
            and jn_solver.get("stable_quadratic_formula") == "q_formula_v1"
            and jn_solver.get("exact_double_root_count") == 1
            and jn_solver.get("resolvable_near_double_root_count") == 2
            and jn_covariance == {
                "passed": True,
                "reason_code": "invalid_hc3_covariance",
                "message": JN_INVALID_COVARIANCE_MESSAGE,
                "variance_rule": "finite_and_strictly_positive_across_tested_range",
            }
        ),
        "binary_endogenous_original_scope": (
            binary_outcome.get("passed") is True
            and binary_outcome.get("reason_code") == "binary_process_equation_outcome"
            and binary_outcome.get("rejected_outcomes") == ["M1", "Y"]
            and binary_outcome.get("original_sample_only") is True
            and binary_outcome.get("continuous_fixture_accepted") is True
        ),
        "collapsed_semantic_probe_grid": collapsed_probe == {
            "passed": True,
            "reason_code": "collapsed_process_probe_grid",
            "message": (
                "PROCESS continuous moderator W does not have three distinct finite "
                "mean-minus-SD, mean, and mean-plus-SD probes in f64"
            ),
            "semantic_assignment": "canonical_grid_index_primary_outer_conditioning_inner",
        },
        "reference_condition_policy": reference_condition == {
            "passed": True,
            "column": "Reference condition",
            "value": REFERENCE_CONDITION,
            "continuous_coded_value": 0.0,
            "binary_raw_value": 0.0,
            "maximum_absolute_difference": 0.0,
        },
        "exact_bootstrap": finite(report.get("bootstrap_exact_arithmetic", {}).get("maximum_absolute_difference")) and report["bootstrap_exact_arithmetic"]["maximum_absolute_difference"] <= 5e-11,
        "python_thresholds": report.get("independent_python", {}).get("comparison", {}).get("passed") is True,
        "r_thresholds": external_r.get("passed") is True and external_r.get("point_comparison", {}).get("passed") is True and external_r.get("comparison", {}).get("passed") is True,
        "r_numerical_and_reference_condition_contract": (
            r_numerical.get("passed") is True
            and r_numerical.get("scale_aware_solver", {}).get("passed") is True
            and r_numerical.get("johnson_neyman_root_solver", {}).get("passed") is True
            and r_numerical.get("johnson_neyman_invalid_covariance") == {
                "passed": True,
                "reason_code": "invalid_hc3_covariance",
                "message": JN_INVALID_COVARIANCE_MESSAGE,
                "variance_rule": "finite_and_strictly_positive_across_tested_range",
            }
            and r_numerical.get("binary_endogenous_outcome", {}).get("passed") is True
            and r_numerical.get("collapsed_probe_grid") == {
                "passed": True,
                "reason_code": "collapsed_process_probe_grid",
                "semantic_assignment": "canonical_grid_index_primary_outer_conditioning_inner",
            }
            and r_reference_condition == {
                "passed": True,
                "column": "Reference condition",
                "value": REFERENCE_CONDITION,
                "continuous_coded_value": 0,
                "binary_raw_value": 0,
            }
        ),
        "scope_numerical_contract": (
            report.get("scope", {}).get("equation_solver") == {
                "algorithm": "scale_aware_thin_svd_v1",
                "normalization": "non_intercept_welford_mean_population_rms_v1",
                "rank_tolerance_multiplier": 100.0,
            }
            and report.get("scope", {}).get("johnson_neyman_solver") == {
                "domain_normalization": "coded_range_to_minus_one_plus_one_v1",
                "coefficient_tolerance_multiplier": 64.0,
                "root_deduplication_tolerance_multiplier": 128.0,
                "invalid_covariance_reason": "invalid_hc3_covariance",
                "invalid_covariance_message": JN_INVALID_COVARIANCE_MESSAGE,
            }
            and report.get("scope", {}).get("reference_condition") == REFERENCE_CONDITION
            and report.get("scope", {}).get("binary_endogenous_outcome_policy")
            == "reject_exact_0_1_in_original_complete_sample_only"
            and report.get("scope", {}).get("semantic_probe_grid") == {
                "assignment": "canonical_grid_index_primary_outer_conditioning_inner",
                "collapsed_reason": "collapsed_process_probe_grid",
            }
        ),
        "capacity_contract": report.get("scope", {}).get("capacity") == {
            "top_level_predictors_maximum": 8,
            "controls_maximum": 1,
            "equation_non_intercept_terms_maximum": 50,
        },
        "candidate_recipe_status": (
            report.get("scope", {}).get("recipe_status")
            == "candidate_regression_process_v2_plus_bootstrap_v1_bounded_scope"
            and "validated" not in report.get("scope", {}).get("recipe_status", "")
        ),
        "release_cli_bound": cli["passed"],
        "fixture_and_r_script_bound": fixture["passed"] and r_script["passed"],
        "fresh_report": freshness["passed"],
    }
    return {"checks": checks, "tested_cli": cli, "fixture": fixture, "r_script": r_script, "report_freshness": freshness, "passed": all(checks.values())}


def independent_reference_graph_counts(report: dict[str, Any]) -> dict[str, int]:
    graph = report.get("point_reference", {}).get("reference", {}) if isinstance(report.get("point_reference"), dict) else {}
    if not isinstance(graph, dict):
        return {}
    plots = graph.get("plots") if isinstance(graph.get("plots"), list) else []
    johnson_neyman = graph.get("johnson_neyman") if isinstance(graph.get("johnson_neyman"), list) else []
    families = [
        graph.get("reference_effects"), graph.get("conditional_indirect_effects"),
        graph.get("moderated_mediation_indices"), graph.get("simple_slopes"),
    ]
    if not all(isinstance(rows, list) for rows in families):
        return {}
    johnson_neyman_region_rows = 0
    for row in johnson_neyman:
        if not isinstance(row, dict):
            return {}
        if row.get("status") == "available":
            regions = row.get("regions")
            if not isinstance(regions, list):
                return {}
            johnson_neyman_region_rows += len(regions)
        else:
            johnson_neyman_region_rows += 1
    return {
        "completeCases": graph.get("complete_cases"),
        "omittedCases": graph.get("omitted_cases"),
        "equations": len(graph.get("equations", [])) if isinstance(graph.get("equations"), list) else -1,
        "paths": len(graph.get("paths", [])) if isinstance(graph.get("paths"), list) else -1,
        "moderations": len(graph.get("moderations", [])) if isinstance(graph.get("moderations"), list) else -1,
        "referenceEffects": len(families[0]),
        "conditionalIndirectEffects": len(families[1]),
        "moderatedMediationIndices": len(families[2]),
        "simpleSlopes": len(families[3]),
        "plots": len(plots),
        "conditionalPlotPoints": sum(
            len(series.get("points", []))
            for plot in plots if isinstance(plot, dict)
            for series in plot.get("series", []) if isinstance(series, dict) and isinstance(series.get("points"), list)
        ),
        "johnsonNeyman": len(johnson_neyman),
        "johnsonNeymanRegionRows": johnson_neyman_region_rows,
        "availableJohnsonNeyman": sum(
            row.get("status") == "available" for row in johnson_neyman if isinstance(row, dict)
        ),
        "johnsonNeymanCurvePoints": sum(
            len(row.get("curve_points", []))
            for row in johnson_neyman
            if isinstance(row, dict) and row.get("status") == "available" and isinstance(row.get("curve_points"), list)
        ),
        "estimands": sum(len(rows) for rows in families),
    }


def independent_reference_johnson_neyman_analysis_keys(
    report: dict[str, Any],
) -> list[list[str]]:
    graph = report.get("point_reference", {}).get("reference", {}) if isinstance(
        report.get("point_reference"), dict
    ) else {}
    rows = graph.get("johnson_neyman") if isinstance(graph, dict) else None
    if not isinstance(rows, list):
        return []
    keys: list[list[str]] = []
    for row in rows:
        if not isinstance(row, dict):
            return []
        moderation_id = row.get("moderation_id")
        solved_moderator = row.get("solved_moderator")
        conditioning_values = row.get("conditioning_values")
        if (
            not isinstance(moderation_id, str)
            or not isinstance(solved_moderator, str)
            or not isinstance(conditioning_values, list)
        ):
            return []
        labels: list[str] = []
        for value in conditioning_values:
            if (
                not isinstance(value, dict)
                or not isinstance(value.get("variable"), str)
                or not finite(value.get("raw_value"))
                or not finite(value.get("coded_value"))
            ):
                return []
            labels.append(
                f"{value['variable']} = {float(value['raw_value']):.4f} "
                f"(coded {float(value['coded_value']):.4f})"
            )
        keys.append([moderation_id, solved_moderator, "; ".join(labels)])
    return keys


def packaged_attestation(
    root: Path,
    report: dict[str, Any],
    report_path: Path,
    reference_report: dict[str, Any] | None = None,
) -> dict[str, Any]:
    schema_path = root / "validation" / "process_v2_packaged_acceptance.schema.json"
    schema_errors: list[dict[str, str]] = []
    try:
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema, format_checker=FormatChecker())
        schema_errors = [
            {
                "path": "/" + "/".join(str(part) for part in error.absolute_path),
                "message": error.message,
            }
            for error in sorted(
                validator.iter_errors(report),
                key=lambda row: tuple(str(part) for part in row.absolute_path),
            )
        ]
    except Exception as error:
        schema_errors = [{"path": "/", "message": f"schema_validation_failed:{error}"}]
    raw_artifacts = report.get("artifacts") if isinstance(report.get("artifacts"), dict) else {}
    xlsx = artifact_attestation(root, raw_artifacts.get("xlsx"))
    archive = artifact_attestation(root, raw_artifacts.get("project_archive"))
    resource_artifact = artifact_attestation(root, raw_artifacts.get("resource_report"))
    resource_samples_artifact = artifact_attestation(root, raw_artifacts.get("resource_samples"))
    resource_phases_artifact = artifact_attestation(root, raw_artifacts.get("resource_phases"))
    raw_phase_snapshots = raw_artifacts.get("resource_phase_snapshots")
    phase_snapshot_artifacts = [
        artifact_attestation(root, row) for row in raw_phase_snapshots
    ] if isinstance(raw_phase_snapshots, list) else []
    xlsx_scan = xlsx_witness_attestation(root / str(xlsx.get("path") or ""))
    archive_scan = archive_witness_attestation(root / str(archive.get("path") or ""))
    screenshots = [artifact_attestation(root, row) for row in raw_artifacts.get("screenshots", [])] if isinstance(raw_artifacts.get("screenshots"), list) else []
    tested_product = report.get("tested_product") if isinstance(report.get("tested_product"), dict) else {}
    cli = binary_attestation(
        root, tested_product.get("qpls_cli_exe"), "target/release/qpls.exe",
        REFERENCE_PRODUCT_SOURCES,
    )
    desktop = binary_attestation(root, tested_product.get("quickpls_desktop_exe"), "target/release/quickpls-desktop.exe", PACKAGED_PRODUCT_SOURCES + [path.relative_to(root).as_posix() for path in (root / "dist").rglob("*") if path.is_file()] if (root / "dist").is_dir() else PACKAGED_PRODUCT_SOURCES)
    actual_dist = directory_manifest(root, "dist")
    dist = tested_product.get("dist_bundle") if isinstance(tested_product.get("dist_bundle"), dict) else {}
    checks_obj = report.get("checks") if isinstance(report.get("checks"), dict) else {}
    generated, completed = timestamp(report.get("generated_at_utc")), timestamp(report.get("completed_at_utc"))
    export = checks_obj.get("export") if isinstance(checks_obj.get("export"), dict) else {}
    save_reopen = checks_obj.get("save_reopen") if isinstance(checks_obj.get("save_reopen"), dict) else {}
    scan = export.get("witness_scan") if isinstance(export.get("witness_scan"), dict) else {}
    process_table_contract = export.get("process_table_contract") if isinstance(export.get("process_table_contract"), dict) else {}
    result_check = checks_obj.get("results") if isinstance(checks_obj.get("results"), dict) else {}
    expected_counts = independent_reference_graph_counts(reference_report or {})
    expected_johnson_neyman_analysis_keys = independent_reference_johnson_neyman_analysis_keys(
        reference_report or {}
    )
    workflow = checks_obj.get("workflow") if isinstance(checks_obj.get("workflow"), dict) else {}
    setup = checks_obj.get("setup") if isinstance(checks_obj.get("setup"), dict) else {}
    cancellation = checks_obj.get("cancellation") if isinstance(checks_obj.get("cancellation"), dict) else {}
    cancelled_retry_setup = checks_obj.get("cancelled_retry_setup") if isinstance(checks_obj.get("cancelled_retry_setup"), dict) else {}
    witness_boundary = checks_obj.get("witness_boundary") if isinstance(checks_obj.get("witness_boundary"), dict) else {}
    resource_reset = checks_obj.get("resource_reset") if isinstance(checks_obj.get("resource_reset"), dict) else {}
    resources = checks_obj.get("resources") if isinstance(checks_obj.get("resources"), dict) else {}
    runtime_preflight = checks_obj.get("runtime_preflight") if isinstance(checks_obj.get("runtime_preflight"), dict) else {}
    resource_document = load_optional(root / str(resource_artifact.get("path") or ""))
    phase_document = load_optional(root / str(resource_phases_artifact.get("path") or ""))
    resource_archive_disk = resource_document.get("disk", {}).get("project_archive", {}) if isinstance(resource_document.get("disk"), dict) else {}
    resource_xlsx_disk = resource_document.get("disk", {}).get("xlsx_export", {}) if isinstance(resource_document.get("disk"), dict) else {}
    archive_actual_size = archive.get("actual_size")
    xlsx_actual_size = xlsx.get("actual_size")
    resource_disk_contract = (
        isinstance(resource_archive_disk, dict) and isinstance(resource_xlsx_disk, dict)
        and isinstance(archive_actual_size, int) and not isinstance(archive_actual_size, bool)
        and isinstance(xlsx_actual_size, int) and not isinstance(xlsx_actual_size, bool)
        and isinstance(resource_archive_disk.get("initial_bytes"), int)
        and not isinstance(resource_archive_disk.get("initial_bytes"), bool)
        and resource_archive_disk["initial_bytes"] > 0
        and resource_archive_disk == {
            "path": archive.get("path"),
            "initial_bytes": resource_archive_disk["initial_bytes"],
            "final_bytes": archive_actual_size,
            "delta_bytes": archive_actual_size - resource_archive_disk["initial_bytes"],
        }
        and resource_archive_disk["delta_bytes"] > 0
        and resource_xlsx_disk == {
            "path": xlsx.get("path"), "initial_bytes": 0,
            "final_bytes": xlsx_actual_size, "delta_bytes": xlsx_actual_size,
        }
    )
    checkpoint_contracts = [
        ("initial_idle", "model_free_fixture", "data", 0, 0),
        ("post_cancellation_idle", "cancelled_setup_no_result", "data", 0, 0),
        ("post_completed_cycle_1_idle", "one_result_reopened_original", "results", 1, 1),
        ("post_completed_history_2_idle", "two_results_retained_history", "results", 2, 2),
        ("post_completed_cycle_2_idle", "one_result_reopened_reset_clone", "results", 1, 1),
    ]
    raw_samples: list[dict[str, Any]] = []
    raw_samples_parse_ok = resource_samples_artifact.get("passed") is True
    try:
        raw_sample_path = root / str(resource_samples_artifact.get("path") or "")
        raw_samples = [
            json.loads(line) for line in raw_sample_path.read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]
        raw_samples_parse_ok = raw_samples_parse_ok and all(isinstance(row, dict) for row in raw_samples)
    except (OSError, ValueError, json.JSONDecodeError):
        raw_samples_parse_ok = False

    def median_int(values: list[int]) -> int:
        ordered = sorted(values)
        middle = len(ordered) // 2
        return ordered[middle] if len(ordered) % 2 else (ordered[middle - 1] + ordered[middle]) // 2

    def p95_int(values: list[int]) -> int:
        ordered = sorted(values)
        return ordered[max(0, math.ceil(len(ordered) * 0.95) - 1)]

    role_names = list(RESOURCE_ROLE_NAMES)
    checkpoint_rows = resource_document.get("idle_checkpoints")
    checkpoint_rows = checkpoint_rows if isinstance(checkpoint_rows, list) else []
    checkpoint_diagnostics = resource_document.get("checkpoint_diagnostics")
    checkpoint_diagnostics = checkpoint_diagnostics if isinstance(checkpoint_diagnostics, list) else []
    checkpoint_contract = len(checkpoint_rows) == len(checkpoint_contracts) and raw_samples_parse_ok
    checkpoint_diagnostic_contract = len(checkpoint_diagnostics) == len(checkpoint_contracts)
    snapshot_attestations: list[dict[str, Any]] = []
    previous_phase: datetime | None = None
    for index, row in enumerate(checkpoint_rows):
        if not isinstance(row, dict):
            checkpoint_contract = False
            continue
        phase = timestamp(row.get("phase_recorded_at_utc"))
        window_start = timestamp(row.get("window_start_utc"))
        window_end = timestamp(row.get("window_end_utc"))
        next_phase = timestamp(checkpoint_rows[index + 1].get("phase_recorded_at_utc")) \
            if index + 1 < len(checkpoint_rows) and isinstance(checkpoint_rows[index + 1], dict) else None
        expected_name, expected_kind, expected_surface, expected_results, expected_witnesses = checkpoint_contracts[index]
        logical = row.get("logical_state") if isinstance(row.get("logical_state"), dict) else {}
        effective = row.get("effective_archive") if isinstance(row.get("effective_archive"), dict) else {}
        effective_logical = effective.get("logical_state") if isinstance(effective.get("logical_state"), dict) else {}
        eligible = []
        if window_start is not None and window_end is not None:
            eligible = [
                sample for sample in raw_samples
                if (sample_time := timestamp(sample.get("recorded_at_utc"))) is not None
                and window_start <= sample_time < window_end
                and (next_phase is None or sample_time < next_phase)
            ]
        sample_times = [sample.get("recorded_at_utc") for sample in eligible]
        recorded_sample_times = row.get("sample_recorded_at_utc")
        metrics = {
            "median_working_set_bytes": median_int([int(sample["total_working_set_bytes"]) for sample in eligible]),
            "p95_working_set_bytes": p95_int([int(sample["total_working_set_bytes"]) for sample in eligible]),
            "median_private_memory_bytes": median_int([int(sample["total_private_memory_bytes"]) for sample in eligible]),
            "p95_private_memory_bytes": p95_int([int(sample["total_private_memory_bytes"]) for sample in eligible]),
            "median_handle_count": median_int([int(sample["total_handle_count"]) for sample in eligible]),
            "p95_handle_count": p95_int([int(sample["total_handle_count"]) for sample in eligible]),
            "median_thread_count": median_int([int(sample["total_thread_count"]) for sample in eligible]),
            "p95_thread_count": p95_int([int(sample["total_thread_count"]) for sample in eligible]),
            "median_process_count": median_int([len(sample.get("processes", [])) for sample in eligible]),
            "p95_process_count": p95_int([len(sample.get("processes", [])) for sample in eligible]),
        } if len(eligible) >= 6 else {}
        role_window = bounded_process_role_window(eligible)
        selected = logical.get("selected_run_id")
        selected_exact = selected is None if expected_results == 0 else isinstance(selected, str) and bool(selected)
        effective_attestation = artifact_attestation(root, {
            "path": effective.get("path"), "size": effective.get("bytes"), "sha256": effective.get("sha256"),
        })
        snapshot_attestation = process_resource_snapshot_attestation(
            root,
            effective,
            expected_results=expected_results,
            expected_witnesses=expected_witnesses,
            expected_selected_run_id=selected,
        )
        snapshot_attestations.append(snapshot_attestation)
        checkpoint_contract = checkpoint_contract and (
            row.get("name") == expected_name
            and row.get("idle_settle_milliseconds") == 5_000
            and row.get("capture_delay_milliseconds") == 500
            and row.get("sample_window_milliseconds") == 5_000
            and row.get("sample_count") == len(eligible) >= 6
            and recorded_sample_times == sample_times
            and all(row.get(name) == value for name, value in metrics.items())
            and row.get("process_role_counts") == role_window.get("modal_role_counts")
            and row.get("process_roles_bounded_and_terminally_stable") is True
            and role_window.get("passed") is True
            and row.get("process_role_window") == role_window
            and logical == {
                "surface": expected_surface, "completed_result_count": expected_results,
                "witness_count": expected_witnesses, "selected_run_id": selected,
                "state_kind": expected_kind,
            }
            and selected_exact
            and effective_logical.get("manifestValid") is True
            and effective_logical.get("completedResultCount") == expected_results
            and effective_logical.get("witnessCount") == expected_witnesses
            and effective_logical.get("selectedRunId") == selected
            and effective_attestation.get("passed") is True
            and snapshot_attestation.get("passed") is True
            and phase is not None and window_start == phase + timedelta(milliseconds=500)
            and window_end == phase + timedelta(milliseconds=5500)
            and (previous_phase is None or phase > previous_phase)
        )
        diagnostic = checkpoint_diagnostics[index] if index < len(checkpoint_diagnostics) else None
        checkpoint_diagnostic_contract = checkpoint_diagnostic_contract and diagnostic == {
            "name": expected_name,
            "passed": True,
            "phase_present": True,
            "phase_recorded_at_utc": row.get("phase_recorded_at_utc"),
            "window_start_utc": row.get("window_start_utc"),
            "window_end_utc": row.get("window_end_utc"),
            "eligible_sample_recorded_at_utc": sample_times,
            "eligible_sample_count": len(eligible),
            "expected_idle_settle_milliseconds": 5_000,
            "actual_idle_settle_milliseconds": 5_000,
            "expected_capture_delay_milliseconds": 500,
            "actual_capture_delay_milliseconds": 500,
            "expected_sample_window_milliseconds": 5_000,
            "actual_sample_window_milliseconds": 5_000,
            "minimum_samples": 6,
            "failure_reasons": [],
        }
        if phase is not None:
            previous_phase = phase
    process_roles_bounded_and_terminally_stable = len(checkpoint_rows) == 5 and all(
        isinstance(row, dict)
        and row.get("process_roles_bounded_and_terminally_stable") is True
        and isinstance(row.get("process_role_window"), dict)
        and row["process_role_window"].get("passed") is True
        for row in checkpoint_rows
    )
    reported_phase_snapshot_descriptors = [
        {
            "path": row.get("effective_archive", {}).get("path"),
            "size": row.get("effective_archive", {}).get("bytes"),
            "sha256": row.get("effective_archive", {}).get("sha256"),
        }
        for row in checkpoint_rows if isinstance(row, dict)
        and isinstance(row.get("effective_archive"), dict)
    ]
    reported_phase_source_paths = [
        row.get("effective_archive", {}).get("source_path")
        for row in checkpoint_rows if isinstance(row, dict)
        and isinstance(row.get("effective_archive"), dict)
    ]
    expected_phase_source_paths = [
        archive.get("path"), archive.get("path"),
        f"{archive.get('path')}.autosave" if isinstance(archive.get("path"), str) else None,
        f"{archive.get('path')}.autosave" if isinstance(archive.get("path"), str) else None,
        f"{resource_reset.get('reset_path')}.autosave"
        if isinstance(resource_reset.get("reset_path"), str) else None,
    ]
    phase_entries = phase_document.get("phases") if isinstance(phase_document.get("phases"), dict) else {}
    phase_document_contract = (
        resource_phases_artifact.get("passed") is True
        and resource_document.get("phase_document") == raw_artifacts.get("resource_phases")
        and phase_document.get("schema_version") == 2
        and phase_document.get("feature_id") == FEATURE_ID
        and phase_document.get("method_version") == METHOD_VERSION
        and list(phase_entries) == [row[0] for row in checkpoint_contracts]
    )
    for index, (expected_name, *_rest) in enumerate(checkpoint_contracts):
        phase_entry = phase_entries.get(expected_name)
        checkpoint_row = checkpoint_rows[index] if index < len(checkpoint_rows) else None
        phase_document_contract = phase_document_contract and (
            isinstance(phase_entry, dict)
            and set(phase_entry) == {
                "recorded_at_utc", "idle_settle_milliseconds", "capture_delay_milliseconds",
                "sample_window_milliseconds", "logical_state", "effective_archive",
                "primary_archive", "export",
            }
            and isinstance(checkpoint_row, dict)
            and timestamp(phase_entry.get("recorded_at_utc"))
            == timestamp(checkpoint_row.get("phase_recorded_at_utc"))
            and phase_entry.get("idle_settle_milliseconds") == 5_000
            and phase_entry.get("capture_delay_milliseconds") == 500
            and phase_entry.get("sample_window_milliseconds") == 5_000
            and phase_entry.get("logical_state") == checkpoint_row.get("logical_state")
            and phase_entry.get("effective_archive") == checkpoint_row.get("effective_archive")
            and isinstance(phase_entry.get("primary_archive"), dict)
            and set(phase_entry["primary_archive"]) == {"path", "bytes"}
            and isinstance(phase_entry.get("export"), dict)
            and set(phase_entry["export"]) == {"path", "bytes"}
        )
    phase_snapshot_contract = (
        len(phase_snapshot_artifacts) == 5
        and len(snapshot_attestations) == 5
        and all(row.get("passed") is True for row in phase_snapshot_artifacts)
        and all(row.get("passed") is True for row in snapshot_attestations)
        and raw_phase_snapshots == reported_phase_snapshot_descriptors
        and resource_document.get("phase_snapshots") == reported_phase_snapshot_descriptors
        and len({row.get("path") for row in reported_phase_snapshot_descriptors}) == 5
        and reported_phase_source_paths == expected_phase_source_paths
        and [phase_entries.get(name, {}).get("effective_archive") for name, *_ in checkpoint_contracts]
        == [row.get("effective_archive") for row in checkpoint_rows if isinstance(row, dict)]
    )
    cleanup_path = report_path.parent / PROCESS_CLEANUP_REPORT
    cleanup = load_optional(cleanup_path)
    first_sample = resources.get("first_sample") if isinstance(resources.get("first_sample"), dict) else {}
    resource_first_sample = resource_document.get("first_sample") if isinstance(resource_document.get("first_sample"), dict) else {}
    first_sample_processes = first_sample.get("processes") if isinstance(first_sample.get("processes"), list) else []
    raw_first_sample = raw_samples[0] if raw_samples and isinstance(raw_samples[0], dict) else {}
    raw_first_processes = raw_first_sample.get("processes") if isinstance(raw_first_sample.get("processes"), list) else []
    canonical_first_role_counts = {
        role: sum(
            1 for process in raw_first_processes
            if isinstance(process, dict) and process.get("role") == role
        )
        for role in role_names
    }
    expected_first_sample = {
        **raw_first_sample,
        "process_role_counts": canonical_first_role_counts,
    } if raw_first_sample else {}
    first_sample_contract = (
        first_sample == resource_first_sample
        and first_sample == cleanup.get("resource_monitor_first_sample")
        and first_sample == expected_first_sample
        and first_sample.get("root_present") is True
        and first_sample.get("root_pid") == cleanup.get("launched_pid")
        and isinstance(first_sample.get("total_working_set_bytes"), int)
        and not isinstance(first_sample.get("total_working_set_bytes"), bool)
        and first_sample["total_working_set_bytes"] > 0
        and isinstance(first_sample.get("total_private_memory_bytes"), int)
        and not isinstance(first_sample.get("total_private_memory_bytes"), bool)
        and first_sample["total_private_memory_bytes"] > 0
        and isinstance(first_sample.get("total_handle_count"), int)
        and first_sample["total_handle_count"] > 0
        and isinstance(first_sample.get("total_thread_count"), int)
        and first_sample["total_thread_count"] > 0
        and isinstance(first_sample.get("process_role_counts"), dict)
        and set(first_sample["process_role_counts"]) == set(role_names)
        and first_sample["process_role_counts"] == canonical_first_role_counts
        and first_sample["process_role_counts"].get("desktop_root") == 1
        and len(first_sample_processes) >= 1
        and sum(
            row.get("working_set_bytes", 0)
            for row in first_sample_processes
            if isinstance(row, dict) and isinstance(row.get("working_set_bytes"), int)
            and not isinstance(row.get("working_set_bytes"), bool)
        ) == first_sample["total_working_set_bytes"]
        and sum(row.get("private_memory_bytes", 0) for row in first_sample_processes if isinstance(row, dict))
        == first_sample["total_private_memory_bytes"]
        and sum(row.get("handle_count", 0) for row in first_sample_processes if isinstance(row, dict))
        == first_sample["total_handle_count"]
        and sum(row.get("thread_count", 0) for row in first_sample_processes if isinstance(row, dict))
        == first_sample["total_thread_count"]
        and all(
            isinstance(row, dict) and row.get("role") in role_names
            and isinstance(row.get("private_memory_bytes"), int) and row["private_memory_bytes"] > 0
            and isinstance(row.get("handle_count"), int) and row["handle_count"] > 0
            and isinstance(row.get("thread_count"), int) and row["thread_count"] > 0
            for row in first_sample_processes
        )
        and any(
            isinstance(row, dict) and row.get("pid") == cleanup.get("launched_pid")
            and isinstance(row.get("working_set_bytes"), int) and row["working_set_bytes"] > 0
            for row in first_sample_processes
        )
        and timestamp(first_sample.get("recorded_at_utc")) is not None
    )
    source_report = root / str(report.get("source_report", ""))
    source_document = load_optional(source_report)
    source_checks = source_document.get("checks") if isinstance(source_document.get("checks"), dict) else {}
    source_runtime_preflight = source_checks.get("runtimePreflight") if isinstance(source_checks.get("runtimePreflight"), dict) else {}
    source_results = source_checks.get("processV2Results") if isinstance(source_checks.get("processV2Results"), dict) else {}
    source_cancelled_retry_setup = source_checks.get("processV2CancelledRetrySetup") if isinstance(source_checks.get("processV2CancelledRetrySetup"), dict) else {}
    source_save_reopen = source_checks.get("processV2SaveReopen") if isinstance(source_checks.get("processV2SaveReopen"), dict) else {}
    source_repeated_completion = source_checks.get("processV2RepeatedCompletion") if isinstance(source_checks.get("processV2RepeatedCompletion"), dict) else {}
    source_resource_reset = source_checks.get("processV2ResourceResetClone") if isinstance(source_checks.get("processV2ResourceResetClone"), dict) else {}
    source_completed_ids_before = source_repeated_completion.get("completedRunIdsBefore")
    source_completed_ids_after = source_repeated_completion.get("completedRunIdsAfter")
    source_added_run_ids = source_repeated_completion.get("addedRunIds")
    repeated_completion_identity_bound = (
        source_repeated_completion.get("passed") is True
        and source_repeated_completion.get("activeLifecycleCaptured") is True
        and isinstance(source_repeated_completion.get("priorRunId"), str)
        and bool(source_repeated_completion["priorRunId"])
        and isinstance(source_repeated_completion.get("repeatedRunId"), str)
        and bool(source_repeated_completion["repeatedRunId"])
        and source_repeated_completion["priorRunId"] != source_repeated_completion["repeatedRunId"]
        and isinstance(source_completed_ids_before, list)
        and len(source_completed_ids_before) == 1
        and all(isinstance(value, str) and bool(value) for value in source_completed_ids_before)
        and len(set(source_completed_ids_before)) == len(source_completed_ids_before)
        and source_completed_ids_before == [source_repeated_completion["priorRunId"]]
        and isinstance(source_completed_ids_after, list)
        and len(source_completed_ids_after) == len(source_completed_ids_before) + 1
        and all(isinstance(value, str) and bool(value) for value in source_completed_ids_after)
        and len(set(source_completed_ids_after)) == len(source_completed_ids_after)
        and set(source_completed_ids_before).issubset(source_completed_ids_after)
        and isinstance(source_added_run_ids, list)
        and source_added_run_ids == [source_repeated_completion["repeatedRunId"]]
        and set(source_completed_ids_after) - set(source_completed_ids_before)
        == {source_repeated_completion["repeatedRunId"]}
        and source_repeated_completion.get("completedRunCountBefore") == len(source_completed_ids_before)
        and source_repeated_completion.get("completedRunCount") == len(source_completed_ids_after)
        and source_repeated_completion.get("uniqueCompletedRunCount") == len(set(source_completed_ids_after))
        and source_repeated_completion.get("autoSelectedRunId") == source_repeated_completion["repeatedRunId"]
        and source_repeated_completion.get("explicitlySelectedRunId") == source_repeated_completion["repeatedRunId"]
        and source_repeated_completion.get("initialSelectedTable") == "process_model_summary"
    )
    reset_original_artifact = artifact_attestation(root, resource_reset.get("original_archive"))
    reset_clone_artifact = artifact_attestation(root, resource_reset.get("reset_archive"))
    source_reset_identity = source_resource_reset.get("identity") if isinstance(source_resource_reset.get("identity"), dict) else {}
    source_reset_logical = source_resource_reset.get("logicalState") if isinstance(source_resource_reset.get("logicalState"), dict) else {}
    def settled_autosave_attestation(
        value: Any,
        *,
        primary_path: str | None,
        primary_durability: bool,
        expected_logical_state: dict[str, Any],
        require_capture: bool,
    ) -> dict[str, Any]:
        if not isinstance(value, dict):
            return {"passed": False, "artifacts": []}
        autosave_path = f"{primary_path}.autosave" if isinstance(primary_path, str) else None
        required = [autosave_path, f"{autosave_path}.identity.json" if autosave_path else None]
        allowed = sorted([
            *required,
            f"{autosave_path}.bak" if autosave_path else None,
            *(
                [f"{primary_path}.bak", f"{primary_path}.identity.json"]
                if primary_durability and primary_path else []
            ),
        ]) if autosave_path else []
        artifacts = [
            artifact_attestation(root, row) for row in value.get("artifacts", [])
        ] if isinstance(value.get("artifacts"), list) else []
        reported_artifacts = value.get("artifacts") if isinstance(value.get("artifacts"), list) else []
        present = value.get("present") if isinstance(value.get("present"), list) else []
        captures = value.get("capturedArtifacts") if isinstance(value.get("capturedArtifacts"), list) else []
        captured_artifacts = [
            artifact_attestation(root, row.get("snapshot")) for row in captures if isinstance(row, dict)
        ]
        capture_contract = (
            not require_capture
            or (
                len(captures) == len(reported_artifacts)
                and len(captured_artifacts) == len(captures)
                and all(row.get("passed") is True for row in captured_artifacts)
                and len({row.get("path") for row in captured_artifacts}) == len(captured_artifacts)
                and all(
                    isinstance(capture, dict) and isinstance(source, dict)
                    and capture.get("source_path") == source.get("path")
                    and capture.get("source_size") == source.get("size")
                    and capture.get("source_sha256") == source.get("sha256")
                    and attested.get("actual_size") == source.get("size")
                    and attested.get("actual_sha256") == source.get("sha256")
                    for capture, source, attested in zip(captures, reported_artifacts, captured_artifacts)
                )
            )
        )
        reported_descriptor_shape = (
            len(reported_artifacts) == len(present)
            and all(
                isinstance(row, dict) and set(row) == {"path", "size", "sha256"}
                and isinstance(row.get("path"), str) and row["path"].startswith("validation/results/")
                and isinstance(row.get("size"), int) and not isinstance(row.get("size"), bool)
                and row["size"] > 0 and isinstance(row.get("sha256"), str)
                and re.fullmatch(r"[0-9a-f]{64}", row["sha256"]) is not None
                for row in reported_artifacts
            )
        )
        return {
            "artifacts": artifacts, "captured_artifacts": captured_artifacts,
            "passed": (
                value.get("prefix") == primary_path
                and value.get("coversEverySiblingPrefix") is True
                and value.get("required") == required
                and value.get("allowed") == allowed
                and value.get("missing") == [] and value.get("forbidden") == []
                and value.get("exactAllowedIdentity") is True
                and value.get("autosavePath") == autosave_path
                and value.get("present") == [row.get("path") for row in reported_artifacts]
                and set(required).issubset(present) and set(present).issubset(allowed)
                and len(artifacts) == len(present)
                and reported_descriptor_shape
                and (require_capture or all(row.get("passed") is True for row in artifacts))
                and value.get("logicalState") == expected_logical_state
                and capture_contract
            ),
        }

    cycle1_expected_logical = checkpoint_rows[2].get("effective_archive", {}).get("logical_state") if len(checkpoint_rows) > 2 else {}
    cycle1_settled_source = source_save_reopen.get("settledAutosave")
    cycle1_after_source = source_save_reopen.get("autosaveAfterCheckpoint")
    cycle1_settled_packaged = save_reopen.get("cycle_1_settled_autosave")
    cycle1_after_packaged = save_reopen.get("cycle_1_autosave_after_checkpoint")
    cycle1_autosave_attestations = [
        settled_autosave_attestation(
            value, primary_path=archive.get("path"), primary_durability=True,
            expected_logical_state=cycle1_expected_logical, require_capture=True,
        )
        for value in (
            cycle1_settled_source, cycle1_after_source,
            cycle1_settled_packaged, cycle1_after_packaged,
        )
    ]
    cycle1_autosave_contract = (
        all(row.get("passed") is True for row in cycle1_autosave_attestations)
        and cycle1_settled_source == cycle1_after_source
        and cycle1_settled_packaged == cycle1_after_packaged
        and cycle1_settled_source == cycle1_settled_packaged
    )
    cycle1_capture_paths = [
        root / row["path"]
        for row in cycle1_autosave_attestations[0].get("captured_artifacts", [])
        if isinstance(row.get("path"), str)
    ] if cycle1_autosave_attestations else []
    reset_expected_logical = checkpoint_rows[-1].get("effective_archive", {}).get("logical_state") if checkpoint_rows else {}
    settled_reset_autosave = settled_autosave_attestation(
        source_resource_reset.get("settledAutosave"),
        primary_path=resource_reset.get("reset_path"), primary_durability=False,
        expected_logical_state=reset_expected_logical, require_capture=False,
    )
    reset_autosave_after_checkpoint = settled_autosave_attestation(
        source_resource_reset.get("autosaveAfterCheckpoint"),
        primary_path=resource_reset.get("reset_path"), primary_durability=False,
        expected_logical_state=reset_expected_logical, require_capture=False,
    )
    resource_reset_contract = (
        resource_reset.get("passed") is True and source_resource_reset.get("passed") is True
        and reset_original_artifact.get("passed") is True and reset_clone_artifact.get("passed") is True
        and reset_original_artifact.get("path") == archive.get("path")
        and reset_original_artifact.get("actual_size") == reset_clone_artifact.get("actual_size")
        and reset_original_artifact.get("actual_sha256") == reset_clone_artifact.get("actual_sha256")
        and resource_reset.get("original_path") == reset_original_artifact.get("path")
        and resource_reset.get("reset_path") == reset_clone_artifact.get("path")
        and resource_reset.get("distinct_path") is True
        and resource_reset.get("result_id") == resource_reset.get("run_id")
        and resource_reset.get("result_id") == source_reset_identity.get("resultId")
        and resource_reset.get("recipe_id") == source_reset_identity.get("recipeId")
        and resource_reset.get("run_id") == source_reset_identity.get("runId")
        and resource_reset.get("completed_result_count") == source_reset_logical.get("completedResultCount") == 1
        and resource_reset.get("witness_count") == source_reset_logical.get("witnessCount") == 1
        and all(resource_reset.get(name) is True for name in (
            "no_sidecars_before_copy", "no_sidecars_after_copy", "no_sidecars_before_open",
            "settled_autosave_sidecars_exact", "autosave_sidecars_stable_after_checkpoint",
            "recovery_disclosure_absent",
        ))
        and resource_reset.get("table_ids") == EXPECTED_TABLE_IDS
        and resource_reset.get("selected_run_id") == resource_reset.get("run_id")
        and resource_reset.get("selected_table_id") == "process_model_summary"
        and source_resource_reset.get("resetTableIds") == EXPECTED_TABLE_IDS
        and source_resource_reset.get("selectedRunId") == resource_reset.get("run_id")
        and source_resource_reset.get("selectedTableId") == "process_model_summary"
        and source_resource_reset.get("sidecarsBeforeCopy", {}).get("present") == []
        and source_resource_reset.get("sidecarsAfterCopy", {}).get("present") == []
        and source_resource_reset.get("sidecarsBeforeOpen", {}).get("present") == []
        and settled_reset_autosave.get("passed") is True
        and reset_autosave_after_checkpoint.get("passed") is True
        and source_resource_reset.get("settledAutosave", {}).get("artifacts")
        == source_resource_reset.get("autosaveAfterCheckpoint", {}).get("artifacts")
        and source_resource_reset.get("recoveryDisclosureAbsent") is True
        and resource_reset.get("source_check") == "processV2ResourceResetClone"
    )
    source_export = source_checks.get("processV2Export") if isinstance(source_checks.get("processV2Export"), dict) else {}
    source_native_xlsx = source_export.get("nativeXlsx") if isinstance(source_export.get("nativeXlsx"), dict) else {}
    resource_generated = timestamp(resource_document.get("generated_at_utc"))
    cleanup_generated = timestamp(cleanup.get("generated_at_utc"))
    cleanup_fresh = cleanup_path.is_file() and report_path.is_file() and cleanup_path.stat().st_mtime_ns >= report_path.stat().st_mtime_ns
    cleanup_passed = (
        cleanup.get("passed") is True
        and isinstance(cleanup.get("launched_pid"), int) and not isinstance(cleanup.get("launched_pid"), bool)
        and cleanup["launched_pid"] > 0 and cleanup.get("parent_exit_confirmed") is True
        and cleanup.get("lingering_descendant_pids") == []
        and cleanup.get("graceful_close_exit_code") == 0
        and cleanup.get("graceful_exit_confirmed") is True
        and cleanup.get("forced_parent_termination") is False
        and cleanup.get("forced_descendant_pids") == []
        and cleanup.get("forced_resource_monitor_termination") is False
        and isinstance(cleanup.get("resource_monitor_pid"), int)
        and not isinstance(cleanup.get("resource_monitor_pid"), bool)
        and cleanup["resource_monitor_pid"] > 0
        and cleanup.get("resource_monitor_exit_confirmed") is True
        and cleanup.get("resource_monitor_exit_code") == 0
        and cleanup.get("resource_monitor_stderr") in (None, "")
        and cleanup.get("resource_monitor_terminal_reason") == "stop_signal"
        and cleanup.get("resource_monitor_first_sample") == first_sample
        and cleanup_fresh
    )
    checkpoint_by_name = {
        row.get("name"): row for row in checkpoint_rows if isinstance(row, dict) and isinstance(row.get("name"), str)
    }
    initial_checkpoint = checkpoint_by_name.get("initial_idle", {})
    cancellation_checkpoint = checkpoint_by_name.get("post_cancellation_idle", {})
    cycle1_checkpoint = checkpoint_by_name.get("post_completed_cycle_1_idle", {})
    history_checkpoint = checkpoint_by_name.get("post_completed_history_2_idle", {})
    cycle2_checkpoint = checkpoint_by_name.get("post_completed_cycle_2_idle", {})
    memory = resource_document.get("memory") if isinstance(resource_document.get("memory"), dict) else {}
    cancel_ws_tolerance = max(134_217_728, math.ceil(initial_checkpoint.get("median_working_set_bytes", 0) * 0.35))
    cancel_private_tolerance = max(134_217_728, math.ceil(initial_checkpoint.get("median_private_memory_bytes", 0) * 0.35))
    equal_ws_tolerance = max(67_108_864, math.ceil(cycle1_checkpoint.get("median_working_set_bytes", 0) * 0.10))
    equal_private_tolerance = max(67_108_864, math.ceil(cycle1_checkpoint.get("median_private_memory_bytes", 0) * 0.10))
    cancellation_within = (
        cancellation_checkpoint.get("median_working_set_bytes", math.inf)
        <= initial_checkpoint.get("median_working_set_bytes", 0) + cancel_ws_tolerance
        and cancellation_checkpoint.get("median_private_memory_bytes", math.inf)
        <= initial_checkpoint.get("median_private_memory_bytes", 0) + cancel_private_tolerance
    )
    equal_ws_within = cycle2_checkpoint.get("median_working_set_bytes", math.inf) <= (
        cycle1_checkpoint.get("median_working_set_bytes", 0) + equal_ws_tolerance
    )
    equal_private_within = cycle2_checkpoint.get("median_private_memory_bytes", math.inf) <= (
        cycle1_checkpoint.get("median_private_memory_bytes", 0) + equal_private_tolerance
    )
    equal_handles_within = cycle2_checkpoint.get("median_handle_count", math.inf) <= (
        cycle1_checkpoint.get("median_handle_count", 0) + 64
    )
    equal_threads_within = cycle2_checkpoint.get("median_thread_count", math.inf) <= (
        cycle1_checkpoint.get("median_thread_count", 0) + 16
    )
    equal_roles_exact = cycle1_checkpoint.get("process_role_counts") == cycle2_checkpoint.get("process_role_counts")
    retained_history_disclosure = {
        "checkpoint": "post_completed_history_2_idle",
        "median_working_set_bytes": history_checkpoint.get("median_working_set_bytes"),
        "median_private_memory_bytes": history_checkpoint.get("median_private_memory_bytes"),
        "completed_result_count": history_checkpoint.get("logical_state", {}).get("completed_result_count"),
        "witness_count": history_checkpoint.get("logical_state", {}).get("witness_count"),
        "qualification_role": "disclosure_only_not_a_threshold",
    }
    peak_ws = max((int(row.get("total_working_set_bytes", 0)) for row in raw_samples), default=0)
    peak_private = max((int(row.get("total_private_memory_bytes", 0)) for row in raw_samples), default=0)
    memory_contract = memory == {
        "policy": "bounded_equal_logical_state_window_median_v2",
        "peak_working_set_bytes": peak_ws,
        "peak_private_memory_bytes": peak_private,
        "peak_working_set_under_2_gib": 0 < peak_ws < 2_147_483_648,
        "cancellation_working_set_tolerance_bytes": cancel_ws_tolerance,
        "cancellation_private_memory_tolerance_bytes": cancel_private_tolerance,
        "cancellation_within_baseline_tolerance": cancellation_within,
        "equal_state_working_set_tolerance_bytes": equal_ws_tolerance,
        "equal_state_private_memory_tolerance_bytes": equal_private_tolerance,
        "equal_state_working_set_within_tolerance": equal_ws_within,
        "equal_state_private_memory_within_tolerance": equal_private_within,
        "equal_state_handle_tolerance": 64,
        "equal_state_thread_tolerance": 16,
        "equal_state_handle_count_within_tolerance": equal_handles_within,
        "equal_state_thread_count_within_tolerance": equal_threads_within,
        "equal_state_process_roles_exact": equal_roles_exact,
        "process_roles_bounded_and_terminally_stable": process_roles_bounded_and_terminally_stable,
        "retained_history_disclosure": retained_history_disclosure,
        "phase_snapshots_attested": phase_snapshot_contract,
        "phase_document_attested": phase_document_contract,
        "conclusion": "bounded_post_replacement_recovery_v2",
        "cancellation_cycle_count": 1,
        "completed_cycle_count": 2,
        "idle_checkpoint_count": 5,
        "idle_settle_milliseconds": 5_000,
        "idle_checkpoints_ordered_and_distinct": checkpoint_contract,
        "capture_delay_milliseconds": 500,
        "sample_window_milliseconds": 5_000,
        "minimum_samples_per_checkpoint": 6,
        "checkpoint_diagnostic_count": len(checkpoint_diagnostics),
        "checkpoint_diagnostics_all_passed": checkpoint_diagnostic_contract,
    }
    expected_resource_summary = {
        "passed": True,
        "sample_count": len(raw_samples),
        "raw_sample_count": len(raw_samples),
        "first_sample": first_sample,
        "monitor_terminal_reason": "stop_signal",
        "peak_working_set_bytes": peak_ws,
        "peak_private_memory_bytes": peak_private,
        "peak_working_set_under_2_gib": 0 < peak_ws < 2_147_483_648,
        "policy": "bounded_equal_logical_state_window_median_v2",
        "cancellation_within_baseline_tolerance": cancellation_within,
        "equal_state_working_set_within_tolerance": equal_ws_within,
        "equal_state_private_memory_within_tolerance": equal_private_within,
        "equal_state_handle_count_within_tolerance": equal_handles_within,
        "equal_state_thread_count_within_tolerance": equal_threads_within,
        "equal_state_process_roles_exact": equal_roles_exact,
        "process_roles_bounded_and_terminally_stable": process_roles_bounded_and_terminally_stable,
        "retained_history_disclosure": retained_history_disclosure,
        "phase_snapshots_attested": phase_snapshot_contract,
        "phase_document_attested": phase_document_contract,
        "conclusion": "bounded_post_replacement_recovery_v2",
        "cancellation_cycle_count": 1,
        "completed_cycle_count": 2,
        "idle_checkpoint_count": 5,
        "idle_settle_milliseconds": 5_000,
        "idle_checkpoints_ordered_and_distinct": checkpoint_contract,
        "capture_delay_milliseconds": 500,
        "sample_window_milliseconds": 5_000,
        "minimum_samples_per_checkpoint": 6,
        "checkpoint_diagnostic_count": len(checkpoint_diagnostics),
        "checkpoint_diagnostics_all_passed": checkpoint_diagnostic_contract,
        "artifact_disk_deltas_recorded": resource_disk_contract,
        "zero_lingering_descendants": cleanup.get("lingering_descendant_pids") == [],
        "graceful_exit_confirmed": cleanup.get("graceful_exit_confirmed") is True,
        "parent_absent": cleanup.get("parent_exit_confirmed") is True,
        "forced_parent_termination": cleanup.get("forced_parent_termination"),
        "forced_descendant_pids": cleanup.get("forced_descendant_pids"),
        "forced_resource_monitor_termination": cleanup.get("forced_resource_monitor_termination"),
        "source_check": "processV2Resources",
    }

    def packaged_page_state(state: Any) -> dict[str, Any] | None:
        if not isinstance(state, dict):
            return None
        return {
            "index": state.get("index"),
            "url": state.get("url"),
            "origin": state.get("origin"),
            "title": state.get("title"),
            "shell_visible": state.get("shellVisible") is True,
            "tauri_runtime": state.get("tauriRuntime") is True,
        }

    source_enumerated_pages = source_runtime_preflight.get("enumeratedPages")
    source_enumerated_pages = source_enumerated_pages if isinstance(source_enumerated_pages, list) else []
    expected_runtime_preflight = {
        "passed": source_runtime_preflight.get("passed") is True,
        "expected_origin": source_runtime_preflight.get("expectedOrigin"),
        "enumerated_pages": [packaged_page_state(row) for row in source_enumerated_pages],
        "qualifying_page_count": source_runtime_preflight.get("qualifyingPageCount"),
        "pre_reload": packaged_page_state(source_runtime_preflight.get("preReload")),
        "reload_count": source_runtime_preflight.get("reloadCount"),
        "post_reload": packaged_page_state(source_runtime_preflight.get("postReload")),
        "same_origin": source_runtime_preflight.get("sameOrigin") is True,
        "source_check": "runtimePreflight",
    }
    enumerated_pages = runtime_preflight.get("enumerated_pages")
    enumerated_pages = enumerated_pages if isinstance(enumerated_pages, list) else []
    qualifying_pages = [
        row for row in enumerated_pages
        if isinstance(row, dict) and row.get("shell_visible") is True and row.get("tauri_runtime") is True
    ]
    pre_reload = runtime_preflight.get("pre_reload") if isinstance(runtime_preflight.get("pre_reload"), dict) else {}
    post_reload = runtime_preflight.get("post_reload") if isinstance(runtime_preflight.get("post_reload"), dict) else {}
    packaged_url_pattern = re.compile(r"^http://tauri\.localhost(?:[/?#].*)?$")
    runtime_preflight_contract = (
        runtime_preflight == expected_runtime_preflight
        and runtime_preflight.get("passed") is True
        and runtime_preflight.get("expected_origin") == PACKAGED_TAURI_ORIGIN
        and runtime_preflight.get("qualifying_page_count") == 1
        and len(qualifying_pages) == 1
        and pre_reload in enumerated_pages
        and pre_reload == qualifying_pages[0]
        and runtime_preflight.get("reload_count") == 1
        and runtime_preflight.get("same_origin") is True
        and pre_reload.get("origin") == PACKAGED_TAURI_ORIGIN
        and post_reload.get("origin") == PACKAGED_TAURI_ORIGIN
        and pre_reload.get("origin") == post_reload.get("origin")
        and pre_reload.get("shell_visible") is True and pre_reload.get("tauri_runtime") is True
        and post_reload.get("shell_visible") is True and post_reload.get("tauri_runtime") is True
        and isinstance(pre_reload.get("url"), str)
        and packaged_url_pattern.fullmatch(pre_reload["url"]) is not None
        and isinstance(post_reload.get("url"), str)
        and packaged_url_pattern.fullmatch(post_reload["url"]) is not None
        and [row.get("index") for row in enumerated_pages if isinstance(row, dict)]
        == list(range(len(enumerated_pages)))
        and runtime_preflight.get("source_check") == "runtimePreflight"
    )

    source_focused_run = source_document.get("focusedRun") if isinstance(source_document.get("focusedRun"), dict) else {}
    source_screenshots = source_document.get("screenshots") if isinstance(source_document.get("screenshots"), list) else []
    source_screenshot_paths: list[str] = []
    for value in source_screenshots:
        if not isinstance(value, str):
            source_screenshot_paths.append("")
            continue
        try:
            source_screenshot_paths.append(Path(value).resolve().relative_to(root.resolve()).as_posix())
        except (OSError, ValueError):
            source_screenshot_paths.append(value.replace("\\", "/"))
    packaged_screenshot_paths = [row.get("path") for row in screenshots]
    process_source_isolated = (
        frozenset(source_checks) == PROCESS_SOURCE_CHECKS
        and source_focused_run.get("scope") == "process_v2"
        and source_focused_run.get("priorGeneratedAt") is None
        and source_focused_run.get("completedAt") == report.get("completed_at_utc")
        and source_document.get("generatedAt") == report.get("generated_at_utc")
        and source_document.get("consoleErrors") == [] and source_document.get("failures") == []
        and source_checks.get("recentProjectsRestored") is True
        and len(source_screenshot_paths) >= 5
        and len(source_screenshot_paths) == len(set(source_screenshot_paths))
        and all(re.search(r"/(?:18[0-9])-tauri-native-process-v2-", row, re.IGNORECASE) for row in source_screenshot_paths)
        and source_screenshot_paths == packaged_screenshot_paths
    )
    freshness = report_freshness(
        root,
        report_path,
        PACKAGED_SOURCES,
        [
            root / "target/release/quickpls-desktop.exe",
            *[path for path in (root / "dist").rglob("*") if path.is_file()],
            source_report,
            *[
                root / row["path"] for row in reported_phase_snapshot_descriptors
                if isinstance(row.get("path"), str)
            ],
            *cycle1_capture_paths,
        ],
    )
    checks = {
        "draft202012_schema_valid": schema_path.is_file() and schema_errors == [],
        "utf8_text_integrity": mojibake_matches(json.dumps(report, ensure_ascii=False)) == [],
        "identity": (
            report.get("schema_version") == "quickpls.packaged_acceptance.v1"
            and report.get("kind") == "quickpls3_scoped_tauri_process_v2_acceptance"
            and report.get("feature_id") == FEATURE_ID and report.get("method_version") == METHOD_VERSION
            and report.get("bootstrap_method_version") == BOOTSTRAP_METHOD_VERSION
            and report.get("catalogue_snapshot_date") == CATALOGUE_SNAPSHOT_DATE
            and report.get("target") == "windows_10_11_x64_packaged_tauri"
            and report.get("runtime") == "tauri-webview2-cdp"
            and report.get("generator") == "validation/v247_tauri_native_acceptance.mjs"
            and report.get("source_report") == PACKAGED_SOURCE_REPORT
            and isinstance(report.get("endpoint"), str)
            and re.fullmatch(r"http://127\.0\.0\.1:[0-9]+", report["endpoint"]) is not None
        ),
        "passed": report.get("passed") is True,
        "ordered_timestamps": generated is not None and completed is not None and completed >= generated,
        "exact_checks": frozenset(checks_obj) == PACKAGED_CHECKS and all(isinstance(checks_obj[name], dict) and checks_obj[name].get("passed") is True for name in PACKAGED_CHECKS),
        "runtime_preflight_contract": runtime_preflight_contract,
        "workflow_contract": workflow == {
            "passed": True, "completed": True, "active_lifecycle_captured": True,
            "model_free": True, "graph_defined_without_numbered_templates": True,
            "source_check": "processV2Workflow",
        },
        "setup_contract": (
            setup.get("passed") is True and setup.get("outcome") == "Y"
            and setup.get("focal_predictor") == "X" and setup.get("paths") == 8
            and setup.get("top_level_predictors") == 7
            and setup.get("top_level_predictors_maximum") == 8
            and setup.get("moderators") == 2 and setup.get("moderations") == 3
            and setup.get("controls") == 1 and setup.get("bootstrap_replicates") == 10_000
            and setup.get("controls_maximum") == 1
            and setup.get("equation_non_intercept_terms_maximum") == 50
            and isinstance(setup.get("workers"), int) and not isinstance(setup.get("workers"), bool)
            and 1 <= setup["workers"] <= 64
            and isinstance(setup.get("seed"), int) and not isinstance(setup.get("seed"), bool)
            and 0 <= setup["seed"] <= 4_294_967_295
            and setup.get("source_check") == "processV2Setup"
        ),
        "exact_result_contract": (
            result_check.get("initial_selected_table") == "process_model_summary"
            and result_check.get("table_ids") == EXPECTED_TABLE_IDS
            and result_check.get("exact_table_ids") is True
            and result_check.get("expected_counts_source") == "validation/process_v2_reference.py:reference_graph"
            and result_check.get("expected_graph_counts") == expected_counts
            and result_check.get("equation_count") == expected_counts.get("equations")
            and result_check.get("reference_effect_rows") == expected_counts.get("referenceEffects")
            and result_check.get("conditional_indirect_rows") == expected_counts.get("conditionalIndirectEffects")
            and result_check.get("moderated_mediation_index_rows") == expected_counts.get("moderatedMediationIndices")
            and result_check.get("simple_slope_rows") == expected_counts.get("simpleSlopes")
            and result_check.get("conditional_plot_point_rows") == expected_counts.get("conditionalPlotPoints")
            and result_check.get("johnson_neyman_rows") == expected_counts.get("johnsonNeymanRegionRows")
            and result_check.get("johnson_neyman_analysis_count") == expected_counts.get("johnsonNeyman")
            and result_check.get("johnson_neyman_analysis_keys")
            == expected_johnson_neyman_analysis_keys
            and result_check.get("johnson_neyman_curve_point_rows") == expected_counts.get("johnsonNeymanCurvePoints")
            and result_check.get("bootstrap_estimand_rows") == expected_counts.get("estimands")
            and all(result_check.get(name) is True for name in (
                "passed", "failure_disclosure_truthful", "validation_witness_not_rendered",
                "no_na_fabrication", "generic_regression_shell_not_applicable",
                "accessible_non_color_plot_semantics", "reference_effect_columns_exact",
                "reference_condition_rows_exact", "candidate_promotion_warnings_exact",
                "curve_warning_disclosure_exact",
            ))
            and result_check.get("source_check") == "processV2Results"
        ),
        "xlsx_bound": xlsx["passed"] and export.get("artifact_sha256") == xlsx["actual_sha256"],
        "archive_bound": archive["passed"] and save_reopen.get("archive_sha256") == archive["actual_sha256"],
        "screenshots_bound": len(screenshots) >= 5 and all(row["passed"] for row in screenshots),
        "resource_report_bound": resource_artifact["passed"],
        "resource_samples_bound": resource_samples_artifact["passed"],
        "resource_phases_bound": resource_phases_artifact["passed"] and phase_document_contract,
        "resource_phase_snapshots_bound": phase_snapshot_contract,
        "artifact_paths_unique": len({
            row.get("path") for row in [
                xlsx, archive, resource_artifact, resource_samples_artifact, resource_phases_artifact,
                *phase_snapshot_artifacts,
                *cycle1_autosave_attestations[0].get("captured_artifacts", []),
                *screenshots,
            ]
        }) == 10 + len(cycle1_autosave_attestations[0].get("captured_artifacts", [])) + len(screenshots),
        "witness_scan_fail_closed": (
            scan.get("passed") is True and scan.get("extraction_errors") == []
            and scan.get("forbidden_matches") == [] and bool(scan.get("worksheet_members"))
            and xlsx_scan["passed"] and xlsx_scan.get("mojibake_matches") == []
            and scan.get("scanned_xml_and_rels_members") == xlsx_scan["scanned_xml_and_rels_members"]
            and scan.get("worksheet_members") == xlsx_scan["worksheet_members"]
            and scan.get("worksheet_row_counts") == xlsx_scan["worksheet_row_counts"]
        ),
        "xlsx_process_table_contract_independently_verified": (
            process_table_contract == {
                "passed": True,
                "reference_sheet": "Reference effects",
                "reference_columns": ["Effect ID", "Kind", "Path", "Estimate", "Reference condition"],
                "reference_effect_rows": expected_counts.get("referenceEffects"),
                "reference_condition": REFERENCE_CONDITION,
                "candidate_status": "experimental",
                "promotion_pending_warning": PROCESS_PROMOTION_PENDING_WARNING,
                "curve_warning_disclosure": PROCESS_CURVE_WARNING_DISCLOSURE,
                "curve_warning_disclosure_exact": True,
                "required_shared_strings_verified": True,
            }
            and xlsx_scan.get("process_table_contract", {}).get("passed") is True
            and xlsx_scan["process_table_contract"].get("reference_columns")
            == ["Effect ID", "Kind", "Path", "Estimate", "Reference condition"]
            and xlsx_scan["process_table_contract"].get("reference_effect_rows")
            == expected_counts.get("referenceEffects")
            and xlsx_scan["process_table_contract"].get("reference_condition_values")
            == [REFERENCE_CONDITION] * expected_counts.get("referenceEffects", 0)
            and xlsx_scan["process_table_contract"].get("curve_warning_disclosure_exact") is True
            and xlsx_scan["process_table_contract"].get("run_provenance_warning_absent") is True
            and all(
                xlsx_scan["process_table_contract"].get("status_by_sheet", {}).get(name)
                == "experimental"
                for name in EXPECTED_WORKBOOK_SHEETS
            )
            and all(
                (xlsx_scan["process_table_contract"].get("warning_by_sheet", {}).get(name) or "").startswith(
                    PROCESS_PROMOTION_PENDING_WARNING
                )
                for name in EXPECTED_WORKBOOK_SHEETS[:-1]
            )
            and xlsx_scan["process_table_contract"].get("warning_by_sheet", {}).get("Run provenance")
            in (None, "")
        ),
        "export_contract": (
            export.get("passed") is True and export.get("validation_witness_excluded") is True
            and export.get("source_check") == "processV2Export"
            and export.get("workbook_sheets") == EXPECTED_WORKBOOK_SHEETS
            and process_table_contract.get("passed") is True
        ),
        "save_reopen_contract": (
            save_reopen.get("passed") is True
            and save_reopen.get("same_run_restored") is True
            and save_reopen.get("initial_selected_table") == "process_model_summary"
            and save_reopen.get("project_checksum_matches") is True
            and save_reopen.get("archive_witness_validated") is True
            and save_reopen.get("archive_sha256") == archive["actual_sha256"]
            and save_reopen.get("source_check") == "processV2SaveReopen"
            and cycle1_autosave_contract
        ),
        "archive_witness_independently_verified": archive_scan["passed"],
        "cancellation_contract": cancellation == {
            "passed": True, "active_lifecycle_captured": True,
            "no_partial_result": True, "source_check": "processV2Cancellation",
        },
        "cancelled_retry_setup_contract": (
            cancelled_retry_setup.get("passed") is True
            and cancelled_retry_setup.get("read_only") is True
            and cancelled_retry_setup.get("exact_frozen_setup_match") is True
            and isinstance(cancelled_retry_setup.get("snapshot"), dict)
            and cancelled_retry_setup.get("snapshot") == cancelled_retry_setup.get("frozen_setup")
            and cancelled_retry_setup["snapshot"].get("outcome") == "Y"
            and cancelled_retry_setup["snapshot"].get("focalPredictor") == "X"
            and cancelled_retry_setup["snapshot"].get("paths") == [
                {"from": "X", "to": "Y"}, {"from": "X", "to": "M1"},
                {"from": "M1", "to": "M2"}, {"from": "M2", "to": "Y"},
                {"from": "X", "to": "M3"}, {"from": "M3", "to": "Y"},
                {"from": "X", "to": "M4"}, {"from": "M4", "to": "Y"},
            ]
            and cancelled_retry_setup["snapshot"].get("moderators") == [
                {"variable": "W", "scale": "continuous"},
                {"variable": "B", "scale": "binary_0_1"},
            ]
            and cancelled_retry_setup["snapshot"].get("moderations") == [
                {"edge": "X -> Y", "primary": "W", "conditioning": "B"},
                {"edge": "X -> M3", "primary": "W", "conditioning": ""},
                {"edge": "M4 -> Y", "primary": "B", "conditioning": ""},
            ]
            and cancelled_retry_setup["snapshot"].get("selectedControls") == ["C"]
            and cancelled_retry_setup["snapshot"].get("samples") == str(setup.get("bootstrap_replicates"))
            and cancelled_retry_setup["snapshot"].get("workers") == str(setup.get("workers"))
            and cancelled_retry_setup["snapshot"].get("seed") == str(setup.get("seed"))
            and cancelled_retry_setup["snapshot"].get("blockers") == []
            and cancelled_retry_setup["snapshot"].get("profileAriaBusy") == "false"
            and cancelled_retry_setup["snapshot"].get("startEnabled") is True
            and cancelled_retry_setup.get("source_check") == "processV2CancelledRetrySetup"
        ),
        "witness_boundary_contract": witness_boundary == {
            "passed": True, "archive_only": True, "witness_method_version": WITNESS_VERSION,
            "estimand_order_exact": True,
            "bootstrap_index_partition_exact": True, "jackknife_index_partition_exact": True,
            "excluded_from_results": True, "excluded_from_exports": True,
            "source_check": "processV2WitnessBoundary",
        },
        "resource_reset_contract": resource_reset_contract,
        "resource_contract": (
            resources == expected_resource_summary
            and first_sample_contract and checkpoint_contract and checkpoint_diagnostic_contract
            and phase_snapshot_contract and phase_document_contract and memory_contract
            and process_roles_bounded_and_terminally_stable
            and cancellation_within and equal_ws_within and equal_private_within
            and equal_handles_within and equal_threads_within and equal_roles_exact
            and 0 < peak_ws < 2_147_483_648
            and cleanup_passed and resource_disk_contract and raw_samples_parse_ok
        ),
        "resource_report_independently_verified": (
            resource_document.get("schema_version") == 1
            and resource_document.get("target") == "process_v2_packaged_resource_report"
            and resource_document.get("feature_id") == FEATURE_ID
            and resource_document.get("method_version") == METHOD_VERSION
            and resource_document.get("launched_pid") == cleanup.get("launched_pid")
            and resource_document.get("sample_interval_milliseconds") == 250
            and resource_document.get("capture_delay_milliseconds") == 500
            and resource_document.get("sample_window_milliseconds") == 5_000
            and resource_document.get("sample_count") == resources.get("sample_count")
            and resource_document.get("raw_sample_count") == resources.get("raw_sample_count")
            and resource_document.get("monitor_terminal_reason") == "stop_signal"
            and first_sample_contract
            and resource_document.get("raw_samples") == raw_artifacts.get("resource_samples")
            and resource_document.get("phase_document") == raw_artifacts.get("resource_phases")
            and resource_document.get("phase_snapshots") == raw_artifacts.get("resource_phase_snapshots")
            and resource_samples_artifact.get("passed") is True
            and resource_phases_artifact.get("passed") is True
            and phase_snapshot_contract
            and phase_document_contract
            and memory_contract
            and checkpoint_contract
            and checkpoint_diagnostic_contract
            and resource_disk_contract
            and resource_document.get("process_cleanup") == {
                "graceful_close_exit_code": 0,
                "graceful_exit_confirmed": True,
                "forced_parent_termination": False,
                "forced_descendant_pids": [],
                "forced_resource_monitor_termination": False,
                "parent_exit_confirmed": True,
                "lingering_descendant_pids": [],
                "resource_monitor_exit_confirmed": True,
                "resource_monitor_exit_code": 0,
                "resource_monitor_stderr": "",
                "resource_monitor_terminal_reason": "stop_signal",
            }
            and resource_document.get("passed") is True
        ),
        "resource_cleanup_timestamps_ordered": (
            completed is not None and resource_generated is not None and cleanup_generated is not None
            and resource_generated >= completed and cleanup_generated >= resource_generated
        ),
        "desktop_bound_fresh": desktop["passed"],
        "provisioning_cli_bound_fresh": cli["passed"],
        "dist_bound": dist == actual_dist and actual_dist["file_count"] > 0,
        "source_report_bound": (
            source_document.get("passed") is True
            and source_document.get("feature_id") == FEATURE_ID
            and source_document.get("method_version") == METHOD_VERSION
            and source_document.get("bootstrap_method_version") == BOOTSTRAP_METHOD_VERSION
            and source_document.get("catalogue_snapshot_date") == CATALOGUE_SNAPSHOT_DATE
            and source_document.get("runtime") == "tauri-webview2-cdp"
            and process_source_isolated
            and source_runtime_preflight.get("passed") is True
            and mojibake_matches(json.dumps(source_document, ensure_ascii=False)) == []
            and all(source_checks.get(name, {}).get("passed") is True for name in (
                "processV2Workflow", "processV2Setup", "processV2Results", "processV2Export",
                "processV2SaveReopen", "processV2Cancellation", "processV2CancelledRetrySetup",
                "processV2WitnessBoundary",
            ))
            and source_results.get("referenceEffectColumnsExact") is True
            and source_results.get("referenceConditionRowsExact") is True
            and source_results.get("candidatePromotionWarningsExact") is True
            and source_results.get("curveWarningDisclosureExact") is True
            and source_results.get("johnsonNeymanRows")
            == expected_counts.get("johnsonNeymanRegionRows")
            and source_results.get("johnsonNeymanAnalysisCount")
            == expected_counts.get("johnsonNeyman")
            and source_results.get("johnsonNeymanAnalysisKeys")
            == expected_johnson_neyman_analysis_keys
            and source_cancelled_retry_setup.get("passed") is True
            and source_cancelled_retry_setup.get("readOnly") is True
            and source_cancelled_retry_setup.get("exactFrozenSetupMatch") is True
            and source_cancelled_retry_setup.get("snapshot") == cancelled_retry_setup.get("snapshot")
            and source_cancelled_retry_setup.get("frozenSetup") == cancelled_retry_setup.get("frozen_setup")
            and source_native_xlsx.get("processTableContract") == process_table_contract
            and repeated_completion_identity_bound
        ),
        "exact_pid_cleanup_confirmed": cleanup_passed,
        "zero_console_errors_failures": report.get("console_errors") == [] and report.get("failures") == [],
        "fresh_report": freshness["passed"],
    }
    return {
        "checks": checks,
        "artifacts": {"xlsx": xlsx, "xlsx_scan": xlsx_scan, "project_archive": archive, "archive_scan": archive_scan, "resource_report": resource_artifact, "resource_samples": resource_samples_artifact, "resource_phases": resource_phases_artifact, "resource_phase_snapshots": phase_snapshot_artifacts, "resource_phase_snapshot_scans": snapshot_attestations, "screenshots": screenshots},
        "tested_product": {"cli": cli, "desktop": desktop, "dist": {"reported": dist, "actual": actual_dist, "passed": dist == actual_dist}},
        "schema_validation": {
            "path": "validation/process_v2_packaged_acceptance.schema.json",
            "draft": "2020-12", "errors": schema_errors, "passed": schema_errors == [],
        },
        "process_cleanup": {"path": cleanup_path.relative_to(root).as_posix(), "document": cleanup, "fresh": cleanup_fresh, "passed": cleanup_passed},
        "report_freshness": freshness, "passed": all(checks.values()),
    }


def visual_attestation(root: Path, report: dict[str, Any], report_path: Path) -> dict[str, Any]:
    rows = report.get("checks", {}).get("processV2", []) if isinstance(report.get("checks"), dict) else []
    rows = rows if isinstance(rows, list) else []
    expected_options = [
        {"value": "ols", "label": "Ordinary least squares"},
        {"value": "logistic", "label": "Binary logistic (outcome coded 0/1)"},
        {"value": "process", "label": "Graph-defined Path Analysis / PROCESS"},
    ]
    def row_passes(row: Any) -> bool:
        if not isinstance(row, dict): return False
        setup = row.get("setup") if isinstance(row.get("setup"), dict) else {}
        a11y = row.get("accessibility") if isinstance(row.get("accessibility"), dict) else {}
        truth = row.get("truthAndOverflow") if isinstance(row.get("truthAndOverflow"), dict) else {}
        bounds = row.get("dialogBounds") if isinstance(row.get("dialogBounds"), dict) else {}
        stable = setup.get("stableRowIdentity") if isinstance(setup.get("stableRowIdentity"), dict) else {}
        expected_row_orders = {
            "paths": [[f"nd-process-path-from-{index}", f"nd-process-path-to-{index}"] for index in range(8)],
            "moderators": [[f"nd-process-moderator-variable-{index}", f"nd-process-moderator-scale-{index}"] for index in range(2)],
            "moderations": [[
                f"nd-process-moderation-edge-{index}", f"nd-process-moderation-primary-{index}",
                f"nd-process-moderation-conditioning-{index}",
            ] for index in range(3)],
        }
        stable_rows_pass = stable.get("passed") is True
        for name, expected_order in expected_row_orders.items():
            group = stable.get(name) if isinstance(stable.get(name), dict) else {}
            mutations = group.get("mutations") if isinstance(group.get("mutations"), list) else []
            enabled = [mutation for mutation in mutations if isinstance(mutation, dict) and mutation.get("disabled") is False]
            disabled = [mutation for mutation in mutations if isinstance(mutation, dict) and mutation.get("disabled") is True]
            stable_rows_pass = stable_rows_pass and (
                group.get("passed") is True
                and group.get("initialRowOrder") == expected_order
                and group.get("finalRowOrder") == expected_order
                and group.get("rowCount") == len(expected_order)
                and group.get("selectCount") == sum(len(row_ids) for row_ids in expected_order)
                and group.get("enabledSelectCount") == len(enabled) > 0
                and group.get("disabledSelectCount") == len(disabled)
                and group.get("changedSelectCount") == len(enabled)
                and len(mutations) == group.get("selectCount")
                and all(
                    mutation.get("changed") is True
                    and mutation.get("focusedBefore") is True
                    and mutation.get("focusedAfterChange") is True
                    and mutation.get("rowOrderUnchanged") is True
                    for mutation in enabled
                )
                and all(
                    mutation.get("changed") is False and mutation.get("rowOrderUnchanged") is True
                    for mutation in disabled
                )
            )
        return (
            row.get("fixture") == {"variables": 9, "models": 0}
            and row.get("dataSurface") is True and row.get("dialogOpened") is True
            and row.get("regressionTypeOptions") == expected_options and row.get("regressionType") == "process"
            and setup.get("outcome") == "Y" and setup.get("focal") == "X"
            and setup.get("pathRows") == 8 and setup.get("moderatorRows") == 2 and setup.get("moderationRows") == 3
            and setup.get("pathsExact") is True and setup.get("moderatorsExact") is True
            and setup.get("moderationsExact") is True
            and setup.get("selectedControls") == ["C"]
            and setup.get("capacity") == {
                "topLevelPredictors": 7, "topLevelPredictorsMaximum": 8,
                "controls": 1, "controlsMaximum": 1,
                "equationNonInterceptTermsMaximum": 50,
            }
            and setup.get("bootstrap") == "enabled" and setup.get("samples") == "10000"
            and setup.get("samplesBounds") == {"min": "99", "max": "10000", "step": "1"}
            and isinstance(setup.get("workers"), str) and setup["workers"].isdigit()
            and 1 <= int(setup["workers"]) <= 64
            and setup.get("workersBounds") == {"min": "1", "max": "64", "step": None}
            and isinstance(setup.get("seed"), str) and setup["seed"].isdigit()
            and 0 <= int(setup["seed"]) <= 4_294_967_295
            and setup.get("seedBounds") == {"min": "0", "max": "4294967295", "step": None}
            and setup.get("startLabel") == "Start graph-defined path analysis with bootstrap"
            and setup.get("startDisabledInBrowserPreview") is True
            and isinstance(setup.get("runtimeBlockers"), list) and len(setup["runtimeBlockers"]) == 1
            and setup.get("unexpectedBlockers") == [] and setup.get("profileReady") is True
            and setup.get("scopeExact") is True and setup.get("bootstrapScopeExact") is True
            and setup.get("previewExact") is True and setup.get("previewAccessible") is True
            and stable_rows_pass
            and all(a11y.get(name) is True for name in ("controlsLabeled", "groupsNamed", "keyboardReachable", "focusRestored"))
            and truth.get("noFabricatedRunState") is True and truth.get("noHorizontalOverflow") is True
            and bounds.get("withinHorizontalViewport") is True and bounds.get("pageHorizontalOverflow") is False
            and row.get("completedResult", {}).get("synthesizedByHarness") is False
        )
    generated = timestamp(report.get("generatedAt") or report.get("generated_at_utc"))
    freshness = report_freshness(
        root,
        report_path,
        VISUAL_SOURCES,
        [path for path in (root / "dist").rglob("*") if path.is_file()],
    )
    checks = {
        "overall_passed": report.get("passed") is True,
        "utf8_text_integrity": mojibake_matches(json.dumps(report, ensure_ascii=False)) == [],
        "zero_failures_console": report.get("failures") == [] and report.get("consoleErrors") == [],
        "exact_three_viewports": len(rows) == 3 and {row.get("viewport") for row in rows if isinstance(row, dict)} == VISUAL_VIEWPORTS,
        "all_contracts": len(rows) == 3 and all(row_passes(row) for row in rows),
        "fresh_sources_and_dist": generated is not None and freshness["passed"],
    }
    return {"checks": checks, "source_freshness": freshness["rows"], "passed": all(checks.values())}


def boundary_attestation(root: Path, report: dict[str, Any], report_path: Path) -> dict[str, Any]:
    checks_obj = report.get("checks") if isinstance(report.get("checks"), dict) else {}
    executions = report.get("executions") if isinstance(report.get("executions"), dict) else {}
    expected_build_commands = {
        target: [
            "cargo", "test", "--release", "-p", target.replace("_", "-"),
            "--lib", "--no-run", "--message-format=json",
        ]
        for target in BOUNDARY_SUITES
    }
    descriptors = report.get("test_executables") if isinstance(report.get("test_executables"), dict) else {}
    target_sources = BOUNDARY_PRODUCT_SOURCES
    binaries = {target: binary_attestation(root, descriptors.get(target), descriptors.get(target, {}).get("path", ""), target_sources[target]) for target in BOUNDARY_SUITES}
    executable_paths = {
        target: descriptors.get(target, {}).get("path") if isinstance(descriptors.get(target), dict) else None
        for target in BOUNDARY_SUITES
    }
    executable_locations = all(
        isinstance(path, str)
        and re.fullmatch(rf"target/release/deps/{target}-[0-9a-f]+(?:\.exe)?", path) is not None
        for target, path in executable_paths.items()
    )
    freshness = report_freshness(
        root,
        report_path,
        ["validation/process_v2_boundary_gate.py", *sorted({path for rows in target_sources.values() for path in rows})],
        [root / str(descriptor.get("path", "")) for descriptor in descriptors.values() if isinstance(descriptor, dict)],
    )
    exact_sets = all(frozenset(checks_obj.get(target, {})) == expected and all(checks_obj[target].get(name) is True for name in expected) for target, expected in BOUNDARY_SUITES.items())
    expected_all = frozenset(name for names in BOUNDARY_SUITES.values() for name in names)
    executions_pass = frozenset(executions) == expected_all and all(executions[name].get("passed") is True and executions[name].get("exit_code") == 0 and "1 passed; 0 failed" in executions[name].get("stdout_tail", "") for name in expected_all)
    checks = {
        "identity": report.get("target") == "process_v2_focused_rust_boundary_tests" and report.get("feature_id") == FEATURE_ID and report.get("method_version") == METHOD_VERSION,
        "passed": report.get("passed") is True,
        "exact_named_checks": exact_sets,
        "exact_serial_release_build_commands": report.get("build_commands") == expected_build_commands,
        "single_cargo_job_policy": report.get("environment") == {"CARGO_BUILD_JOBS": "1"},
        "exact_executions_pass": executions_pass,
        "executables_bound_fresh": all(row["passed"] for row in binaries.values()),
        "release_test_executable_locations": executable_locations,
        "fresh_report": freshness["passed"],
    }
    return {"checks": checks, "binaries": binaries, "report_freshness": freshness, "passed": all(checks.values())}


def frontend_attestation(root: Path, report: dict[str, Any], report_path: Path) -> dict[str, Any]:
    source_rows = report.get("source_artifacts")
    source_rows = source_rows if isinstance(source_rows, list) else []
    source_paths = [row.get("path") for row in source_rows if isinstance(row, dict)]
    artifacts = [artifact_attestation(root, row) for row in source_rows]
    vitest = report.get("vitest") if isinstance(report.get("vitest"), dict) else {}
    tsc = report.get("tsc") if isinstance(report.get("tsc"), dict) else {}
    test_rows = vitest.get("test_files") if isinstance(vitest.get("test_files"), list) else []
    test_paths = [row.get("path") for row in test_rows if isinstance(row, dict)]
    total_assertions = sum(
        row.get("assertions", 0)
        for row in test_rows
        if isinstance(row, dict) and isinstance(row.get("assertions"), int)
        and not isinstance(row.get("assertions"), bool)
    )
    expected_vitest_command = ["npx", "vitest", "run", *FRONTEND_TEST_FILES, "--reporter=json"]
    expected_tsc_command = ["npx", "tsc", "-b", "--pretty", "false"]
    freshness = report_freshness(root, report_path, FRONTEND_GATE_SOURCES)
    checks = {
        "identity": (
            report.get("schema_version") == 1
            and report.get("target") == "process_v2_focused_frontend_gate"
            and report.get("feature_id") == FEATURE_ID
            and report.get("method_version") == METHOD_VERSION
        ),
        "passed": report.get("passed") is True,
        "exact_commands": (
            report.get("commands") == {
                "vitest": expected_vitest_command,
                "tsc": expected_tsc_command,
            }
        ),
        "exact_test_manifest": (
            report.get("test_files") == list(FRONTEND_TEST_FILES)
            and len(test_paths) == len(set(test_paths)) == len(FRONTEND_TEST_FILES)
            and frozenset(test_paths) == frozenset(FRONTEND_TEST_FILES)
        ),
        "every_vitest_file_and_assertion_passed": (
            vitest.get("passed") is True and vitest.get("exit_code") == 0
            and len(test_rows) == len(FRONTEND_TEST_FILES)
            and all(
                isinstance(row, dict) and row.get("status") == "passed"
                and isinstance(row.get("assertions"), int) and not isinstance(row.get("assertions"), bool)
                and row["assertions"] > 0
                and row.get("passed_assertions") == row["assertions"]
                and row.get("failed_assertions") == 0
                for row in test_rows
            )
            and isinstance(vitest.get("total_tests"), int)
            and not isinstance(vitest.get("total_tests"), bool)
            and vitest["total_tests"] == total_assertions > 0
            and vitest.get("passed_tests") == vitest["total_tests"]
            and vitest.get("failed_tests") == 0
            and vitest.get("pending_tests") == 0
        ),
        "typescript_project_check_passed": tsc.get("passed") is True and tsc.get("exit_code") == 0,
        "exact_source_digest_manifest": (
            report.get("source_stable_during_gate") is True
            and source_paths == FRONTEND_GATE_SOURCES
            and len(artifacts) == len(FRONTEND_GATE_SOURCES)
            and all(row["passed"] for row in artifacts)
        ),
        "fresh_report": freshness["passed"],
    }
    return {
        "checks": checks,
        "source_artifacts": artifacts,
        "report_freshness": freshness,
        "passed": all(checks.values()),
    }


def documentation_attestation(root: Path) -> dict[str, Any]:
    path = root / "docs" / "methods" / "PROCESS_V2.md"
    text = path.read_text(encoding="utf-8") if path.is_file() else ""
    required = [
        METHOD_VERSION, BOOTSTRAP_METHOD_VERSION, "graph-defined", "numbered PROCESS templates",
        "HC3", "Johnson-Neyman", "percentile", "BCa", "listwise",
        "Binary/logistic outcomes", "weights", "clusters", "studentized", "regression_process_v1",
        "high_leverage_hc3_instability", "invalid_hc3_covariance",
        "degenerate_simple_slope_variance", "thin SVD", "64 * machine_epsilon",
        "128 * machine_epsilon", "three distinct finite", "original complete-sample raw mean",
        "experimental",
    ]
    checks = {phrase: phrase.lower() in text.lower() for phrase in required}
    references = {identifier: identifier in text for identifier in DOCUMENTATION_REFERENCE_IDENTIFIERS}
    return {
        "path": "docs/methods/PROCESS_V2.md", "present": path.is_file(),
        "required_phrases": checks, "reference_identifiers": references,
        "passed": path.is_file() and all(checks.values()) and all(references.values()),
    }


def scientific_source_attestation(root: Path) -> dict[str, Any]:
    resampling = root / "crates/qpls-resampling/src/lib.rs"
    estimation = root / "crates/qpls-estimation/src/pls.rs"
    resampling_text = resampling.read_text(encoding="utf-8") if resampling.is_file() else ""
    estimation_text = estimation.read_text(encoding="utf-8") if estimation.is_file() else ""
    required_resampling = [
        "summarize_process_bootstrap_estimands", "zero_bootstrap_standard_error",
        "incomplete_jackknife", "zero_jackknife_variance", "nonfinite_adjusted_probability",
        "regression_process_bootstrap_validation_witness_v1", "high_leverage_hc3_instability",
        "invalid_hc3_covariance", "degenerate_simple_slope_variance",
        "process_indexed_case_stream_v1",
    ]
    required_estimation = [
        "process_bootstrap_estimands_at_reference",
        "equation_complete_case_mean_v1", "hc3_v1", "student_t_residual_df_v1",
        "high_leverage_hc3_instability",
        "invalid_hc3_covariance", "degenerate_simple_slope_variance",
        "process_scale_aware_ols", "PROCESS_RELATIVE_RANK_TOLERANCE_MULTIPLIER",
        "process_johnson_neyman_coded_roots",
        "PROCESS_JN_COEFFICIENT_TOLERANCE_MULTIPLIER",
        "PROCESS_JN_ROOT_DEDUP_TOLERANCE_MULTIPLIER",
        "binary_process_equation_outcome", "collapsed_process_probe_grid",
        JN_INVALID_COVARIANCE_MESSAGE,
    ]
    checks = {
        **{f"resampling:{token}": token in resampling_text for token in required_resampling},
        **{f"estimation:{token}": token in estimation_text for token in required_estimation},
    }
    return {"checks": checks, "passed": all(checks.values())}


def build_audit(root: Path = ROOT, results: Path | None = None) -> dict[str, Any]:
    results = results or root / "validation" / "results"
    reference_report = load_optional(results / REFERENCE_REPORT)
    packaged_report = load_optional(results / PACKAGED_REPORT)
    visual_report = load_optional(results / VISUAL_REPORT)
    boundary_report = load_optional(results / BOUNDARY_REPORT)
    frontend_report = load_optional(results / FRONTEND_REPORT)
    reference = reference_attestation(root, reference_report, results / REFERENCE_REPORT)
    packaged = packaged_attestation(root, packaged_report, results / PACKAGED_REPORT, reference_report)
    visual = visual_attestation(root, visual_report, results / VISUAL_REPORT)
    boundary = boundary_attestation(root, boundary_report, results / BOUNDARY_REPORT)
    frontend = frontend_attestation(root, frontend_report, results / FRONTEND_REPORT)
    docs = documentation_attestation(root)
    source = scientific_source_attestation(root)
    text_integrity = text_integrity_attestation(root)
    checks = [
        {"name": "independent_numerical_reference", "passed": reference["passed"]},
        {"name": "focused_rust_boundaries_executed", "passed": boundary["passed"]},
        {"name": "focused_frontend_contracts_and_typecheck", "passed": frontend["passed"]},
        {"name": "browser_visual_acceptance", "passed": visual["passed"]},
        {"name": "genuine_packaged_vertical_slice", "passed": packaged["passed"]},
        {"name": "frozen_scope_documented", "passed": docs["passed"]},
        {"name": "scientific_seams_and_tokens_present", "passed": source["passed"]},
        {"name": "process_utf8_text_integrity", "passed": text_integrity["passed"]},
    ]
    return {
        "schema_version": 1,
        "integrity_contract": "process_v2_exact_current_reference_visual_packaged_boundary_binding_v1",
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "target": TARGET, "feature_id": FEATURE_ID, "method_version": METHOD_VERSION,
        "bootstrap_method_version": BOOTSTRAP_METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "passed": all(row["passed"] for row in checks),
        "reference_report": reference_report, "packaged_acceptance": packaged_report,
        "browser_visual_acceptance": visual_report, "boundary_report": boundary_report,
        "frontend_gate_report": frontend_report,
        "reference_attestation": reference, "packaged_attestation": packaged,
        "visual_attestation": visual, "boundary_attestation": boundary,
        "frontend_attestation": frontend,
        "docs": docs, "scientific_source": source,
        "text_integrity": text_integrity, "checks": checks,
        "note": "Historical regression_process_v1 evidence remains archive-readable but cannot satisfy any PROCESS v2 promotion condition.",
    }


def main() -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    report = build_audit()
    output = RESULTS / OUTPUT
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {output} | passed={report['passed']}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
