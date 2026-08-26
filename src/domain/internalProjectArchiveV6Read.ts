import {
  parseProjectAnalysisRecipeV4Wire,
  parseInternalProjectArchiveV6Wire,
  supportsGeneralSemV1,
  type ProjectAnalysisRecipeV4Wire,
  type InternalProjectArchiveV6Wire,
} from "./internalProjectArchiveV6Wire";
import {
  INTERNAL_PROJECT_ARCHIVE_V6_LABS_ACCESS_V1,
  internalProjectArchiveV6AccessPairV1,
  type InternalProjectArchiveV6AccessV1,
} from "./internalProjectArchiveV6Access";

const LOWER_SHA256 = /^[0-9a-f]{64}$/;
const CANONICAL_UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

type WireRecord = Record<string, unknown>;

/** Historical surface retained for callers that explicitly request Labs. */
export const INTERNAL_PROJECT_ARCHIVE_V6_READ_SURFACE =
  INTERNAL_PROJECT_ARCHIVE_V6_LABS_ACCESS_V1.surface;

export type InternalProjectArchiveV6ReadRequestV1 =
  InternalProjectArchiveV6AccessV1 & {
  archivePath: string;
};

export interface InternalProjectArchiveV6ManifestV1 {
  schema_version: 6;
  project_id: string;
  name: string;
  created_at: string;
  modified_at: string;
  engine_version: string;
  checksum_algorithm: "sha256";
  checksums: Record<string, string>;
}

export interface InternalProjectArchiveV6ResidentDatasetV1 {
  datasetId: string;
  name: string;
  fingerprint: string;
  rowCount: number;
  columnCount: number;
  sampleSize: number | null;
  arrowResident: true;
}

export interface InternalProjectArchiveV6ReadCountsV1 {
  datasets: number;
  models: number;
  recipes: number;
  historicalRecipes: number;
  historicalResults: number;
  canonicalResultDocuments: number;
}

export interface InternalProjectArchiveV6GeneralSemExecutionAuthorityV1 {
  schemaVersion: 1;
  projectId: string;
  datasetId: string;
  datasetFingerprint: string;
  modelId: string;
  modelScientificSha256: string;
  recipeId: string;
  recipeDocumentSha256: string;
  recipe: ProjectAnalysisRecipeV4Wire;
}

export interface InternalProjectArchiveV6ReadSnapshotV1 {
  schemaVersion: 1;
  access: "read_only";
  loader: "strict_schema6_zip";
  archivePath: string;
  archiveSha256: string;
  archiveBytes: number;
  manifest: InternalProjectArchiveV6ManifestV1;
  project: InternalProjectArchiveV6Wire;
  residentDatasets: InternalProjectArchiveV6ResidentDatasetV1[];
  counts: InternalProjectArchiveV6ReadCountsV1;
  /** Native-resolved exact execution authority for a bounded marked project. */
  generalSemExecutionAuthority?: InternalProjectArchiveV6GeneralSemExecutionAuthorityV1 | null;
  sourceRecheckedUnchanged: true;
}

export interface InternalProjectArchiveV6ReadDiagnosticV1 {
  code: string;
  message: string;
  correctiveAction: string;
}

export type InternalProjectArchiveV6ReadOutcomeV1 =
  | { status: "ok"; value: InternalProjectArchiveV6ReadSnapshotV1 }
  | { status: "blocked"; diagnostic: InternalProjectArchiveV6ReadDiagnosticV1 };

export class InternalProjectArchiveV6ReadWireError extends Error {
  constructor(
    public readonly code: string,
    public readonly path: string,
    message: string,
  ) {
    super(message);
    this.name = "InternalProjectArchiveV6ReadWireError";
  }
}

function fail(code: string, path: string, message: string): never {
  throw new InternalProjectArchiveV6ReadWireError(code, path, message);
}

function recordAt(value: unknown, path: string): WireRecord {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail("schema6_archive_read.object_required", path, `${path} must be an object.`);
  }
  return value as WireRecord;
}

