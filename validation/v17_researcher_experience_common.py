from __future__ import annotations

import csv
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
SAMPLE_CATALOG_PATH = "src/data/bundledSampleProjects.v1.json"

LEGACY_BUNDLED_SAMPLE_CONTRACTS = {
    "corporate_reputation": {
        "projectName": "Corporate Reputation Sample",
        "datasetFileName": "corporate_reputation_smartpls_mean_replaced_v1.csv",
        "caseCount": 344,
        "constructCount": 8,
        "pathCount": 13,
    },
    "simple_pls": {
        "projectName": "Simple Reflective PLS Sample",
        "datasetFileName": "simple_reflective.csv",
        "caseCount": 6,
        "constructCount": 2,
        "pathCount": 1,
    },
    "mediation": {
        "projectName": "Mediation Sample",
        "datasetFileName": "mediation_sample.csv",
        "caseCount": 8,
        "constructCount": 3,
        "pathCount": 2,
    },
    "organizational_identification": {
        "projectName": "Organizational Identification Model",
        "datasetFileName": "organizational_identification_v1.csv",
        "caseCount": 305,
        "constructCount": 4,
        "pathCount": 3,
    },
}

REQUESTED_OI_SAMPLE_SEMANTICS = {
    "organizational_identification_mediation": {
        "label_tokens": ("Mediation",),
        "detail_tokens": ("parallel mediation", "direct effects"),
        "paths": {
            ("org_prestige", "org_identification"),
            ("org_identification", "affective_commitment_joy"),
            ("org_prestige", "affective_commitment_joy"),
            ("org_identification", "affective_commitment_love"),
            ("org_prestige", "affective_commitment_love"),
        },
        "interactions": set(),
        "higher_order": set(),
        "reference_paths": {
            ("org_prestige", "org_identification"),
            ("org_identification", "affective_commitment_joy"),
            ("org_prestige", "affective_commitment_joy"),
            ("org_identification", "affective_commitment_love"),
            ("org_prestige", "affective_commitment_love"),
        },
        "reference_interactions": set(),
        "reference_kind": "supplied_screenshot_three_decimal_values",
        "secondary_screenshot_field": None,
        "reference_boundary_tokens": (),
        "metadata_boundary_tokens": ("shared_oi_dataset",),
    },
    "organizational_identification_moderation": {
        "label_tokens": ("Two-Outcome Moderation",),
        "detail_tokens": ("jointly moderating", "simultaneous multiple two-way moderation point run"),
        "project_kind": "general_sem_v1",
        "paths": {
            ("org_identification", "affective_commitment_joy"),
            ("org_prestige", "affective_commitment_joy"),
            ("interaction_oi_op_acj", "affective_commitment_joy"),
            ("org_identification", "affective_commitment_love"),
            ("org_prestige", "affective_commitment_love"),
            ("interaction_oi_op_acl", "affective_commitment_love"),
        },
        "interactions": {
            (
                "oi_x_op_to_acj",
                "org_identification",
                "org_prestige",
                "interaction_oi_op_acj",
                "affective_commitment_joy",
                "two_stage_product_score",
            ),
            (
                "oi_x_op_to_acl",
                "org_identification",
                "org_prestige",
                "interaction_oi_op_acl",
                "affective_commitment_love",
                "two_stage_product_score",
            ),
        },
        "higher_order": set(),
        "reference_paths": {
            ("org_identification", "affective_commitment_joy"),
            ("org_identification", "affective_commitment_love"),
        },
        "reference_interactions": {"oi_x_op_to_acj", "oi_x_op_to_acl"},
        "reference_kind": "quickpls_general_sem_simultaneous_two_outcome_moderation_three_decimal_values",
        "secondary_screenshot_field": None,
        "reference_boundary_tokens": (
            "strict schema-6 General SEM",
            "both outcome equations",
            "point-estimation",
            "joint-stage R-squared",
        ),
        "metadata_boundary_tokens": (
            "strict_general_sem_simultaneous_point_estimation_only",
            "no_causal_moderation_claim",
            "joint_stage_r_squared_not_yet_persisted",
        ),
    },
    "organizational_identification_moderated_mediation": {
        "label_tokens": ("Moderated Mediation", "Point Topology"),
        "detail_tokens": ("Second-stage", "point topology", "not dedicated conditional-effect inference"),
        "paths": {
            ("org_identification", "affective_commitment_joy"),
            ("org_identification", "affective_commitment_love"),
            ("affective_commitment_joy", "affective_commitment_love"),
            ("org_prestige", "affective_commitment_love"),
            ("interaction_acj_op_acl", "affective_commitment_love"),
        },
        "interactions": {
            (
                "acj_x_op_to_acl",
                "affective_commitment_joy",
                "org_prestige",
                "interaction_acj_op_acl",
                "affective_commitment_love",
                "two_stage_product_score",
            ),
        },
        "higher_order": set(),
        "reference_paths": {
            ("org_identification", "affective_commitment_joy"),
            ("org_identification", "affective_commitment_love"),
            ("affective_commitment_joy", "affective_commitment_love"),
        },
        "reference_interactions": {"acj_x_op_to_acl"},
        "reference_kind": "quickpls_qualified_single_interaction_point_topology_three_decimal_values",
        "secondary_screenshot_field": "supplied_screenshot_values",
        "reference_boundary_tokens": (
            "dedicated moderated-mediation method is not bundled",
            "not claimed identical",
            "strict General SEM",
            "conditional-effect inference requires a separately qualified workflow",
        ),
        "metadata_boundary_tokens": (
            "point_topology_only",
            "not_dedicated_moderated_mediation_inference",
            "not_claimed_as_strict_general_sem_screenshot_parity",
        ),
    },
    "organizational_identification_higher_order": {
        "label_tokens": ("Higher-Order",),
        "detail_tokens": ("disjoint two-stage",),
        "paths": {
            ("org_prestige", "org_identification"),
            ("org_identification", "affective_commitment"),
            ("org_prestige", "affective_commitment"),
        },
        "interactions": set(),
        "higher_order": {
            (
                "affective_commitment",
                ("affective_commitment_joy", "affective_commitment_love"),
                "two_stage",
            ),
        },
        "reference_paths": {
            ("org_prestige", "org_identification"),
            ("org_identification", "affective_commitment"),
            ("org_prestige", "affective_commitment"),
        },
        "reference_interactions": set(),
        "reference_kind": "quickpls_disjoint_two_stage_three_decimal_values",
        "secondary_screenshot_field": "supplied_screenshot_values",
        "reference_boundary_tokens": ("disjoint two-stage", "not identical", "parity is not claimed"),
        "metadata_boundary_tokens": ("supported_quickpls_disjoint_two_stage", "not_claimed_as_screenshot_parity"),
    },
}


