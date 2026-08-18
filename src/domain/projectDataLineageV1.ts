import type {
  DatasetTransformationSpecV2,
} from "./datasetTransformationsV2";
import type {
  Dataset,
  DatasetTransformationLineageRecordV2,
  DatasetVersionRecord,
} from "../types";

const CANONICAL_UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const LOWER_SHA256 = /^[0-9a-f]{64}$/;
const TRANSFORM_OPERATION_ID = /^dataset_transform:[0-9a-f]{24}$/;
const RFC3339 = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/;

export class ProjectDataLineageV1Error extends Error {
  constructor(
    public readonly code: string,
    public readonly path: string,
    message: string,
  ) {
    super(message);
    this.name = "ProjectDataLineageV1Error";
  }
}

function fail(code: string, path: string, message: string): never {
  throw new ProjectDataLineageV1Error(code, path, message);
}

function object(value: unknown, path: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return fail("data_lineage.object_required", path, `${path} must be an object.`);
  }
  return value as Record<string, unknown>;
}

function exactKeys(
  value: Record<string, unknown>,
  required: readonly string[],
  optional: readonly string[],
  path: string,
) {
  const allowed = new Set([...required, ...optional]);
  for (const key of required) {
    if (!(key in value)) fail("data_lineage.field_missing", `${path}.${key}`, `${path}.${key} is required.`);
  }
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) fail("data_lineage.field_unknown", `${path}.${key}`, `${path}.${key} is not part of the v1 contract.`);
  }
}

function text(value: unknown, path: string, allowEmpty = false): string {
  if (typeof value !== "string" || (!allowEmpty && value.length === 0)) {
    return fail("data_lineage.text_required", path, `${path} must be a${allowEmpty ? "" : " non-empty"} string.`);
  }
  return value;
}

function optionalText(value: unknown, path: string): string | null {
  if (value === null) return null;
  return text(value, path);
}

function finite(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return fail("data_lineage.finite_number_required", path, `${path} must be finite.`);
  }
  return Object.is(value, -0) ? 0 : value;
}

function count(value: unknown, path: string): number {
  const parsed = finite(value, path);
  if (!Number.isSafeInteger(parsed) || parsed < 0) {
    return fail("data_lineage.count_invalid", path, `${path} must be a non-negative safe integer.`);
  }
  return parsed;
}

function canonicalUuid(value: unknown, path: string): string {
  const parsed = text(value, path);
  if (!CANONICAL_UUID.test(parsed)) {
    return fail("data_lineage.uuid_noncanonical", path, `${path} must be a canonical lowercase UUID.`);
  }
  return parsed;
}

function timestamp(value: unknown, path: string): string {
  const parsed = text(value, path);
  if (!RFC3339.test(parsed) || !Number.isFinite(Date.parse(parsed))) {
    return fail("data_lineage.timestamp_invalid", path, `${path} must be an RFC 3339 timestamp.`);
  }
  return parsed;
}

function nullableTimestamp(value: unknown, path: string): string | null {
  return value === null ? null : timestamp(value, path);
}

function oneOf<T extends string>(value: unknown, allowed: readonly T[], path: string): T {
  if (typeof value !== "string" || !allowed.includes(value as T)) {
    return fail("data_lineage.enum_invalid", path, `${path} has an unsupported value.`);
  }
  return value as T;
}

function datasetCell(value: unknown, path: string, missingAllowed: boolean): string | number | null {
  if (value === null && missingAllowed) return null;
  if (typeof value === "string") return value;
  if (typeof value === "number" && Number.isFinite(value) && Number.isSafeInteger(value) === Number.isInteger(value)) {
    return Object.is(value, -0) ? 0 : value;
  }
  return fail("data_lineage.cell_invalid", path, `${path} must be a JavaScript-safe finite number${missingAllowed ? ", string, or null" : " or string"}.`);
}

function nullableLabel(value: unknown, path: string): string | null | undefined {
  if (value === undefined) return undefined;
  if (value === null || typeof value === "string") return value;
  return fail("data_lineage.label_invalid", path, `${path} must be a string or null.`);
}

