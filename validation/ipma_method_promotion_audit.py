#!/usr/bin/env python3
"""Native promotion gate for the canonical bounded IPMA v1 workflow.

Estimator/reference evidence is necessary but not sufficient. Promotion also
requires exact source contracts for native setup/results and project
persistence plus genuine packaged-Tauri run, export, save, and reopen evidence.
The current gate is expected to remain false until that packaged evidence is
captured; browser fixtures or synthetic completed results cannot satisfy it.
"""

from __future__ import annotations

import json
from pathlib import Path

from second_batch_promotion_common import ROOT, audit_method


RESULTS = ROOT / "validation" / "results"
METHOD_VERSION = "ipma_v1"
PERFORMANCE_SCALE = "min_max_0_100_from_standardized_scores_v1"
PROVENANCE_METHOD_VERSION = "pls_pm_v1+ipma_v1+pls_mediation_v1+pls_assessment_v7"
EXPECTED_PREDECESSORS = ["x", "z", "m"]
EXPECTED_INDICATORS = ["x1", "z1", "m1"]


def load_json(path: Path) -> dict:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def source_contains(path: str, *snippets: str) -> bool:
    source = ROOT / path
    if not source.exists():
        return False
    text = source.read_text(encoding="utf-8")
    return all(snippet in text for snippet in snippets)


reference = load_json(RESULTS / "ipma_reference_report.json")
reference_checks = reference.get("checks", {})
reference_contract_passed = (
    reference.get("passed") is True
    and reference.get("method_version") == METHOD_VERSION
    and reference.get("performance_scale") == PERFORMANCE_SCALE
    and reference.get("provenance_method_version") == PROVENANCE_METHOD_VERSION
    and reference.get("scope")
    == {
        "kind": "predecessor_only_observed_sample_ipma",
        "selected_target": "y",
        "performance_range": [0.0, 100.0],
        "theoretical_range_correction": False,
        "cipma": False,
        "resampling_inference": False,
    }
    and reference.get("fixture", {}).get("expected_predecessor_constructs")
    == EXPECTED_PREDECESSORS
    and reference.get("fixture", {}).get("expected_predecessor_indicators")
    == EXPECTED_INDICATORS
    and reference.get("fixture", {}).get("excluded_target_and_unrelated_constructs")
    == ["y", "u", "v"]
    and reference.get("fixture", {}).get("excluded_target_and_unrelated_indicators")
    == ["y1", "u1", "v1"]
    and bool(reference_checks)
    and all(value is True for value in reference_checks.values())
)

canonical_engine_contract = (
    source_contains(
        "crates/qpls-core/src/validation.rs",
        "pub fn resolve_ipma_targets",
        "pub fn ipma_predecessor_constructs",
        "construct.id != target",
        '"ipma.path_weighting_required"',
        '"ipma.standardized_required"',
        '"ipma.listwise_required"',
        '"ipma.resampling_unsupported"',
    )
    and source_contains(
        "crates/qpls-estimation/src/pls.rs",
        f'pub const IPMA_METHOD_VERSION: &str = "{METHOD_VERSION}";',
        f'pub const IPMA_PERFORMANCE_SCALE: &str = "{PERFORMANCE_SCALE}";',
        "ipma_predecessor_constructs(recipe, target)",
        "direct and indirect structural predecessors only",
        "Theoretical-range performance and cIPMA are unsupported",
    )
    and source_contains(
        "crates/qpls-runner/src/lib.rs",
        "recipe.settings.method == qpls_core::AnalysisMethod::Ipma",
        "base_versions.insert(1, IPMA_METHOD_VERSION)",
        "{PLS_METHOD_VERSION}+{IPMA_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}",
    )
)

project_persistence_contract = source_contains(
    "crates/qpls-project/src/lib.rs",
    "AnalysisMethod::Ipma => Some(IPMA_METHOD_VERSION)",
    "validate_ipma_payload_contract",
    "ipma.performance_scale != IPMA_PERFORMANCE_SCALE",
    "ipma_predecessor_constructs(recipe, target)",
    "runner_generated_ipma_commits_saves_reopens_and_rejects_contract_tampering",
)