SECTION_CHECKS: dict[str, dict[str, list[tuple[str, str]]]] = {
    "v170_confidence_scope": {
        "src/components/Ui.tsx": [
            ("Why trust this result?", "method-scope drawer entry point"),
            ("Method Confidence", "run confidence panel"),
            ("Runtime dependency", "offline/runtime dependency note"),
            ("QuickPLS does not claim SmartPLS project import or equivalence", "bounded non-equivalence wording"),
        ],
        "src/components/AnalysisCatalog.tsx": [
            ("<MethodScopeDrawer", "scope drawer appears in Setup"),
            ("calculation-preview-panel", "calculation output preview"),
        ],
        "src/components/ReportsWorkspace.tsx": [
            ("<MethodScopeDrawer", "scope drawer appears in Reports"),
            ("<MethodConfidencePanel", "method confidence appears in Reports"),
        ],
    },
    "v171_workflow": {
        "src/components/DataWorkspace.tsx": [
            ("Open Model Designer", "Data to Model transition"),
            ("Create Constructs From Prefixes", "prefix-based construct bridge"),
        ],
        "src/components/AnalysisCatalog.tsx": [
            ("After run", "post-run workflow preview"),
            ("Produced outputs", "pre-run output preview"),
            ("Unavailable outputs", "pre-run unavailable output preview"),
        ],
        "src/components/RunWorkspace.tsx": [
            ("Open results", "Run to Results handoff"),
        ],
        "src/components/RunHistory.tsx": [
            ("Prepare report", "Results to Report handoff or report workflow copy"),
        ],
    },
    "v172_sem_designer": {
        "src/components/ModelCanvas.tsx": [
            ("Focus Diagram", "focus diagram mode"),
            ("Check diagram for publication", "publication diagram check"),
            ("shortest boundary", "shortest path geometry comment or copy"),
            ("quickpls:focus-edge", "diagram object focusing from results"),
        ],
        "src/styles.css": [
            ("focus-diagram-mode", "focus mode layout style"),
        ],
    },
    "v173_reportability": {
        "src/components/RunHistory.tsx": [
            ("ReportabilityChecklist", "reportability checklist component usage"),
            ("reportabilityItems", "value-driven checklist builder"),
            ("Indicator reliability", "indicator reliability checklist item"),
            ("Threshold colors", "threshold color control"),
            ("Do not report p values or confidence intervals", "inference caveat"),
        ],
        "src/components/Ui.tsx": [
            ("Reportability checklist", "shared checklist UI"),
            ("Threshold colors are methodological guidance", "threshold caveat"),
        ],
    },
    "v174_research_tables": {
        "src/components/Ui.tsx": [
            ("ResearchTable", "reusable research table shell"),
            ("sticky-col", "sticky first column support"),
            ("rows.length", "row count metadata"),
            ("columns.length", "column count metadata"),
        ],
        "src/components/RunHistory.tsx": [
            ("Search result tables", "results table search"),
            ("Export table", "current table export"),
            ("Result precision", "precision control"),
            ("Copy table", "copy table affordance"),
        ],
    },
    "v175_publication_pack": {
        "src/components/ReportsWorkspace.tsx": [
            ("Reviewer pack", "reviewer-pack preset"),
            ("Reviewer pack contents", "reviewer-pack preview"),
            ("validation artifact index", "validation index reference"),
            ("Include interpretation notes", "explicit interpretation opt-in"),
            ("WYSIWYG SVG export", "audited SVG figure path"),
        ],
    },
    "v176_samples_guided": {
        "src/App.tsx": [
            ("<NativeDesktopApp />", "production entry mounts the native desktop app"),
        ],
        "src/native/NativeDesktopApp.tsx": [
            ('import { BUNDLED_SAMPLE_PROJECTS } from "../domain/bundledSampleCatalog"', "shared production sample catalogue"),
            ("NATIVE_BUNDLED_SAMPLE_PROJECTS.map((sample)", "launcher renders every catalog sample"),
            ("onOpenSample={openNativeSampleProject}", "live launcher binds exact sample action"),
            ('commandEvent("open-demo-project", { sampleId })', "selected sample identity forwarded"),
            ('case "project.open-demo": commandEvent("open-demo-project"); return;', "File menu preserves corporate default"),
        ],
        "src/native/NativeDesktopController.tsx": [
            ("parseNativeSampleProjectId(requestedSampleId)", "native event validates selected catalog identity"),
            ("DEFAULT_BUNDLED_SAMPLE_PROJECT_ID", "native event uses the catalog default"),
            ("launchNativeBundledSampleV1(sampleId", "native controller routes the selected sample by project authority"),
            ("openNativeDemoProject(ordinarySampleId)", "ordinary sample keeps the editable project loader"),
            ('openProjectAtResolvedPath(archivePath, "general_sem_v1")', "General SEM sample uses strict schema-6 activation"),
        ],
        "src/services/projectService.ts": [
            ('invoke<NativeProjectSnapshot>("open_demo_project", { sampleId })', "Tauri invocation forwards camelCase sample ID"),
            ('invoke<unknown>("materialize_bundled_general_sem_sample", { sampleId })', "strict sample invokes the schema-6 materializer"),
        ],
        "src-tauri/src/lib.rs": [
            ("build_bundled_sample_project(sample_id)", "catalog-backed bundled sample selector"),
        ],
        "src-tauri/src/sample_projects.rs": [
            ('include_str!("../../src/data/bundledSampleProjects.v1.json")', "backend embeds the shared sample catalogue"),
            ("serde_json::from_str(SAMPLE_CATALOG_JSON)", "backend parses the shared sample catalogue"),
            (".find(|sample| sample.id == sample_id)", "backend resolves sample identities from the catalogue"),
            ("for sample in &catalog.samples", "backend tests iterate the catalogue"),
            ("append_validated_result(recipe, result)", "sample contains a contract-validated completed result"),
            ("materialize_bundled_general_sem_sample_v1", "General SEM sample uses its strict archive builder"),
            ("save_project(&path, &project)", "sample save test"),
            ("let reopened = load_project(&path).unwrap()", "sample reopen test"),
        ],
        "src/domain/bundledSampleCatalog.ts": [
            ('import bundledSampleCatalogDocument from "../data/bundledSampleProjects.v1.json"', "frontend imports the shared sample catalogue"),
            ("parseBundledSampleCatalog", "frontend validates the shared sample catalogue"),
            ("export const BUNDLED_SAMPLE_PROJECTS", "frontend exposes catalog-backed sample cards"),
            ("parseNativeSampleProjectId", "frontend validates catalog-backed sample identities"),
        ],
    },
}


