import { describe, expect, it } from "vitest";
import type { CanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import {
  canonicalResultDocumentJsonSha256V1,
  parseInternalProjectSchema6ResultReadOutcomeV1,
  validateArchivedCbsemMissingDataExecutionV1,
  validateArchivedCbsemCfaScoreLmV1,
  validateArchivedCbsemExactCaseBootstrapV1,
  validateArchivedCbsemRmseaIntervalV1,
  validateArchivedPlsNonlinearEffectsV1,
  validateArchivedPlsScoreExecutionV2,
  type InternalProjectSchema6ResultReadRequestV1,
} from "./internalProjectSchema6ResultRead";
import { cbsemCfaScoreLmChiSquare1PValueV1 } from "./internalRecipeV4CbsemExecution";
import { parseInternalRecipeV4CompletedResultV1 } from "./internalRecipeV4PlsExecution";

const sourceDigest = "d".repeat(64);
const request: InternalProjectSchema6ResultReadRequestV1 = {
  surface: "internal_labs",
  experimentalLabsEnabled: true,
  archivePath: "D:\\study-v6.json",
  expectedSourceSha256: sourceDigest,
};

const CROSS_RUNTIME_CANONICAL_JSON = '{"charts":[],"document_id":"result.contract:1","exclusions":[],"footnotes":[],"notices":[],"presentation":{"chart_defaults":{},"default_section_id":"results","default_table_id":"numeric_contract","missing_value_label":"-","precision":4},"provenance":{"capability_cell":{"capability_id":"smartpls.pls_algorithm","capability_version":"pls_pm_v1","cell_id":"qpls3.pls.algorithm","registry_schema_version":2},"completed_at":"2026-08-14T00:00:01Z","dataset_fingerprint":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","dataset_id":"dataset-1","engine_version":"compiled_recipe_v4_pls_plan_v2_execution_v3","method_version":"pls_pm_v1","model_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","model_id":"model-1","project_id":"project-1","recipe_digest":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc","recipe_id":"recipe-1","run_id":"run-1","seed":null,"started_at":"2026-08-14T00:00:00Z","workers":1},"schema_version":2,"sections":[{"chart_ids":[],"id":"results","table_ids":["numeric_contract"],"title":"Results"}],"tables":[{"columns":[{"data_type":"number","description":"Integral floating-point value","id":"integral","label":"Integral"},{"data_type":"number","description":"Non-integral floating-point value","id":"non_integral","label":"Non-integral"}],"footnote_ids":[],"id":"numeric_contract","rows":[{"cells":[{"kind":"number","value":1.0},{"kind":"number","value":0.9954396945354063}],"id":"row_1"}],"title":"Numeric contract"}],"title":"Cross-runtime numeric contract"}';

function documentFixture(): CanonicalResultDocumentV2 {
  return JSON.parse(CROSS_RUNTIME_CANONICAL_JSON) as CanonicalResultDocumentV2;
}

async function outcomeFixture() {
  const canonicalDocument = documentFixture();
  return {
    status: "ok",
    value: {
      schemaVersion: 1,
      projectId: canonicalDocument.provenance.project_id,
      archivePath: request.archivePath,
      sourceDocumentSha256: sourceDigest,
      canonicalResultDocumentCount: 1,
      documents: [{
        documentId: canonicalDocument.document_id,
        runId: canonicalDocument.provenance.run_id,
        canonicalDocumentSha256: await canonicalResultDocumentJsonSha256V1(
          CROSS_RUNTIME_CANONICAL_JSON,
        ),
        immutable: true,
        canonicalDocumentJson: CROSS_RUNTIME_CANONICAL_JSON,
        canonicalDocument,
      }],
      sourceRecheckedUnchanged: true,
    },
  };
}

function generalSemDocumentFixture(): CanonicalResultDocumentV2 {
  const document = documentFixture();
  const capabilityCell = { ...document.provenance.capability_cell };
  document.provenance.dataset_fingerprint = `v2:${document.provenance.dataset_fingerprint}`;
  document.capability_cells = [capabilityCell];
  document.sections = document.sections.map((section) => ({
    ...section,
    capability_cells: [capabilityCell],
  }));
  document.tables = document.tables.map((table) => ({
    ...table,
    capability_cells: [capabilityCell],
  }));
  document.general_sem_results = {
    schema_version: 1,
    identification_diagnostics: [{
      diagnostic_id: "identification_model_1",
      trace: {
        model_id: document.provenance.model_id,
        capability_cell: capabilityCell,
      },
      scope: "model",
      subject_id: document.provenance.model_id,
      status: "identified",
      code: "identified",
      message: "The compiled model passed identification checks.",
      degrees_of_freedom: 1,
    }],
  };
  return document;
}

async function generalSemOutcomeFixture() {
  const canonicalDocument = generalSemDocumentFixture();
  const canonicalDocumentJson = JSON.stringify(canonicalDocument);
  return {
    status: "ok",
    value: {
      schemaVersion: 1,
      projectId: canonicalDocument.provenance.project_id,
      archivePath: request.archivePath,
      sourceDocumentSha256: sourceDigest,
      canonicalResultDocumentCount: 1,
      documents: [{
        documentId: canonicalDocument.document_id,
        runId: canonicalDocument.provenance.run_id,
        canonicalDocumentSha256: await canonicalResultDocumentJsonSha256V1(
          canonicalDocumentJson,
        ),
        immutable: true,
        canonicalDocumentJson,
        canonicalDocument,
      }],
      sourceRecheckedUnchanged: true,
    },
  };
}

async function synchronizeGeneralSemAttachment(
  response: Awaited<ReturnType<typeof generalSemOutcomeFixture>>,
): Promise<void> {
  const attachment = response.value.documents[0];
  attachment.canonicalDocumentJson = JSON.stringify(attachment.canonicalDocument);
  attachment.canonicalDocumentSha256 = await canonicalResultDocumentJsonSha256V1(
    attachment.canonicalDocumentJson,
  );
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function scoreExecutionDocumentFixture(): CanonicalResultDocumentV2 {
  const document = documentFixture();
  document.provenance.method_version = "pls_score_execution_v2";
  document.provenance.engine_version = "compiled_recipe_v4_pls_plan_v2_execution_v4";
  const textColumn = (id: string) => ({
    id, label: id, data_type: "text" as const, description: id,
  });
  const numberColumn = (id: string) => ({
    id, label: id, data_type: "number" as const, description: id,
  });
  document.tables = [
    {
      id: "estimation_summary",
      title: "Estimation summary",
      columns: [
        { id: "converged", label: "Converged", data_type: "boolean", description: "Converged" },
        numberColumn("iterations"),
        numberColumn("used_observations"),
        numberColumn("omitted_observations"),
      ],
      rows: [{
        id: "run",
        cells: [
          { kind: "boolean", value: true },
          { kind: "number", value: 4 },
          { kind: "number", value: 100 },
          { kind: "number", value: 0 },
        ],
      }],
      footnote_ids: [],
    },
    {
      id: "score_execution_summary",
      title: "Score execution",
      columns: [
        textColumn("contract_version"),
        numberColumn("maximum_iterations"),
        numberColumn("stop_criterion"),
        numberColumn("estimated_block_count"),
        numberColumn("fixed_block_count"),
        numberColumn("performed_iterations"),
        numberColumn("estimated_block_updates"),
      ],
      rows: [{
        id: "execution",
        cells: [
          { kind: "text", value: "pls_score_execution_v2" },
          { kind: "number", value: 3_000 },
          { kind: "number", value: 1e-7 },
          { kind: "number", value: 1 },
          { kind: "number", value: 1 },
          { kind: "number", value: 4 },
          { kind: "number", value: 4 },
        ],
      }],
      footnote_ids: [],
    },
    {
      id: "score_execution_weights",
      title: "Score weights",
      columns: [
        textColumn("construct_id"),
        textColumn("indicator_id"),
        textColumn("block_kind"),
        textColumn("estimated_mode"),
        textColumn("requested_initialization"),
        textColumn("normalization"),
        numberColumn("requested_weight"),
        numberColumn("resolved_initial_or_fixed_weight"),
        numberColumn("final_outer_weight"),
      ],
      rows: [
        {
          id: "score_weight_0000",
          cells: [
            { kind: "text", value: "x" },
            { kind: "text", value: "x_stable" },
            { kind: "text", value: "estimated" },
            { kind: "text", value: "mode_a" },
            { kind: "text", value: "standard" },
            { kind: "missing", reason: "not_applicable" },
            { kind: "number", value: 1 },
            { kind: "number", value: 0.5 },
            { kind: "number", value: -0.27 },
          ],
        },
        {
          id: "score_weight_0001",
          cells: [
            { kind: "text", value: "y" },
            { kind: "text", value: "y_stable" },
            { kind: "text", value: "fixed_unit" },
            { kind: "missing", reason: "not_applicable" },
            { kind: "missing", reason: "not_applicable" },
            { kind: "text", value: "unit_variance" },
            { kind: "number", value: 1 },
            { kind: "number", value: 0.75 },
            { kind: "number", value: 0.75 },
          ],
        },
      ],
      footnote_ids: [],
    },
  ];
  document.sections = [{
    id: "results",
    title: "Results",
    table_ids: document.tables.map((table) => table.id),
    chart_ids: [],
  }];
  document.presentation.default_table_id = "score_execution_weights";
  return document;
}

function fixedCustomNormalizationDocument(
  normalization: "none" | "sum_to_one" | "unit_variance",
  requested: readonly [number, number],
  resolved: readonly [number, number],
): CanonicalResultDocumentV2 {
  const document = scoreExecutionDocumentFixture();
  const table = document.tables.find((candidate) => candidate.id === "score_execution_weights")!;
  const first = table.rows[1];
  first.cells[2] = { kind: "text", value: "fixed_custom" };
  first.cells[5] = { kind: "text", value: normalization };
  first.cells[6] = { kind: "number", value: requested[0] };
  first.cells[7] = { kind: "number", value: resolved[0] };
  first.cells[8] = { kind: "number", value: resolved[0] };
  const second = clone(first);
  second.id = "score_weight_0002";
  second.cells[1] = { kind: "text", value: "y_stable_2" };
  second.cells[6] = { kind: "number", value: requested[1] };
  second.cells[7] = { kind: "number", value: resolved[1] };
  second.cells[8] = { kind: "number", value: resolved[1] };
  table.rows.push(second);
  return document;
}

function appendFixedScaleReceipt(
  document: CanonicalResultDocumentV2,
  scale = 0.5,
  center = 0.125,
): CanonicalResultDocumentV2 {
  const weights = document.tables.find((table) => table.id === "score_execution_weights")!;
  const fixedRows = weights.rows.filter((row) => (
    row.cells[2].kind === "text" && row.cells[2].value !== "estimated"
  ));
  document.tables.push({
    id: "fixed_score_scale_receipt",
    title: "Fixed-score scale receipt",
    columns: [
      "contract_version",
      "construct_id",
      "indicator_id",
      "pre_standardization_center",
      "pre_standardization_scale",
      "resolved_scoring_coefficient",
      "effective_unit_score_weight",
    ].map((id) => ({ id, label: id, data_type: id.includes("version") || id.includes("_id") ? "text" as const : "number" as const, description: id })),
    rows: fixedRows.map((row, index) => {
      const coefficient = row.cells[7].kind === "number" ? row.cells[7].value : Number.NaN;
      return {
        id: `fixed_score_scale_${index.toString().padStart(4, "0")}`,
        cells: [
          { kind: "text" as const, value: "pls_fixed_score_scale_receipt_v1" },
          clone(row.cells[0]),
          clone(row.cells[1]),
          { kind: "number" as const, value: center },
          { kind: "number" as const, value: scale },
          { kind: "number" as const, value: coefficient },
          { kind: "number" as const, value: coefficient / scale },
        ],
      };
    }),
    footnote_ids: [],
  });
  const runDetails = document.sections.find((section) => section.id === "run_details");
  if (runDetails) runDetails.table_ids.push("fixed_score_scale_receipt");
  else document.sections.push({
    id: "run_details",
    title: "Run details",
    table_ids: ["fixed_score_scale_receipt"],
    chart_ids: [],
  });
  return document;
}

function appendCurrentPlsTypedFamilies(document: CanonicalResultDocumentV2): CanonicalResultDocumentV2 {
  const textColumn = (id: string) => ({ id, label: id, data_type: "text" as const, description: id });
  const numberColumn = (id: string) => ({ id, label: id, data_type: "number" as const, description: id });
  const text = (value: string) => ({ kind: "text" as const, value });
  const number = (value: number) => ({ kind: "number" as const, value });
  document.tables.push(
    {
      id: "point_estimate_attribution",
      title: "Point attribution",
      columns: [
        "contract_version", "preprocessing", "indicator_centering", "indicator_scaling",
        "outer_weights", "outer_loadings", "construct_scores", "structural_paths", "effects",
      ].map(textColumn),
      rows: [{
        id: "attribution",
        cells: [
          "pls_point_estimate_attribution_v1",
          "standardized",
          "sample_mean",
          "sample_standard_deviation",
          "preprocessed_indicator_to_unit_variance_construct_score",
          "indicator_construct_score_correlation",
          "zero_mean_unit_variance_construct_score",
          "standardized_construct_score_regression",
          "standardized_structural_path_decomposition",
        ].map(text),
      }],
      footnote_ids: [],
    },
    {
      id: "algorithm_convergence_receipt",
      title: "Convergence",
      columns: [
        textColumn("contract_version"), textColumn("weighting_scheme"),
        numberColumn("maximum_iterations"), numberColumn("stop_criterion"),
        textColumn("comparison"), numberColumn("performed_iterations"),
        numberColumn("estimated_block_updates"), textColumn("termination_reason"),
        numberColumn("final_max_outer_weight_change"),
      ],
      rows: [{
        id: "convergence",
        cells: [
          text("pls_algorithm_convergence_receipt_v1"), text("path"), number(3_000),
          number(1e-7), text("less_than_or_equal"), number(4), number(4),
          text("converged_tolerance"), number(1e-8),
        ],
      }],
      footnote_ids: [],
    },
    {
      id: "algorithm_block_order",
      title: "Block order",
      columns: [
        numberColumn("block_ordinal"), textColumn("construct_id"),
        numberColumn("indicator_ordinal"), textColumn("indicator_id"),
        textColumn("update_rule"), textColumn("initialization"),
      ],
      rows: [
        {
          id: "algorithm_block_0000_indicator_0000",
          cells: [number(0), text("x"), number(0), text("x_stable"), text("mode_a_covariance"), text("standard_unit_weights")],
        },
        {
          id: "algorithm_block_0001_indicator_0000",
          cells: [number(1), text("y"), number(0), text("y_stable"), text("fixed_no_update"), text("fixed_unit_weights")],
        },
      ],
      footnote_ids: [],
    },
  );
  document.sections.push({
    id: "run_details",
    title: "Run details",
    table_ids: ["point_estimate_attribution", "algorithm_convergence_receipt", "algorithm_block_order"],
    chart_ids: [],
  });
  return document;
}

function nonlinearDocumentFixture(): CanonicalResultDocumentV2 {
  const document = scoreExecutionDocumentFixture();
  document.tables = document.tables.filter((table) => table.id === "estimation_summary");
  document.sections = [];
  appendCurrentPlsTypedFamilies(document);
  const pointAttribution = clone(
    document.tables.find((table) => table.id === "point_estimate_attribution")!,
  );
  document.tables = [document.tables.find((table) => table.id === "estimation_summary")!];
  const base = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.pls_algorithm",
    cell_id: "qpls3.pls.algorithm",
    capability_version: "pls_pm_v1",
  };
  const primary = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.nonlinear_relationships",
    cell_id: "qpls3.pls.nonlinear_quadratic",
    capability_version: "pls_quadratic_nonlinear_effects_v1",
  };
  const textColumn = (id: string) => ({ id, label: id, data_type: "text" as const, description: id });
  const numberColumn = (id: string) => ({ id, label: id, data_type: "number" as const, description: id });
  const text = (value: string) => ({ kind: "text" as const, value });
  const number = (value: number) => ({ kind: "number" as const, value });
  document.title = "PLS nonlinear quadratic diagnostics";
  document.provenance.capability_cell = primary;
  document.provenance.method_version = "pls_quadratic_nonlinear_effects_v1";
  document.provenance.engine_version = "compiled_recipe_v4_pls_plan_v2_execution_v7";
  document.capability_cells = [primary, base];
  document.tables.push(
    {
      id: "outer_model",
      title: "Outer model",
      columns: [textColumn("construct"), textColumn("indicator"), numberColumn("weight"), numberColumn("loading")],
      rows: [{ id: "outer_0000", cells: [text("x"), text("x_one"), number(1), number(0.8)] }],
      footnote_ids: [],
    },
    {
      id: "structural_paths",
      title: "Structural paths",
      columns: [textColumn("source"), textColumn("target"), numberColumn("coefficient")],
      rows: [{ id: "path_0000", cells: [text("x"), text("y"), number(0.25)] }],
      footnote_ids: [],
    },
    {
      id: "effects",
      title: "Effects",
      columns: [textColumn("source"), textColumn("target"), numberColumn("direct"), numberColumn("indirect"), numberColumn("total")],
      rows: [{ id: "effect_0000", cells: [text("x"), text("y"), number(0.25), number(0), number(0.25)] }],
      footnote_ids: [],
    },
    {
      id: "r_squared",
      title: "R-squared",
      columns: [textColumn("construct"), numberColumn("r_squared")],
      rows: [{ id: "r_squared_0000", cells: [text("y"), number(0.4)] }],
      footnote_ids: [],
    },
    pointAttribution,
    {
      id: "nonlinear_quadratic_diagnostics",
      title: "Quadratic diagnostics",
      columns: [
        textColumn("source"), textColumn("target"), numberColumn("linear_coefficient"),
        numberColumn("quadratic_coefficient"), numberColumn("standard_error"),
        numberColumn("t_statistic"), numberColumn("p_value_two_sided"), textColumn("warning"),
      ],
      rows: [{
        id: "nonlinear_quadratic_diagnostic_0000",
        cells: [
          text("x"), text("y"), number(0.25), number(0.1), number(0.05), number(2),
          number(0.0455), { kind: "missing", reason: "not_estimated" },
        ],
      }],
      footnote_ids: [],
    },
    {
      id: "nonlinear_equation_fit",
      title: "Nonlinear equation fit",
      columns: [
        textColumn("target"), numberColumn("linear_r_squared"),
        numberColumn("augmented_r_squared"), numberColumn("delta_r_squared"),
      ],
      rows: [{
        id: "nonlinear_equation_fit_0000",
        cells: [text("y"), number(0.4), number(0.45), number(0.04999999999999999)],
      }],
      footnote_ids: [],
    },
    {
      id: "nonlinear_method_scope",
      title: "Nonlinear method scope",
      columns: [textColumn("method_version"), textColumn("term"), textColumn("warning")],
      rows: [{
        id: "nonlinear_method_scope",
        cells: [
          text("pls_quadratic_nonlinear_effects_v1"),
          text("centered_squared_construct_score_v1"),
          text("Nonlinear effects are validated for the documented QuickPLS v1.2.3 fixed-score quadratic diagnostic scope; diagnostics use fixed PLS construct scores and centered squared score terms."),
        ],
      }],
      footnote_ids: [],
    },
  );
  document.tables.forEach((table) => {
    table.capability_cells = [
      ["nonlinear_quadratic_diagnostics", "nonlinear_equation_fit", "nonlinear_method_scope"].includes(table.id)
        ? primary
        : base,
    ];
  });
  document.sections = [
    {
      id: "run_details", title: "Run details",
      table_ids: ["estimation_summary", "point_estimate_attribution"],
      chart_ids: [], capability_cells: [base],
    },
    { id: "measurement_model", title: "Measurement model", table_ids: ["outer_model"], chart_ids: [], capability_cells: [base] },
    { id: "structural_model", title: "Structural model", table_ids: ["structural_paths", "effects", "r_squared"], chart_ids: [], capability_cells: [base] },
    {
      id: "nonlinear_relationships", title: "Nonlinear relationships",
      table_ids: ["nonlinear_quadratic_diagnostics", "nonlinear_equation_fit", "nonlinear_method_scope"],
      chart_ids: [], capability_cells: [primary],
    },
  ];
  document.presentation.default_section_id = "nonlinear_relationships";
  document.presentation.default_table_id = "nonlinear_quadratic_diagnostics";
  return document;
}