function parseArithmeticRight(value: unknown, path: string) {
  const candidate = object(value, path);
  const kind = oneOf(candidate.kind, ["column", "constant"] as const, `${path}.kind`);
  if (kind === "column") {
    exactKeys(candidate, ["kind", "column"], [], path);
    text(candidate.column, `${path}.column`);
  } else {
    exactKeys(candidate, ["kind", "value"], [], path);
    finite(candidate.value, `${path}.value`);
  }
}

function parseGroupRule(value: unknown, path: string) {
  const candidate = object(value, path);
  const kind = oneOf(candidate.kind, ["values", "numeric_range"] as const, `${path}.kind`);
  if (kind === "values") {
    exactKeys(candidate, ["kind", "output", "values"], ["label"], path);
    datasetCell(candidate.output, `${path}.output`, false);
    if (!Array.isArray(candidate.values) || candidate.values.length === 0) {
      fail("data_lineage.group_values_invalid", `${path}.values`, `${path}.values must be non-empty.`);
    }
    candidate.values.forEach((item, index) => datasetCell(item, `${path}.values.${index}`, false));
  } else {
    exactKeys(candidate, ["kind", "output", "minimum", "maximum", "include_minimum", "include_maximum"], ["label"], path);
    datasetCell(candidate.output, `${path}.output`, false);
    if (candidate.minimum !== null) finite(candidate.minimum, `${path}.minimum`);
    if (candidate.maximum !== null) finite(candidate.maximum, `${path}.maximum`);
    if (typeof candidate.include_minimum !== "boolean" || typeof candidate.include_maximum !== "boolean") {
      fail("data_lineage.group_bound_invalid", path, `${path} range-inclusion flags must be boolean.`);
    }
  }
  nullableLabel(candidate.label, `${path}.label`);
}

function parseDeclaredTarget(candidate: Record<string, unknown>, path: string) {
  text(candidate.target_column, `${path}.target_column`);
  oneOf(candidate.target_type, ["numeric", "text", "boolean"] as const, `${path}.target_type`);
  oneOf(candidate.target_scale, ["continuous", "ordinal", "nominal", "binary", "identifier"] as const, `${path}.target_scale`);
  nullableLabel(candidate.target_label, `${path}.target_label`);
  if (candidate.value_labels !== undefined) {
    const labels = object(candidate.value_labels, `${path}.value_labels`);
    Object.entries(labels).forEach(([key, label]) => text(label, `${path}.value_labels.${key}`, true));
  }
}