def read_text(relative_path: str) -> str:
    return (ROOT / relative_path).read_text(encoding="utf-8")


def audit_bundled_sample_catalog() -> tuple[
    list[str],
    list[dict[str, str]],
    list[dict[str, str]],
    dict[str, object],
]:
    checked_files = [SAMPLE_CATALOG_PATH]
    passed_checks: list[dict[str, str]] = []
    missing: list[dict[str, str]] = []

    def check(condition: bool, label: str, detail: str, file: str = SAMPLE_CATALOG_PATH) -> None:
        if not condition:
            missing.append({"file": file, "label": label, "snippet": detail})

    try:
        catalog = json.loads(read_text(SAMPLE_CATALOG_PATH))
    except (OSError, json.JSONDecodeError) as error:
        check(False, "shared sample catalogue parses", str(error))
        return checked_files, passed_checks, missing, {"sample_count": 0, "sample_ids": []}

    check(catalog.get("schemaVersion") == 1, "shared sample catalogue schema", "schemaVersion must equal 1")
    datasets = catalog.get("datasets")
    construct_sets = catalog.get("constructSets")
    samples = catalog.get("samples")
    check(isinstance(datasets, list), "catalog datasets are declared", "datasets must be an array")
    check(isinstance(construct_sets, dict), "catalog construct sets are declared", "constructSets must be an object")
    check(isinstance(samples, list) and bool(samples), "catalog samples are declared", "samples must be a non-empty array")
    if not isinstance(datasets, list) or not isinstance(construct_sets, dict) or not isinstance(samples, list):
        return checked_files, passed_checks, missing, {"sample_count": 0, "sample_ids": []}

    dataset_by_id: dict[str, dict[str, object]] = {}
    dataset_shapes: dict[str, tuple[int, int]] = {}
    dataset_headers: dict[str, set[str]] = {}
    for index, candidate in enumerate(datasets):
        if not isinstance(candidate, dict):
            check(False, f"dataset {index} is structured", "dataset entry must be an object")
            continue
        dataset_id = candidate.get("id")
        check(isinstance(dataset_id, str) and bool(dataset_id), f"dataset {index} has an identity", "id must be a non-empty string")
        if not isinstance(dataset_id, str) or not dataset_id:
            continue
        check(dataset_id not in dataset_by_id, f"dataset {dataset_id} identity is unique", "duplicate dataset id")
        if dataset_id in dataset_by_id:
            continue
        dataset_by_id[dataset_id] = candidate
        source_path = candidate.get("sourcePath")
        if not isinstance(source_path, str) or not source_path:
            check(False, f"dataset {dataset_id} has a source path", "sourcePath must be a non-empty string")
            continue
        checked_files.append(source_path)
        absolute_source = ROOT / source_path
        check(absolute_source.is_file(), f"dataset {dataset_id} source exists", source_path, source_path)
        if not absolute_source.is_file():
            continue
        expected_sha256 = candidate.get("sha256")
        actual_sha256 = hashlib.sha256(absolute_source.read_bytes()).hexdigest()
        check(
            isinstance(expected_sha256, str) and actual_sha256 == expected_sha256,
            f"dataset {dataset_id} digest matches",
            f"expected {expected_sha256!r}, observed {actual_sha256}",
            source_path,
        )
        with absolute_source.open("r", encoding="utf-8-sig", newline="") as handle:
            rows = list(csv.reader(handle))
        if rows:
            dataset_shapes[dataset_id] = (max(0, len(rows) - 1), len(rows[0]))
            dataset_headers[dataset_id] = set(rows[0])
            check(
                all(len(row) == len(rows[0]) for row in rows),
                f"dataset {dataset_id} has a rectangular CSV shape",
                "all CSV rows must have the header width",
                source_path,
            )
        else:
            check(False, f"dataset {dataset_id} is non-empty", "CSV must include a header", source_path)

    sample_ids: list[str] = []
    seen_sample_ids: set[str] = set()
    reference_by_sample_id: dict[str, dict[str, object]] = {}
    for index, candidate in enumerate(samples):
        if not isinstance(candidate, dict):
            check(False, f"sample {index} is structured", "sample entry must be an object")
            continue
        sample_id = candidate.get("id")
        check(isinstance(sample_id, str) and bool(sample_id), f"sample {index} has an identity", "id must be a non-empty string")
        if not isinstance(sample_id, str) or not sample_id:
            continue
        sample_ids.append(sample_id)
        check(sample_id not in seen_sample_ids, f"sample {sample_id} identity is unique", "duplicate sample id")
        seen_sample_ids.add(sample_id)

        dataset_id = candidate.get("datasetId")
        check(
            all(isinstance(candidate.get(field), str) and candidate.get(field).strip() for field in ("label", "detail", "projectName", "sampleVersion")),
            f"sample {sample_id} has complete launcher and project copy",
            "label, detail, projectName, and sampleVersion must be non-empty strings",
        )
        model = candidate.get("model") if isinstance(candidate.get("model"), dict) else {}
        acceptance = candidate.get("acceptance") if isinstance(candidate.get("acceptance"), dict) else {}
        metadata = candidate.get("metadata") if isinstance(candidate.get("metadata"), dict) else {}
        construct_set_id = model.get("constructSetId")
        base_constructs = construct_sets.get(construct_set_id, []) if isinstance(construct_set_id, str) else []
        extra_constructs = model.get("extraConstructs", [])
        paths = model.get("paths", [])
        runs = candidate.get("runs", [])
        declared_constructs = len(base_constructs) + len(extra_constructs) if isinstance(base_constructs, list) and isinstance(extra_constructs, list) else -1
        declared_paths = len(paths) if isinstance(paths, list) else -1
        check(dataset_id in dataset_by_id, f"sample {sample_id} resolves its dataset", f"unknown datasetId {dataset_id!r}")
        check(isinstance(base_constructs, list) and bool(base_constructs), f"sample {sample_id} resolves its construct set", f"unknown constructSetId {construct_set_id!r}")
        check(isinstance(runs, list) and bool(runs), f"sample {sample_id} contains a completed run recipe", "runs must be a non-empty array")
        constructs = [
            construct
            for construct in [*base_constructs, *extra_constructs]
            if isinstance(construct, dict)
        ] if isinstance(base_constructs, list) and isinstance(extra_constructs, list) else []
        raw_construct_ids = [construct.get("id") for construct in constructs]
        construct_ids = [construct_id for construct_id in raw_construct_ids if isinstance(construct_id, str) and construct_id]
        check(
            len(construct_ids) == declared_constructs
            and len(set(construct_ids)) == len(construct_ids),
            f"sample {sample_id} construct identities are complete and unique",
            "base and extra construct ids must be non-empty and unique",
        )
        path_pairs = {
            (path.get("source"), path.get("target"))
            for path in paths
            if isinstance(path, dict)
            and isinstance(path.get("source"), str)
            and isinstance(path.get("target"), str)
        } if isinstance(paths, list) else set()
        check(
            len(path_pairs) == declared_paths
            and all(source in construct_ids and target in construct_ids for source, target in path_pairs),
            f"sample {sample_id} paths are unique and resolve their endpoints",
            "every structural path must be unique and reference known constructs",
        )
        header = dataset_headers.get(dataset_id, set()) if isinstance(dataset_id, str) else set()
        modeled_indicators = [
            indicator
            for construct in base_constructs
            if isinstance(construct, dict) and isinstance(construct.get("indicators"), list)
            for indicator in construct["indicators"]
        ] if isinstance(base_constructs, list) else []
        check(
            bool(header) and all(isinstance(indicator, str) and indicator in header for indicator in modeled_indicators),
            f"sample {sample_id} indicators resolve to its dataset",
            "every base-construct indicator must identify a CSV column",
        )
        check(
            acceptance.get("constructCount") == declared_constructs,
            f"sample {sample_id} construct count matches its model",
            f"acceptance.constructCount must equal {declared_constructs}",
        )
        check(
            acceptance.get("pathCount") == declared_paths,
            f"sample {sample_id} path count matches its model",
            f"acceptance.pathCount must equal {declared_paths}",
        )
        shape = dataset_shapes.get(dataset_id) if isinstance(dataset_id, str) else None
        if shape:
            check(
                acceptance.get("caseCount") == shape[0] and acceptance.get("columnCount") == shape[1],
                f"sample {sample_id} data shape matches its fixture",
                f"acceptance shape must equal {shape[0]} cases by {shape[1]} columns",
            )

        reference_path = acceptance.get("referencePath")
        expected_values = acceptance.get("expectedValueCount")
        if reference_path is not None or expected_values is not None:
            if not isinstance(reference_path, str) or not reference_path:
                check(False, f"sample {sample_id} reference path is declared", "referencePath must be a non-empty string")
            else:
                checked_files.append(reference_path)
                absolute_reference = ROOT / reference_path
                check(absolute_reference.is_file(), f"sample {sample_id} reference exists", reference_path, reference_path)
                if absolute_reference.is_file():
                    try:
                        reference_bytes = absolute_reference.read_bytes()
                        reference = json.loads(reference_bytes.decode("utf-8"))
                        if isinstance(reference, dict):
                            reference_by_sample_id[sample_id] = reference
                        values = reference.get("values") if isinstance(reference, dict) else None
                        check(
                            isinstance(values, dict) and len(values) == expected_values,
                            f"sample {sample_id} reference value count matches",
                            f"expected {expected_values!r} values",
                            reference_path,
                        )
                        expected_reference_hash = metadata.get("smartpls_reference_sha256")
                        observed_reference_hash = hashlib.sha256(reference_bytes).hexdigest()
                        check(
                            metadata.get("smartpls_reference") == reference_path,
                            f"sample {sample_id} metadata resolves the accepted reference",
                            "metadata.smartpls_reference must equal acceptance.referencePath",
                            reference_path,
                        )
                        check(
                            expected_reference_hash == observed_reference_hash,
                            f"sample {sample_id} reference digest matches",
                            f"expected {expected_reference_hash!r}, observed {observed_reference_hash}",
                            reference_path,
                        )
                        reference_sample_id = reference.get("sample_id") if isinstance(reference, dict) else None
                        check(
                            reference_sample_id is None or reference_sample_id == sample_id,
                            f"sample {sample_id} reference identity matches",
                            f"reference sample_id must equal {sample_id!r}",
                            reference_path,
                        )
                        dataset = dataset_by_id.get(dataset_id) if isinstance(dataset_id, str) else None
                        reference_fingerprint = reference.get("dataset_fingerprint") if isinstance(reference, dict) else None
                        check(
                            reference_fingerprint is None
                            or (isinstance(dataset, dict) and reference_fingerprint == dataset.get("expectedFingerprint")),
                            f"sample {sample_id} reference dataset fingerprint matches",
                            "reference dataset_fingerprint must equal the catalog dataset fingerprint",
                            reference_path,
                        )
                        source_screenshot_hash = (
                            reference.get("source_screenshot_sha256") or reference.get("screenshot_sha256")
                        ) if isinstance(reference, dict) else None
                        check(
                            source_screenshot_hash is None
                            or source_screenshot_hash == metadata.get("smartpls_screenshot_sha256"),
                            f"sample {sample_id} screenshot identity matches",
                            "reference screenshot SHA-256 must equal catalog metadata",
                            reference_path,
                        )
                        reference_precision = reference.get("precision_decimals") if isinstance(reference, dict) else None
                        if reference_precision is None and isinstance(reference, dict):
                            reference_precision = reference.get("reference_precision_decimals")
                        check(
                            reference_precision is None
                            or str(reference_precision) == metadata.get("reference_precision_decimals"),
                            f"sample {sample_id} reference precision matches",
                            "reference precision must equal catalog metadata",
                            reference_path,
                        )
                    except json.JSONDecodeError as error:
                        check(False, f"sample {sample_id} reference parses", str(error), reference_path)

    requested_oi_ids = list(REQUESTED_OI_SAMPLE_SEMANTICS)
    expected_sample_ids = [*LEGACY_BUNDLED_SAMPLE_CONTRACTS, *requested_oi_ids]
    check(
        sample_ids == expected_sample_ids,
        "the eight-sample catalog contains all four selected OI additions",
        f"expected sample ids in order: {expected_sample_ids!r}",
    )
    requested_scope_summary: dict[str, dict[str, object]] = {}
    for sample_id, expected in REQUESTED_OI_SAMPLE_SEMANTICS.items():
        sample = next((item for item in samples if isinstance(item, dict) and item.get("id") == sample_id), None)
        if not isinstance(sample, dict):
            continue
        model = sample.get("model") if isinstance(sample.get("model"), dict) else {}
        metadata = sample.get("metadata") if isinstance(sample.get("metadata"), dict) else {}
        label = sample.get("label") if isinstance(sample.get("label"), str) else ""
        detail = sample.get("detail") if isinstance(sample.get("detail"), str) else ""
        check(
            sample.get("datasetId") == "organizational_identification_v1"
            and model.get("constructSetId") == "organizational_identification_v1",
            f"requested sample {sample_id} reuses the OI dataset and construct set",
            "datasetId and model.constructSetId must both be organizational_identification_v1",
        )
        expected_project_kind = expected.get("project_kind", "ordinary_v1")
        observed_project_kind = sample.get("projectKind", "ordinary_v1")
        check(
            observed_project_kind == expected_project_kind,
            f"requested sample {sample_id} uses its declared project authority",
            f"expected projectKind {expected_project_kind!r}, observed {observed_project_kind!r}",
        )
        check(
            all(token.lower() in label.lower() for token in expected["label_tokens"])
            and all(token.lower() in detail.lower() for token in expected["detail_tokens"]),
            f"requested sample {sample_id} copy states its scientific scope",
            f"label/detail must include {expected['label_tokens']!r} and {expected['detail_tokens']!r}",
        )
        observed_paths = {
            (path.get("source"), path.get("target"))
            for path in model.get("paths", [])
            if isinstance(path, dict)
            and isinstance(path.get("source"), str)
            and isinstance(path.get("target"), str)
        } if isinstance(model.get("paths"), list) else set()
        check(
            observed_paths == expected["paths"],
            f"requested sample {sample_id} topology matches its screenshot contract",
            f"expected paths {sorted(expected['paths'])!r}, observed {sorted(observed_paths)!r}",
        )
        observed_interactions = {
            (
                interaction.get("id"),
                interaction.get("predictor"),
                interaction.get("moderator"),
                interaction.get("product_construct"),
                interaction.get("outcome"),
                interaction.get("method"),
            )
            for interaction in model.get("interactions", [])
            if isinstance(interaction, dict)
            and all(
                isinstance(interaction.get(field), str)
                for field in ("id", "predictor", "moderator", "product_construct", "outcome", "method")
            )
        } if isinstance(model.get("interactions"), list) else set()
        check(
            observed_interactions == expected["interactions"],
            f"requested sample {sample_id} interaction semantics match",
            f"expected {expected['interactions']!r}, observed {observed_interactions!r}",
        )
        observed_higher_order = {
            (
                higher_order.get("id"),
                tuple(higher_order.get("components", [])) if isinstance(higher_order.get("components"), list) else (),
                higher_order.get("method"),
            )
            for higher_order in model.get("higher_order_constructs", [])
            if isinstance(higher_order, dict)
            and isinstance(higher_order.get("id"), str)
            and isinstance(higher_order.get("components"), list)
            and all(isinstance(component, str) for component in higher_order.get("components", []))
            and isinstance(higher_order.get("method"), str)
        } if isinstance(model.get("higher_order_constructs"), list) else set()
        check(
            observed_higher_order == expected["higher_order"],
            f"requested sample {sample_id} higher-order semantics match",
            f"expected {expected['higher_order']!r}, observed {observed_higher_order!r}",
        )

        reference = reference_by_sample_id.get(sample_id, {})
        values = reference.get("values") if isinstance(reference.get("values"), dict) else {}
        observed_reference_paths = {
            tuple(key.removeprefix("path:").split("->", 1))
            for key in values
            if isinstance(key, str) and key.startswith("path:") and "->" in key
        }
        observed_reference_interactions = {
            key.split(":", 1)[1]
            for key in values
            if isinstance(key, str)
            and (key.startswith("interaction:") or key.startswith("scientific_gamma:"))
        }
        check(
            reference.get("reference_kind") == expected["reference_kind"],
            f"requested sample {sample_id} reference scope is explicit",
            f"reference_kind must equal {expected['reference_kind']!r}",
        )
        check(
            observed_reference_paths == expected["reference_paths"]
            and observed_reference_interactions == expected["reference_interactions"],
            f"requested sample {sample_id} reference parameters match its declared evidence scope",
            "reference path and interaction keys must match the accepted screenshot/current-engine scope",
        )
        secondary_field = expected["secondary_screenshot_field"]
        if isinstance(secondary_field, str):
            check(
                isinstance(reference.get(secondary_field), dict) and bool(reference.get(secondary_field)),
                f"requested sample {sample_id} preserves the supplied screenshot comparison",
                f"reference must contain a non-empty {secondary_field}",
            )
        reference_boundary = reference.get("qualification_boundary") or reference.get("evidence_boundary") or ""
        metadata_boundary = metadata.get("evidence_boundary") if isinstance(metadata.get("evidence_boundary"), str) else ""
        check(
            all(token.lower() in str(reference_boundary).lower() for token in expected["reference_boundary_tokens"]),
            f"requested sample {sample_id} reference discloses its scope boundary",
            f"reference boundary must include {expected['reference_boundary_tokens']!r}",
        )
        check(
            all(token.lower() in metadata_boundary.lower() for token in expected["metadata_boundary_tokens"]),
            f"requested sample {sample_id} catalog metadata discloses its scope boundary",
            f"metadata evidence_boundary must include {expected['metadata_boundary_tokens']!r}",
        )
        requested_scope_summary[sample_id] = {
            "reference_kind": reference.get("reference_kind"),
            "reference_path": sample.get("acceptance", {}).get("referencePath") if isinstance(sample.get("acceptance"), dict) else None,
            "evidence_boundary": metadata_boundary,
            "uses_shared_oi_dataset": sample.get("datasetId") == "organizational_identification_v1",
        }

    legacy_ids = list(LEGACY_BUNDLED_SAMPLE_CONTRACTS)
    check(sample_ids[: len(legacy_ids)] == legacy_ids, "legacy sample identities and order are preserved", repr(legacy_ids))
    check(catalog.get("defaultSampleId") == "corporate_reputation", "legacy default sample is preserved", "defaultSampleId must remain corporate_reputation")
    for sample_id, expected in LEGACY_BUNDLED_SAMPLE_CONTRACTS.items():
        sample = next((item for item in samples if isinstance(item, dict) and item.get("id") == sample_id), None)
        dataset = dataset_by_id.get(sample.get("datasetId")) if isinstance(sample, dict) else None
        acceptance = sample.get("acceptance") if isinstance(sample, dict) and isinstance(sample.get("acceptance"), dict) else {}
        observed = {
            "projectName": sample.get("projectName") if isinstance(sample, dict) else None,
            "datasetFileName": dataset.get("fileName") if isinstance(dataset, dict) else None,
            "caseCount": acceptance.get("caseCount"),
            "constructCount": acceptance.get("constructCount"),
            "pathCount": acceptance.get("pathCount"),
        }
        check(observed == expected, f"legacy sample {sample_id} contract is preserved", f"expected {expected!r}, observed {observed!r}")

    check(
        isinstance(catalog.get("defaultSampleId"), str) and catalog.get("defaultSampleId") in seen_sample_ids,
        "catalog default identifies a listed sample",
        "defaultSampleId must resolve to samples[].id",
    )
    if not missing:
        passed_checks.append({"file": SAMPLE_CATALOG_PATH, "label": "shared sample catalogue contract"})
        passed_checks.extend(
            {"file": str(dataset["sourcePath"]), "label": f"catalog dataset {dataset_id} contract"}
            for dataset_id, dataset in dataset_by_id.items()
        )
        passed_checks.extend(
            {"file": SAMPLE_CATALOG_PATH, "label": f"catalog sample {sample_id} contract"}
            for sample_id in sample_ids
        )
        passed_checks.append({"file": SAMPLE_CATALOG_PATH, "label": "legacy bundled sample contracts"})
    return checked_files, passed_checks, missing, {
        "sample_count": len(sample_ids),
        "sample_ids": sample_ids,
        "dataset_count": len(dataset_by_id),
        "legacy_sample_ids": legacy_ids,
        "requested_oi_samples": requested_scope_summary,
    }