function meanReplacementDocumentFixture(): CanonicalResultDocumentV2 {
  const document = documentFixture();
  document.provenance.capability_cell = {
    registry_schema_version: 2,
    capability_id: "smartpls.cbsem",
    cell_id: "qpls3.cbsem.ml",
    capability_version: "cbsem_ml_v1",
  };
  document.capability_cells = [document.provenance.capability_cell];
  document.provenance.method_version = "cbsem_ml_exact_parameter_table_v3";
  document.provenance.engine_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v4";
  const textColumn = (id: string) => ({
    id, label: id, data_type: "text" as const, description: id,
  });
  const numberColumn = (id: string) => ({
    id, label: id, data_type: "number" as const, description: id,
  });
  const booleanColumn = (id: string) => ({
    id, label: id, data_type: "boolean" as const, description: id,
  });
  const number = (value: number) => ({ kind: "number" as const, value });
  const text = (value: string) => ({ kind: "text" as const, value });
  const boolean = (value: boolean) => ({ kind: "boolean" as const, value });
  const sourceFingerprint = document.provenance.dataset_fingerprint;
  document.tables = [
    {
      id: "estimation_summary",
      title: "Estimation summary",
      columns: [
        textColumn("execution_adapter_version"),
        textColumn("estimator_method_version"),
        textColumn("moment_input_method_version"),
        numberColumn("compiled_moment_schema_version"),
        booleanColumn("mean_structure"),
        textColumn("input"),
        booleanColumn("converged"),
        numberColumn("sample_size"),
        numberColumn("declared_sample_size"),
        numberColumn("omitted_observations"),
        textColumn("covariance_denominator"),
        textColumn("canonical_covariance_sha256"),
        textColumn("canonical_observed_means_sha256"),
      ],
      rows: [{
        id: "run",
        cells: [
          text("compiled_recipe_v4_cbsem_plan_v2_execution_v4"),
          text("cbsem_ml_exact_parameter_table_v3"),
          text("cbsem_ml_compiled_moment_input_mean_replacement_v1"),
          number(4),
          boolean(false),
          text("raw"),
          boolean(true),
          number(20),
          { kind: "missing", reason: "not_estimated" },
          number(0),
          text("maximum_likelihood_n"),
          text("9".repeat(64)),
          { kind: "missing", reason: "not_estimated" },
        ],
      }],
      footnote_ids: [],
    },
    {
      id: "canonical_ml_covariance",
      title: "Canonical covariance",
      columns: [
        textColumn("row"),
        { ...numberColumn("column_0000"), label: "x" },
        { ...numberColumn("column_0001"), label: "y" },
      ],
      rows: [
        { id: "row_0000", cells: [text("x"), number(1), number(0.2)] },
        { id: "row_0001", cells: [text("y"), number(0.2), number(1)] },
      ],
      footnote_ids: [],
    },
    {
      id: "missing_data_execution",
      title: "Missing-data execution",
      columns: [
        textColumn("method_version"),
        textColumn("policy"),
        textColumn("archive_validation_scope"),
        booleanColumn("raw_replay_performed"),
        textColumn("source_dataset_id"),
        textColumn("source_dataset_fingerprint"),
        numberColumn("source_row_count"),
        numberColumn("retained_row_count"),
        numberColumn("omitted_row_count"),
        numberColumn("modeled_variable_count"),
        numberColumn("imputed_cell_count"),
        numberColumn("affected_case_count"),
        numberColumn("variable_warning_threshold"),
        numberColumn("high_missingness_threshold"),
        textColumn("missingness_sha256"),
        textColumn("completed_matrix_sha256"),
        textColumn("receipt_sha256"),
      ],
      rows: [{
        id: "execution",
        cells: [
          text("mean_replacement_v1"),
          text("mean_replacement"),
          text("descriptor_identity_shape_and_receipt_only"),
          boolean(false),
          text(document.provenance.dataset_id),
          text(sourceFingerprint),
          number(20),
          number(20),
          number(0),
          number(2),
          number(3),
          number(2),
          number(0.05),
          number(0.15),
          text("1".repeat(64)),
          text("2".repeat(64)),
          text("3".repeat(64)),
        ],
      }],
      footnote_ids: [],
    },
    {
      id: "mean_replacement_variables",
      title: "Variables",
      columns: [
        numberColumn("variable_order"),
        textColumn("variable_id"),
        textColumn("source_column"),
        textColumn("canonical_missing_markers_json"),
        numberColumn("observed_count"),
        numberColumn("missing_count"),
        numberColumn("replacement_mean"),
        numberColumn("missing_fraction"),
        textColumn("warning_level"),
      ],
      rows: [
        {
          id: "mean_replacement_variable_0000",
          cells: [
            number(0), text("observed:x"), text("x"), text("[\"NA\"]"),
            number(19), number(1), number(10), number(0.05), text("at_least_five_percent"),
          ],
        },
        {
          id: "mean_replacement_variable_0001",
          cells: [
            number(1), text("observed:y"), text("y"), text("[\"NA\"]"),
            number(18), number(2), number(20), number(0.1), text("at_least_five_percent"),
          ],
        },
      ],
      footnote_ids: [],
    },
    {
      id: "mean_replacement_cells",
      title: "Cells",
      columns: [
        numberColumn("row_index_zero_based"),
        numberColumn("variable_order"),
        textColumn("variable_id"),
        textColumn("source_column"),
        numberColumn("replacement_mean"),
        numberColumn("case_missing_fraction"),
        booleanColumn("high_missingness_warning"),
      ],
      rows: [
        {
          id: "mean_replacement_cell_000000",
          cells: [number(0), number(0), text("observed:x"), text("x"), number(10), number(1), boolean(true)],
        },
        {
          id: "mean_replacement_cell_000001",
          cells: [number(0), number(1), text("observed:y"), text("y"), number(20), number(1), boolean(true)],
        },
        {
          id: "mean_replacement_cell_000002",
          cells: [number(1), number(1), text("observed:y"), text("y"), number(20), number(0.5), boolean(true)],
        },
      ],
      footnote_ids: [],
    },
  ];
  document.sections = [
    { id: "run", title: "Run", table_ids: ["estimation_summary", "canonical_ml_covariance"], chart_ids: [] },
    {
      id: "missing_data",
      title: "Missing data",
      table_ids: ["missing_data_execution", "mean_replacement_variables", "mean_replacement_cells"],
      chart_ids: [],
    },
  ];
  document.presentation.default_section_id = "missing_data";
  document.presentation.default_table_id = "missing_data_execution";
  return document;
}