function exactRecordAt(
  value: unknown,
  required: readonly string[],
  path: string,
): WireRecord {
  const record = recordAt(value, path);
  const allowed = new Set(required);
  for (const key of required) {
    if (!Object.prototype.hasOwnProperty.call(record, key)) {
      fail("schema6_archive_read.field_missing", `${path}.${key}`, `${path}.${key} is required.`);
    }
  }
  for (const key of Object.keys(record)) {
    if (!allowed.has(key)) {
      fail("schema6_archive_read.field_unknown", `${path}.${key}`, `${path}.${key} is not part of the read-only schema-6 bridge.`);
    }
  }
  return record;
}

function exactRecordWithOptionalAt(
  value: unknown,
  required: readonly string[],
  optional: readonly string[],
  path: string,
): WireRecord {
  const record = recordAt(value, path);
  const allowed = new Set([...required, ...optional]);
  for (const key of required) {
    if (!Object.prototype.hasOwnProperty.call(record, key)) {
      fail("schema6_archive_read.field_missing", `${path}.${key}`, `${path}.${key} is required.`);
    }
  }
  for (const key of Object.keys(record)) {
    if (!allowed.has(key)) {
      fail("schema6_archive_read.field_unknown", `${path}.${key}`, `${path}.${key} is not part of the read-only schema-6 bridge.`);
    }
  }
  return record;
}

function textAt(value: unknown, path: string, allowEmpty = false): string {
  if (typeof value !== "string" || (!allowEmpty && !value.trim())) {
    fail("schema6_archive_read.text_required", path, `${path} must be a${allowEmpty ? "" : " nonempty"} string.`);
  }
  return value;
}

export function parseInternalProjectArchiveV6ReadRequestV1(
  input: unknown,
): InternalProjectArchiveV6ReadRequestV1 {
  const path = "request";
  const request = exactRecordAt(
    input,
    ["surface", "experimentalLabsEnabled", "archivePath"],
    path,
  );
  const access = internalProjectArchiveV6AccessPairV1(
    request.surface,
    request.experimentalLabsEnabled,
  );
  if (!access) {
    fail(
      "schema6_archive_read.surface_pair_invalid",
      path,
      "Archive inspection requires exact internal_labs/true or standard_multimod_v1/false access.",
    );
  }
  return {
    ...access,
    archivePath: textAt(request.archivePath, `${path}.archivePath`),
  };
}

function sha256At(value: unknown, path: string): string {
  const digest = textAt(value, path);
  if (!LOWER_SHA256.test(digest)) {
    fail("schema6_archive_read.sha256_invalid", path, `${path} must be a lowercase SHA-256 digest.`);
  }
  return digest;
}

function countAt(value: unknown, path: string, positive = false): number {
  if (!Number.isSafeInteger(value) || Object.is(value, -0) || (value as number) < (positive ? 1 : 0)) {
    fail("schema6_archive_read.count_invalid", path, `${path} must be a ${positive ? "positive" : "nonnegative"} safe integer.`);
  }
  return value as number;
}

function parseManifest(value: unknown): InternalProjectArchiveV6ManifestV1 {
  const manifest = exactRecordAt(value, [
    "schema_version",
    "project_id",
    "name",
    "created_at",
    "modified_at",
    "engine_version",
    "checksum_algorithm",
    "checksums",
  ], "outcome.value.manifest");
  if (manifest.schema_version !== 6) {
    fail("schema6_archive_read.manifest_schema", "outcome.value.manifest.schema_version", "The strict ZIP manifest must use schema version 6.");
  }
  const projectId = textAt(manifest.project_id, "outcome.value.manifest.project_id");
  if (!CANONICAL_UUID.test(projectId)) {
    fail("schema6_archive_read.project_id_invalid", "outcome.value.manifest.project_id", "The manifest project id must be a canonical lowercase UUID.");
  }
  if (manifest.checksum_algorithm !== "sha256") {
    fail("schema6_archive_read.checksum_algorithm", "outcome.value.manifest.checksum_algorithm", "The strict ZIP manifest must use SHA-256 checksums.");
  }
  const checksumWire = recordAt(manifest.checksums, "outcome.value.manifest.checksums");
  const checksums = Object.fromEntries(
    Object.entries(checksumWire).map(([name, digest]) => [
      textAt(name, `outcome.value.manifest.checksums.${name}`),
      sha256At(digest, `outcome.value.manifest.checksums.${name}`),
    ]),
  );
  if (!("project.json" in checksums)) {
    fail("schema6_archive_read.project_checksum_missing", "outcome.value.manifest.checksums", "The validated ZIP manifest must bind project.json.");
  }
  return {
    schema_version: 6,
    project_id: projectId,
    name: textAt(manifest.name, "outcome.value.manifest.name", true),
    created_at: textAt(manifest.created_at, "outcome.value.manifest.created_at"),
    modified_at: textAt(manifest.modified_at, "outcome.value.manifest.modified_at"),
    engine_version: textAt(manifest.engine_version, "outcome.value.manifest.engine_version"),
    checksum_algorithm: "sha256",
    checksums,
  };
}