native_setup_and_results_contract = (
    source_contains(
        "src/native/nativeAnalysisCatalog.ts",
        '| "ipma"',
        'kind: "ipma"',
        'categoryId: "assessment"',
        "Map each structural predecessor's total importance against observed-range construct performance for one endogenous target.",
    )
    and source_contains(
        "src/native/nativeAnalysisRecipe.ts",
        'kind: "ipma"',
        'label: "Importance-Performance Map Analysis"',
        "validateIpmaModel",
        "requiredSingleTarget(settings.ipmaTargets)",
    )
    and source_contains(
        "src/native/nativePlsReadiness.ts",
        'settings.method === "ipma"',
        "Choose exactly one endogenous target construct",
        "0–100 observed-range scaling of standardized composite scores",
        "no theoretical-range correction is applied",
    )
    and source_contains(
        "src/native/nativeResults.ts",
        '"ipma_constructs"',
        '"ipma_indicators"',
        '"ipma_scope"',
        "Predecessor construct",
        "Theoretical-range correction",
        "NATIVE_IPMA_SCOPE_NOTE",
    )
    and source_contains(
        "src/native/nativeIpmaCanonicalIntegration.test.ts",
        METHOD_VERSION,
        PERFORMANCE_SCALE,
        PROVENANCE_METHOD_VERSION,
        'not.toContain("N/A")',
        "No theoretical-range correction is applied",
    )
)

tauri = load_json(RESULTS / "v247_tauri_native_acceptance.json")
tauri_checks = tauri.get("checks", {})
required_tauri_checks = [
    "ipmaFixtureProvisioning",
    "visibleIpmaModelBuild",
    "ipmaCalculationDialog",
    "ipmaRunning",
    "ipmaResult",
    "ipmaExport",
    "ipmaSaveReopen",
]
ipma_result = tauri_checks.get("ipmaResult", {})
ipma_export = tauri_checks.get("ipmaExport", {})
ipma_reopen = tauri_checks.get("ipmaSaveReopen", {})
ipma_navigation = ipma_result.get("navigation", {})
ipma_run_properties = ipma_result.get("runDetails", {}).get("properties", {})
ipma_archive = ipma_reopen.get("archive", {})
packaged_native_passed = (
    tauri.get("passed") is True
    and not tauri.get("failures")
    and not tauri.get("consoleErrors")
    and all(name in tauri_checks for name in required_tauri_checks)
    and tauri_checks.get("visibleIpmaModelBuild", {}).get("constructs") == 6
    and tauri_checks.get("visibleIpmaModelBuild", {}).get("assignedIndicators") == 6
    and tauri_checks.get("visibleIpmaModelBuild", {}).get("structuralPaths") == 6
    and tauri_checks.get("ipmaCalculationDialog", {}).get("predecessorOnly") is True
    and tauri_checks.get("ipmaCalculationDialog", {}).get("observedRange") is True
    and tauri_checks.get("ipmaCalculationDialog", {}).get("noCipmaOrInferenceClaim") is True
    and tauri_checks.get("ipmaCalculationDialog", {}).get("startEnabled") is True
    and tauri_checks.get("ipmaRunning", {}).get("status") in {"queued", "validating", "running"}
    and tauri_checks.get("ipmaRunning", {}).get("cancelVisible") is True
    and ipma_result.get("autoOpenedSurface") == "results"
    and ipma_result.get("autoOpenedTable") == "Construct importance and performance"
    and ipma_run_properties.get("Method version") == PROVENANCE_METHOD_VERSION
    and ipma_run_properties.get("Method") == "Importance-Performance Map Analysis"
    and "Recorded seed" not in ipma_run_properties
    and ipma_navigation.get("predecessorOnly") is True
    and ipma_navigation.get("excludesTargetAndUnrelatedConstructRows") is True
    and ipma_navigation.get("excludesTargetAndUnrelatedIndicatorRows") is True
    and ipma_navigation.get("noPlaceholderOrUnsupportedClaims") is True
    and ipma_navigation.get("constructLabels") == ["M", "X", "Z"]
    and ipma_navigation.get("indicatorLabels") == ["m1", "x1", "z1"]
    and ipma_navigation.get("constructs", {}).get("rowCount") == 3
    and ipma_navigation.get("indicators", {}).get("rowCount") == 3
    and ipma_navigation.get("scopeValues", {}).get("Method version") == METHOD_VERSION
    and ipma_export.get("ipmaTablesIncluded") is True
    and ipma_export.get("nativeXlsx", {}).get("attempted") is True
    and ipma_export.get("nativeXlsx", {}).get("file", {}).get("isFile") is True
    and ipma_reopen.get("sameRunRestored") is True
    and ipma_reopen.get("navigation", {}).get("predecessorOnly") is True
    and ipma_reopen.get("navigation", {}).get("noPlaceholderOrUnsupportedClaims") is True
    and ipma_archive.get("methodVersion") == PROVENANCE_METHOD_VERSION
    and ipma_archive.get("payloadMethodVersion") == METHOD_VERSION
    and ipma_archive.get("ipmaMethodVersion") == METHOD_VERSION
    and ipma_archive.get("performanceScale") == PERFORMANCE_SCALE
    and ipma_archive.get("constructRows") == 3
    and ipma_archive.get("indicatorRows") == 3
    and ipma_archive.get("actualPredecessors") == ipma_archive.get("expectedPredecessors")
    and ipma_archive.get("actualIndicatorConstructs") == ipma_archive.get("expectedPredecessors")
    and ipma_archive.get("indicators") == ["m1", "x1", "z1"]
    and ipma_archive.get("finiteConstructRows") is True
    and ipma_archive.get("finiteIndicatorRows") is True
    and ipma_archive.get("excludesTargetAndUnrelatedRows") is True
    and ipma_archive.get("forbiddenNestedPayloads") == []
    and ipma_archive.get("recipe", {}).get("method") == "ipma"
    and ipma_archive.get("recipe", {}).get("weightingScheme") == "path"
    and ipma_archive.get("recipe", {}).get("preprocessing") == "standardized"
    and ipma_archive.get("recipe", {}).get("missingData") == "listwise_deletion"
    and ipma_archive.get("recipe", {}).get("bootstrapSamples") == 0
    and ipma_archive.get("recipe", {}).get("studentizedInnerSamples") == 0
    and ipma_archive.get("recipe", {}).get("permutationSamples") == 0
    and ipma_archive.get("recipe", {}).get("caseWeightColumn") is None
    and ipma_archive.get("recipe", {}).get("constructs") == 6
    and ipma_archive.get("recipe", {}).get("paths") == 6
    and ipma_archive.get("recipe", {}).get("controls") == 0
    and ipma_archive.get("recipe", {}).get("interactions") == 0
    and ipma_archive.get("recipe", {}).get("higherOrderConstructs") == 0
)

