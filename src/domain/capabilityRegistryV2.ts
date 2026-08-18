import rawCapabilityRegistryV2 from "../../validation/capabilities/capability_registry_v2.json";
import {
  capabilityAvailabilityV2,
  type CapabilityAvailabilityV2,
  type CapabilitySurfaceCellV2,
  type CoverageStateV2,
  type EvidenceStateV2,
  type ProductSurfaceV2,
} from "./capabilitySurfaceV2";

const COVERAGE_STATES = ["full", "partial", "absent", "intentionally_excluded"] as const;
const EVIDENCE_STATES = [
  "absent",
  "engine_only",
  "archive_qualified",
  "native_qualified",
  "release_qualified",
] as const;
const PRODUCT_SURFACES = ["standard", "labs", "legacy", "internal"] as const;
const PRODUCT_AREAS = ["diagram", "data", "settings", "calculation", "results", "reporting"] as const;
const OFFICIAL_LIFECYCLES = ["active", "legacy"] as const;
const QUALIFICATION_LINK_KEYS = [
  "registry_schema_version",
  "capability_id",
  "cell_id",
  "capability_version",
] as const;

const TOP_LEVEL_KEYS = [
  "registry_schema_version",
  "registry_id",
  "registry_version",
  "frozen_on",
  "catalogue_snapshot",
  "comparison_policy",
  "state_contract",
  "customer_visibility_policy",
  "surface_contract",
  "product_area_contract",
  "qualification_link_contract",
  "exclusions",
  "legacy_catalogue_contract",
  "capabilities",
] as const;

const CAPABILITY_KEYS = [
  "capability_id",
  "catalogue_position",
  "official_family",
  "official_method",
  "official_lifecycle",
  "official_url",
  "coverage_state",
  "evidence_state",
  "surface",
  "product_areas",
  "model_predicates",
  "data_predicates",
  "supported_model_predicate",
  "supported_data_predicate",
  "scope_statement",
  "settings_references",
  "settings_schema",
  "result_references",
  "result_schema",
  "documentation_reference",
  "qualification_references",
  "qualification_links",
  "qualification_spec",
  "known_differences",
  "option_cells",
  "legacy_row",
] as const;

const OPTION_CELL_KEYS = [
  "capability_id",
  "cell_id",
  "capability_version",
  "coverage_state",
  "evidence_state",
  "surface",
  "supported_model_predicate",
  "supported_data_predicate",
  "settings_schema",
  "result_schema",
  "documentation_reference",
  "qualification_spec",
  "known_differences",
] as const;

const FROZEN_COVERAGE_COUNTS: Readonly<Record<CoverageStateV2, number>> = {
  full: 0,
  partial: 32,
  absent: 11,
  intentionally_excluded: 2,
};

export type ProductAreaV2 = (typeof PRODUCT_AREAS)[number];
export type OfficialLifecycleV2 = (typeof OFFICIAL_LIFECYCLES)[number];

export interface QualificationLinkV2 {
  readonly registry_schema_version: 2;
  readonly capability_id: string;
  readonly cell_id: string;
  readonly capability_version: string;
}

export interface CapabilityPredicatesV2 {
  readonly official: readonly string[];
  readonly quickpls: readonly string[];
}

export interface CapabilityOptionCellV2 extends CapabilitySurfaceCellV2 {
  readonly supported_model_predicate: CapabilityPredicatesV2;
  readonly supported_data_predicate: CapabilityPredicatesV2;
  readonly settings_schema: Readonly<{ references: readonly string[] }>;
  readonly result_schema: Readonly<{ references: readonly string[] }>;
  readonly documentation_reference: string;
  readonly qualification_spec: Readonly<{
    references: readonly string[];
    links: readonly [QualificationLinkV2];
  }>;
  readonly known_differences: readonly string[];
}

export interface CapabilityRegistryRowV2 {
  readonly capability_id: string;
  readonly catalogue_position: number;
  readonly official_family: string;
  readonly official_method: string;
  readonly official_lifecycle: OfficialLifecycleV2;
  readonly official_url: string;
  /** Conservative compatibility projection; never use this to resolve an option. */
  readonly coverage_state: CoverageStateV2;
  /** Conservative compatibility projection; never use this to resolve an option. */
  readonly evidence_state: EvidenceStateV2;
  /** Conservative compatibility projection; never use this to resolve an option. */
  readonly surface: ProductSurfaceV2;
  readonly product_areas: readonly ProductAreaV2[];
  readonly model_predicates: CapabilityPredicatesV2;
  readonly data_predicates: CapabilityPredicatesV2;
  readonly supported_model_predicate: CapabilityPredicatesV2;
  readonly supported_data_predicate: CapabilityPredicatesV2;
  readonly scope_statement: string;
  readonly settings_references: readonly string[];
  readonly settings_schema: Readonly<{ references: readonly string[] }>;
  readonly result_references: readonly string[];
  readonly result_schema: Readonly<{ references: readonly string[] }>;
  readonly documentation_reference: string;
  readonly qualification_references: readonly string[];
  readonly qualification_links: readonly QualificationLinkV2[];
  readonly qualification_spec: Readonly<{
    references: readonly string[];
    links: readonly QualificationLinkV2[];
  }>;
  readonly known_differences: readonly string[];
  readonly option_cells: readonly CapabilityOptionCellV2[];
  readonly legacy_row: Readonly<Record<string, unknown>>;
}