def run_section(section: str, result_name: str) -> bool:
    checks = SECTION_CHECKS[section]
    missing: list[dict[str, str]] = []
    passed_checks: list[dict[str, str]] = []
    for relative_path, snippets in checks.items():
        text = read_text(relative_path)
        for snippet, label in snippets:
            if snippet in text:
                passed_checks.append({"file": relative_path, "label": label})
            else:
                missing.append({"file": relative_path, "label": label, "snippet": snippet})
    catalog_summary: dict[str, object] | None = None
    checked_files = list(checks.keys())
    if section == "v176_samples_guided":
        catalog_files, catalog_passed, catalog_missing, catalog_summary = audit_bundled_sample_catalog()
        checked_files.extend(catalog_files)
        passed_checks.extend(catalog_passed)
        missing.extend(catalog_missing)
    native_backend_bound = section == "v176_samples_guided"
    payload = {
        "passed": not missing,
        "section": section,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "checked_files": sorted(set(checked_files)),
        "passed_checks": passed_checks,
        "missing": missing,
        "frontend_only": not native_backend_bound,
        "native_backend_bound": native_backend_bound,
        "numerical_outputs_changed": False,
    }
    if catalog_summary is not None:
        payload["bundled_sample_catalog"] = catalog_summary
    RESULTS.mkdir(parents=True, exist_ok=True)
    (RESULTS / result_name).write_text(json.dumps(payload, indent=2), encoding="utf-8")
    if missing:
        for item in missing:
            print(f"missing {item['label']} in {item['file']}: {item['snippet']}")
    print(json.dumps({"result": result_name, "passed": payload["passed"]}, indent=2))
    return payload["passed"]


def aggregate(result_files: Iterable[str], result_name: str) -> bool:
    missing_files = [name for name in result_files if not (RESULTS / name).exists()]
    failed = []
    for name in result_files:
        if name in missing_files:
            continue
        data = json.loads((RESULTS / name).read_text(encoding="utf-8"))
        if not data.get("passed"):
            failed.append(name)
    payload = {
        "passed": not missing_files and not failed,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "required_artifacts": list(result_files),
        "missing_files": missing_files,
        "failed_artifacts": failed,
        "frontend_only": True,
        "numerical_outputs_changed": False,
    }
    RESULTS.mkdir(parents=True, exist_ok=True)
    (RESULTS / result_name).write_text(json.dumps(payload, indent=2), encoding="utf-8")
    if missing_files or failed:
        print(json.dumps(payload, indent=2))
    else:
        print(json.dumps({"result": result_name, "passed": True}, indent=2))
    return payload["passed"]
