from __future__ import annotations

import json
import re
from pathlib import Path

from validation.cbsem_exact_case_bootstrap_v1_factory import (
    EXPECTED_PACKAGED_CHECKS,
)


ROOT = Path(__file__).resolve().parents[1]
HARNESS = ROOT / "validation" / "v247_tauri_native_acceptance.mjs"
WRAPPER = ROOT / "validation" / "run_cbsem_exact_case_bootstrap_v1_native_acceptance.ps1"
WORKSPACE = ROOT / "src" / "native" / "NativeRecipeV4CbsemWorkspace.tsx"
DOMAIN = ROOT / "src" / "domain" / "internalRecipeV4CbsemWorkspace.ts"
APPEND_CONTRACT = ROOT / "src" / "domain" / "internalProjectSchema6ResultAppend.ts"
TAURI_APPEND = ROOT / "src-tauri" / "src" / "project_schema6_result_append.rs"
PROJECT_SCHEMA6 = ROOT / "crates" / "qpls-project" / "src" / "project_schema_v6.rs"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def test_raw_report_writer_matches_the_factorys_exact_twelve_check_contract() -> None:
    source = read(HARNESS)
    writer = source[
        source.index("async function writeCbsemExactBootstrapPackagedEvidence()") :
        source.index("async function processV2ResourceArtifactState")
    ]
    emitted = set(
        re.findall(r"^\s{4}([a-z][a-z0-9_]+):\s*\{\s*passed:", writer, re.MULTILINE)
    )

    assert emitted == EXPECTED_PACKAGED_CHECKS
    assert "process_cleanup: { passed: false, pending_wrapper_supervisor: true }" in writer
    assert "binary_artifacts: { desktop, cli }" in writer
    assert "...(manifest.qualification.source_requirements.packaged_acceptance ?? [])" in writer
    manifest = json.loads(
        (ROOT / "validation" / "methods" / "cbsem_exact_case_bootstrap_v1.manifest.json").read_text(
            encoding="utf-8"
        )
    )
    assert "validation/run_cbsem_exact_case_bootstrap_v1_native_acceptance.ps1" in manifest[
        "qualification"
    ]["source_requirements"]["packaged_acceptance"]
    assert '"validation/test_cbsem_exact_case_bootstrap_v1_packaged_producer.py"' in writer
    assert '"src/native/NativeRecipeV4CbsemWorkspace.tsx"' in writer
    assert '"crates/qpls-project/src/project_schema_v6.rs"' in writer


def test_genuine_packaged_harness_covers_the_exact_method_and_release_boundaries() -> None:
    source = read(HARNESS)
    focused = source[
        source.index("async function runFocusedExactCbsemBootstrapReopen") :
        source.index("async function runFocusedExactCbsemBootstrapAcceptance")
    ]

    for token in (
        'acceptanceScope === "cbsem_exact_bootstrap"',
        'QUICKPLS_CBSEM_EXACT_PHASE',
        '"execute"',
        '"reopen"',
        'const cbsemExactBootstrapSamples = 1_000;',
        '"percentile_type7"',
        '"analytic_studentized_type7"',
        '"bca_type7"',
        'progress[aria-label="CB-SEM Recipe-v4 job progress"]',
        '"Append exact native document"',
        '"Reopen and verify completed run"',
        '#nd-cbsem-v4-export-xlsx',
        '#nd-cbsem-v4-stored-results',
        'createExactCbsemSchema6Copy',
        'inspect_internal_project_upgrade_v6',
        'plan_internal_project_upgrade_v6',
        'execute_internal_project_upgrade_v6',
        'outcome.destinationInspection.value?.access !== "current_v6_archive"',
        'captureActualTauriViewportMatrix',
        'exactWorkspace: true',
        'distinctDesktopProcessRequired: true',
        'externalRequests.length === 0',
        'if ((await engineDetails.getAttribute("open")) === null)',
        'Exact-CB native job terminated without a result',
        'document.querySelector(".nd-cbsem-v4-failure")',
        'timeout: 180_000',
        'const expectedMinimumUsable = Math.max(1_000, Math.ceil(0.9 * requested));',
        'summary["Unavailable reason"] === "insufficient_usable_refits"',
        'result.tables.exact_case_bootstrap_successful_refits.rows.length === usable',
    ):
        assert token in source

    assert 'if (!await engineDetails.getAttribute("open"))' not in source
    assert "usable >= 950" not in source

    for table_id in (
        "exact_case_bootstrap_summary",
        "exact_case_bootstrap_parameter_intervals",
        "exact_case_bootstrap_successful_refits",
        "exact_case_bootstrap_failures",
        "exact_case_bootstrap_hypothesis_tests",
        "exact_case_bootstrap_studentized_summary",
        "exact_case_bootstrap_studentized_point_standard_errors",
        "exact_case_bootstrap_studentized_parameter_intervals",
        "exact_case_bootstrap_studentized_refit_standard_errors",
        "exact_case_bootstrap_bca_summary",
        "exact_case_bootstrap_bca_parameter_intervals",
        "exact_case_bootstrap_bca_successful_delete_one_refits",
        "exact_case_bootstrap_bca_failures",
    ):
        assert f'"{table_id}"' in source

    viewport_matrix = re.search(
        r"const ctaPlsViewports\s*=\s*\[(.*?)\];", source, re.DOTALL
    )
    assert viewport_matrix is not None
    for viewport in ("1024x700", "1280x720", "1440x900"):
        assert viewport in viewport_matrix.group(1)
    assert "passed: externalRequests.length === 0" in focused