no_cipma_overclaim = (
    source_contains(
        "docs/methods/IPMA_V1.md",
        "It is not cIPMA",
        "The target itself and unrelated constructs are excluded.",
        "observed complete-sample score ranges",
        "must not be presented as full SmartPLS IPMA parity",
    )
    and source_contains(
        "docs/METHOD_COMPATIBILITY.md",
        "Importance-Performance Map Analysis (IPMA)",
        "Theoretical-range correction, alternate SmartPLS representations, NCA integration, and cIPMA are unsupported.",
    )
)


raise SystemExit(audit_method(
    "ipma",
    "Bounded predecessor-only IPMA using exact PLS total effects and observed-sample 0-100 performance from listwise-standardized scores; theoretical-range correction, resampling inference, and cIPMA are excluded.",
    [
        "ipma_reference_report.json",
        "v247_tauri_native_acceptance.json",
    ],
    ["IPMA_V1.md"],
    [
        {
            "name": "canonical_independent_reference",
            "passed": reference_contract_passed,
            "detail": "The independent fixture must prove exact predecessor-only identities, target/unrelated exclusion, observed-sample 0-100 scaling, nested version, and full provenance version.",
        },
        {
            "name": "canonical_engine_contract",
            "passed": canonical_engine_contract,
            "detail": "Core, estimator, and runner sources must freeze target resolution, predecessor selection, fixed settings, scale identity, limitation warning, and exact IPMA provenance.",
        },
        {
            "name": "strict_project_persistence",
            "passed": project_persistence_contract,
            "detail": "qpls-project must validate exact IPMA rows, algebra, scale, versions, and unsupported-shape exclusions and must reject tampering across save/reopen.",
        },
        {
            "name": "native_setup_results_and_export_contract",
            "passed": native_setup_and_results_contract,
            "detail": "Native source and integration tests must expose one explicit endogenous target, predecessor-only tables/map, exact provenance, observed-range disclosure, and no fabricated N/A output.",
        },
        {
            "name": "packaged_native_workflow",
            "passed": packaged_native_passed,
            "detail": "A current packaged-Tauri artifact must prove a genuine IPMA setup/run, predecessor-only results, XLSX export, explicit save, and reopen. Source tests and browser fixtures cannot satisfy this check.",
        },
        {
            "name": "no_cipma_or_full_smartpls_overclaim",
            "passed": no_cipma_overclaim,
            "detail": "Current method and compatibility documentation must explicitly exclude theoretical-range correction, broader SmartPLS representations, NCA integration, resampling inference, and cIPMA.",
        },
    ],
))