function parseTransformationSpec(value: unknown, path: string): DatasetTransformationSpecV2 {
  const candidate = object(value, path);
  const kind = oneOf(candidate.kind, ["add_column", "missing_markers", "reverse_scale", "standardize", "recode", "arithmetic", "row_aggregate", "dummy", "group"] as const, `${path}.kind`);
  if (kind === "add_column") {
    exactKeys(candidate, ["kind", "target_column", "value", "target_type", "target_scale"], ["target_label", "value_labels"], path);
    datasetCell(candidate.value, `${path}.value`, true);
    parseDeclaredTarget(candidate, path);
  } else if (kind === "missing_markers") {
    exactKeys(candidate, ["kind", "columns"], [], path);
    if (!Array.isArray(candidate.columns) || candidate.columns.length === 0) fail("data_lineage.missing_marker_columns_invalid", `${path}.columns`, `${path}.columns must be non-empty.`);
    candidate.columns.forEach((value, index) => {
      const columnPath = `${path}.columns.${index}`;
      const column = object(value, columnPath);
      exactKeys(column, ["source_column", "target_column", "markers", "target_type", "target_scale"], ["target_label", "value_labels"], columnPath);
      text(column.source_column, `${columnPath}.source_column`);
      if (!Array.isArray(column.markers) || column.markers.length === 0) fail("data_lineage.missing_markers_invalid", `${columnPath}.markers`, `${columnPath}.markers must be non-empty.`);
      column.markers.forEach((marker, markerIndex) => datasetCell(marker, `${columnPath}.markers.${markerIndex}`, false));
      parseDeclaredTarget(column, columnPath);
    });
  } else if (kind === "reverse_scale") {
    exactKeys(candidate, ["kind", "source_column", "target_column", "scale_min", "scale_max"], ["target_label"], path);
    text(candidate.source_column, `${path}.source_column`);
    text(candidate.target_column, `${path}.target_column`);
    finite(candidate.scale_min, `${path}.scale_min`);
    finite(candidate.scale_max, `${path}.scale_max`);
  } else if (kind === "standardize") {
    exactKeys(candidate, ["kind", "source_column", "target_column", "denominator"], ["target_label"], path);
    text(candidate.source_column, `${path}.source_column`);
    text(candidate.target_column, `${path}.target_column`);
    oneOf(candidate.denominator, ["sample_n_minus_one"] as const, `${path}.denominator`);
  } else if (kind === "recode") {
    exactKeys(candidate, ["kind", "source_column", "target_column", "mappings", "unmapped", "target_type", "target_scale"], ["target_label", "value_labels"], path);
    text(candidate.source_column, `${path}.source_column`);
    text(candidate.target_column, `${path}.target_column`);
    if (!Array.isArray(candidate.mappings) || candidate.mappings.length === 0) fail("data_lineage.mappings_invalid", `${path}.mappings`, `${path}.mappings must be non-empty.`);
    candidate.mappings.forEach((item, index) => {
      const mapping = object(item, `${path}.mappings.${index}`);
      exactKeys(mapping, ["source", "target"], [], `${path}.mappings.${index}`);
      datasetCell(mapping.source, `${path}.mappings.${index}.source`, false);
      datasetCell(mapping.target, `${path}.mappings.${index}.target`, true);
    });
    oneOf(candidate.unmapped, ["keep", "missing", "error"] as const, `${path}.unmapped`);
    oneOf(candidate.target_type, ["numeric", "text", "boolean"] as const, `${path}.target_type`);
    oneOf(candidate.target_scale, ["continuous", "ordinal", "nominal", "binary", "identifier"] as const, `${path}.target_scale`);
    if (candidate.value_labels !== undefined) parseDeclaredTarget(candidate, path);
  } else if (kind === "arithmetic") {
    exactKeys(candidate, ["kind", "left_column", "right", "operator", "target_column"], ["target_label"], path);
    text(candidate.left_column, `${path}.left_column`);
    parseArithmeticRight(candidate.right, `${path}.right`);
    oneOf(candidate.operator, ["add", "subtract", "multiply", "divide"] as const, `${path}.operator`);
    text(candidate.target_column, `${path}.target_column`);
  } else if (kind === "row_aggregate") {
    exactKeys(candidate, ["kind", "source_columns", "operation", "missing_policy", "target_column"], ["minimum_non_missing", "target_label"], path);
    if (!Array.isArray(candidate.source_columns) || candidate.source_columns.length === 0) fail("data_lineage.source_columns_invalid", `${path}.source_columns`, `${path}.source_columns must be non-empty.`);
    candidate.source_columns.forEach((column, index) => text(column, `${path}.source_columns.${index}`));
    oneOf(candidate.operation, ["sum", "mean"] as const, `${path}.operation`);
    oneOf(candidate.missing_policy, ["propagate", "available"] as const, `${path}.missing_policy`);
    if (candidate.minimum_non_missing !== undefined) count(candidate.minimum_non_missing, `${path}.minimum_non_missing`);
    text(candidate.target_column, `${path}.target_column`);
  } else if (kind === "dummy") {
    exactKeys(candidate, ["kind", "source_column", "match_value", "missing_policy", "target_column"], ["target_label"], path);
    text(candidate.source_column, `${path}.source_column`);
    datasetCell(candidate.match_value, `${path}.match_value`, false);
    oneOf(candidate.missing_policy, ["missing", "zero"] as const, `${path}.missing_policy`);
    text(candidate.target_column, `${path}.target_column`);
  } else {
    exactKeys(candidate, ["kind", "source_column", "rules", "unmatched", "target_column"], ["target_label"], path);
    text(candidate.source_column, `${path}.source_column`);
    if (!Array.isArray(candidate.rules) || candidate.rules.length === 0) fail("data_lineage.group_rules_invalid", `${path}.rules`, `${path}.rules must be non-empty.`);
    candidate.rules.forEach((rule, index) => parseGroupRule(rule, `${path}.rules.${index}`));
    oneOf(candidate.unmatched, ["missing", "error"] as const, `${path}.unmatched`);
    text(candidate.target_column, `${path}.target_column`);
  }
  nullableLabel(candidate.target_label, `${path}.target_label`);
  return candidate as unknown as DatasetTransformationSpecV2;
}