def test_wrapper_has_only_public_qualification_and_report_flags_and_supervises_two_processes() -> None:
    source = read(WRAPPER)
    parameter_block = re.match(r"\s*param\((.*?)\)\s*\n", source, re.DOTALL)
    assert parameter_block is not None
    public_parameters = set(re.findall(r"\$(\w+)", parameter_block.group(1)))
    assert public_parameters == {"RunQualifiedAcceptance", "PackagedReportPath"}

    for token in (
        'Invoke-ExactCbsemPhase -Phase "execute"',
        'Invoke-ExactCbsemPhase -Phase "reopen"',
        "monitor_quickpls_network.ps1",
        "close_tauri_test_window.mjs",
        'sampled_exact_process_tree_tcp_v1',
        'platform_background_egress_observation',
        'platform_background_egress_observed',
        'commercial_zero_egress_passed',
        'strictZeroProcessEgressClaimed',
        '$raw.checks.process_cleanup',
        'forced_process_cleanup_used = $false',
        'python $factoryPath --adapt-packaged --packaged-report $PackagedReportPath',
        'packaged_acceptance.identity.json',
        'method_audit.identity.json',
    ):
        assert token in source

    assert '"cbsem_exact_case_bootstrap_v1_packaged_acceptance.json"' in source
    assert '"v247_tauri_native_acceptance_cbsem_exact_bootstrap.json"' in source
    assert "runtime target must be new" in source
    assert "WriteAllText(" in source


def test_product_can_atomically_append_recipe_and_result_then_select_it_after_relaunch() -> None:
    workspace = read(WORKSPACE)
    domain = read(DOMAIN)
    append_contract = read(APPEND_CONTRACT)
    tauri = read(TAURI_APPEND)
    project = read(PROJECT_SCHEMA6)

    assert "recipe?: AnalysisRecipeV4<AnalysisRecipeV4MissingDataPolicy>" in append_contract
    assert "appendInternalLabsRecipeV4CbsemResultV1(completed, completedRecipe" in workspace
    assert "readStoredInternalLabsRecipeV4CbsemResultsV1(identity, services.read)" in workspace
    assert 'id="nd-cbsem-v4-stored-results"' in workspace
    assert 'data-canonical-table-id={table.id}' in workspace
    assert "storedExactCaseBootstrapEntriesV1" in domain
    assert 'inspection.access !== "current_v6_archive"' in domain
    assert 'inspection.sourceKind !== "project_archive"' in domain
    assert 'surface: "standard_exact_cbsem"' in domain
    assert "recipe," in domain
    assert "recipe: Option<AnalysisRecipeV4>" in tauri
    assert "append_recipe_v4_and_canonical_result_document_v2_file_v6" in tauri
    assert "pub fn append_recipe_v4_and_canonical_result_document_v2_file_v6" in project
    assert "source.recipes.push(recipe);" in project
    assert "source.ensure_valid()?;" in project
    assert "atomic_replace_with_rollback_v6" in project