export interface CapabilityRegistryDocumentV2 {
  readonly registry_schema_version: 2;
  readonly registry_id: "quickpls.capability_registry.v2";
  readonly registry_version: string;
  readonly frozen_on: string;
  readonly catalogue_snapshot: Readonly<Record<string, unknown>>;
  readonly comparison_policy: Readonly<Record<string, unknown>>;
  readonly state_contract: Readonly<Record<string, unknown>>;
  readonly customer_visibility_policy: Readonly<Record<string, unknown>>;
  readonly surface_contract: Readonly<Record<string, unknown>>;
  readonly product_area_contract: Readonly<Record<string, unknown>>;
  readonly qualification_link_contract: Readonly<Record<string, unknown>>;
  readonly exclusions: readonly Readonly<Record<string, unknown>>[];
  readonly legacy_catalogue_contract: Readonly<Record<string, unknown>>;
  readonly capabilities: readonly CapabilityRegistryRowV2[];
}

export interface CapabilityRegistryParseOptionsV2 {
  /**
   * Keeps the shipped 14 August 2026 state distribution immutable. Set false
   * only for tests or a future, explicitly revised registry snapshot; all
   * structural and fail-closed checks still apply.
   */
  readonly requireFrozenStateDistribution?: boolean;
}

export interface QuickPlsCellMatchV2 {
  readonly row: CapabilityRegistryRowV2;
  readonly cell: CapabilityOptionCellV2;
  readonly link: QualificationLinkV2;
}

export interface CapabilityRegistrySummaryV2 {
  readonly row_count: number;
  readonly active_row_count: number;
  readonly coverage: Readonly<Record<CoverageStateV2, number>>;
  readonly surfaces: Readonly<Record<ProductSurfaceV2, number>>;
  readonly option_cell_count: number;
  readonly option_cell_coverage: Readonly<Record<CoverageStateV2, number>>;
  readonly option_cell_surfaces: Readonly<Record<ProductSurfaceV2, number>>;
}

export interface ProductCapabilityProjectionV2 {
  readonly id: string;
  readonly catalogue_position: number;
  readonly family: string;
  readonly method: string;
  readonly lifecycle: OfficialLifecycleV2;
  readonly reference: string;
  readonly product_areas: readonly ProductAreaV2[];
  readonly channel: ProductSurfaceV2;
  readonly availability: Readonly<{
    visibility: CapabilityAvailabilityV2["visibility"];
    selectable: boolean;
    label: CapabilityAvailabilityV2["customer_label"];
  }>;
}

function fail(path: string, message: string): never {
  throw new Error(`Capability Registry V2 ${path}: ${message}`);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function recordAt(value: unknown, path: string): Record<string, unknown> {
  if (!isRecord(value)) fail(path, "must be an object");
  return value;
}

function arrayAt(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) fail(path, "must be an array");
  return value;
}

function stringAt(value: unknown, path: string): string {
  if (typeof value !== "string" || value.trim().length === 0) fail(path, "must be a non-empty string");
  return value;
}

function integerAt(value: unknown, path: string): number {
  if (!Number.isInteger(value)) fail(path, "must be an integer");
  return value as number;
}

function assertExactKeys(value: Record<string, unknown>, expected: readonly string[], path: string): void {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    fail(path, `must contain exactly [${wanted.join(", ")}]; received [${actual.join(", ")}]`);
  }
}

function assertExactArray(value: unknown, expected: readonly unknown[], path: string): void {
  const actual = arrayAt(value, path);
  if (actual.length !== expected.length || actual.some((item, index) => item !== expected[index])) {
    fail(path, `must equal [${expected.join(", ")}]`);
  }
}

function enumAt<T extends string>(value: unknown, allowed: readonly T[], path: string): T {
  if (typeof value !== "string" || !allowed.includes(value as T)) {
    fail(path, `must be one of [${allowed.join(", ")}]`);
  }
  return value as T;
}

function stringArrayAt(value: unknown, path: string): string[] {
  const items = arrayAt(value, path).map((item, index) => stringAt(item, `${path}[${index}]`));
  if (new Set(items).size !== items.length) fail(path, "must not contain duplicate values");
  return items;
}

function validateReferenceSchema(value: unknown, path: string): void {
  const record = recordAt(value, path);
  assertExactKeys(record, ["references"], path);
  if (stringArrayAt(record.references, `${path}.references`).length === 0) fail(`${path}.references`, "must not be empty");
}

function validatePredicates(value: unknown, path: string): void {
  const record = recordAt(value, path);
  assertExactKeys(record, ["official", "quickpls"], path);
  if (stringArrayAt(record.official, `${path}.official`).length === 0) fail(`${path}.official`, "must not be empty");
  if (stringArrayAt(record.quickpls, `${path}.quickpls`).length === 0) fail(`${path}.quickpls`, "must not be empty");
}