function specInputColumns(spec: DatasetTransformationSpecV2): string[] {
  if (spec.kind === "add_column") return [];
  if (spec.kind === "missing_markers") return spec.columns.map((column) => column.source_column);
  if (spec.kind === "arithmetic") return spec.right.kind === "column"
    ? [spec.left_column, spec.right.column]
    : [spec.left_column];
  if (spec.kind === "row_aggregate") return [...spec.source_columns];
  return [spec.source_column];
}

function specOutputColumns(spec: DatasetTransformationSpecV2): string[] {
  return spec.kind === "missing_markers"
    ? spec.columns.map((column) => column.target_column)
    : [spec.target_column];
}

async function sha256Hex(value: string): Promise<string> {
  const digest = await globalThis.crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function parseTransformationReceipt(
  value: unknown,
  path: string,
): Promise<DatasetTransformationLineageRecordV2> {
  const receipt = object(value, path);
  exactKeys(receipt, [
    "schema_version", "engine", "operation_id", "source_dataset_id",
    "source_dataset_fingerprint", "output_dataset_id", "output_dataset_fingerprint",
    "created_at", "spec_sha256", "spec", "input_columns", "output_columns",
    "source_row_count", "output_missing_count",
  ], [], path);
  if (receipt.schema_version !== 2 || receipt.engine !== "qpls.dataset_transform.v2") {
    fail("data_lineage.transform_identity_invalid", path, `${path} has an unsupported schema or engine.`);
  }
  const operationId = text(receipt.operation_id, `${path}.operation_id`);
  if (!TRANSFORM_OPERATION_ID.test(operationId)) fail("data_lineage.operation_id_invalid", `${path}.operation_id`, `${path}.operation_id is invalid.`);
  canonicalUuid(receipt.source_dataset_id, `${path}.source_dataset_id`);
  canonicalUuid(receipt.output_dataset_id, `${path}.output_dataset_id`);
  text(receipt.source_dataset_fingerprint, `${path}.source_dataset_fingerprint`);
  text(receipt.output_dataset_fingerprint, `${path}.output_dataset_fingerprint`);
  timestamp(receipt.created_at, `${path}.created_at`);
  const specSha256 = text(receipt.spec_sha256, `${path}.spec_sha256`);
  if (!LOWER_SHA256.test(specSha256)) fail("data_lineage.spec_sha256_invalid", `${path}.spec_sha256`, `${path}.spec_sha256 must be lowercase SHA-256.`);
  const spec = parseTransformationSpec(receipt.spec, `${path}.spec`);
  if (!Array.isArray(receipt.input_columns) || !Array.isArray(receipt.output_columns)) fail("data_lineage.transform_columns_invalid", path, `${path} columns must be arrays.`);
  const inputColumns = receipt.input_columns.map((column, index) => text(column, `${path}.input_columns.${index}`));
  const outputColumns = receipt.output_columns.map((column, index) => text(column, `${path}.output_columns.${index}`));
  count(receipt.source_row_count, `${path}.source_row_count`);
  count(receipt.output_missing_count, `${path}.output_missing_count`);
  const expectedInputs = specInputColumns(spec);
  if (JSON.stringify(inputColumns) !== JSON.stringify(expectedInputs)
      || JSON.stringify(outputColumns) !== JSON.stringify(specOutputColumns(spec))) {
    fail("data_lineage.transform_columns_mismatch", path, `${path} columns differ from its exact specification.`);
  }
  // Rust owns canonical transformation JSON and replay. Re-serializing f64 values
  // with JSON.stringify can change lexical forms such as 1.0 to 1, so this
  // untrusted-reader layer validates the stored hash shape and its binding into
  // operation_id without pretending to recompute the Rust canonical digest.
  const expectedOperationDigest = await sha256Hex(`${receipt.source_dataset_fingerprint}\u0000${specSha256}\u0000${receipt.output_dataset_id}`);
  if (operationId !== `dataset_transform:${expectedOperationDigest.slice(0, 24)}`) {
    fail("data_lineage.operation_id_mismatch", `${path}.operation_id`, `${path}.operation_id differs from the source/spec/output identity.`);
  }
  return receipt as unknown as DatasetTransformationLineageRecordV2;
}

export async function parseProjectDatasetVersionRecordV1(
  value: unknown,
  path = "datasetVersion",
): Promise<DatasetVersionRecord> {
  const record = object(value, path);
  exactKeys(record, [
    "datasetId", "parentDatasetId", "operation", "createdAt", "summary",
    "sourceColumn", "targetColumn",
  ], ["transformation"], path);
  const datasetId = canonicalUuid(record.datasetId, `${path}.datasetId`);
  const parentDatasetId = record.parentDatasetId === null
    ? null
    : canonicalUuid(record.parentDatasetId, `${path}.parentDatasetId`);
  const operation = oneOf(record.operation, ["import", "metadata", "recode", "transform"] as const, `${path}.operation`);
  const createdAt = nullableTimestamp(record.createdAt, `${path}.createdAt`);
  text(record.summary, `${path}.summary`);
  const sourceColumn = optionalText(record.sourceColumn, `${path}.sourceColumn`);
  const targetColumn = optionalText(record.targetColumn, `${path}.targetColumn`);
  const transformation = record.transformation === undefined
    ? undefined
    : await parseTransformationReceipt(record.transformation, `${path}.transformation`);

  if (operation === "import") {
    if (parentDatasetId !== null || sourceColumn !== null || targetColumn !== null || transformation !== undefined) {
      fail("data_lineage.import_shape_invalid", path, `${path} import must be a root without columns or transformation.`);
    }
  } else if (operation === "metadata") {
    if (parentDatasetId === null || sourceColumn === null || targetColumn !== null || transformation !== undefined) {
      fail("data_lineage.metadata_shape_invalid", path, `${path} metadata shape is invalid.`);
    }
  } else if (operation === "recode") {
    if (parentDatasetId === null || sourceColumn === null || targetColumn === null || transformation !== undefined) {
      fail("data_lineage.recode_shape_invalid", path, `${path} referential recode shape is invalid.`);
    }
  } else {
    if (parentDatasetId === null || createdAt === null || targetColumn === null || !transformation) {
      fail("data_lineage.transform_shape_invalid", path, `${path} reconstructable transform shape is invalid.`);
    }
    if (transformation.source_dataset_id !== parentDatasetId
        || transformation.output_dataset_id !== datasetId
        || transformation.created_at !== createdAt
        || (transformation.input_columns[0] ?? null) !== sourceColumn
        || transformation.output_columns[0] !== targetColumn) {
      fail("data_lineage.transform_outer_mismatch", path, `${path} outer fields differ from the transformation receipt.`);
    }
  }
  return record as unknown as DatasetVersionRecord;
}

function datasetRowCount(dataset: Dataset): number {
  return dataset.rowCount ?? dataset.rows.length;
}

function sameBaseShape(parent: Dataset, output: Dataset): boolean {
  return (parent.kind ?? "raw") === (output.kind ?? "raw")
    && (parent.sampleSize ?? null) === (output.sampleSize ?? null)
    && datasetRowCount(parent) === datasetRowCount(output);
}

function parentPlusTargetsShape(parent: Dataset, output: Dataset, targets: readonly string[]): boolean {
  return sameBaseShape(parent, output)
    && targets.length > 0
    && output.columns.length === parent.columns.length + targets.length
    && parent.columns.every((column, index) => output.columns[index] === column)
    && targets.every((target, index) => output.columns[parent.columns.length + index] === target);
}

export async function parseProjectDatasetVersionRecordsV1(
  value: unknown,
  datasets: readonly Dataset[],
  path = "datasetVersions",
): Promise<DatasetVersionRecord[]> {
  // Older project snapshots did not expose the reserved field at all.
  if (value === undefined) return [];
  if (!Array.isArray(value)) fail("data_lineage.records_invalid", path, `${path} must be an array when present.`);
  const records = await Promise.all(value.map((record, index) => parseProjectDatasetVersionRecordV1(record, `${path}.${index}`)));
  const datasetsById = new Map<string, Dataset>();
  for (const [index, dataset] of datasets.entries()) {
    const id = canonicalUuid(dataset.id, `datasets.${index}.id`);
    if (datasetsById.has(id)) fail("data_lineage.dataset_duplicate", `datasets.${index}.id`, `Dataset ${id} is duplicated.`);
    datasetsById.set(id, dataset);
  }
  const recordsById = new Map<string, DatasetVersionRecord>();
  const operationIds = new Set<string>();
  for (const [index, record] of records.entries()) {
    if (recordsById.has(record.datasetId)) fail("data_lineage.record_duplicate", `${path}.${index}.datasetId`, `Dataset ${record.datasetId} has duplicate lineage records.`);
    recordsById.set(record.datasetId, record);
    const output = datasetsById.get(record.datasetId);
    if (!output) fail("data_lineage.output_unknown", `${path}.${index}.datasetId`, `Dataset ${record.datasetId} does not exist.`);
    const parent = record.parentDatasetId ? datasetsById.get(record.parentDatasetId) : undefined;
    if (record.parentDatasetId && !parent) fail("data_lineage.parent_unknown", `${path}.${index}.parentDatasetId`, `Parent dataset ${record.parentDatasetId} does not exist.`);
    if (record.parentDatasetId === record.datasetId) fail("data_lineage.self_parent", `${path}.${index}.parentDatasetId`, `A dataset cannot be its own parent.`);
    if (record.operation === "metadata" && parent) {
      if (!sameBaseShape(parent, output) || JSON.stringify(parent.columns) !== JSON.stringify(output.columns) || !parent.columns.includes(record.sourceColumn!)) {
        fail("data_lineage.metadata_dataset_mismatch", `${path}.${index}`, `Metadata lineage differs from the resident dataset shape.`);
      }
    }
    if (record.operation === "recode" && parent) {
      if ((parent.kind ?? "raw") !== "raw" || (output.kind ?? "raw") !== "raw"
          || !parent.columns.includes(record.sourceColumn!)
          || !parentPlusTargetsShape(parent, output, [record.targetColumn!])) {
        fail("data_lineage.derived_dataset_mismatch", `${path}.${index}`, `Derived lineage differs from the resident dataset shape.`);
      }
    }
    if (record.operation === "transform" && parent && record.transformation) {
      if ((parent.kind ?? "raw") !== "raw" || (output.kind ?? "raw") !== "raw"
          || record.transformation.input_columns.some((column) => !parent.columns.includes(column))
          || !parentPlusTargetsShape(parent, output, record.transformation.output_columns)) {
        fail("data_lineage.derived_dataset_mismatch", `${path}.${index}`, `Derived lineage differs from the resident dataset shape.`);
      }
    }
    if (record.transformation) {
      if (record.transformation.source_dataset_fingerprint !== parent?.fingerprint
          || record.transformation.output_dataset_fingerprint !== output.fingerprint
          || record.transformation.source_row_count !== (parent ? datasetRowCount(parent) : -1)
          || record.transformation.output_missing_count > datasetRowCount(output) * record.transformation.output_columns.length) {
        fail("data_lineage.transform_dataset_mismatch", `${path}.${index}.transformation`, `Transformation receipt differs from the resident datasets.`);
      }
      if (operationIds.has(record.transformation.operation_id)) fail("data_lineage.operation_id_duplicate", `${path}.${index}.transformation.operation_id`, `Transformation operation ID is duplicated.`);
      operationIds.add(record.transformation.operation_id);
    }
  }
  for (const record of records) {
    const visited = new Set<string>();
    let current: DatasetVersionRecord | undefined = record;
    while (current) {
      if (visited.has(current.datasetId)) fail("data_lineage.cycle", path, `Dataset lineage contains a cycle through ${current.datasetId}.`);
      visited.add(current.datasetId);
      current = current.parentDatasetId ? recordsById.get(current.parentDatasetId) : undefined;
    }
  }
  return records;
}