function parseResidentDataset(
  value: unknown,
  index: number,
): InternalProjectArchiveV6ResidentDatasetV1 {
  const path = `outcome.value.residentDatasets[${index}]`;
  const dataset = exactRecordAt(value, [
    "datasetId",
    "name",
    "fingerprint",
    "rowCount",
    "columnCount",
    "sampleSize",
    "arrowResident",
  ], path);
  if (dataset.arrowResident !== true) {
    fail("schema6_archive_read.arrow_not_resident", `${path}.arrowResident`, "Every returned dataset must have passed resident Arrow validation.");
  }
  return {
    datasetId: textAt(dataset.datasetId, `${path}.datasetId`),
    name: textAt(dataset.name, `${path}.name`, true),
    fingerprint: textAt(dataset.fingerprint, `${path}.fingerprint`),
    rowCount: countAt(dataset.rowCount, `${path}.rowCount`),
    columnCount: countAt(dataset.columnCount, `${path}.columnCount`),
    sampleSize: dataset.sampleSize === null ? null : countAt(dataset.sampleSize, `${path}.sampleSize`),
    arrowResident: true,
  };
}

function parseCounts(value: unknown): InternalProjectArchiveV6ReadCountsV1 {
  const path = "outcome.value.counts";
  const counts = exactRecordAt(value, [
    "datasets",
    "models",
    "recipes",
    "historicalRecipes",
    "historicalResults",
    "canonicalResultDocuments",
  ], path);
  return {
    datasets: countAt(counts.datasets, `${path}.datasets`),
    models: countAt(counts.models, `${path}.models`),
    recipes: countAt(counts.recipes, `${path}.recipes`),
    historicalRecipes: countAt(counts.historicalRecipes, `${path}.historicalRecipes`),
    historicalResults: countAt(counts.historicalResults, `${path}.historicalResults`),
    canonicalResultDocuments: countAt(counts.canonicalResultDocuments, `${path}.canonicalResultDocuments`),
  };
}

function parseGeneralSemExecutionAuthority(
  value: unknown,
): InternalProjectArchiveV6GeneralSemExecutionAuthorityV1 {
  const path = "outcome.value.generalSemExecutionAuthority";
  const authority = exactRecordAt(value, [
    "schemaVersion",
    "projectId",
    "datasetId",
    "datasetFingerprint",
    "modelId",
    "modelScientificSha256",
    "recipeId",
    "recipeDocumentSha256",
    "recipe",
  ], path);
  if (authority.schemaVersion !== 1) {
    fail("schema6_archive_read.general_sem_authority_schema", `${path}.schemaVersion`, "The General SEM execution authority must use schema version 1.");
  }
  const projectId = textAt(authority.projectId, `${path}.projectId`);
  if (!CANONICAL_UUID.test(projectId)) {
    fail("schema6_archive_read.project_id_invalid", `${path}.projectId`, "The General SEM authority project id must be a canonical lowercase UUID.");
  }
  const recipeId = textAt(authority.recipeId, `${path}.recipeId`);
  if (!CANONICAL_UUID.test(recipeId)) {
    fail("schema6_archive_read.recipe_id_invalid", `${path}.recipeId`, "The General SEM authority recipe id must be a canonical lowercase UUID.");
  }
  const recipe = parseProjectAnalysisRecipeV4Wire(authority.recipe, `${path}.recipe`);
  if (recipe.id !== recipeId) {
    fail("schema6_archive_read.general_sem_recipe_identity", path, "The General SEM authority recipe id differs from its resident RecipeV4 document.");
  }
  return {
    schemaVersion: 1,
    projectId,
    datasetId: textAt(authority.datasetId, `${path}.datasetId`),
    datasetFingerprint: textAt(authority.datasetFingerprint, `${path}.datasetFingerprint`),
    modelId: textAt(authority.modelId, `${path}.modelId`),
    modelScientificSha256: sha256At(authority.modelScientificSha256, `${path}.modelScientificSha256`),
    recipeId,
    recipeDocumentSha256: sha256At(authority.recipeDocumentSha256, `${path}.recipeDocumentSha256`),
    recipe,
  };
}

