#!/usr/bin/env python3
"""Strict promotion gate for the bounded standalone NCA v2 workflow.

The legacy nca_v1 implementation is archive-readable but scientifically
superseded. This gate requires corrected independent numerical evidence,
executable recipe guards, exact runner provenance, strict project persistence,
no-model desktop snapshots, and genuine packaged-Tauri workflow evidence.
"""

from __future__ import annotations

import json
import math
import subprocess
import sys
from pathlib import Path

from second_batch_promotion_common import ROOT, audit_method


RESULTS = ROOT / "validation" / "results"
METHOD_VERSION = "nca_v2"
LEGACY_METHOD_VERSION = "nca_v1"
ASSESSMENT_VERSION = "assessment_not_applicable_v1"
ASSESSMENT_WARNING = "PLS assessment is not applicable to standalone raw-data analyses."
EXPECTED_SCOPE = {"minimum_x": 0.0, "maximum_x": 3.0, "minimum_y": 1.0, "maximum_y": 4.0}
EXPECTED_PEERS = [{"x": 0.0, "y": 1.0}, {"x": 1.0, "y": 3.0}, {"x": 3.0, "y": 4.0}]
EXPECTED_CE_EFFECT = 5.0 / 9.0
EXPECTED_CR_SLOPE = 13.0 / 14.0
EXPECTED_CR_INTERCEPT = 10.0 / 7.0
EXPECTED_CR_EFFECT = 36.0 / 91.0
EXPECTED_NCA_WARNING = (
    "NCA v2 is limited to the documented numeric X/Y CE-FDH and CR-FDH scope "
    "with observed-range bottlenecks; multiple conditions, latent-score NCA, "
    "cIPMA, and broader ceiling variants remain unsupported."
)
EXPECTED_DIALOG_SCOPE = (
    "Numeric observed-variable CE-FDH and CR-FDH analysis with observed-range "
    "bottlenecks. Multiple conditions, latent-score NCA, cIPMA, and broader "
    "ceiling variants are not included."
)
EXPECTED_CE_REQUIREMENTS = [100.0 / 3.0] * 6 + [100.0] * 3
EXPECTED_CR_REQUIREMENTS = [
    None,
    6.153846153846154,
    16.923076923076923,
    27.692307692307693,
    38.46153846153846,
    49.23076923076923,
    60.0,
    70.76923076923077,
    81.53846153846153,
]
EXPECTED_NCA_KEYS = sorted(
    [
        "bottlenecks",
        "ce_fdh_peers",
        "ceiling",
        "ceilings",
        "method_version",
        "observations",
        "permutation_samples",
        "scope",
        "usable_permutations",
        "warnings",
        "x",
        "y",
    ]
)
TOLERANCE = 1e-9


def load_json(path: Path) -> dict:
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return {}


def source_contains(path: str, *snippets: str) -> bool:
    source = ROOT / path
    if not source.exists():
        return False
    text = source.read_text(encoding="utf-8")
    return all(snippet in text for snippet in snippets)


