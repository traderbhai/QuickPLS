import bundledSampleCatalogDocument from "../data/bundledSampleProjects.v1.json";

declare const nativeSampleProjectIdBrand: unique symbol;

export type NativeSampleProjectId = string & {
  readonly [nativeSampleProjectIdBrand]: "NativeSampleProjectId";
};

export interface BundledSampleProjectCard {
  readonly id: NativeSampleProjectId;
  readonly label: string;
  readonly detail: string;
}

export interface ParsedBundledSampleCatalog {
  readonly schemaVersion: 1;
  readonly defaultSampleId: NativeSampleProjectId;
  readonly samples: readonly BundledSampleProjectCard[];
}

const SAMPLE_ID_PATTERN = /^[a-z0-9]+(?:_[a-z0-9]+)*$/u;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function requiredTrimmedString(
  value: unknown,
  path: string,
): string {
  if (typeof value !== "string" || !value.trim()) {
    throw new Error(`${path} must be a non-empty string.`);
  }
  return value.trim();
}

function brandedSampleId(value: unknown, path: string): NativeSampleProjectId {
  const id = requiredTrimmedString(value, path);
  if (!SAMPLE_ID_PATTERN.test(id)) {
    throw new Error(`${path} must use lowercase snake_case.`);
  }
  return id as NativeSampleProjectId;
}

export function parseBundledSampleCatalog(value: unknown): ParsedBundledSampleCatalog {
  if (!isRecord(value)) {
    throw new Error("Bundled sample catalog must be an object.");
  }
  if (value.schemaVersion !== 1) {
    throw new Error("Bundled sample catalog schemaVersion must equal 1.");
  }
  if (!Array.isArray(value.datasets)) {
    throw new Error("Bundled sample catalog datasets must be an array.");
  }
  if (!Array.isArray(value.samples) || value.samples.length === 0) {
    throw new Error("Bundled sample catalog samples must be a non-empty array.");
  }

  const seenIds = new Set<string>();
  const samples = value.samples.map((candidate, index): BundledSampleProjectCard => {
    const path = `Bundled sample catalog samples[${index}]`;
    if (!isRecord(candidate)) {
      throw new Error(`${path} must be an object.`);
    }
    const id = brandedSampleId(candidate.id, `${path}.id`);
    if (seenIds.has(id)) {
      throw new Error(`Bundled sample catalog contains duplicate sample id ${JSON.stringify(id)}.`);
    }
    seenIds.add(id);
    return Object.freeze({
      id,
      label: requiredTrimmedString(candidate.label, `${path}.label`),
      detail: requiredTrimmedString(candidate.detail, `${path}.detail`),
    });
  });
  const defaultSampleId = brandedSampleId(
    value.defaultSampleId,
    "Bundled sample catalog defaultSampleId",
  );
  if (!seenIds.has(defaultSampleId)) {
    throw new Error("Bundled sample catalog defaultSampleId must identify a listed sample.");
  }
  return Object.freeze({
    schemaVersion: 1,
    defaultSampleId,
    samples: Object.freeze(samples),
  });
}

const parsedCatalog = parseBundledSampleCatalog(bundledSampleCatalogDocument);
const bundledSampleIds = new Set<string>(parsedCatalog.samples.map((sample) => sample.id));

export const BUNDLED_SAMPLE_PROJECTS = parsedCatalog.samples;
export const DEFAULT_BUNDLED_SAMPLE_PROJECT_ID = parsedCatalog.defaultSampleId;

export function parseNativeSampleProjectId(value: unknown): NativeSampleProjectId | null {
  return typeof value === "string" && bundledSampleIds.has(value)
    ? value as NativeSampleProjectId
    : null;
}