function rmseaIntervalDocumentFixture(
  adapter = "compiled_recipe_v4_cbsem_plan_v2_execution_v5",
): CanonicalResultDocumentV2 {
  const document = documentFixture();
  document.provenance.capability_cell = {
    registry_schema_version: 2,
    capability_id: "smartpls.cbsem",
    cell_id: "qpls3.cbsem.ml",
    capability_version: "cbsem_ml_v1",
  };
  document.capability_cells = [document.provenance.capability_cell];
  document.provenance.engine_version = adapter;
  document.provenance.method_version = adapter.endsWith("_v3") || adapter.endsWith("_v6")
    ? "cbsem_ml_exact_parameter_table_v4"
    : "cbsem_ml_exact_parameter_table_v3";
  const columns = [
    "fit_method_version",
    "chi_square",
    "degrees_of_freedom",
    "p_value",
    "cfi",
    "tli",
    "rmsea",
    "rmsea_interval_method_version",
    "rmsea_interval_confidence_level",
    "rmsea_ci_lower",
    "rmsea_ci_upper",
    "srmr",
    "aic",
    "bic",
  ];
  document.tables = [{
    id: "fit_indices",
    title: "Fit indices",
    columns: columns.map((id, index) => ({
      id,
      label: id,
      data_type: index === 0 || index === 7 ? "text" as const : "number" as const,
      description: id,
    })),
    rows: [{
      id: "model",
      cells: [
        { kind: "text", value: "cbsem_fit_v1" },
        { kind: "number", value: 12.4 },
        { kind: "number", value: 5 },
        { kind: "number", value: 0.0296 },
        { kind: "number", value: 0.982 },
        { kind: "number", value: 0.973 },
        { kind: "number", value: 0.072 },
        { kind: "text", value: "rmsea_noncentral_chi_square_inversion_90_n_minus_one_v1" },
        { kind: "number", value: 0.9 },
        { kind: "number", value: 0.021 },
        { kind: "number", value: 0.121 },
        { kind: "number", value: 0.031 },
        { kind: "number", value: 1012.4 },
        { kind: "number", value: 1054.2 },
      ],
    }],
    footnote_ids: [],
  }];
  return document;
}

function appendScoreLmTable(document: CanonicalResultDocumentV2): void {
  const ids = [
    "method_version", "scope", "parameter_id", "kind", "lhs", "rhs", "status",
    "score", "efficient_score", "candidate_information", "efficient_information",
    "modification_index", "expected_parameter_change", "degrees_of_freedom", "p_value",
    "unavailable_reason",
  ];
  document.tables.push({
    id: "modification_index_score_tests",
    title: "Score tests",
    columns: ids.map((id, index) => ({
      id, label: id, data_type: index <= 6 || index === 15 ? "text" as const : "number" as const,
      description: id,
    })),
    rows: [{
      id: "score_lm_0000",
      cells: [
        { kind: "text", value: "cbsem_cfa_score_lm_v1" },
        { kind: "text", value: "covariance_only_declared_zero_residual_covariances" },
        { kind: "text", value: "parameter:residual_covariance:x1:x2" },
        { kind: "text", value: "residual_covariance" },
        { kind: "text", value: "x1" },
        { kind: "text", value: "x2" },
        { kind: "text", value: "available" },
        { kind: "number", value: 2 },
        { kind: "number", value: 2 },
        { kind: "number", value: 1 },
        { kind: "number", value: 1 },
        { kind: "number", value: 4 },
        { kind: "number", value: 2 },
        { kind: "number", value: 1 },
        { kind: "number", value: cbsemCfaScoreLmChiSquare1PValueV1(4) },
        { kind: "missing", reason: "not_applicable" },
      ],
    }],
    footnote_ids: [],
  });
  document.sections.push({
    id: "modification_indices",
    title: "Modification indices",
    table_ids: ["modification_index_score_tests"],
    chart_ids: [],
  });
}

function exactCaseBootstrapDocumentFixture(): CanonicalResultDocumentV2 {
  const document = rmseaIntervalDocumentFixture("compiled_recipe_v4_cbsem_plan_v2_execution_v9");
  appendScoreLmTable(document);
  document.provenance.seed = 91;
  const textColumn = (id: string) => ({ id, label: id, data_type: "text" as const, description: id });
  const numberColumn = (id: string) => ({ id, label: id, data_type: "number" as const, description: id });
  const summaryColumns = [
    "method_version", "estimator_method_version", "source_dataset_id", "source_dataset_fingerprint",
    "outer_recipe_analytical_identity_sha256", "base_point_result_sha256", "compiler_analytical_identity_sha256",
    "plan_sha256", "model_scientific_sha256", "complete_case_sample_size", "complete_case_universe_digest_method",
    "complete_case_universe_sha256", "covariance_denominator", "sample_indices_digest_method",
    "sampling_positions_digest_method", "interval_method", "confidence_level", "requested_replicates",
    "attempted_refits", "usable_replicates", "failed_replicates", "minimum_usable_fraction",
    "minimum_usable_replicates", "seed_decimal", "stream_token", "retry_policy", "max_attempts_per_replicate",
    "parameter_ids_json", "inference_status", "unavailable_reason_code", "unavailable_message", "archive_validation_scope",
  ];
  const textIndices = new Set([0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 23, 24, 25, 27, 28, 29, 30, 31]);
  document.tables.push({
    id: "estimation_summary", title: "Estimation", columns: Array.from({ length: 13 }, (_, index) => numberColumn(`c${index}`)),
    rows: [{ id: "run", cells: Array.from({ length: 13 }, (_, index) => ({ kind: "number" as const, value: index === 12 ? 10 : 1 })) }], footnote_ids: [],
  }, {
    id: "parameters", title: "Parameters",
    columns: ["name", "parameter_id", "kind", "lhs", "rhs", "estimate", "standard_error", "z", "p_two_sided", "fixed"].map((id, index) => ({ id, label: id, data_type: index < 5 ? "text" as const : index === 9 ? "boolean" as const : "number" as const, description: id })),
    rows: [{ id: "parameter_0000", cells: [
      { kind: "text", value: "loading:x1" }, { kind: "text", value: "parameter:loading:f1:x1" },
      { kind: "text", value: "loading" }, { kind: "text", value: "f1" }, { kind: "text", value: "x1" },
      { kind: "number", value: 1 }, { kind: "number", value: 0.1 }, { kind: "number", value: 10 },
      { kind: "number", value: 0 }, { kind: "boolean", value: false },
    ] }], footnote_ids: [],
  }, {
    id: "exact_case_bootstrap_summary", title: "Summary",
    columns: summaryColumns.map((id, index) => textIndices.has(index) ? textColumn(id) : numberColumn(id)),
    rows: [{ id: "bootstrap", cells: [
      { kind: "text", value: "cbsem_exact_case_bootstrap_v1" },
      { kind: "text", value: "cbsem_ml_exact_parameter_table_v3" },
      { kind: "text", value: document.provenance.dataset_id }, { kind: "text", value: document.provenance.dataset_fingerprint },
      { kind: "text", value: document.provenance.recipe_digest }, { kind: "text", value: "1".repeat(64) },
      { kind: "text", value: "2".repeat(64) }, { kind: "text", value: "3".repeat(64) },
      { kind: "text", value: document.provenance.model_digest }, { kind: "number", value: 10 },
      { kind: "text", value: "sha256_source_fingerprint_and_ordered_complete_case_u64_indices_v1" },
      { kind: "text", value: "4".repeat(64) }, { kind: "text", value: "maximum_likelihood_n" },
      { kind: "text", value: "sha256_source_fingerprint_and_ordered_u64_indices_v1" },
      { kind: "text", value: "sha256_stream_seed_replicate_complete_case_n_and_ordered_sampling_positions_v1" },
      { kind: "text", value: "percentile_type7_v1" }, { kind: "number", value: 0.95 },
      { kind: "number", value: 1000 }, { kind: "number", value: 1000 }, { kind: "number", value: 1000 },
      { kind: "number", value: 0 }, { kind: "number", value: 0.9 }, { kind: "number", value: 1000 },
      { kind: "text", value: "91" }, { kind: "text", value: "quickpls_cbsem_exact_cfa_ml_case_bootstrap_v1" },
      { kind: "text", value: "no_retry_fixed_preplanned_primary_draws_v1" }, { kind: "number", value: 1 },
      { kind: "text", value: "[\"parameter:loading:f1:x1\"]" }, { kind: "text", value: "available" },
      { kind: "missing", reason: "not_applicable" }, { kind: "missing", reason: "not_applicable" },
      { kind: "text", value: "schedule_and_arithmetic_only_no_raw_refit_replay_or_source_row_digest_recomputation" },
    ] }], footnote_ids: [],
  }, {
    id: "exact_case_bootstrap_parameter_intervals", title: "Intervals",
    columns: ["parameter_id", "original", "bootstrap_mean", "bias", "standard_error", "percentile_lower", "percentile_upper", "usable_replicates"].map((id, index) => index === 0 ? textColumn(id) : numberColumn(id)),
    rows: [{ id: "bootstrap_interval_0000", cells: [
      { kind: "text", value: "parameter:loading:f1:x1" }, { kind: "number", value: 1 },
      { kind: "number", value: 2 }, { kind: "number", value: 1 }, { kind: "number", value: 0 },
      { kind: "number", value: 2 }, { kind: "number", value: 2 }, { kind: "number", value: 1000 },
    ] }], footnote_ids: [],
  }, {
    id: "exact_case_bootstrap_successful_refits", title: "Refits",
    columns: ["replicate_index", "sampling_positions_sha256", "sample_indices_sha256", "parameter_estimates_json", "iterations", "objective", "gradient_norm"].map((id, index) => [1, 2, 3].includes(index) ? textColumn(id) : numberColumn(id)),
    rows: Array.from({ length: 1000 }, (_, replicate) => ({
      id: `bootstrap_refit_${String(replicate).padStart(5, "0")}`,
      cells: [{ kind: "number" as const, value: replicate }, { kind: "text" as const, value: "5".repeat(64) },
        { kind: "text" as const, value: "6".repeat(64) }, { kind: "text" as const, value: "[2]" },
        { kind: "number" as const, value: 2 }, { kind: "number" as const, value: 0.1 }, { kind: "number" as const, value: 0.01 }],
    })), footnote_ids: [],
  }, {
    id: "exact_case_bootstrap_failures", title: "Failures",
    columns: ["replicate_index", "sampling_positions_sha256", "sample_indices_sha256", "kind", "message"].map((id, index) => index === 0 ? numberColumn(id) : textColumn(id)),
    rows: [], footnote_ids: [],
  });
  document.sections.push({
    id: "bootstrap_inference", title: "Bootstrap inference",
    table_ids: ["exact_case_bootstrap_summary", "exact_case_bootstrap_parameter_intervals", "exact_case_bootstrap_successful_refits", "exact_case_bootstrap_failures"], chart_ids: [],
  });
  return document;
}