function assertSnapshotBindings(snapshot: InternalProjectArchiveV6ReadSnapshotV1): void {
  const { manifest, project, counts, residentDatasets, generalSemExecutionAuthority } = snapshot;
  if (manifest.project_id !== project.project_id
    || manifest.name !== project.name
    || manifest.created_at !== project.created_at
    || manifest.modified_at !== project.modified_at) {
    fail("schema6_archive_read.manifest_project_mismatch", "outcome.value.manifest", "Manifest identity or timestamps differ from the validated project document.");
  }

  const expectedCounts: InternalProjectArchiveV6ReadCountsV1 = {
    datasets: project.datasets.length,
    models: project.models.length,
    recipes: project.recipes.length,
    historicalRecipes: project.historical_recipes.length,
    historicalResults: project.historical_results.length,
    canonicalResultDocuments: project.canonical_result_documents.length,
  };
  for (const key of Object.keys(expectedCounts) as (keyof InternalProjectArchiveV6ReadCountsV1)[]) {
    if (counts[key] !== expectedCounts[key]) {
      fail("schema6_archive_read.count_mismatch", `outcome.value.counts.${key}`, `The ${key} count differs from the validated project document.`);
    }
  }

  if (residentDatasets.length !== project.datasets.length) {
    fail("schema6_archive_read.resident_dataset_count", "outcome.value.residentDatasets", "Every project dataset must have one validated resident Arrow summary.");
  }
  residentDatasets.forEach((resident, index) => {
    const descriptor = project.datasets[index];
    const path = `outcome.value.residentDatasets[${index}]`;
    if (resident.datasetId !== descriptor.id
      || resident.name !== descriptor.name
      || resident.fingerprint !== descriptor.fingerprint
      || resident.rowCount !== descriptor.schema.case_count
      || resident.columnCount !== descriptor.schema.columns.length
      || resident.sampleSize !== descriptor.schema.sample_size) {
      fail("schema6_archive_read.resident_dataset_mismatch", path, "The resident Arrow summary differs from its schema-6 dataset descriptor.");
    }
  });

  if (supportsGeneralSemV1(project)) {
    if (!generalSemExecutionAuthority) {
      if (project.datasets.length === 0 && project.models.length === 0 && project.recipes.length === 0) return;
      fail("schema6_archive_read.general_sem_authority_missing", "outcome.value.generalSemExecutionAuthority", "A populated general_sem_v1 archive must expose its native execution authority.");
    }
    const authority = generalSemExecutionAuthority;
    const dataset = project.datasets.find((candidate) => candidate.id === authority.datasetId);
    const model = project.models.find((candidate) => candidate.model_id === authority.modelId);
    const recipe = project.recipes.find((candidate) => candidate.id === authority.recipeId);
    const modelScientificSha256 = model?.payload.kind === "sem_model_v4"
      ? model.payload.scientific_sha256
      : null;
    const binding = recipe?.model_binding.kind === "project_sem_model_v4_reference"
      ? recipe.model_binding
      : null;
    if (authority.projectId !== project.project_id
      || dataset?.fingerprint !== authority.datasetFingerprint
      || modelScientificSha256 !== authority.modelScientificSha256
      || binding?.model_id !== authority.modelId
      || binding.scientific_sha256 !== authority.modelScientificSha256
      || recipe?.dataset_fingerprint !== authority.datasetFingerprint
      || JSON.stringify(recipe) !== JSON.stringify(authority.recipe)) {
      fail("schema6_archive_read.general_sem_authority_mismatch", "outcome.value.generalSemExecutionAuthority", "The native General SEM execution authority differs from the strictly parsed project document.");
    }
  } else if (generalSemExecutionAuthority) {
    fail("schema6_archive_read.general_sem_authority_unmarked", "outcome.value.generalSemExecutionAuthority", "An unmarked schema-6 archive cannot expose General SEM execution authority.");
  }
}