function qualificationLinkAt(value: unknown, path: string, ownerCapabilityId: string): QualificationLinkV2 {
  const link = recordAt(value, path);
  assertExactKeys(link, QUALIFICATION_LINK_KEYS, path);
  if (link.registry_schema_version !== 2) fail(`${path}.registry_schema_version`, "must equal 2");
  const capabilityId = stringAt(link.capability_id, `${path}.capability_id`);
  const cellId = stringAt(link.cell_id, `${path}.cell_id`);
  const capabilityVersion = stringAt(link.capability_version, `${path}.capability_version`);
  if (capabilityId !== ownerCapabilityId) fail(`${path}.capability_id`, `must equal owner ${ownerCapabilityId}`);
  if (!/^smartpls\.[a-z0-9_]+$/.test(capabilityId)) fail(`${path}.capability_id`, "has an invalid identity format");
  if (!/^qpls3\.[a-z0-9_.]+$/.test(cellId)) fail(`${path}.cell_id`, "has an invalid identity format");
  if (!/^[a-z0-9][a-z0-9_.-]*$/.test(capabilityVersion)) fail(`${path}.capability_version`, "has an invalid identity format");
  return {
    registry_schema_version: 2,
    capability_id: capabilityId,
    cell_id: cellId,
    capability_version: capabilityVersion,
  };
}

function linkIdentity(link: QualificationLinkV2): string {
  return `${link.registry_schema_version}::${link.capability_id}::${link.cell_id}::${link.capability_version}`;
}