def run(command: list[str], timeout: int = 480) -> dict:
    try:
        process = subprocess.run(
            command,
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return {
            "command": command,
            "returncode": process.returncode,
            "passed": process.returncode == 0,
            "stdout_tail": process.stdout[-2000:],
            "stderr_tail": process.stderr[-2000:],
        }
    except subprocess.TimeoutExpired as error:
        return {
            "command": command,
            "returncode": None,
            "passed": False,
            "stdout_tail": (error.stdout or "")[-2000:] if isinstance(error.stdout, str) else "",
            "stderr_tail": (error.stderr or "")[-2000:] if isinstance(error.stderr, str) else "",
            "timeout_seconds": timeout,
        }


def close(left, right, tolerance: float = TOLERANCE) -> bool:
    return (
        isinstance(left, (int, float))
        and not isinstance(left, bool)
        and math.isfinite(float(left))
        and abs(float(left) - float(right)) <= tolerance
    )


def value(source: dict, snake: str, camel: str):
    if not isinstance(source, dict):
        return None
    return source.get(snake, source.get(camel))


def scope_matches(scope: dict) -> bool:
    return isinstance(scope, dict) and all(
        close(value(scope, key, "".join([key.split("_")[0], *[part.title() for part in key.split("_")[1:]]])), expected)
        for key, expected in EXPECTED_SCOPE.items()
    )


def peers_match(peers) -> bool:
    return (
        isinstance(peers, list)
        and len(peers) == len(EXPECTED_PEERS)
        and all(
            isinstance(actual, dict)
            and close(actual.get("x"), expected["x"])
            and close(actual.get("y"), expected["y"])
            for actual, expected in zip(peers, EXPECTED_PEERS)
        )
    )


def zero_or_empty(value) -> bool:
    return value == 0 or value == []


def visible_number(value):
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def permutation_p_value(value, samples: int = 19) -> bool:
    return (
        close(value, value)
        and float(value) >= 1.0 / (samples + 1) - TOLERANCE
        and float(value) <= 1.0 + TOLERANCE
        and close(float(value) * (samples + 1), round(float(value) * (samples + 1)))
    )


def exact_row_keys(row: dict, expected: set[str]) -> bool:
    return isinstance(row, dict) and set(row) == expected


def exact_bottleneck_rows(rows) -> bool:
    if not isinstance(rows, list) or len(rows) != 18:
        return False
    expected_keys = {"ceiling", "outcome_percent", "required_x_percent", "status"}
    ce_rows = [row for row in rows if isinstance(row, dict) and row.get("ceiling") == "ce_fdh"]
    cr_rows = [row for row in rows if isinstance(row, dict) and row.get("ceiling") == "cr_fdh"]
    if len(ce_rows) != 9 or len(cr_rows) != 9:
        return False
    for index, (row, requirement) in enumerate(zip(ce_rows, EXPECTED_CE_REQUIREMENTS), start=1):
        if (
            not exact_row_keys(row, expected_keys)
            or row.get("outcome_percent") != index * 10
            or row.get("status") != "required"
            or not close(row.get("required_x_percent"), requirement)
        ):
            return False
    for index, (row, requirement) in enumerate(zip(cr_rows, EXPECTED_CR_REQUIREMENTS), start=1):
        if not exact_row_keys(row, expected_keys) or row.get("outcome_percent") != index * 10:
            return False
        if requirement is None:
            if row.get("required_x_percent") is not None or row.get("status") != "not_necessary":
                return False
        elif row.get("status") != "required" or not close(row.get("required_x_percent"), requirement):
            return False
    return True


commands = [
    run([sys.executable, "validation/v08_extended_methods_reference.py", "--section", "nca"]),
    run(["cargo", "test", "--release", "-p", "qpls-core", "bounded_nca_v2", "--", "--nocapture"]),
    run(["cargo", "test", "--release", "-p", "qpls-estimation", "nca_v2", "--", "--nocapture"]),
    run([
        "cargo",
        "test",
        "--release",
        "-p",
        "qpls-runner",
        "standalone_nca_uses_exact_v2_provenance_and_versionless_assessment_warning",
        "--",
        "--nocapture",
    ]),
    run([
        "cargo",
        "test",
        "--release",
        "-p",
        "qpls-project",
        "runner_generated_nca_v2_commits_saves_reopens_and_rejects_contract_tampering",
        "--",
        "--nocapture",
    ]),
    run([
        "cargo",
        "test",
        "--release",
        "-p",
        "quickpls-desktop",
        "standalone_nca_recipe_never_reopens_as_a_phantom_active_model",
        "--",
        "--nocapture",
    ]),
]
executable_gates_passed = all(item["passed"] for item in commands)

reference = load_json(RESULTS / "v08_extended_methods_reference_report.json")
nca_reference = reference.get("checks", {}).get("nca", {})
reference_contract_passed = (
    reference.get("passed") is True
    and reference.get("target") == "nca_v2 bounded backend qualification"
    and reference.get("selected_section") == "nca"
    and nca_reference.get("passed") is True
    and nca_reference.get("method_version") == METHOD_VERSION
    and nca_reference.get("max_abs_difference", math.inf) <= 1e-6
    and nca_reference.get("ce_fdh_peers_ordered") is True
    and nca_reference.get("cr_fdh_regression_checked") is True
    and nca_reference.get("all_bottleneck_rows_checked") is True
    and nca_reference.get("permutation_p_value_lattice") is True
    and nca_reference.get("seeded_repeat_deterministic") is True
    and nca_reference.get("exact_standalone_envelope") is True
    and nca_reference.get("exact_ceiling_shape") is True
    and nca_reference.get("scope")
    == {
        "observed_numeric_variables": 2,
        "listwise_complete_rows_minimum": 3,
        "ceilings": ["ce_fdh", "cr_fdh"],
        "latent_score_nca": False,
        "multiple_conditions": False,
        "cipma": False,
    }
)

canonical_engine_contract = (
    source_contains(
        "crates/qpls-estimation/src/pls.rs",
        f'pub const NCA_METHOD_VERSION_V1: &str = "{LEGACY_METHOD_VERSION}";',
        f'pub const NCA_METHOD_VERSION: &str = "{METHOD_VERSION}";',
        "fn nca_ce_fdh_peers",
        "fn nca_cr_fdh_line",
        "fn nca_permutation_indices",
        "quickpls:nca-permutation:v2",
        "fn nca_bottleneck_rows",
        "pub fn nca_analysis_matches_v2_contract",
        "multiple conditions, latent-score NCA, cIPMA, and broader ceiling variants remain unsupported",
    )
    and source_contains(
        "crates/qpls-core/src/validation.rs",
        '"nca.variables_required"',
        '"nca.variables_distinct"',
        '"nca.ceiling"',
        '"nca.permutation_samples"',
        '"nca.raw_values_required"',
        '"nca.weighting_sentinel"',
        '"nca.case_weights_unsupported"',
        '"nca.external_resampling_unsupported"',
        "bounded_nca_v2_accepts_a_no_model_raw_numeric_recipe",
        "bounded_nca_v2_rejects_ambiguous_variables_and_incompatible_settings",
    )
    and source_contains(
        "crates/qpls-runner/src/lib.rs",
        "recipe.settings.method == qpls_core::AnalysisMethod::Nca",
        "base_versions = vec![NCA_METHOD_VERSION]",
        ASSESSMENT_VERSION,
        ASSESSMENT_WARNING,
        "standalone_nca_uses_exact_v2_provenance_and_versionless_assessment_warning",
    )
)

project_and_no_model_contract = (
    source_contains(
        "crates/qpls-project/src/lib.rs",
        "AnalysisMethod::Nca => Some(NCA_METHOD_VERSION)",
        "validate_nca_payload_contract",
        "validate_legacy_nca_v1_contract",
        "nca_analysis_matches_v2_contract",
        'const CODE: &str = "nca.legacy_method_version";',
        "runner_generated_nca_v2_commits_saves_reopens_and_rejects_contract_tampering",
        ASSESSMENT_WARNING,
    )
    and source_contains(
        "src-tauri/src/lib.rs",
        "fn recipe_uses_editable_model",
        "AnalysisMethod::Pca | AnalysisMethod::Regression | AnalysisMethod::Nca",
        "standalone_nca_recipe_never_reopens_as_a_phantom_active_model",
    )
)

legacy_and_scope_docs = (
    source_contains(
        "docs/methods/NCA_V1.md",
        "Status: superseded; archive-readable only.",
        "does not reinterpret a stored `nca_v1` payload as v2",
        "not the current executable NCA method",
    )
    and source_contains(
        "docs/methods/NCA_V2.md",
        "At least three complete rows",
        "ordinary least-squares line",
        "Full feature, numerical, visual, or workflow parity with SmartPLS or the R NCA package",
        "packaged-native setup/results/XLSX/save/reopen acceptance",
    )
)

tauri = load_json(RESULTS / "v247_tauri_native_acceptance.json")
tauri_checks = tauri.get("checks", {})
required_tauri_checks = [
    "ncaReferenceFixture",
    "ncaFixtureProvisioning",
    "ncaCalculationDialog",
    "ncaRunning",
    "ncaResult",
    "ncaExport",
    "ncaSaveReopen",
]
fixture = tauri_checks.get("ncaReferenceFixture", {})
expected = fixture.get("expected", {})
cr_expected = expected.get("crFdh", {})
provisioning = tauri_checks.get("ncaFixtureProvisioning", {})
initial_archive = provisioning.get("initialArchive", {})
dialog = tauri_checks.get("ncaCalculationDialog", {})
running = tauri_checks.get("ncaRunning", {})
result = tauri_checks.get("ncaResult", {})
navigation = result.get("navigation", {})
export = tauri_checks.get("ncaExport", {})
reopen = tauri_checks.get("ncaSaveReopen", {})
archive = reopen.get("archive", {})

expected_method_labels = [
    "PLS-SEM Algorithm",
    "Consistent PLS",
    "Weighted PLS",
    "CCA composite residual diagnostics",
    "Importance-Performance Map Analysis",
    "PLS-SEM Bootstrapping",
    "Structural Path Randomization",
    "Two-Group Permutation MGA",
    "Deterministic Construct Prediction",
    "Necessary Condition Analysis",
]
expected_x_options = [
    {"value": "", "label": "Select a numeric variable", "disabled": False},
    {"value": "x", "label": "x", "disabled": False},
    {"value": "y", "label": "y", "disabled": False},
]
expected_y_options = [
    {"value": "", "label": "Select a different numeric variable", "disabled": False},
    {"value": "x", "label": "x", "disabled": True},
    {"value": "y", "label": "y", "disabled": False},
]
expected_ceiling_modes = [
    {"value": "both", "label": "CE-FDH and CR-FDH"},
    {"value": "ce_fdh", "label": "CE-FDH"},
    {"value": "cr_fdh", "label": "CR-FDH"},
]
expected_tree_items = [
    "Necessary conditions",
    "Ceiling effect sizes and permutation inference",
    "CR-FDH ceiling coefficients",
    "Observed-range bottlenecks",
    "Calculation scope",
]

expected_fixture_passed = (
    fixture.get("columns") == ["x", "y"]
    and fixture.get("rows") == 4
    and fixture.get("completeCases") == 4
    and fixture.get("deterministic") is True
    and scope_matches(expected.get("scope", {}))
    and peers_match(expected.get("ceFdhPeers"))
    and close(expected.get("ceFdhEffectSize"), EXPECTED_CE_EFFECT)
    and close(cr_expected.get("slope"), EXPECTED_CR_SLOPE)
    and close(cr_expected.get("intercept"), EXPECTED_CR_INTERCEPT)
    and close(cr_expected.get("effectSize"), EXPECTED_CR_EFFECT)
    and expected.get("ceiling") == "both"
    and expected.get("permutationSamples") == 19
    and expected.get("seed") == 20_260_811
)

dialog_passed = (
    dialog.get("methods") == expected_method_labels
    and dialog.get("initiallySelectedMethod") == "Necessary Condition Analysis"
    and dialog.get("selectedMethod") == "Necessary Condition Analysis"
    and dialog.get("category") == "Standalone analysis"
    and dialog.get("xOptions") == expected_x_options
    and dialog.get("yOptionsAfterX") == expected_y_options
    and dialog.get("selectedX") == "x"
    and dialog.get("selectedY") == "y"
    and dialog.get("ceilingModes") == expected_ceiling_modes
    and dialog.get("selectedCeiling") == "both"
    and dialog.get("permutations")
    == {"value": "19", "min": "1", "max": "10000", "step": "1"}
    and dialog.get("seed")
    == {"value": "20260811", "min": "0", "max": "4294967295"}
    and dialog.get("variableData") == "Observed numeric values (fixed)"
    and dialog.get("validatedScope") == EXPECTED_DIALOG_SCOPE
    and dialog.get("unsupportedControls") == 0
    and dialog.get("startEnabled") is True
    and dialog.get("blockers") == []
    and dialog.get("noModelBlocker") is True
    and dialog.get("noBroaderProductClaim") is True
    and dialog.get("noIdleRunState") is True
)

effect_values = navigation.get("effectValues", {})
ce_visible = effect_values.get("CE-FDH", {})
cr_visible = effect_values.get("CR-FDH", {})
cr_line_rows = navigation.get("crLine", {}).get("rows", [])
scope_values = navigation.get("scopeValues", {})
plot = navigation.get("plot", {})
navigation_passed = (
    navigation.get("initialSelectedTable") == "Ceiling effect sizes and permutation inference"
    and navigation.get("treeItems") == expected_tree_items
    and navigation.get("effects", {}).get("rowCount") == 2
    and navigation.get("effects", {}).get("headers")
    == ["Ceiling line", "Effect size", "Permutation p"]
    and [row[0] for row in navigation.get("effects", {}).get("rows", []) if row]
    == ["CE-FDH", "CR-FDH"]
    and close(ce_visible.get("effectSize"), EXPECTED_CE_EFFECT, 0.00005)
    and close(cr_visible.get("effectSize"), EXPECTED_CR_EFFECT, 0.00005)
    and permutation_p_value(ce_visible.get("permutationP"))
    and permutation_p_value(cr_visible.get("permutationP"))
    and navigation.get("pValueLattice") is True
    and navigation.get("crLine", {}).get("rowCount") == 1
    and navigation.get("crLine", {}).get("headers") == ["Ceiling line", "Slope", "Intercept"]
    and len(cr_line_rows) == 1
    and cr_line_rows[0][0] == "CR-FDH"
    and close(visible_number(cr_line_rows[0][1]), EXPECTED_CR_SLOPE, 0.00005)
    and close(visible_number(cr_line_rows[0][2]), EXPECTED_CR_INTERCEPT, 0.00005)
    and navigation.get("bottlenecks", {}).get("rowCount") == 18
    and navigation.get("bottlenecks", {}).get("headers")
    == ["Ceiling line", "Outcome (% observed range)", "Condition requirement"]
    and navigation.get("bottlenecksMatch") is True
    and navigation.get("scope", {}).get("rowCount") == 10
    and navigation.get("scope", {}).get("headers") == ["Field", "Value"]
    and scope_values.get("Condition variable (X)") == "x"
    and scope_values.get("Outcome variable (Y)") == "y"
    and scope_values.get("Analyzed observations") == "4"
    and scope_values.get("X observed range") == "0.000000 to 3.000000"
    and scope_values.get("Y observed range") == "1.000000 to 4.000000"
    and scope_values.get("Ceiling lines") == "CE-FDH and CR-FDH"
    and scope_values.get("Requested permutations") == "19"
    and scope_values.get("Usable permutations") == "19"
    and scope_values.get("Missing data") == "Listwise deletion"
    and scope_values.get("Method version") == METHOD_VERSION
    and plot.get("figures") == 1
    and plot.get("accessibleSvgs") == 1
    and plot.get("namedImages") == 1
    and plot.get("labelledBy") == "nd-nca-plot-title nd-nca-plot-description"
    and plot.get("directTitleCount") == 1
    and plot.get("title") == "Necessary condition ceiling plot for x and y"
    and plot.get("descriptionCount") == 1
    and "CE-FDH peer 0, 1" in plot.get("description", "")
    and "CE-FDH peer 1, 3" in plot.get("description", "")
    and "CE-FDH peer 3, 4" in plot.get("description", "")
    and "CR-FDH slope 0.9286 and intercept 1.4286" in plot.get("description", "")
    and plot.get("captionTitle") == "Necessary condition ceiling plot"
    and plot.get("captionPair") == "x → y"
    and plot.get("ceFdhPaths") == 2
    and plot.get("crFdhLines") == 2
    and plot.get("ceFdhPeers") == 3
    and navigation.get("noModelOrQualityTree") is True
    and navigation.get("noPlaceholder") is True
    and navigation.get("noBroaderNcaClaim") is True
)

ceilings = archive.get("ceilings", [])
ce_ceiling = next(
    (row for row in ceilings if isinstance(row, dict) and row.get("ceiling") == "ce_fdh"),
    {},
)
cr_ceiling = next(
    (row for row in ceilings if isinstance(row, dict) and row.get("ceiling") == "cr_fdh"),
    {},
)
ceiling_keys = {"ceiling", "effect_size", "permutation_p_value", "slope", "intercept"}
recipe = archive.get("recipe", {})
assessment = archive.get("assessment", {})
archive_passed = (
    isinstance(initial_archive.get("packageVersion"), str)
    and initial_archive.get("packageVersion")
    and initial_archive.get("manifestEngineVersion") == initial_archive.get("packageVersion")
    and initial_archive.get("models") == 0
    and initial_archive.get("recipes") == 0
    and initial_archive.get("results") == 0
    and initial_archive.get("activeModelId") is None
    and initial_archive.get("nodes") == 0
    and initial_archive.get("edges") == 0
    and archive.get("resultId") == reopen.get("expectedRunId")
    and archive.get("resultStatus") == "completed"
    and archive.get("provenanceMethod") == "nca"
    and archive.get("provenanceMethodVersion") == METHOD_VERSION
    and archive.get("provenanceEngineVersion") == archive.get("packageVersion")
    and archive.get("manifestEngineVersion") == archive.get("packageVersion")
    and archive.get("provenanceSeed") == 20_260_811
    and archive.get("payloadKind") == "pls_pm_v1"
    and archive.get("estimationMethodVersion") == METHOD_VERSION
    and archive.get("usedObservations") == 4
    and archive.get("omittedObservations") == 0
    and assessment.get("methodVersion") == ASSESSMENT_VERSION
    and assessment.get("warnings") == [ASSESSMENT_WARNING]
    and archive.get("ncaKeys") == EXPECTED_NCA_KEYS
    and archive.get("exactCeilingRows") is True
    and archive.get("exactBottleneckRows") is True
    and archive.get("exactPeerRows") is True
    and archive.get("ncaMethodVersion") == METHOD_VERSION
    and archive.get("ceiling") == "both"
    and archive.get("permutationSamples") == 19
    and archive.get("usablePermutations") == 19
    and archive.get("x") == "x"
    and archive.get("y") == "y"
    and archive.get("observations") == 4
    and scope_matches(archive.get("scope", {}))
    and peers_match(archive.get("peers"))
    and len(ceilings) == 2
    and exact_row_keys(ce_ceiling, ceiling_keys)
    and close(ce_ceiling.get("effect_size"), EXPECTED_CE_EFFECT)
    and permutation_p_value(ce_ceiling.get("permutation_p_value"))
    and ce_ceiling.get("slope") is None
    and ce_ceiling.get("intercept") is None
    and exact_row_keys(cr_ceiling, ceiling_keys)
    and close(cr_ceiling.get("effect_size"), EXPECTED_CR_EFFECT)
    and permutation_p_value(cr_ceiling.get("permutation_p_value"))
    and close(cr_ceiling.get("slope"), EXPECTED_CR_SLOPE)
    and close(cr_ceiling.get("intercept"), EXPECTED_CR_INTERCEPT)
    and exact_bottleneck_rows(archive.get("bottlenecks"))
    and archive.get("scopeMatches") is True
    and archive.get("peersMatch") is True
    and archive.get("ceilingGeometryMatches") is True
    and archive.get("bottlenecksMatch") is True
    and archive.get("pValueLattice") is True
    and archive.get("warnings") == [EXPECTED_NCA_WARNING]
    and archive.get("forbiddenNestedPayloads") == []
    and recipe.get("method") == "nca"
    and recipe.get("weightingScheme") == "path"
    and recipe.get("preprocessing") == "unstandardized"
    and recipe.get("missingData") == "listwise_deletion"
    and recipe.get("seed") == 20_260_811
    and recipe.get("bootstrapSamples") == 0
    and recipe.get("studentizedInnerSamples") == 0
    and recipe.get("permutationSamples") == 0
    and recipe.get("caseWeightColumn") is None
    and recipe.get("ncaX") == "x"
    and recipe.get("ncaY") == "y"
    and recipe.get("ncaCeiling") == "both"
    and recipe.get("ncaPermutationSamples") == "19"
    and recipe.get("constructs") == 0
    and recipe.get("paths") == 0
    and recipe.get("controls") == 0
    and recipe.get("interactions") == 0
    and recipe.get("higherOrderConstructs") == 0
    and archive.get("models") == 0
    and archive.get("activeModelId") is None
    and archive.get("nodes") == 0
    and archive.get("edges") == 0
    and archive.get("runModelId") is None
    and archive.get("runModelSnapshot") is None
    and archive.get("runSeed") == 20_260_811
)

run_details = result.get("runDetails", {})
run_properties = run_details.get("properties", {})
result_properties = result.get("resultProperties", {})
result_passed = (
    result.get("autoOpenedSurface") == "results"
    and result.get("autoOpenedTable") == "Ceiling effect sizes and permutation inference"
    and navigation_passed
    and run_properties.get("Method version") == METHOD_VERSION
    and run_properties.get("Method") == "Necessary Condition Analysis"
    and run_properties.get("Recorded seed") == "20260811"
    and run_properties.get("Condition (X)") == "x"
    and run_properties.get("Outcome (Y)") == "y"
    and run_properties.get("Observations") == "4"
    and run_properties.get("Ceiling lines") == "CE-FDH and CR-FDH"
    and run_properties.get("Requested permutations") == "19"
    and run_properties.get("Usable permutations") == "19"
    and run_properties.get("Missing data") == "Listwise deletion"
    and "Weighting" not in run_properties
    and "Preprocessing" not in run_properties
    and run_details.get("logEntries", 0) >= 1
    and result_properties.get("Method") == "Necessary Condition Analysis"
    and result_properties.get("Status") == "Completed"
    and result_properties.get("Condition (X)") == "x"
    and result_properties.get("Outcome (Y)") == "y"
    and result_properties.get("Observations") == "4"
    and result_properties.get("Ceiling lines") == "CE-FDH and CR-FDH"
    and result_properties.get("Requested permutations") == "19"
    and result_properties.get("Usable permutations") == "19"
    and result_properties.get("Recorded seed") == "20260811"
    and result.get("editModelCommand", {}).get("count") == 0
    and result.get("editModelCommand", {}).get("enabled") is False
    and result.get("editDataCommand", {}).get("count") == 1
    and result.get("editDataCommand", {}).get("enabled") is True
)

native_xlsx = export.get("nativeXlsx", {})
expected_export_formats = [
    "CSV tables",
    "HTML report",
    "Reviewer pack",
    "XLSX workbook",
    "Print / PDF",
]
expected_xlsx_sheets = [
    "Ceiling effect sizes and permut",
    "CR-FDH ceiling coefficients",
    "Observed-range bottlenecks",
    "Calculation scope",
    "Run provenance",
]
export_passed = (
    export.get("selectedRunId") == result.get("runId")
    and export.get("formats") == expected_export_formats
    and export.get("xlsxEnabled") is True
    and export.get("modelDiagramCount") == 0
    and export.get("ncaTablesIncluded") is True
    and native_xlsx.get("attempted") is True
    and native_xlsx.get("helper", {}).get("ready", {}).get("passed") is True
    and native_xlsx.get("helper", {}).get("completion", {}).get("passed") is True
    and native_xlsx.get("file", {}).get("isFile") is True
    and native_xlsx.get("file", {}).get("size", 0) > 0
    and all(sheet in native_xlsx.get("workbookSheets", []) for sheet in expected_xlsx_sheets)
)

packaged_native_passed = (
    tauri.get("passed") is True
    and not tauri.get("failures")
    and not tauri.get("consoleErrors")
    and all(name in tauri_checks for name in required_tauri_checks)
    and expected_fixture_passed
    and dialog_passed
    and running.get("status") in {"queued", "validating", "running"}
    and bool(running.get("phase"))
    and running.get("cancelVisible") is True
    and result_passed
    and export_passed
    and reopen.get("attempted") is True
    and reopen.get("reopenedThroughRecentProjectRow") is True
    and reopen.get("sameRunRestored") is True
    and reopen.get("sameVisibleEffectsRestored") is True
    and reopen.get("selectedRunId") == reopen.get("expectedRunId")
    and reopen.get("navigation", {}).get("initialSelectedTable")
    == "Ceiling effect sizes and permutation inference"
    and reopen.get("navigation", {}).get("bottlenecksMatch") is True
    and reopen.get("navigation", {}).get("noModelOrQualityTree") is True
    and reopen.get("navigation", {}).get("noPlaceholder") is True
    and reopen.get("navigation", {}).get("noBroaderNcaClaim") is True
    and archive_passed
)

raise SystemExit(
    audit_method(
        "nca",
        "Standalone observed numeric X/Y NCA v2 with exact CE-FDH record-high peers, CR-FDH OLS through those peers, seeded permutation p values, observed-range bottlenecks, strict archive persistence, and no phantom SEM model; broader variants and parity claims are excluded.",
        [
            "v08_extended_methods_reference_report.json",
            "v247_tauri_native_acceptance.json",
        ],
        ["NCA_V2.md", "NCA_V1.md"],
        [
            {
                "name": "executable_backend_gates",
                "passed": executable_gates_passed,
                "detail": "Focused core, estimator, runner, project, and desktop no-model tests plus the independent reference must execute successfully.",
                "commands": commands,
            },
            {
                "name": "independent_ce_cr_reference",
                "passed": reference_contract_passed,
                "detail": "The independent reference must check ordered record-high CE peers, NumPy OLS CR-FDH, both effects, every bottleneck, exact standalone envelope, p-value lattice, and seeded repeat determinism.",
            },
            {
                "name": "canonical_nca_v2_engine_and_runner_contract",
                "passed": canonical_engine_contract,
                "detail": "Core, estimation, and runner sources must freeze nca_v2, bounded recipe guards, independent seeded permutations, limitation wording, exact provenance, and the versionless assessment warning.",
            },
            {
                "name": "strict_project_persistence_and_no_model_contract",
                "passed": project_and_no_model_contract,
                "detail": "qpls-project must reject NCA contract tampering while preserving explicit v1 legacy reads, and the desktop snapshot must exclude standalone recipe placeholders from active-model resolution.",
            },
            {
                "name": "legacy_supersession_and_no_parity_overclaim",
                "passed": legacy_and_scope_docs,
                "detail": "Documentation must mark nca_v1 archive-only, define the corrected bounded v2 equations, and explicitly exclude broad SmartPLS/R-package parity.",
            },
            {
                "name": "genuine_packaged_native_workflow",
                "passed": packaged_native_passed,
                "detail": "A current packaged-Tauri artifact must prove the transparent four-row fixture, no-model setup, genuine run lifecycle, exact geometry and provenance, NCA XLSX export, save/reopen, and canonical models=[]/activeModelId=null. Source or browser-only fixtures cannot satisfy this check.",
            },
        ],
    )
)