function parseSnapshot(value: unknown): InternalProjectArchiveV6ReadSnapshotV1 {
  const path = "outcome.value";
  const snapshot = exactRecordWithOptionalAt(value, [
    "schemaVersion",
    "access",
    "loader",
    "archivePath",
    "archiveSha256",
    "archiveBytes",
    "manifest",
    "project",
    "residentDatasets",
    "counts",
    "sourceRecheckedUnchanged",
  ], ["generalSemExecutionAuthority"], path);
  if (snapshot.schemaVersion !== 1) {
    fail("schema6_archive_read.snapshot_schema", `${path}.schemaVersion`, "The schema-6 read snapshot must use contract version 1.");
  }
  if (snapshot.access !== "read_only") {
    fail("schema6_archive_read.read_only_required", `${path}.access`, "The schema-6 bridge must remain read-only.");
  }
  if (snapshot.loader !== "strict_schema6_zip") {
    fail("schema6_archive_read.strict_loader_required", `${path}.loader`, "The snapshot must come from the dedicated strict schema-6 ZIP loader.");
  }
  if (snapshot.sourceRecheckedUnchanged !== true) {
    fail("schema6_archive_read.source_recheck_required", `${path}.sourceRecheckedUnchanged`, "The archive source must be rechecked after strict validation.");
  }
  if (!Array.isArray(snapshot.residentDatasets)) {
    fail("schema6_archive_read.array_required", `${path}.residentDatasets`, "residentDatasets must be an array.");
  }
  const parsed: InternalProjectArchiveV6ReadSnapshotV1 = {
    schemaVersion: 1,
    access: "read_only",
    loader: "strict_schema6_zip",
    archivePath: textAt(snapshot.archivePath, `${path}.archivePath`),
    archiveSha256: sha256At(snapshot.archiveSha256, `${path}.archiveSha256`),
    archiveBytes: countAt(snapshot.archiveBytes, `${path}.archiveBytes`, true),
    manifest: parseManifest(snapshot.manifest),
    project: parseInternalProjectArchiveV6Wire(snapshot.project),
    residentDatasets: snapshot.residentDatasets.map(parseResidentDataset),
    counts: parseCounts(snapshot.counts),
    generalSemExecutionAuthority: snapshot.generalSemExecutionAuthority == null
      ? null
      : parseGeneralSemExecutionAuthority(snapshot.generalSemExecutionAuthority),
    sourceRecheckedUnchanged: true,
  };
  assertSnapshotBindings(parsed);
  return parsed;
}

export function parseInternalProjectArchiveV6ReadOutcomeV1(
  input: unknown,
): InternalProjectArchiveV6ReadOutcomeV1 {
  const outcome = recordAt(input, "outcome");
  if (outcome.status === "ok") {
    exactRecordAt(outcome, ["status", "value"], "outcome");
    return { status: "ok", value: parseSnapshot(outcome.value) };
  }
  if (outcome.status === "blocked") {
    exactRecordAt(outcome, ["status", "diagnostic"], "outcome");
    const diagnostic = exactRecordAt(outcome.diagnostic, [
      "code",
      "message",
      "correctiveAction",
    ], "outcome.diagnostic");
    return {
      status: "blocked",
      diagnostic: {
        code: textAt(diagnostic.code, "outcome.diagnostic.code"),
        message: textAt(diagnostic.message, "outcome.diagnostic.message"),
        correctiveAction: textAt(diagnostic.correctiveAction, "outcome.diagnostic.correctiveAction"),
      },
    };
  }
  return fail("schema6_archive_read.status_invalid", "outcome.status", "The schema-6 read outcome status must be ok or blocked.");
}