function sameStringArray(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function optionCellAt(
  value: unknown,
  path: string,
  ownerCapabilityId: string,
  ownerLifecycle: OfficialLifecycleV2,
): CapabilityOptionCellV2 {
  const cell = recordAt(value, path);
  assertExactKeys(cell, OPTION_CELL_KEYS, path);
  const capabilityId = stringAt(cell.capability_id, `${path}.capability_id`);
  const cellId = stringAt(cell.cell_id, `${path}.cell_id`);
  const capabilityVersion = stringAt(cell.capability_version, `${path}.capability_version`);
  if (capabilityId !== ownerCapabilityId) fail(`${path}.capability_id`, `must equal owner ${ownerCapabilityId}`);
  if (!/^qpls3\.[a-z0-9_.]+$/.test(cellId)) fail(`${path}.cell_id`, "has an invalid identity format");
  if (!/^[a-z0-9][a-z0-9_.-]*$/.test(capabilityVersion)) fail(`${path}.capability_version`, "has an invalid identity format");
  const coverage = enumAt(cell.coverage_state, COVERAGE_STATES, `${path}.coverage_state`);
  const evidence = enumAt(cell.evidence_state, EVIDENCE_STATES, `${path}.evidence_state`);
  const surface = enumAt(cell.surface, PRODUCT_SURFACES, `${path}.surface`);
  validatePredicates(cell.supported_model_predicate, `${path}.supported_model_predicate`);
  validatePredicates(cell.supported_data_predicate, `${path}.supported_data_predicate`);
  validateReferenceSchema(cell.settings_schema, `${path}.settings_schema`);
  validateReferenceSchema(cell.result_schema, `${path}.result_schema`);
  const documentationReference = stringAt(cell.documentation_reference, `${path}.documentation_reference`);
  if (!documentationReference.startsWith("https://smartpls.com/")) {
    fail(`${path}.documentation_reference`, "must be an official SmartPLS URL");
  }
  if (stringArrayAt(cell.known_differences, `${path}.known_differences`).length === 0) {
    fail(`${path}.known_differences`, "must not be empty");
  }

  const specification = recordAt(cell.qualification_spec, `${path}.qualification_spec`);
  assertExactKeys(specification, ["references", "links"], `${path}.qualification_spec`);
  if (stringArrayAt(specification.references, `${path}.qualification_spec.references`).length === 0) {
    fail(`${path}.qualification_spec.references`, "must not be empty");
  }
  const links = arrayAt(specification.links, `${path}.qualification_spec.links`).map((link, index) =>
    qualificationLinkAt(link, `${path}.qualification_spec.links[${index}]`, ownerCapabilityId),
  );
  if (links.length !== 1) fail(`${path}.qualification_spec.links`, "must contain exactly one link");
  const link = links[0]!;
  if (link.cell_id !== cellId || link.capability_version !== capabilityVersion) {
    fail(`${path}.qualification_spec.links[0]`, "must exactly mirror the option-cell identity");
  }
  if (surface === "standard" && ((coverage !== "full" && coverage !== "partial") || evidence !== "release_qualified")) {
    fail(path, "Standard requires documented coverage and release-qualified evidence"); // customer-copy-lint: allow-internal
  }
  if (coverage === "absent" && evidence !== "absent") {
    fail(path, "absent coverage requires absent evidence");
  }
  if (
    coverage === "intentionally_excluded"
    && (evidence !== "absent" || surface !== "legacy" || ownerLifecycle !== "legacy")
  ) {
    fail(path, "intentional exclusions require an evidence-absent Legacy cell and owner");
  }
  if (ownerLifecycle === "legacy" && coverage !== "intentionally_excluded") {
    fail(path, "Legacy catalogue rows must remain intentionally excluded");
  }
  if (ownerLifecycle === "active" && coverage === "intentionally_excluded") {
    fail(path, "active catalogue rows cannot contain intentional exclusions");
  }
  return cell as unknown as CapabilityOptionCellV2;
}

function deriveRowProjectionV2(
  cells: readonly CapabilityOptionCellV2[],
  path: string,
): Pick<CapabilitySurfaceCellV2, "coverage_state" | "evidence_state" | "surface"> {
  if (cells.length === 0) fail(path, "must contain at least one authoritative option cell");
  const coverage = cells.map((cell) => cell.coverage_state);
  let coverageState: CoverageStateV2;
  if (coverage.every((state) => state === "intentionally_excluded")) coverageState = "intentionally_excluded";
  else if (coverage.includes("intentionally_excluded")) fail(path, "cannot mix active and intentionally excluded cells");
  else if (coverage.includes("absent")) coverageState = "absent";
  else if (coverage.includes("partial")) coverageState = "partial";
  else coverageState = "full";

  const evidenceState = cells.reduce<EvidenceStateV2>((least, cell) => (
    EVIDENCE_STATES.indexOf(cell.evidence_state) < EVIDENCE_STATES.indexOf(least)
      ? cell.evidence_state
      : least
  ), cells[0]!.evidence_state);
  const surfaces = cells.map((cell) => cell.surface);
  let surface: ProductSurfaceV2;
  if (surfaces.every((value) => value === "legacy")) surface = "legacy";
  else if (surfaces.every((value) => value === "internal")) surface = "internal";
  else if (surfaces.every((value) => value === "standard")) surface = "standard";
  else surface = "labs";
  return { coverage_state: coverageState, evidence_state: evidenceState, surface };
}

function capabilityRowAt(value: unknown, path: string): CapabilityRegistryRowV2 {
  const row = recordAt(value, path);
  assertExactKeys(row, CAPABILITY_KEYS, path);
  const capabilityId = stringAt(row.capability_id, `${path}.capability_id`);
  if (!/^smartpls\.[a-z0-9_]+$/.test(capabilityId)) fail(`${path}.capability_id`, "has an invalid identity format");
  const cataloguePosition = integerAt(row.catalogue_position, `${path}.catalogue_position`);
  if (cataloguePosition < 1 || cataloguePosition > 45) fail(`${path}.catalogue_position`, "must be between 1 and 45");
  const lifecycle = enumAt(row.official_lifecycle, OFFICIAL_LIFECYCLES, `${path}.official_lifecycle`);
  const officialUrl = stringAt(row.official_url, `${path}.official_url`);
  if (!officialUrl.startsWith("https://smartpls.com/")) fail(`${path}.official_url`, "must be an official SmartPLS URL");
  const coverage = enumAt(row.coverage_state, COVERAGE_STATES, `${path}.coverage_state`);
  const evidence = enumAt(row.evidence_state, EVIDENCE_STATES, `${path}.evidence_state`);
  const surface = enumAt(row.surface, PRODUCT_SURFACES, `${path}.surface`);
  const productAreas = arrayAt(row.product_areas, `${path}.product_areas`).map((area, index) =>
    enumAt(area, PRODUCT_AREAS, `${path}.product_areas[${index}]`),
  );
  if (productAreas.length === 0 || new Set(productAreas).size !== productAreas.length) {
    fail(`${path}.product_areas`, "must be non-empty and unique");
  }

  validatePredicates(row.model_predicates, `${path}.model_predicates`);
  validatePredicates(row.data_predicates, `${path}.data_predicates`);
  validatePredicates(row.supported_model_predicate, `${path}.supported_model_predicate`);
  validatePredicates(row.supported_data_predicate, `${path}.supported_data_predicate`);
  const settingsReferences = stringArrayAt(row.settings_references, `${path}.settings_references`);
  const resultReferences = stringArrayAt(row.result_references, `${path}.result_references`);
  const qualificationReferences = stringArrayAt(row.qualification_references, `${path}.qualification_references`);
  const knownDifferences = stringArrayAt(row.known_differences, `${path}.known_differences`);
  if ([settingsReferences, resultReferences, qualificationReferences, knownDifferences].some((items) => items.length === 0)) {
    fail(path, "reference and known-difference lists must not be empty");
  }
  validateReferenceSchema(row.settings_schema, `${path}.settings_schema`);
  validateReferenceSchema(row.result_schema, `${path}.result_schema`);

  const qualificationLinks = arrayAt(row.qualification_links, `${path}.qualification_links`).map((link, index) =>
    qualificationLinkAt(link, `${path}.qualification_links[${index}]`, capabilityId),
  );
  if (qualificationLinks.length === 0) fail(`${path}.qualification_links`, "must not be empty");
  const localLinkIds = qualificationLinks.map(linkIdentity);
  if (new Set(localLinkIds).size !== localLinkIds.length) fail(`${path}.qualification_links`, "contains a duplicate identity");

  const qualificationSpec = recordAt(row.qualification_spec, `${path}.qualification_spec`);
  assertExactKeys(qualificationSpec, ["references", "links"], `${path}.qualification_spec`);
  const specReferences = stringArrayAt(qualificationSpec.references, `${path}.qualification_spec.references`);
  const specLinks = arrayAt(qualificationSpec.links, `${path}.qualification_spec.links`).map((link, index) =>
    qualificationLinkAt(link, `${path}.qualification_spec.links[${index}]`, capabilityId),
  );
  if (!sameStringArray(qualificationReferences, specReferences)) {
    fail(`${path}.qualification_spec.references`, "must exactly mirror qualification_references");
  }
  if (!sameStringArray(localLinkIds, specLinks.map(linkIdentity))) {
    fail(`${path}.qualification_spec.links`, "must exactly mirror qualification_links");
  }

  const optionCells = arrayAt(row.option_cells, `${path}.option_cells`).map((cell, index) =>
    optionCellAt(cell, `${path}.option_cells[${index}]`, capabilityId, lifecycle),
  );
  if (optionCells.length === 0) fail(`${path}.option_cells`, "must not be empty");
  const optionIdentities = optionCells.map((cell) => `${cell.capability_id}::${cell.cell_id}::${cell.capability_version}`);
  if (new Set(optionIdentities).size !== optionIdentities.length) {
    fail(`${path}.option_cells`, "contains a duplicate authoritative identity");
  }
  const optionLinkIds = optionCells.map((cell) => linkIdentity(cell.qualification_spec.links[0]));
  if (!sameStringArray(localLinkIds, optionLinkIds)) {
    fail(`${path}.qualification_links`, "must exactly mirror authoritative option-cell links");
  }
  const optionReferences = optionCells.flatMap((cell) => cell.qualification_spec.references)
    .filter((reference, index, values) => values.indexOf(reference) === index);
  if (!sameStringArray(qualificationReferences, optionReferences)) {
    fail(`${path}.qualification_references`, "must exactly mirror authoritative option-cell references");
  }
  for (const cell of optionCells) {
    if (cell.documentation_reference !== officialUrl) {
      fail(`${path}.option_cells`, `${cell.cell_id} documentation_reference must equal official_url`);
    }
    if (!cell.settings_schema.references.includes(officialUrl)) {
      fail(`${path}.option_cells`, `${cell.cell_id} settings_schema must include official_url`);
    }
    if (!cell.result_schema.references.includes(officialUrl)) {
      fail(`${path}.option_cells`, `${cell.cell_id} result_schema must include official_url`);
    }
  }
  const projection = deriveRowProjectionV2(optionCells, `${path}.option_cells`);
  if (coverage !== projection.coverage_state || evidence !== projection.evidence_state || surface !== projection.surface) {
    fail(path, "row coverage_state, evidence_state, and surface must equal the derived option-cell projection");
  }

  const legacyRow = recordAt(row.legacy_row, `${path}.legacy_row`);
  const requiredLegacyFields = [
    "id",
    "catalogue_position",
    "official_family",
    "official_method",
    "status",
    "priority",
    "target_release",
    "competitor_scope",
    "quickpls_scope",
    "quickpls_capability_ids",
    "implementation_evidence",
    "dependencies",
    "remaining_gap",
  ];
  for (const field of requiredLegacyFields) {
    if (!(field in legacyRow)) fail(`${path}.legacy_row`, `is missing required field ${field}`);
  }
  if (legacyRow.id !== capabilityId || legacyRow.catalogue_position !== cataloguePosition) {
    fail(`${path}.legacy_row`, "identity must mirror the registry row");
  }

  if (surface === "standard" && ((coverage !== "full" && coverage !== "partial") || evidence !== "release_qualified")) {
    fail(path, "Standard requires documented coverage and release-qualified evidence"); // customer-copy-lint: allow-internal
  }
  if (coverage === "absent" && (lifecycle !== "active" || evidence !== "absent" || surface !== "labs")) {
    fail(path, "absent coverage must remain an unavailable active Labs commitment");
  }
  if (
    coverage === "intentionally_excluded" &&
    (lifecycle !== "legacy" || evidence !== "absent" || surface !== "legacy")
  ) {
    fail(path, "intentional exclusions must be evidence-absent Legacy rows");
  }

  return row as unknown as CapabilityRegistryRowV2;
}

function countBy<T extends string>(values: readonly T[], keys: readonly T[]): Record<T, number> {
  const result = Object.fromEntries(keys.map((key) => [key, 0])) as Record<T, number>;
  for (const value of values) result[value] += 1;
  return result;
}

function declaredCount(record: Record<string, unknown>, key: string, path: string): number {
  const value = integerAt(record[key], `${path}.${key}`);
  if (value < 0) fail(`${path}.${key}`, "must be non-negative");
  return value;
}

function assertCounts<T extends string>(
  declared: Record<string, unknown>,
  actual: Readonly<Record<T, number>>,
  keys: readonly T[],
  path: string,
): void {
  assertExactKeys(declared, keys, path);
  for (const key of keys) {
    if (declaredCount(declared, key, path) !== actual[key]) fail(`${path}.${key}`, `must equal actual count ${actual[key]}`);
  }
}

function cloneAndFreeze<T>(value: T): T {
  const clone = JSON.parse(JSON.stringify(value)) as T;
  const freeze = (item: unknown): void => {
    if (typeof item !== "object" || item === null || Object.isFrozen(item)) return;
    for (const child of Object.values(item)) freeze(child);
    Object.freeze(item);
  };
  freeze(clone);
  return clone;
}

export function parseCapabilityRegistryV2(
  input: unknown,
  options: CapabilityRegistryParseOptionsV2 = {},
): CapabilityRegistryDocumentV2 {
  const root = recordAt(input, "root");
  assertExactKeys(root, TOP_LEVEL_KEYS, "root");
  if (root.registry_schema_version !== 2) fail("root.registry_schema_version", "must equal 2");
  if (root.registry_id !== "quickpls.capability_registry.v2") {
    fail("root.registry_id", "must equal quickpls.capability_registry.v2");
  }
  if (!/^2\.\d+\.\d+$/.test(stringAt(root.registry_version, "root.registry_version"))) {
    fail("root.registry_version", "must be a semantic V2 version");
  }
  stringAt(root.frozen_on, "root.frozen_on");

  const snapshot = recordAt(root.catalogue_snapshot, "root.catalogue_snapshot");
  if (snapshot.capability_row_count !== 45 || snapshot.active_row_count !== 43) {
    fail("root.catalogue_snapshot", "must declare the frozen 45-row / 43-active baseline");
  }
  recordAt(root.comparison_policy, "root.comparison_policy");

  const stateContract = recordAt(root.state_contract, "root.state_contract");
  assertExactArray(stateContract.coverage_states, COVERAGE_STATES, "root.state_contract.coverage_states");
  assertExactArray(stateContract.evidence_states, EVIDENCE_STATES, "root.state_contract.evidence_states");
  if (stateContract.active_row_count !== 43) fail("root.state_contract.active_row_count", "must equal 43");
  stringAt(stateContract.active_definition, "root.state_contract.active_definition");
  stringAt(stateContract.separation_rule, "root.state_contract.separation_rule");

  const visibilityPolicy = recordAt(root.customer_visibility_policy, "root.customer_visibility_policy");
  assertExactKeys(visibilityPolicy, [
    "policy_version",
    "standard_match",
    "scoped_standard_match",
    "otherwise_channel",
    "labs_requires_opt_in",
    "evidence_labels",
    "availability_rule",
    "non_customer_surfaces",
  ], "root.customer_visibility_policy");
  if (visibilityPolicy.policy_version !== 2) {
    fail("root.customer_visibility_policy.policy_version", "must equal 2");
  }
  const standardMatch = recordAt(visibilityPolicy.standard_match, "root.customer_visibility_policy.standard_match");
  assertExactKeys(standardMatch, ["surface", "coverage_state", "evidence_state"], "root.customer_visibility_policy.standard_match");
  if (
    standardMatch.surface !== "standard" ||
    standardMatch.coverage_state !== "full" ||
    standardMatch.evidence_state !== "release_qualified"
  ) {
    fail("root.customer_visibility_policy.standard_match", "must preserve the full + release-qualified Standard gate"); // customer-copy-lint: allow-internal
  }
  const scopedStandardMatch = recordAt(
    visibilityPolicy.scoped_standard_match,
    "root.customer_visibility_policy.scoped_standard_match",
  );
  assertExactKeys(
    scopedStandardMatch,
    ["surface", "coverage_state", "evidence_state", "scope_statement_required"],
    "root.customer_visibility_policy.scoped_standard_match",
  );
  if (
    scopedStandardMatch.surface !== "standard" ||
    scopedStandardMatch.coverage_state !== "partial" ||
    scopedStandardMatch.evidence_state !== "release_qualified" ||
    scopedStandardMatch.scope_statement_required !== true
  ) {
    fail(
      "root.customer_visibility_policy.scoped_standard_match",
      "must preserve the documented-scope + release-qualified Standard gate",
    ); // customer-copy-lint: allow-internal
  }
  if (visibilityPolicy.labs_requires_opt_in !== true || visibilityPolicy.evidence_labels !== "internal_only") {
    fail("root.customer_visibility_policy", "must keep Labs opt-in and evidence labels internal");
  }

  const surfaceContract = recordAt(root.surface_contract, "root.surface_contract");
  assertExactArray(surfaceContract.allowed, PRODUCT_SURFACES, "root.surface_contract.allowed");
  const areaContract = recordAt(root.product_area_contract, "root.product_area_contract");
  assertExactArray(areaContract.allowed, PRODUCT_AREAS, "root.product_area_contract.allowed");

  const linkContract = recordAt(root.qualification_link_contract, "root.qualification_link_contract");
  assertExactArray(linkContract.required_shape, QUALIFICATION_LINK_KEYS, "root.qualification_link_contract.required_shape");
  if (linkContract.registry_schema_version !== 2) fail("root.qualification_link_contract.registry_schema_version", "must equal 2");

  const capabilities = arrayAt(root.capabilities, "root.capabilities").map((row, index) =>
    capabilityRowAt(row, `root.capabilities[${index}]`),
  );
  if (capabilities.length !== 45) fail("root.capabilities", "must contain exactly 45 rows");
  const activeCount = capabilities.filter((row) => row.official_lifecycle === "active").length;
  if (activeCount !== 43) fail("root.capabilities", `must contain exactly 43 active rows; received ${activeCount}`);

  const capabilityIds = capabilities.map((row) => row.capability_id);
  if (new Set(capabilityIds).size !== capabilityIds.length) fail("root.capabilities", "contains a duplicate capability_id");
  const positions = capabilities.map((row) => row.catalogue_position);
  if (new Set(positions).size !== positions.length) fail("root.capabilities", "contains a duplicate catalogue_position");
  const expectedPositions = Array.from({ length: 45 }, (_, index) => index + 1);
  if (!positions.every((position, index) => position === expectedPositions[index])) {
    fail("root.capabilities", "must remain ordered at the frozen catalogue positions 1 through 45");
  }
  const allLinkIds = capabilities.flatMap((row) => row.option_cells.map((cell) => linkIdentity(cell.qualification_spec.links[0])));
  if (new Set(allLinkIds).size !== allLinkIds.length) fail("root.capabilities", "contains a duplicate qualification-link identity");

  const coverageCounts = countBy(
    capabilities.map((row) => row.coverage_state),
    COVERAGE_STATES,
  );
  const surfaceCounts = countBy(
    capabilities.map((row) => row.surface),
    PRODUCT_SURFACES,
  );
  assertCounts(recordAt(stateContract.baseline_counts, "root.state_contract.baseline_counts"), coverageCounts, COVERAGE_STATES, "root.state_contract.baseline_counts");
  assertCounts(recordAt(surfaceContract.baseline_counts, "root.surface_contract.baseline_counts"), surfaceCounts, PRODUCT_SURFACES, "root.surface_contract.baseline_counts");

  if (options.requireFrozenStateDistribution !== false) {
    for (const state of COVERAGE_STATES) {
      if (coverageCounts[state] !== FROZEN_COVERAGE_COUNTS[state]) {
        fail(`root.state_contract.baseline_counts.${state}`, `must equal frozen count ${FROZEN_COVERAGE_COUNTS[state]}`);
      }
    }
  }

  const exclusions = arrayAt(root.exclusions, "root.exclusions");
  if (exclusions.length !== 2) fail("root.exclusions", "must contain exactly Blindfolding and GoF");
  const exclusionIds = exclusions.map((exclusion, index) => {
    const item = recordAt(exclusion, `root.exclusions[${index}]`);
    if (item.decision !== "intentionally_excluded") fail(`root.exclusions[${index}].decision`, "must be intentionally_excluded");
    return stringAt(item.capability_id, `root.exclusions[${index}].capability_id`);
  });
  const expectedExclusions = ["smartpls.blindfolding", "smartpls.gof"];
  if (!sameStringArray(exclusionIds, expectedExclusions)) fail("root.exclusions", "must be ordered as Blindfolding and GoF");
  for (const excludedId of expectedExclusions) {
    const excludedRow = capabilities.find((row) => row.capability_id === excludedId);
    if (!excludedRow || excludedRow.coverage_state !== "intentionally_excluded" || excludedRow.surface !== "legacy") {
      fail("root.exclusions", `${excludedId} must resolve to an intentionally excluded Legacy row`);
    }
  }

  recordAt(root.legacy_catalogue_contract, "root.legacy_catalogue_contract");
  return cloneAndFreeze(root) as unknown as CapabilityRegistryDocumentV2;
}

function optionSurfaceCell(cell: CapabilityOptionCellV2): CapabilitySurfaceCellV2 {
  return {
    capability_id: cell.capability_id,
    cell_id: cell.cell_id,
    capability_version: cell.capability_version,
    coverage_state: cell.coverage_state,
    evidence_state: cell.evidence_state,
    surface: cell.surface,
  };
}

export function capabilityOptionCellAvailabilityV2(
  cell: CapabilityOptionCellV2,
  experimentalLabsEnabled: boolean,
): CapabilityAvailabilityV2 {
  return capabilityAvailabilityV2(optionSurfaceCell(cell), experimentalLabsEnabled);
}

/** Compatibility row projection for catalogue/reporting views, never option resolution. */
export function capabilityRowAvailabilityV2(
  row: CapabilityRegistryRowV2,
  experimentalLabsEnabled: boolean,
): CapabilityAvailabilityV2 {
  const projection = deriveRowProjectionV2(row.option_cells, `row ${row.capability_id}`);
  const first = row.option_cells[0];
  if (!first) fail(`row ${row.capability_id}`, "has no option cell");
  return capabilityAvailabilityV2({
    capability_id: row.capability_id,
    cell_id: `${first.cell_id}.row_projection`,
    capability_version: first.capability_version,
    ...projection,
  }, experimentalLabsEnabled);
}

export class CapabilityRegistryV2Adapter {
  readonly document: CapabilityRegistryDocumentV2;
  readonly summary: CapabilityRegistrySummaryV2;
  readonly #byCapabilityId: ReadonlyMap<string, CapabilityRegistryRowV2>;
  readonly #byQuickPlsCellId: ReadonlyMap<string, readonly QuickPlsCellMatchV2[]>;

  constructor(input: unknown, options: CapabilityRegistryParseOptionsV2 = {}) {
    this.document = parseCapabilityRegistryV2(input, options);
    this.#byCapabilityId = new Map(this.document.capabilities.map((row) => [row.capability_id, row]));
    const byCellId = new Map<string, QuickPlsCellMatchV2[]>();
    for (const row of this.document.capabilities) {
      for (const cell of row.option_cells) {
        const link = cell.qualification_spec.links[0];
        const matches = byCellId.get(cell.cell_id) ?? [];
        matches.push(Object.freeze({ row, cell, link }));
        byCellId.set(cell.cell_id, matches);
      }
    }
    this.#byQuickPlsCellId = new Map(
      [...byCellId.entries()].map(([cellId, matches]) => [cellId, Object.freeze([...matches])]),
    );
    this.summary = Object.freeze({
      row_count: this.document.capabilities.length,
      active_row_count: this.document.capabilities.filter((row) => row.official_lifecycle === "active").length,
      coverage: Object.freeze(
        countBy(
          this.document.capabilities.map((row) => row.coverage_state),
          COVERAGE_STATES,
        ),
      ),
      surfaces: Object.freeze(
        countBy(
          this.document.capabilities.map((row) => row.surface),
          PRODUCT_SURFACES,
        ),
      ),
      option_cell_count: this.document.capabilities.reduce((total, row) => total + row.option_cells.length, 0),
      option_cell_coverage: Object.freeze(
        countBy(this.document.capabilities.flatMap((row) => row.option_cells.map((cell) => cell.coverage_state)), COVERAGE_STATES),
      ),
      option_cell_surfaces: Object.freeze(
        countBy(this.document.capabilities.flatMap((row) => row.option_cells.map((cell) => cell.surface)), PRODUCT_SURFACES),
      ),
    });
  }

  capability(capabilityId: string): CapabilityRegistryRowV2 | undefined {
    return this.#byCapabilityId.get(capabilityId);
  }

  requireCapability(capabilityId: string): CapabilityRegistryRowV2 {
    const row = this.capability(capabilityId);
    if (!row) fail("lookup", `unknown capability_id ${capabilityId}`);
    return row;
  }

  quickPlsCell(cellId: string): readonly QuickPlsCellMatchV2[] {
    return this.#byQuickPlsCellId.get(cellId) ?? Object.freeze([]);
  }

  requireOptionCell(capabilityId: string, cellId: string): CapabilityOptionCellV2 {
    const match = this.quickPlsCell(cellId).find((candidate) => candidate.row.capability_id === capabilityId);
    if (!match) fail("lookup", `unknown option cell ${capabilityId}::${cellId}`);
    return match.cell;
  }

  availability(capabilityId: string, cellId: string, experimentalLabsEnabled: boolean): CapabilityAvailabilityV2 {
    return capabilityOptionCellAvailabilityV2(
      this.requireOptionCell(capabilityId, cellId),
      experimentalLabsEnabled,
    );
  }

  rowAvailability(capabilityId: string, experimentalLabsEnabled: boolean): CapabilityAvailabilityV2 {
    return capabilityRowAvailabilityV2(this.requireCapability(capabilityId), experimentalLabsEnabled);
  }

  productProjection(
    capabilityId: string,
    experimentalLabsEnabled: boolean,
  ): ProductCapabilityProjectionV2 {
    const row = this.requireCapability(capabilityId);
    const availability = capabilityRowAvailabilityV2(row, experimentalLabsEnabled);
    return Object.freeze({
      id: row.capability_id,
      catalogue_position: row.catalogue_position,
      family: row.official_family,
      method: row.official_method,
      lifecycle: row.official_lifecycle,
      reference: row.documentation_reference,
      product_areas: Object.freeze([...row.product_areas]),
      channel: row.surface,
      availability: Object.freeze({
        visibility: availability.visibility,
        selectable: availability.selectable,
        label: availability.customer_label,
      }),
    });
  }

  visibleProductCapabilities(experimentalLabsEnabled: boolean): readonly ProductCapabilityProjectionV2[] {
    return Object.freeze(
      this.document.capabilities
        .map((row) => this.productProjection(row.capability_id, experimentalLabsEnabled))
        .filter((capability) => capability.availability.visibility !== "hidden"),
    );
  }
}

/** The shipped application registry is parsed once, strictly, from the frozen JSON source of truth. */
export const capabilityRegistryV2 = new CapabilityRegistryV2Adapter(rawCapabilityRegistryV2);