function exactCaseBootstrapHypothesisDocumentFixture(): CanonicalResultDocumentV2 {
  const document = exactCaseBootstrapDocumentFixture();
  document.provenance.engine_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v10";
  const textColumn = (id: string) => ({ id, label: id, data_type: "text" as const, description: id });
  const numberColumn = (id: string) => ({ id, label: id, data_type: "number" as const, description: id });
  const booleanColumn = (id: string) => ({ id, label: id, data_type: "boolean" as const, description: id });
  const columns = [
    "method_version", "null_hypothesis", "statistic", "tie_policy", "probability_method",
    "decision_rule", "selected_test_tail", "null_value", "significance_level", "usable_replicates",
    "inference_status", "global_unavailable_reason_code", "global_unavailable_message", "parameter_id",
    "parameter_status", "point_estimate", "two_sided_exceedances", "greater_or_equal_exceedances",
    "less_or_equal_exceedances", "p_value_two_sided", "p_value_greater", "p_value_less",
    "selected_exceedances", "selected_p_value", "reject_null", "unavailable_reason",
  ];
  const textIndices = new Set([0, 1, 2, 3, 4, 5, 6, 10, 11, 12, 13, 14, 25]);
  document.tables.push({
    id: "exact_case_bootstrap_hypothesis_tests", title: "Hypothesis tests",
    columns: columns.map((id, index) => index === 24 ? booleanColumn(id) : textIndices.has(index) ? textColumn(id) : numberColumn(id)),
    rows: [{ id: "bootstrap_hypothesis_0000", cells: [
      { kind: "text", value: "cbsem_exact_case_bootstrap_null_centered_test_tail_v1" },
      { kind: "text", value: "compiled_free_parameter_equals_zero_v1" },
      { kind: "text", value: "unstudentized_null_centered_parameter_estimate_v1" },
      { kind: "text", value: "inclusive_ieee_comparison_v1" },
      { kind: "text", value: "plus_one_over_usable_plus_one_v1" },
      { kind: "text", value: "selected_p_value_less_than_or_equal_alpha_v1" },
      { kind: "text", value: "one_sided_greater" }, { kind: "number", value: 0 },
      { kind: "number", value: 0.05 }, { kind: "number", value: 1000 },
      { kind: "text", value: "available" }, { kind: "missing", reason: "not_applicable" },
      { kind: "missing", reason: "not_applicable" }, { kind: "text", value: "parameter:loading:f1:x1" },
      { kind: "text", value: "available" }, { kind: "number", value: 1 },
      { kind: "number", value: 1000 }, { kind: "number", value: 1000 }, { kind: "number", value: 1000 },
      { kind: "number", value: 1 }, { kind: "number", value: 1 }, { kind: "number", value: 1 },
      { kind: "number", value: 1000 }, { kind: "number", value: 1 }, { kind: "boolean", value: false },
      { kind: "missing", reason: "not_applicable" },
    ] }], footnote_ids: [],
  });
  document.sections.push({
    id: "bootstrap_hypothesis_tests", title: "Bootstrap hypothesis tests",
    table_ids: ["exact_case_bootstrap_hypothesis_tests"], chart_ids: [],
  });
  return document;
}

function exactCaseBootstrapStudentizedDocumentFixture(): CanonicalResultDocumentV2 {
  const document = exactCaseBootstrapHypothesisDocumentFixture();
  document.provenance.engine_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v11";
  const textColumn = (id: string) => ({
    id, label: id, data_type: "text" as const, description: id,
  });
  const numberColumn = (id: string) => ({
    id, label: id, data_type: "number" as const, description: id,
  });
  const missing = () => ({ kind: "missing" as const, reason: "not_applicable" as const });
  const summaryColumns = [
    "method_version", "standard_error_method_version", "expected_information_method", "pivot_method",
    "quantile_method", "interval_method", "archive_validation_scope", "confidence_level",
    "minimum_usable_fraction", "minimum_usable_replicates", "studentized_usable_replicates",
    "parameter_ids_json", "inference_status", "unavailable_reason_code", "unavailable_message",
  ];
  const pointColumns = [
    "method_version", "parameter_id", "status", "information_method", "standard_error", "unavailable_reason",
  ];
  const intervalColumns = [
    "parameter_id", "status", "point_estimate", "point_standard_error", "lower_pivot_quantile",
    "upper_pivot_quantile", "interval_lower", "interval_upper", "usable_replicates", "unavailable_reason",
  ];
  const refitColumns = [
    "replicate_index", "status", "information_method", "standard_errors_json", "unavailable_reason",
  ];
  document.tables.push({
    id: "exact_case_bootstrap_studentized_summary",
    title: "Studentized summary",
    columns: summaryColumns.map((id, index) => (
      index >= 7 && index <= 10 ? numberColumn(id) : textColumn(id)
    )),
    rows: [{
      id: "bootstrap_studentized",
      cells: [
        { kind: "text", value: "cbsem_exact_case_bootstrap_analytic_studentized_interval_v1" },
        { kind: "text", value: "cbsem_exact_case_bootstrap_refit_standard_errors_v1" },
        { kind: "text", value: "cbsem_ml_expected_information_delta_method_v1" },
        { kind: "text", value: "outer_estimate_minus_point_estimate_over_outer_analytic_standard_error_v1" },
        { kind: "text", value: "percentile_type7_v1" },
        { kind: "text", value: "reversed_type7_studentized_pivot_v1" },
        { kind: "text", value: "ledger_and_arithmetic_only_no_raw_refit_or_expected_information_replay_v1" },
        { kind: "number", value: 0.95 }, { kind: "number", value: 0.9 },
        { kind: "number", value: 1000 }, { kind: "number", value: 1000 },
        { kind: "text", value: "[\"parameter:loading:f1:x1\"]" },
        { kind: "text", value: "available" }, missing(), missing(),
      ],
    }],
    footnote_ids: [],
  }, {
    id: "exact_case_bootstrap_studentized_point_standard_errors",
    title: "Point standard errors",
    columns: pointColumns.map((id, index) => index === 4 ? numberColumn(id) : textColumn(id)),
    rows: [{
      id: "bootstrap_studentized_point_standard_error_0000",
      cells: [
        { kind: "text", value: "cbsem_exact_case_bootstrap_refit_standard_errors_v1" },
        { kind: "text", value: "parameter:loading:f1:x1" }, { kind: "text", value: "available" },
        { kind: "text", value: "cbsem_ml_expected_information_delta_method_v1" },
        { kind: "number", value: 0.5 }, missing(),
      ],
    }],
    footnote_ids: [],
  }, {
    id: "exact_case_bootstrap_studentized_parameter_intervals",
    title: "Studentized intervals",
    columns: intervalColumns.map((id, index) => (
      index === 0 || index === 1 || index === 9 ? textColumn(id) : numberColumn(id)
    )),
    rows: [{
      id: "bootstrap_studentized_interval_0000",
      cells: [
        { kind: "text", value: "parameter:loading:f1:x1" }, { kind: "text", value: "available" },
        { kind: "number", value: 1 }, { kind: "number", value: 0.5 },
        { kind: "number", value: 1 }, { kind: "number", value: 1 },
        { kind: "number", value: 0.5 }, { kind: "number", value: 0.5 },
        { kind: "number", value: 1000 }, missing(),
      ],
    }],
    footnote_ids: [],
  }, {
    id: "exact_case_bootstrap_studentized_refit_standard_errors",
    title: "Refit standard errors",
    columns: refitColumns.map((id, index) => index === 0 ? numberColumn(id) : textColumn(id)),
    rows: Array.from({ length: 1000 }, (_, replicate) => ({
      id: `bootstrap_studentized_refit_standard_error_${String(replicate).padStart(5, "0")}`,
      cells: [
        { kind: "number" as const, value: replicate }, { kind: "text" as const, value: "available" },
        { kind: "text" as const, value: "cbsem_ml_expected_information_delta_method_v1" },
        { kind: "text" as const, value: "[1]" }, missing(),
      ],
    })),
    footnote_ids: [],
  });
  document.sections.push({
    id: "bootstrap_studentized_inference",
    title: "Analytically studentized bootstrap inference",
    table_ids: [
      "exact_case_bootstrap_studentized_summary",
      "exact_case_bootstrap_studentized_point_standard_errors",
      "exact_case_bootstrap_studentized_parameter_intervals",
      "exact_case_bootstrap_studentized_refit_standard_errors",
    ],
    chart_ids: [],
  });
  return document;
}

function exactCaseBootstrapBcaDocumentFixture(): CanonicalResultDocumentV2 {
  const document = exactCaseBootstrapHypothesisDocumentFixture();
  document.provenance.engine_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v12";
  const textColumn = (id: string) => ({
    id, label: id, data_type: "text" as const, description: id,
  });
  const numberColumn = (id: string) => ({
    id, label: id, data_type: "number" as const, description: id,
  });
  const missing = () => ({ kind: "missing" as const, reason: "not_applicable" as const });
  const summaryColumns = [
    "method_version", "base_bootstrap_method_version", "outer_recipe_analytical_identity_sha256",
    "base_point_result_sha256", "compiler_analytical_identity_sha256", "plan_sha256",
    "model_scientific_sha256", "delete_one_refit_method_version",
    "delete_one_sampling_positions_digest_method", "delete_one_sample_indices_digest_method",
    "bias_correction_method", "acceleration_method", "adjusted_probability_method", "quantile_method",
    "retry_policy", "archive_validation_scope", "confidence_level", "bootstrap_usable_replicates",
    "minimum_bootstrap_usable_replicates", "delete_one_case_count", "successful_delete_one_refits",
    "failed_delete_one_refits", "parameter_ids_json", "inference_status", "unavailable_reason_code",
    "unavailable_message",
  ];
  const summaryText = new Set([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 22, 23, 24, 25]);
  document.tables.push({
    id: "exact_case_bootstrap_bca_summary",
    title: "BCa summary",
    columns: summaryColumns.map((id, index) => summaryText.has(index) ? textColumn(id) : numberColumn(id)),
    rows: [{
      id: "bootstrap_bca",
      cells: [
        { kind: "text", value: "cbsem_exact_case_bootstrap_bca_interval_v1" },
        { kind: "text", value: "cbsem_exact_case_bootstrap_v1" },
        { kind: "text", value: document.provenance.recipe_digest },
        { kind: "text", value: "1".repeat(64) }, { kind: "text", value: "2".repeat(64) },
        { kind: "text", value: "3".repeat(64) }, { kind: "text", value: document.provenance.model_digest },
        { kind: "text", value: "cbsem_exact_case_bootstrap_delete_one_refit_v1" },
        { kind: "text", value: "sha256_complete_case_n_and_ordered_sampling_positions_v1" },
        { kind: "text", value: "sha256_source_fingerprint_and_ordered_u64_indices_v1" },
        { kind: "text", value: "midrank_less_plus_half_ties_no_clamp_v1" },
        { kind: "text", value: "complete_delete_one_jackknife_neumaier_mean_squares_cubes_acceleration_v2" },
        { kind: "text", value: "efron_bca_statrs_inverse_normal_libm_erfc_cdf_adjustment_v2" },
        { kind: "text", value: "percentile_type7_v1" },
        { kind: "text", value: "no_retry_exactly_one_fit_per_omitted_case_v1" },
        { kind: "text", value: "ledger_identity_digest_and_arithmetic_replay_only_no_raw_base_or_delete_one_ml_replay_v1" },
        { kind: "number", value: 0.95 }, { kind: "number", value: 1000 },
        { kind: "number", value: 1000 }, { kind: "number", value: 10 },
        { kind: "number", value: 10 }, { kind: "number", value: 0 },
        { kind: "text", value: "[\"parameter:loading:f1:x1\"]" },
        { kind: "text", value: "available" }, missing(), missing(),
      ],
    }],
    footnote_ids: [],
  }, {
    id: "exact_case_bootstrap_bca_parameter_intervals",
    title: "BCa parameter intervals",
    columns: [
      "parameter_id", "status", "point_estimate", "bias_correction", "acceleration",
      "adjusted_lower_probability", "adjusted_upper_probability", "interval_lower", "interval_upper",
      "usable_replicates", "unavailable_reason",
    ].map((id, index) => index <= 1 || index === 10 ? textColumn(id) : numberColumn(id)),
    rows: [{
      id: "bootstrap_bca_interval_0000",
      cells: [
        { kind: "text", value: "parameter:loading:f1:x1" }, { kind: "text", value: "available" },
        { kind: "number", value: 1 }, { kind: "number", value: 0 }, { kind: "number", value: 0 },
        { kind: "number", value: 0.025000000000000022 }, { kind: "number", value: 0.975 },
        { kind: "number", value: 2 }, { kind: "number", value: 2 }, { kind: "number", value: 1000 },
        missing(),
      ],
    }],
    footnote_ids: [],
  }, {
    id: "exact_case_bootstrap_bca_successful_delete_one_refits",
    title: "BCa successful delete-one refits",
    columns: [
      "omitted_complete_case_position", "omitted_source_row_index", "retained_sampling_positions_sha256",
      "retained_sample_indices_sha256", "parameter_estimates_json", "iterations", "objective", "gradient_norm",
    ].map((id, index) => [2, 3, 4].includes(index) ? textColumn(id) : numberColumn(id)),
    rows: Array.from({ length: 10 }, (_, position) => ({
      id: `bootstrap_bca_delete_one_refit_${String(position).padStart(5, "0")}`,
      cells: [
        { kind: "number" as const, value: position }, { kind: "number" as const, value: position + 1 },
        { kind: "text" as const, value: "7".repeat(64) }, { kind: "text" as const, value: "8".repeat(64) },
        { kind: "text" as const, value: "[1]" }, { kind: "number" as const, value: 2 },
        { kind: "number" as const, value: 0.1 }, { kind: "number" as const, value: 0.01 },
      ],
    })),
    footnote_ids: [],
  }, {
    id: "exact_case_bootstrap_bca_failures",
    title: "BCa delete-one failures",
    columns: [
      "omitted_complete_case_position", "omitted_source_row_index", "retained_sampling_positions_sha256",
      "retained_sample_indices_sha256", "kind", "message",
    ].map((id, index) => index >= 2 ? textColumn(id) : numberColumn(id)),
    rows: [],
    footnote_ids: [],
  });
  document.sections.push({
    id: "bootstrap_bca_inference",
    title: "BCa inference",
    table_ids: [
      "exact_case_bootstrap_bca_summary",
      "exact_case_bootstrap_bca_parameter_intervals",
      "exact_case_bootstrap_bca_successful_delete_one_refits",
      "exact_case_bootstrap_bca_failures",
    ],
    chart_ids: [],
  });
  return document;
}

