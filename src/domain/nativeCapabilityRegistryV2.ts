import {
  capabilityRegistryV2,
  parseCapabilityRegistryV2,
  type CapabilityRegistryDocumentV2,
} from "./capabilityRegistryV2";

const RESPONSE_KEYS = [
  "response_schema_version",
  "registry_schema_version",
  "registry_id",
  "registry_version",
  "source_sha256",
  "capability_row_count",
  "active_row_count",
  "option_cell_count",
  "source_json",
] as const;

const SHA256_HEX = /^[a-f0-9]{64}$/;

export interface NativeCapabilityRegistryV2Wire {
  readonly response_schema_version: 1;
  readonly registry_schema_version: 2;
  readonly registry_id: "quickpls.capability_registry.v2";
  readonly registry_version: string;
  readonly source_sha256: string;
  readonly capability_row_count: number;
  readonly active_row_count: number;
  readonly option_cell_count: number;
  readonly source_json: string;
}

export interface NativeCapabilityRegistryV2Snapshot extends NativeCapabilityRegistryV2Wire {
  readonly registry: CapabilityRegistryDocumentV2;
}

function fail(path: string, message: string): never {
  throw new Error(`Native Capability Registry V2 ${path}: ${message}`);
}

function recordAt(value: unknown, path: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail(path, "must be an object");
  }
  return value as Record<string, unknown>;
}

function requireExactKeys(
  value: Record<string, unknown>,
  expected: readonly string[],
  path: string,
): void {
  const actual = Object.keys(value).sort();
  const required = [...expected].sort();
  if (actual.length !== required.length || actual.some((key, index) => key !== required[index])) {
    fail(path, `must contain exactly ${required.join(", ")}`);
  }
}

function stringAt(value: unknown, path: string): string {
  if (typeof value !== "string" || value.length === 0) fail(path, "must be a nonempty string");
  return value;
}

function countAt(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    fail(path, "must be a nonnegative safe integer");
  }
  return value as number;
}

function stableValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(stableValue);
  if (value !== null && typeof value === "object") {
    const record = value as Record<string, unknown>;
    return Object.fromEntries(
      Object.keys(record).sort().map((key) => [key, stableValue(record[key])]),
    );
  }
  return value;
}

function stableJson(value: unknown): string {
  return JSON.stringify(stableValue(value));
}

async function sha256Hex(value: string): Promise<string> {
  if (!globalThis.crypto?.subtle) fail("root.source_sha256", "cannot be verified on this runtime");
  const digest = await globalThis.crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

/**
 * Parses the native command envelope, verifies its source bytes, and proves
 * that the executable and frontend contain the same registry rows and option
 * cells. It has no effect on method visibility or calculation validation.
 */
export async function parseNativeCapabilityRegistryV2(
  input: unknown,
): Promise<NativeCapabilityRegistryV2Snapshot> {
  const root = recordAt(input, "root");
  requireExactKeys(root, RESPONSE_KEYS, "root");

  if (root.response_schema_version !== 1) fail("root.response_schema_version", "must equal 1");
  if (root.registry_schema_version !== 2) fail("root.registry_schema_version", "must equal 2");
  if (root.registry_id !== "quickpls.capability_registry.v2") {
    fail("root.registry_id", "must equal quickpls.capability_registry.v2");
  }

  const registryVersion = stringAt(root.registry_version, "root.registry_version");
  const sourceSha256 = stringAt(root.source_sha256, "root.source_sha256");
  if (!SHA256_HEX.test(sourceSha256)) fail("root.source_sha256", "must be lowercase SHA-256");
  const capabilityRowCount = countAt(root.capability_row_count, "root.capability_row_count");
  const activeRowCount = countAt(root.active_row_count, "root.active_row_count");
  const optionCellCount = countAt(root.option_cell_count, "root.option_cell_count");
  const sourceJson = stringAt(root.source_json, "root.source_json");

  const observedSha256 = await sha256Hex(sourceJson);
  if (observedSha256 !== sourceSha256) fail("root.source_sha256", "does not match source_json");

  let decoded: unknown;
  try {
    decoded = JSON.parse(sourceJson);
  } catch {
    fail("root.source_json", "must contain valid JSON");
  }
  const registry = parseCapabilityRegistryV2(decoded);
  const actualOptionCellCount = registry.capabilities.reduce(
    (count, row) => count + row.option_cells.length,
    0,
  );
  const actualActiveRowCount = registry.capabilities.filter(
    (row) => row.official_lifecycle === "active",
  ).length;

  if (registry.registry_schema_version !== root.registry_schema_version) {
    fail("root.registry_schema_version", "does not match source_json");
  }
  if (registry.registry_id !== root.registry_id) fail("root.registry_id", "does not match source_json");
  if (registry.registry_version !== registryVersion) {
    fail("root.registry_version", "does not match source_json");
  }
  if (registry.capabilities.length !== capabilityRowCount) {
    fail("root.capability_row_count", "does not match source_json");
  }
  if (actualActiveRowCount !== activeRowCount) fail("root.active_row_count", "does not match source_json");
  if (actualOptionCellCount !== optionCellCount) fail("root.option_cell_count", "does not match source_json");

  const frontendRegistry = capabilityRegistryV2.document;
  if (
    registry.registry_schema_version !== frontendRegistry.registry_schema_version
    || registry.registry_id !== frontendRegistry.registry_id
    || registry.registry_version !== frontendRegistry.registry_version
  ) {
    fail("root", "executable registry identity differs from the frontend registry");
  }
  if (stableJson(registry.capabilities) !== stableJson(frontendRegistry.capabilities)) {
    fail("root.source_json", "executable capability rows or option cells differ from the frontend registry");
  }

  return Object.freeze({
    response_schema_version: 1,
    registry_schema_version: 2,
    registry_id: "quickpls.capability_registry.v2",
    registry_version: registryVersion,
    source_sha256: sourceSha256,
    capability_row_count: capabilityRowCount,
    active_row_count: activeRowCount,
    option_cell_count: optionCellCount,
    source_json: sourceJson,
    registry,
  });
}