describe("internal schema-6 canonical-result read parser", () => {
  it("requires exact attributed RMSEA intervals in adapters v5-v7 while retaining v2-v4", () => {
    for (const adapter of [
      "compiled_recipe_v4_cbsem_plan_v2_execution_v5",
      "compiled_recipe_v4_cbsem_plan_v2_execution_v6",
      "compiled_recipe_v4_cbsem_plan_v2_execution_v7",
    ]) expect(() => validateArchivedCbsemRmseaIntervalV1(rmseaIntervalDocumentFixture(adapter))).not.toThrow();
    const version8 = rmseaIntervalDocumentFixture("compiled_recipe_v4_cbsem_plan_v2_execution_v8");
    appendScoreLmTable(version8);
    expect(() => validateArchivedCbsemRmseaIntervalV1(version8)).not.toThrow();

    for (const adapter of [
      "compiled_recipe_v4_cbsem_plan_v2_execution_v2",
      "compiled_recipe_v4_cbsem_plan_v2_execution_v3",
      "compiled_recipe_v4_cbsem_plan_v2_execution_v4",
    ]) {
      const legacy = rmseaIntervalDocumentFixture(adapter);
      const table = legacy.tables[0];
      const retained = [1, 2, 3, 4, 5, 6, 11, 12, 13];
      table.columns = retained.map((index) => table.columns[index]);
      table.rows[0].cells = retained.map((index) => table.rows[0].cells[index]);
      expect(() => validateArchivedCbsemRmseaIntervalV1(legacy)).not.toThrow();
    }

    const signedZero = rmseaIntervalDocumentFixture();
    signedZero.tables[0].rows[0].cells[6] = { kind: "number", value: -0 };
    signedZero.tables[0].rows[0].cells[9] = { kind: "number", value: -0 };
    signedZero.tables[0].rows[0].cells[10] = { kind: "number", value: 0 };
    expect(() => validateArchivedCbsemRmseaIntervalV1(signedZero)).not.toThrow();

    const missing = rmseaIntervalDocumentFixture();
    missing.tables = [];
    expect(() => validateArchivedCbsemRmseaIntervalV1(missing)).toThrow(/exactly one fit_indices/);

    const wrongEstimator = rmseaIntervalDocumentFixture();
    wrongEstimator.provenance.method_version = "cbsem_ml_exact_parameter_table_v4";
    expect(() => validateArchivedCbsemRmseaIntervalV1(wrongEstimator))
      .toThrow(/unsupported exact CB-SEM estimator\/adapter identity/);

    const unknownAdapter = rmseaIntervalDocumentFixture();
    unknownAdapter.provenance.engine_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v11";
    expect(() => validateArchivedCbsemRmseaIntervalV1(unknownAdapter))
      .not.toThrow();

    const legacyShapeTamper = rmseaIntervalDocumentFixture(
      "compiled_recipe_v4_cbsem_plan_v2_execution_v2",
    );
    expect(() => validateArchivedCbsemRmseaIntervalV1(legacyShapeTamper))
      .toThrow(/drifted column contract/);

    for (const [index, cell] of [
      [7, { kind: "text", value: "wrong" }],
      [8, { kind: "number", value: 0.95 }],
      [9, { kind: "missing", reason: "not_estimated" }],
      [10, { kind: "number", value: 0.01 }],
    ] as const) {
      const tampered = rmseaIntervalDocumentFixture();
      tampered.tables[0].rows[0].cells[index] = cell;
      expect(() => validateArchivedCbsemRmseaIntervalV1(tampered)).toThrow();
    }
  });

  it("validates exact adapter-v8 score\/LM rows and rejects masquerade or arithmetic tampering", () => {
    const document = rmseaIntervalDocumentFixture("compiled_recipe_v4_cbsem_plan_v2_execution_v8");
    appendScoreLmTable(document);
    expect(() => validateArchivedCbsemCfaScoreLmV1(document)).not.toThrow();

    const arithmetic = clone(document);
    arithmetic.tables[1].rows[0].cells[11] = { kind: "number", value: 5 };
    expect(() => validateArchivedCbsemCfaScoreLmV1(arithmetic)).toThrow(/arithmetic/);

    const signedZero = clone(document);
    signedZero.tables[1].rows[0].cells[7] = { kind: "number", value: -0 };
    expect(() => validateArchivedCbsemCfaScoreLmV1(signedZero)).toThrow(/positive zero/);

    const missing = clone(document);
    missing.tables.splice(1, 1);
    expect(() => validateArchivedCbsemCfaScoreLmV1(missing)).toThrow(/requires exactly one/);

    const legacyMasquerade = clone(document);
    legacyMasquerade.provenance.engine_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v5";
    expect(() => validateArchivedCbsemCfaScoreLmV1(legacyMasquerade)).toThrow(/pre-v8/);

    const unavailable = clone(document);
    unavailable.tables[1].rows[0].cells.splice(6, 10,
      { kind: "text", value: "unavailable" },
      ...Array.from({ length: 8 }, () => ({ kind: "missing" as const, reason: "not_estimated" as const })),
      { kind: "text", value: "nuisance_information_unavailable" },
    );
    expect(() => validateArchivedCbsemCfaScoreLmV1(unavailable)).not.toThrow();
    unavailable.tables[1].rows[0].cells[14] = { kind: "number", value: 0.5 };
    expect(() => validateArchivedCbsemCfaScoreLmV1(unavailable)).toThrow(/omit every numeric/);
  });

  it("binds the complete v9 exact case-bootstrap family and rejects partition, arithmetic, and historical injection tamper", () => {
    const document = exactCaseBootstrapDocumentFixture();
    expect(() => validateArchivedCbsemRmseaIntervalV1(document)).not.toThrow();
    expect(() => validateArchivedCbsemCfaScoreLmV1(document)).not.toThrow();
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(document)).not.toThrow();

    const currentCapability = clone(document);
    currentCapability.provenance.capability_cell = {
      registry_schema_version: 2,
      capability_id: "smartpls.cbsem_bootstrapping",
      cell_id: "qpls3.cbsem.bootstrap",
      capability_version: "cbsem_exact_case_bootstrap_v1",
    };
    expect(() => validateArchivedCbsemRmseaIntervalV1(currentCapability)).not.toThrow();
    expect(() => validateArchivedCbsemCfaScoreLmV1(currentCapability)).not.toThrow();
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(currentCapability)).not.toThrow();

    const missing = clone(document);
    missing.tables = missing.tables.filter((table) => table.id !== "exact_case_bootstrap_failures");
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(missing)).toThrow(/complete exact case-bootstrap/);

    const duplicate = clone(document);
    duplicate.tables.find((table) => table.id === "exact_case_bootstrap_successful_refits")!
      .rows[1].cells[0] = { kind: "number", value: 0 };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(duplicate)).toThrow(/replicate order/);

    const arithmetic = clone(document);
    arithmetic.tables.find((table) => table.id === "exact_case_bootstrap_parameter_intervals")!
      .rows[0].cells[3] = { kind: "number", value: -0 };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(arithmetic)).toThrow(/negative zero/);

    const witnessSignedZero = clone(document);
    witnessSignedZero.tables.find((table) => table.id === "exact_case_bootstrap_successful_refits")!
      .rows[0].cells[3] = { kind: "text", value: "[-0]" };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(witnessSignedZero)).toThrow(/signed-zero-safe/);

    const historicalInjection = clone(document);
    historicalInjection.provenance.engine_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v8";
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(historicalInjection)).toThrow(/pre-v9/);
  });

  it("binds the selected-tail v10 receipt to the exact refit ledger and rejects v9 injection or arithmetic tamper", () => {
    const document = exactCaseBootstrapHypothesisDocumentFixture();
    expect(() => validateArchivedCbsemRmseaIntervalV1(document)).not.toThrow();
    expect(() => validateArchivedCbsemCfaScoreLmV1(document)).not.toThrow();
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(document)).not.toThrow();

    const missing = exactCaseBootstrapDocumentFixture();
    missing.provenance.engine_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v10";
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(missing)).toThrow(/adapter v10 requires/);

    const injectedIntoV9 = clone(document);
    injectedIntoV9.provenance.engine_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v9";
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(injectedIntoV9)).toThrow(/injected v10/);

    const defaultTail = clone(document);
    const defaultTailRow = defaultTail.tables.find((table) => table.id === "exact_case_bootstrap_hypothesis_tests")!.rows[0];
    defaultTailRow.cells[6] = { kind: "text", value: "two_sided" };
    defaultTailRow.cells[22] = { kind: "number", value: 1000 };
    defaultTailRow.cells[23] = { kind: "number", value: 1 };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(defaultTail)).not.toThrow();

    const unknownTail = clone(document);
    unknownTail.tables.find((table) => table.id === "exact_case_bootstrap_hypothesis_tests")!
      .rows[0].cells[6] = { kind: "text", value: "upper" };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(unknownTail)).toThrow(/unknown selected test tail/);

    const countTamper = clone(document);
    countTamper.tables.find((table) => table.id === "exact_case_bootstrap_hypothesis_tests")!
      .rows[0].cells[17] = { kind: "number", value: 999 };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(countTamper)).toThrow(/null-centered counts/);

    const decisionTamper = clone(document);
    decisionTamper.tables.find((table) => table.id === "exact_case_bootstrap_hypothesis_tests")!
      .rows[0].cells[24] = { kind: "boolean", value: true };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(decisionTamper)).toThrow(/decision/);

    const unavailable = clone(document);
    const summary = unavailable.tables.find((table) => table.id === "exact_case_bootstrap_summary")!;
    summary.rows[0].cells[17] = { kind: "number", value: 500 };
    summary.rows[0].cells[18] = { kind: "number", value: 500 };
    summary.rows[0].cells[19] = { kind: "number", value: 0 };
    summary.rows[0].cells[20] = { kind: "number", value: 500 };
    summary.rows[0].cells[28] = { kind: "text", value: "unavailable" };
    summary.rows[0].cells[29] = { kind: "text", value: "insufficient_usable_refits" };
    summary.rows[0].cells[30] = { kind: "text", value: "The 500-draw pilot is below the required 1,000 usable refits." };
    unavailable.tables.find((table) => table.id === "exact_case_bootstrap_parameter_intervals")!.rows = [];
    unavailable.tables.find((table) => table.id === "exact_case_bootstrap_successful_refits")!.rows = [];
    unavailable.tables.find((table) => table.id === "exact_case_bootstrap_failures")!.rows = Array.from({ length: 500 }, (_, replicate) => ({
      id: `bootstrap_failure_${String(replicate).padStart(5, "0")}`,
      cells: [{ kind: "number" as const, value: replicate }, { kind: "text" as const, value: "5".repeat(64) },
        { kind: "text" as const, value: "6".repeat(64) }, { kind: "text" as const, value: "non_convergence" },
        { kind: "text" as const, value: "Did not converge." }],
    }));
    const hypothesis = unavailable.tables.find((table) => table.id === "exact_case_bootstrap_hypothesis_tests")!;
    hypothesis.rows[0].cells[9] = { kind: "number", value: 0 };
    hypothesis.rows[0].cells[10] = { kind: "text", value: "unavailable" };
    hypothesis.rows[0].cells[11] = { kind: "text", value: "insufficient_usable_refits" };
    hypothesis.rows[0].cells[12] = { kind: "text", value: "The 500-draw pilot is below the required 1,000 usable refits." };
    hypothesis.rows[0].cells[14] = { kind: "text", value: "unavailable" };
    for (let index = 15; index <= 24; index += 1) hypothesis.rows[0].cells[index] = { kind: "missing", reason: "not_applicable" };
    hypothesis.rows[0].cells[25] = { kind: "text", value: "insufficient_usable_replicates" };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(unavailable)).not.toThrow();
  });

  it("binds the atomic v11 studentized family, typed B500 unavailability, and rejects mixed or arithmetic tamper", () => {
    const document = exactCaseBootstrapStudentizedDocumentFixture();
    expect(() => validateArchivedCbsemRmseaIntervalV1(document)).not.toThrow();
    expect(() => validateArchivedCbsemCfaScoreLmV1(document)).not.toThrow();
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(document)).not.toThrow();

    const injectedIntoV10 = clone(document);
    injectedIntoV10.provenance.engine_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v10";
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(injectedIntoV10)).toThrow(/injected v11/);

    const missingAtomicTable = clone(document);
    missingAtomicTable.tables = missingAtomicTable.tables.filter((table) => (
      table.id !== "exact_case_bootstrap_studentized_refit_standard_errors"
    ));
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(missingAtomicTable)).toThrow(/complete studentized/);

    const archiveScopeTamper = clone(document);
    archiveScopeTamper.tables.find((table) => table.id === "exact_case_bootstrap_studentized_summary")!
      .rows[0].cells[6] = { kind: "text", value: "raw_refits_replayed" };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(archiveScopeTamper)).toThrow(/archive scope/);

    const pointSeTamper = clone(document);
    pointSeTamper.tables.find((table) => table.id === "exact_case_bootstrap_studentized_point_standard_errors")!
      .rows[0].cells[4] = { kind: "number", value: 0 };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(pointSeTamper)).toThrow(/must be positive/);

    const refitOrderTamper = clone(document);
    refitOrderTamper.tables.find((table) => table.id === "exact_case_bootstrap_studentized_refit_standard_errors")!
      .rows[1].cells[0] = { kind: "number", value: 0 };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(refitOrderTamper)).toThrow(/ledger order/);

    const usableCountTamper = clone(document);
    usableCountTamper.tables.find((table) => table.id === "exact_case_bootstrap_studentized_summary")!
      .rows[0].cells[10] = { kind: "number", value: 999 };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(usableCountTamper)).toThrow(/refit standard-error partition/);

    const pivotTamper = clone(document);
    pivotTamper.tables.find((table) => table.id === "exact_case_bootstrap_studentized_parameter_intervals")!
      .rows[0].cells[6] = { kind: "number", value: 0.4 };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(pivotTamper)).toThrow(/Type-7 interval arithmetic/);

    const workloadTamper = clone(document);
    workloadTamper.provenance.workers = 13;
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(workloadTamper)).toThrow(/workload envelope/);

    const pilot = clone(document);
    const baseSummary = pilot.tables.find((table) => table.id === "exact_case_bootstrap_summary")!;
    baseSummary.rows[0].cells[17] = { kind: "number", value: 500 };
    baseSummary.rows[0].cells[18] = { kind: "number", value: 500 };
    baseSummary.rows[0].cells[19] = { kind: "number", value: 500 };
    baseSummary.rows[0].cells[20] = { kind: "number", value: 0 };
    baseSummary.rows[0].cells[28] = { kind: "text", value: "unavailable" };
    baseSummary.rows[0].cells[29] = { kind: "text", value: "insufficient_usable_refits" };
    baseSummary.rows[0].cells[30] = { kind: "text", value: "The 500-draw pilot is below the required 1,000 usable refits." };
    pilot.tables.find((table) => table.id === "exact_case_bootstrap_parameter_intervals")!.rows = [];
    pilot.tables.find((table) => table.id === "exact_case_bootstrap_successful_refits")!.rows.splice(500);
    const hypothesis = pilot.tables.find((table) => table.id === "exact_case_bootstrap_hypothesis_tests")!;
    hypothesis.rows[0].cells[9] = { kind: "number", value: 500 };
    hypothesis.rows[0].cells[10] = { kind: "text", value: "unavailable" };
    hypothesis.rows[0].cells[11] = { kind: "text", value: "insufficient_usable_refits" };
    hypothesis.rows[0].cells[12] = { kind: "text", value: "The 500-draw pilot is below the required 1,000 usable refits." };
    hypothesis.rows[0].cells[14] = { kind: "text", value: "unavailable" };
    for (let index = 15; index <= 24; index += 1) {
      hypothesis.rows[0].cells[index] = { kind: "missing", reason: "not_applicable" };
    }
    hypothesis.rows[0].cells[25] = { kind: "text", value: "insufficient_usable_replicates" };
    const studentizedSummary = pilot.tables.find((table) => (
      table.id === "exact_case_bootstrap_studentized_summary"
    ))!;
    studentizedSummary.rows[0].cells[10] = { kind: "number", value: 500 };
    studentizedSummary.rows[0].cells[12] = { kind: "text", value: "unavailable" };
    studentizedSummary.rows[0].cells[13] = {
      kind: "text", value: "insufficient_studentized_usable_replicates",
    };
    studentizedSummary.rows[0].cells[14] = {
      kind: "text",
      value: "Analytically studentized inference is unavailable because 500 whole-vector usable refits are below the required 1000.",
    };
    pilot.tables.find((table) => table.id === "exact_case_bootstrap_studentized_refit_standard_errors")!
      .rows.splice(500);
    const studentizedInterval = pilot.tables.find((table) => (
      table.id === "exact_case_bootstrap_studentized_parameter_intervals"
    ))!.rows[0];
    studentizedInterval.cells[1] = { kind: "text", value: "unavailable" };
    for (let index = 2; index <= 8; index += 1) {
      studentizedInterval.cells[index] = { kind: "missing", reason: "not_applicable" };
    }
    studentizedInterval.cells[9] = {
      kind: "text", value: "insufficient_studentized_usable_replicates",
    };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(pilot)).not.toThrow();
  });

  it("binds the v12 BCa family, validates exposed Type-7 arithmetic, and preserves typed global failure states", () => {
    const document = exactCaseBootstrapBcaDocumentFixture();
    expect(() => validateArchivedCbsemRmseaIntervalV1(document)).not.toThrow();
    expect(() => validateArchivedCbsemCfaScoreLmV1(document)).not.toThrow();
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(document)).not.toThrow();

    const injectedIntoV11 = clone(document);
    injectedIntoV11.provenance.engine_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v11";
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(injectedIntoV11)).toThrow(/injected v12 BCa artifacts/);

    const missingTable = clone(document);
    missingTable.tables = missingTable.tables.filter((table) => table.id !== "exact_case_bootstrap_bca_failures");
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(missingTable)).toThrow(/complete BCa bootstrap table family/);

    const orderTamper = clone(document);
    orderTamper.sections.find((section) => section.id === "bootstrap_bca_inference")!.table_ids.reverse();
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(orderTamper)).toThrow(/table ownership or order/);

    const archiveTamper = clone(document);
    archiveTamper.tables.find((table) => table.id === "exact_case_bootstrap_bca_summary")!
      .rows[0].cells[15] = { kind: "text", value: "raw_delete_one_ml_replay" };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(archiveTamper)).toThrow(/archive scope/);

    const ledgerTamper = clone(document);
    ledgerTamper.tables.find((table) => table.id === "exact_case_bootstrap_bca_successful_delete_one_refits")!
      .rows[1].cells[0] = { kind: "number", value: 0 };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(ledgerTamper)).toThrow(/delete-one identity or order/);

    const arithmeticTamper = clone(document);
    arithmeticTamper.tables.find((table) => table.id === "exact_case_bootstrap_bca_parameter_intervals")!
      .rows[0].cells[7] = { kind: "number", value: 1.9 };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(arithmeticTamper)).toThrow(/exposed Type-7 arithmetic/);

    const deleteOneFailure = clone(document);
    const summary = deleteOneFailure.tables.find((table) => table.id === "exact_case_bootstrap_bca_summary")!.rows[0];
    summary.cells[20] = { kind: "number", value: 9 };
    summary.cells[21] = { kind: "number", value: 1 };
    summary.cells[23] = { kind: "text", value: "unavailable" };
    summary.cells[24] = { kind: "text", value: "incomplete_delete_one_ledger" };
    summary.cells[25] = {
      kind: "text",
      value: "BCa inference is unavailable because 1 of 10 mandatory delete-one fits failed.",
    };
    const successes = deleteOneFailure.tables.find((table) => (
      table.id === "exact_case_bootstrap_bca_successful_delete_one_refits"
    ))!;
    const omitted = successes.rows.pop()!;
    deleteOneFailure.tables.find((table) => table.id === "exact_case_bootstrap_bca_failures")!.rows.push({
      id: "bootstrap_bca_delete_one_failure_00009",
      cells: [omitted.cells[0], omitted.cells[1], omitted.cells[2], omitted.cells[3],
        { kind: "text", value: "non_convergence" }, { kind: "text", value: "Did not converge." }],
    });
    const interval = deleteOneFailure.tables.find((table) => (
      table.id === "exact_case_bootstrap_bca_parameter_intervals"
    ))!.rows[0];
    interval.cells[1] = { kind: "text", value: "unavailable" };
    for (let index = 2; index <= 9; index += 1) interval.cells[index] = { kind: "missing", reason: "not_applicable" };
    interval.cells[10] = { kind: "text", value: "incomplete_delete_one_ledger" };
    expect(() => validateArchivedCbsemExactCaseBootstrapV1(deleteOneFailure)).not.toThrow();
  });

  it("validates exact CB-SEM mean-replacement tables and rejects table-level tampering", () => {
    const document = meanReplacementDocumentFixture();
    expect(() => validateArchivedCbsemMissingDataExecutionV1(document)).not.toThrow();

    const version7 = clone(document);
    version7.provenance.engine_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v7";
    version7.tables.find((table) => table.id === "estimation_summary")!
      .rows[0].cells[0] = { kind: "text", value: version7.provenance.engine_version };
    expect(() => validateArchivedCbsemMissingDataExecutionV1(version7)).not.toThrow();

    const version2Fingerprint = clone(document);
    version2Fingerprint.tables.find((table) => table.id === "missing_data_execution")!
      .rows[0].cells[5] = {
        kind: "text",
        value: `v2:${document.provenance.dataset_fingerprint}`,
      };
    expect(() => validateArchivedCbsemMissingDataExecutionV1(version2Fingerprint)).not.toThrow();

    const unknownPrefix = clone(document);
    unknownPrefix.tables.find((table) => table.id === "missing_data_execution")!
      .rows[0].cells[5] = {
        kind: "text",
        value: `v3:${document.provenance.dataset_fingerprint}`,
      };
    expect(() => validateArchivedCbsemMissingDataExecutionV1(unknownPrefix))
      .toThrow(/bare lowercase SHA-256 or v2:<lowercase SHA-256>/);

    const uppercaseVersion2 = clone(document);
    uppercaseVersion2.tables.find((table) => table.id === "missing_data_execution")!
      .rows[0].cells[5] = { kind: "text", value: `v2:${"A".repeat(64)}` };
    expect(() => validateArchivedCbsemMissingDataExecutionV1(uppercaseVersion2))
      .toThrow(/bare lowercase SHA-256 or v2:<lowercase SHA-256>/);

    for (const malformedFingerprint of [
      `v2:${"a".repeat(63)}`,
      `v2:${"a".repeat(65)}`,
      "a".repeat(63),
      "a".repeat(65),
    ]) {
      const malformed = clone(document);
      malformed.tables.find((table) => table.id === "missing_data_execution")!
        .rows[0].cells[5] = { kind: "text", value: malformedFingerprint };
      expect(() => validateArchivedCbsemMissingDataExecutionV1(malformed))
        .toThrow(/bare lowercase SHA-256 or v2:<lowercase SHA-256>/);
    }

    const coordinatedUnknownPrefix = clone(document);
    coordinatedUnknownPrefix.provenance.dataset_fingerprint = "c".repeat(64);
    coordinatedUnknownPrefix.tables.find((table) => table.id === "missing_data_execution")!
      .rows[0].cells[5] = {
        kind: "text",
        value: `v3:${coordinatedUnknownPrefix.provenance.dataset_fingerprint}`,
      };
    expect(() => validateArchivedCbsemMissingDataExecutionV1(coordinatedUnknownPrefix))
      .toThrow(/bare lowercase SHA-256 or v2:<lowercase SHA-256>/);

    const coordinatedSuffixMismatch = clone(document);
    coordinatedSuffixMismatch.tables.find((table) => table.id === "missing_data_execution")!
      .rows[0].cells[5] = { kind: "text", value: `v2:${"c".repeat(64)}` };
    expect(() => validateArchivedCbsemMissingDataExecutionV1(coordinatedSuffixMismatch))
      .toThrow(/provenance.*differs from the missing-data execution source identity/);

    const omitted = clone(document);
    omitted.tables = omitted.tables.filter((table) => table.id !== "mean_replacement_cells");
    expect(() => validateArchivedCbsemMissingDataExecutionV1(omitted))
      .toThrow(/drifted mean-replacement method, adapter, or table identity/);

    const reordered = clone(document);
    const reorderedRows = reordered.tables.find(
      (table) => table.id === "mean_replacement_cells",
    )!.rows;
    [reorderedRows[0], reorderedRows[1]] = [reorderedRows[1], reorderedRows[0]];
    expect(() => validateArchivedCbsemMissingDataExecutionV1(reordered))
      .toThrow(/non-canonical|reordered/);

    const meanTamper = clone(document);
    meanTamper.tables.find((table) => table.id === "mean_replacement_cells")!
      .rows[0].cells[4] = { kind: "number", value: 10.25 };
    expect(() => validateArchivedCbsemMissingDataExecutionV1(meanTamper))
      .toThrow(/drifted cell identity/);

    const replayClaim = clone(document);
    replayClaim.tables.find((table) => table.id === "missing_data_execution")!
      .rows[0].cells[3] = { kind: "boolean", value: true };
    expect(() => validateArchivedCbsemMissingDataExecutionV1(replayClaim))
      .toThrow(/descriptor-only validation without raw replay/);

    const countTamper = clone(document);
    countTamper.tables.find((table) => table.id === "missing_data_execution")!
      .rows[0].cells[10] = { kind: "number", value: 4 };
    expect(() => validateArchivedCbsemMissingDataExecutionV1(countTamper))
      .toThrow(/incoherent counts/);
  });

  it("validates exact score-execution tables without constraining final estimated weights", () => {
    const document = scoreExecutionDocumentFixture();
    expect(() => validateArchivedPlsScoreExecutionV2(document)).not.toThrow();

    const changedEstimatedFinal = clone(document);
    changedEstimatedFinal.tables[2].rows[0].cells[8] = { kind: "number", value: 0.91 };
    expect(() => validateArchivedPlsScoreExecutionV2(changedEstimatedFinal)).not.toThrow();

    const changedFixedFinal = clone(document);
    changedFixedFinal.tables[2].rows[1].cells[8] = { kind: "number", value: 0.74 };
    expect(() => validateArchivedPlsScoreExecutionV2(changedFixedFinal))
      .toThrow(/fixed scoring changed after resolution/);

    const partialTables = clone(document);
    partialTables.tables = partialTables.tables.filter(
      (table) => table.id !== "score_execution_weights",
    );
    expect(() => validateArchivedPlsScoreExecutionV2(partialTables))
      .toThrow(/non-allowlisted PLS score adapter generation/);

    const downgraded = clone(document);
    downgraded.provenance.method_version = "pls_pm_v1";
    downgraded.tables = [];
    expect(() => validateArchivedPlsScoreExecutionV2(downgraded))
      .toThrow(/non-allowlisted adapter generation/);
  });

  it("mirrors exact fixed-score normalization receipts and rejects tampering", () => {
    const none = fixedCustomNormalizationDocument("none", [-0, 0.75], [-0, 0.75]);
    const sumToOne = fixedCustomNormalizationDocument(
      "sum_to_one",
      [-0.25, 0.75],
      [-0.5, 1.5],
    );
    const unitVariance = fixedCustomNormalizationDocument(
      "unit_variance",
      [-0.25, 0.75],
      [-0.5, 1.5],
    );
    for (const document of [none, sumToOne, unitVariance]) {
      expect(() => validateArchivedPlsScoreExecutionV2(document)).not.toThrow();
    }

    const signedZeroTamper = fixedCustomNormalizationDocument("none", [-0, 0.75], [0, 0.75]);
    expect(() => validateArchivedPlsScoreExecutionV2(signedZeroTamper))
      .toThrow(/normalization contract/);

    const sumTamper = fixedCustomNormalizationDocument(
      "sum_to_one",
      [-0.25, 0.75],
      [-0.5, 1.500_000_000_000_000_2],
    );
    expect(() => validateArchivedPlsScoreExecutionV2(sumTamper))
      .toThrow(/normalization contract/);

    const zeroSum = fixedCustomNormalizationDocument(
      "sum_to_one",
      [-1, 1],
      [-1, 1],
    );
    expect(() => validateArchivedPlsScoreExecutionV2(zeroSum))
      .toThrow(/normalization contract/);
  });

  it("strictly binds archived fixed-score scale receipts", () => {
    const exact = appendFixedScaleReceipt(fixedCustomNormalizationDocument(
      "unit_variance",
      [-0.25, 0.75],
      [-0.5, 1.5],
    ));
    expect(() => validateArchivedPlsScoreExecutionV2(exact)).not.toThrow();

    const valueTamper = clone(exact);
    const valueCell = valueTamper.tables.find((table) => table.id === "fixed_score_scale_receipt")!
      .rows[0].cells[6];
    if (valueCell.kind !== "number") throw new Error("fixture contract");
    valueCell.value = 1.000_000_000_000_000_2;
    expect(() => validateArchivedPlsScoreExecutionV2(valueTamper))
      .toThrow(/tampered center, scale, coefficient, or effective weight/);

    const orderTamper = clone(exact);
    orderTamper.tables.find((table) => table.id === "fixed_score_scale_receipt")!
      .rows.reverse();
    expect(() => validateArchivedPlsScoreExecutionV2(orderTamper))
      .toThrow(/non-canonical|order or identity/);

    const duplicate = clone(exact);
    const duplicateRows = duplicate.tables.find((table) => table.id === "fixed_score_scale_receipt")!
      .rows;
    duplicateRows[1].cells[2] = clone(duplicateRows[0].cells[2]);
    expect(() => validateArchivedPlsScoreExecutionV2(duplicate))
      .toThrow(/order or identity/);

    const signedZero = appendFixedScaleReceipt(
      fixedCustomNormalizationDocument("none", [-0, 0.75], [-0, 0.75]),
      1,
    );
    signedZero.tables.find((table) => table.id === "fixed_score_scale_receipt")!
      .rows[0].cells[6] = { kind: "number", value: 0 };
    expect(() => validateArchivedPlsScoreExecutionV2(signedZero))
      .toThrow(/tampered center, scale, coefficient, or effective weight/);

    const missingCurrent = scoreExecutionDocumentFixture();
    missingCurrent.provenance.engine_version = "compiled_recipe_v4_pls_plan_v2_execution_v6";
    appendCurrentPlsTypedFamilies(missingCurrent);
    expect(() => validateArchivedPlsScoreExecutionV2(missingCurrent))
      .toThrow(/omitted fixed_score_scale_receipt/);
  });

  it("allowlists legacy and current PLS adapters and requires current typed families", () => {
    const legacy = scoreExecutionDocumentFixture();
    expect(() => validateArchivedPlsScoreExecutionV2(legacy)).not.toThrow();

    const current = clone(legacy);
    current.provenance.engine_version = "compiled_recipe_v4_pls_plan_v2_execution_v6";
    appendCurrentPlsTypedFamilies(current);
    appendFixedScaleReceipt(current);
    expect(() => validateArchivedPlsScoreExecutionV2(current)).not.toThrow();

    const missing = clone(current);
    missing.tables = missing.tables.filter((table) => table.id !== "algorithm_block_order");
    expect(() => validateArchivedPlsScoreExecutionV2(missing))
      .toThrow(/algorithm convergence tables must occur as one exact family/);

    const unknown = clone(current);
    unknown.provenance.engine_version = "compiled_recipe_v4_pls_plan_v2_execution_custom";
    expect(() => validateArchivedPlsScoreExecutionV2(unknown))
      .toThrow(/non-allowlisted PLS score adapter generation/);

    const attributionTamper = clone(current);
    attributionTamper.tables.find((table) => table.id === "point_estimate_attribution")!
      .rows[0].cells[3] = { kind: "text", value: "unit_scale" };
    expect(() => validateArchivedPlsScoreExecutionV2(attributionTamper))
      .toThrow(/drifted preprocessing or scale attribution/);

    const convergenceTamper = clone(current);
    convergenceTamper.tables.find((table) => table.id === "algorithm_block_order")!
      .rows[1].cells[5] = { kind: "text", value: "fixed_custom_weights" };
    expect(() => validateArchivedPlsScoreExecutionV2(convergenceTamper))
      .toThrow(/score-execution block order or semantics/);

    const wrongSection = clone(current);
    wrongSection.sections.find((section) => section.id === "run_details")!
      .table_ids = ["algorithm_convergence_receipt", "algorithm_block_order", "fixed_score_scale_receipt"];
    expect(() => validateArchivedPlsScoreExecutionV2(wrongSection))
      .toThrow(/point_estimate_attribution must belong exactly once/);

    const controls = clone(legacy);
    controls.tables.push({ ...clone(controls.tables[0]), id: "control_estimates" });
    controls.sections.push({
      id: "structural_model",
      title: "Structural model",
      table_ids: ["control_estimates"],
      chart_ids: [],
    });
    expect(() => validateArchivedPlsScoreExecutionV2(controls)).not.toThrow();
    controls.sections[0].table_ids.push("control_estimates");
    expect(() => validateArchivedPlsScoreExecutionV2(controls))
      .toThrow(/control_estimates must belong exactly once to structural_model/);

    const legacyPlain = clone(legacy);
    legacyPlain.provenance.method_version = "pls_pm_v1";
    legacyPlain.provenance.engine_version = "compiled_recipe_v4_pls_plan_v2_execution_v3";
    legacyPlain.tables = legacyPlain.tables.filter(
      (table) => !["score_execution_summary", "score_execution_weights"].includes(table.id),
    );
    expect(() => validateArchivedPlsScoreExecutionV2(legacyPlain)).not.toThrow();
  });

  it("accepts only the owned v7 nonlinear family and rejects history injection and score mixing", () => {
    const exact = nonlinearDocumentFixture();
    expect(() => validateArchivedPlsNonlinearEffectsV1(exact)).not.toThrow();
    expect(() => validateArchivedPlsScoreExecutionV2(exact)).not.toThrow();

    const order = clone(exact);
    order.tables[order.tables.length - 3].rows[0].id = "nonlinear_quadratic_diagnostic_0001";
    expect(() => validateArchivedPlsScoreExecutionV2(order)).toThrow(/non-canonical/);

    const arithmetic = clone(exact);
    arithmetic.tables.find((table) => table.id === "nonlinear_equation_fit")!
      .rows[0].cells[3] = { kind: "number", value: 0.05 };
    expect(() => validateArchivedPlsScoreExecutionV2(arithmetic)).toThrow(/R-squared arithmetic/);

    const ownership = clone(exact);
    ownership.tables.find((table) => table.id === "structural_paths")!
      .capability_cells = [clone(ownership.provenance.capability_cell)];
    expect(() => validateArchivedPlsScoreExecutionV2(ownership)).toThrow(/capability owner/);

    const scoreMix = clone(exact);
    scoreMix.tables.splice(scoreMix.tables.length - 3, 0, {
      ...clone(scoreMix.tables[0]), id: "score_execution_summary",
    });
    expect(() => validateArchivedPlsScoreExecutionV2(scoreMix)).toThrow(/must not mix score/);

    const historical = scoreExecutionDocumentFixture();
    historical.tables.push(clone(exact.tables.find((table) => table.id === "nonlinear_method_scope")!));
    expect(() => validateArchivedPlsScoreExecutionV2(historical)).toThrow(/v3-v6 document contains injected nonlinear/);

    const unknownPrimary = clone(exact);
    unknownPrimary.provenance.capability_cell = {
      registry_schema_version: 2,
      capability_id: "unknown.method",
      cell_id: "unknown.cell",
      capability_version: "unknown_v1",
    };
    expect(() => validateArchivedPlsScoreExecutionV2(unknownPrimary))
      .toThrow(/injected nonlinear artifacts/);

    const unknownMethodIdentity = documentFixture();
    unknownMethodIdentity.provenance.capability_cell = clone(unknownPrimary.provenance.capability_cell);
    unknownMethodIdentity.provenance.method_version = "pls_quadratic_nonlinear_effects_v1";
    expect(() => validateArchivedPlsScoreExecutionV2(unknownMethodIdentity))
      .toThrow(/injected nonlinear artifacts/);

    const unknownAdapterIdentity = documentFixture();
    unknownAdapterIdentity.provenance.capability_cell = clone(unknownPrimary.provenance.capability_cell);
    unknownAdapterIdentity.provenance.engine_version = "compiled_recipe_v4_pls_plan_v2_execution_v7";
    expect(() => validateArchivedPlsScoreExecutionV2(unknownAdapterIdentity))
      .toThrow(/injected nonlinear artifacts/);

    const chart = {
      id: "injected_nonlinear_chart",
      title: "Injected nonlinear chart",
      description: "Not part of the point-only v7 contract.",
      kind: "scatter" as const,
      series: [{ id: "series", label: "Series", points: [{ x: 0, y: 1 }] }],
      source_table_id: "nonlinear_quadratic_diagnostics",
      display: {},
    };
    const unreferencedChart = clone(exact);
    unreferencedChart.charts.push(chart);
    expect(() => validateArchivedPlsScoreExecutionV2(unreferencedChart))
      .toThrow(/must not contain or reference charts/);

    const referencedChart = clone(exact);
    referencedChart.charts.push(chart);
    referencedChart.sections[3].chart_ids.push(chart.id);
    expect(() => validateArchivedPlsScoreExecutionV2(referencedChart))
      .toThrow(/must not contain or reference charts/);
  });

  it("binds a versioned compiler fingerprint to the bare canonical nonlinear digest", () => {
    const canonicalDocument = nonlinearDocumentFixture();
    const attribution = canonicalDocument.tables
      .find((table) => table.id === "point_estimate_attribution")!.rows[0].cells
      .map((cell) => cell.kind === "text" ? cell.value : "");
    const analyticalResult = {
      schema_version: 1,
      provenance: {
        adapter_version: "compiled_recipe_v4_pls_plan_v2_execution_v7",
        compilation_receipt: {
          schema_version: 1,
          recipe_id: canonicalDocument.provenance.recipe_id,
          recipe_document_sha256: "1".repeat(64),
          recipe_analytical_sha256: canonicalDocument.provenance.recipe_digest,
          model_id: canonicalDocument.provenance.model_id,
          model_document_sha256: "2".repeat(64),
          model_scientific_sha256: canonicalDocument.provenance.model_digest,
          dataset_fingerprint: `v2:${canonicalDocument.provenance.dataset_fingerprint}`,
          compiler_target: "pls_plan_v2",
          compiler_version: "recipe_v4_compiler_v1",
          capability_cell: clone(canonicalDocument.provenance.capability_cell),
          plan_sha256: "3".repeat(64),
          analytical_identity_sha256: "4".repeat(64),
        },
        projected_recipe_schema_version: 3,
        projected_recipe_sha256: "5".repeat(64),
        dataset_id: canonicalDocument.provenance.dataset_id,
        estimator_method_version: "pls_quadratic_nonlinear_effects_v1",
      },
      estimation: {
        method_version: "pls_quadratic_nonlinear_effects_v1",
        iterations: 4,
        paths: [{ source: "x", target: "y", coefficient: 0.25 }],
        point_estimate_attribution: {
          contract_version: attribution[0],
          preprocessing: attribution[1],
          indicator_centering: attribution[2],
          indicator_scaling: attribution[3],
          outer_weights: attribution[4],
          outer_loadings: attribution[5],
          construct_scores: attribution[6],
          structural_paths: attribution[7],
          effects: attribution[8],
        },
        nonlinear_effects: {
          method_version: "pls_quadratic_nonlinear_effects_v1",
          term: "centered_squared_construct_score_v1",
          estimates: [{
            source: "x", target: "y", linear_coefficient: 0.25,
            quadratic_coefficient: 0.1, standard_error: 0.05, t_statistic: 2,
            p_value_two_sided: 0.0455, linear_r_squared: 0.4,
            augmented_r_squared: 0.45, delta_r_squared: 0.04999999999999999,
            warning: null,
          }],
          warnings: ["Nonlinear effects are validated for the documented QuickPLS v1.2.3 fixed-score quadratic diagnostic scope; diagnostics use fixed PLS construct scores and centered squared score terms."],
        },
      },
    };
    const completed = { schemaVersion: 1, analyticalResult, canonicalDocument };
    expect(parseInternalRecipeV4CompletedResultV1(completed)).toEqual(completed);

    const mismatch = clone(completed);
    mismatch.analyticalResult.provenance.compilation_receipt.dataset_fingerprint = `v2:${"f".repeat(64)}`;
    expect(() => parseInternalRecipeV4CompletedResultV1(mismatch))
      .toThrow(/differs from the v7 compilation and resident-data identity/);
  });

  it("accepts an exact digest-bound response and preserves the canonical document", async () => {
    const response = await outcomeFixture();

    expect(response.value.documents[0].canonicalDocumentJson).toContain('"value":1.0');
    expect(response.value.documents[0].canonicalDocument.tables[0].rows[0].cells[0])
      .toEqual({ kind: "number", value: 1 });

    await expect(
      parseInternalProjectSchema6ResultReadOutcomeV1(response, request),
    ).resolves.toEqual(response);
  });

  it("round-trips strict General SEM attachments with a versioned dataset fingerprint", async () => {
    const response = await generalSemOutcomeFixture();

    const parsed = await parseInternalProjectSchema6ResultReadOutcomeV1(response, request);

    expect(parsed.status).toBe("ok");
    if (parsed.status !== "ok") throw new Error("Expected an ok schema-6 read outcome.");
    const document = parsed.value.documents[0].canonicalDocument;
    expect(document.provenance.dataset_fingerprint).toBe(`v2:${"b".repeat(64)}`);
    expect(document.general_sem_results?.identification_diagnostics?.[0]).toMatchObject({
      diagnostic_id: "identification_model_1",
      subject_id: document.provenance.model_id,
      status: "identified",
    });
    expect(parsed).toEqual(response);
  });

  it("rejects tampered and unknown General SEM fields after exact JSON digest rebinding", async () => {
    const tampered = await generalSemOutcomeFixture();
    tampered.value.documents[0].canonicalDocument.general_sem_results!
      .identification_diagnostics![0].subject_id = "model-other";
    await synchronizeGeneralSemAttachment(tampered);
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(tampered, request))
      .rejects.toThrow(/general_sem_results\.identification_diagnostics\[0\]\.subject_id.*provenance\.model_id/);

    const unknown = await generalSemOutcomeFixture();
    (unknown.value.documents[0].canonicalDocument.general_sem_results!
      .identification_diagnostics![0] as unknown as Record<string, unknown>).unexpected = true;
    await synchronizeGeneralSemAttachment(unknown);
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(unknown, request))
      .rejects.toThrow(/general_sem_results\.identification_diagnostics\[0\]\.unexpected.*not supported/);
  });

  it("keeps historical schema-6 projects without canonical attachments readable", async () => {
    const response = await outcomeFixture();
    response.value.documents = [];
    response.value.canonicalResultDocumentCount = 0;

    await expect(
      parseInternalProjectSchema6ResultReadOutcomeV1(response, request),
    ).resolves.toEqual(response);
  });

  it("rejects unknown keys at every native response envelope layer", async () => {
    const root = await outcomeFixture() as Record<string, unknown>;
    root.unexpected = true;
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(root, request))
      .rejects.toThrow(/root: must contain exactly/);

    const snapshot = await outcomeFixture();
    (snapshot.value as unknown as Record<string, unknown>).unexpected = true;
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(snapshot, request))
      .rejects.toThrow(/root\.value: must contain exactly/);

    const attachment = await outcomeFixture();
    (attachment.value.documents[0] as unknown as Record<string, unknown>).unexpected = true;
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(attachment, request))
      .rejects.toThrow(/root\.value\.documents\[0\]: must contain exactly/);
  });

  it("rejects document counts that differ from the returned attachment array", async () => {
    const response = await outcomeFixture();
    response.value.canonicalResultDocumentCount = 2;

    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(response, request))
      .rejects.toThrow(/canonicalResultDocumentCount: does not match documents\.length/);
  });

  it("rejects attachment, run, and project identity mismatches", async () => {
    const documentMismatch = await outcomeFixture();
    documentMismatch.value.documents[0].documentId = "result.document:other";
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(documentMismatch, request))
      .rejects.toThrow(/documentId: does not match canonicalDocument\.document_id/);

    const runMismatch = await outcomeFixture();
    runMismatch.value.documents[0].runId = "run-other";
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(runMismatch, request))
      .rejects.toThrow(/runId: does not match canonicalDocument\.provenance\.run_id/);

    const projectMismatch = await outcomeFixture();
    projectMismatch.value.projectId = "project-other";
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(projectMismatch, request))
      .rejects.toThrow(/provenance\.project_id: does not match value\.projectId/);
  });

  it("rejects a structurally invalid or scientifically tampered canonical document", async () => {
    const response = await outcomeFixture();
    response.value.documents[0].canonicalDocument.title = "";

    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(response, request))
      .rejects.toThrow(/canonicalDocument\.title: must be a nonempty string/);
  });

  it("rejects canonical JSON tampering, unknown keys, and malformed JSON", async () => {
    const semanticTamper = await outcomeFixture();
    const changed = JSON.parse(
      semanticTamper.value.documents[0].canonicalDocumentJson,
    ) as CanonicalResultDocumentV2;
    changed.title = "Changed only in the canonical JSON";
    semanticTamper.value.documents[0].canonicalDocumentJson = JSON.stringify(changed);
    semanticTamper.value.documents[0].canonicalDocumentSha256 = await canonicalResultDocumentJsonSha256V1(
      semanticTamper.value.documents[0].canonicalDocumentJson,
    );
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(semanticTamper, request))
      .rejects.toThrow(/canonicalDocument: does not semantically match canonicalDocumentJson/);

    const unknownKey = await outcomeFixture();
    const withUnknown = JSON.parse(
      unknownKey.value.documents[0].canonicalDocumentJson,
    ) as CanonicalResultDocumentV2;
    (withUnknown.tables[0].rows[0].cells[0] as unknown as Record<string, unknown>)
      .unexpected = true;
    unknownKey.value.documents[0].canonicalDocumentJson = JSON.stringify(withUnknown);
    unknownKey.value.documents[0].canonicalDocumentSha256 = await canonicalResultDocumentJsonSha256V1(
      unknownKey.value.documents[0].canonicalDocumentJson,
    );
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(unknownKey, request))
      .rejects.toThrow(/canonicalDocumentJson\.tables\[0\]\.rows\[0\]\.cells\[0\]: contains unknown keys: unexpected/);

    const malformed = await outcomeFixture();
    malformed.value.documents[0].canonicalDocumentJson = "{not-json";
    malformed.value.documents[0].canonicalDocumentSha256 = await canonicalResultDocumentJsonSha256V1(
      malformed.value.documents[0].canonicalDocumentJson,
    );
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(malformed, request))
      .rejects.toThrow(/canonicalDocumentJson: must contain valid JSON/);
  });

  it("rejects lowercase-SHA violations and canonical-document digest mismatches", async () => {
    const uppercaseSource = await outcomeFixture();
    uppercaseSource.value.sourceDocumentSha256 = "D".repeat(64);
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(uppercaseSource, request))
      .rejects.toThrow(/sourceDocumentSha256: must be a lowercase SHA-256 value/);

    const uppercaseAttachment = await outcomeFixture();
    uppercaseAttachment.value.documents[0].canonicalDocumentSha256 = "A".repeat(64);
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(uppercaseAttachment, request))
      .rejects.toThrow(/canonicalDocumentSha256: must be a lowercase SHA-256 value/);

    const digestMismatch = clone(await outcomeFixture());
    digestMismatch.value.documents[0].canonicalDocumentSha256 = "0".repeat(64);
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(digestMismatch, request))
      .rejects.toThrow(/canonicalDocumentSha256: does not match the exact canonicalDocumentJson bytes/);
  });

  it("parses exact blocked diagnostics and rejects added diagnostic fields", async () => {
    const blocked = {
      status: "blocked",
      diagnostic: {
        code: "schema6_result_read.source_changed",
        message: "The source changed.",
        correctiveAction: "Reinspect the project.",
      },
    };
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(blocked)).resolves.toEqual(blocked);

    const tampered = clone(blocked) as typeof blocked & { diagnostic: { debug?: string } };
    tampered.diagnostic.debug = "internal";
    await expect(parseInternalProjectSchema6ResultReadOutcomeV1(tampered))
      .rejects.toThrow(/root\.diagnostic: must contain exactly/);
  });
});
