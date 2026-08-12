export type NativeDataKind = "raw" | "covariance" | "correlation";

export interface NativeDataImportDraft {
  dataKind: NativeDataKind;
  sampleSize: string;
  missingMarkers: string;
}

export interface NativeDataImportRequest {
  dataKind: NativeDataKind;
  sampleSize?: number;
  missingMarkers: string[];
}

export interface NativeDataImportValidation {
  request: NativeDataImportRequest | null;
  error: string | null;
}

export const DEFAULT_NATIVE_DATA_IMPORT_DRAFT: Readonly<NativeDataImportDraft> = {
  dataKind: "raw",
  sampleSize: "",
  missingMarkers: "NA, N/A, ., -99",
};

export const DEFAULT_NATIVE_DATA_IMPORT_REQUEST: Readonly<NativeDataImportRequest> = {
  dataKind: "raw",
  missingMarkers: ["", "NA", "N/A", ".", "-99"],
};

export function parseNativeMissingMarkers(value: string): string[] {
  const markers = value
    .split(/[\r\n,]+/)
    .map((marker) => marker.trim())
    .filter(Boolean);
  return ["", ...new Set(markers)];
}

export function validateNativeDataImportDraft(draft: Readonly<NativeDataImportDraft>): NativeDataImportValidation {
  const missingMarkers = parseNativeMissingMarkers(draft.missingMarkers);
  if (draft.dataKind === "raw") {
    return { request: { dataKind: "raw", missingMarkers }, error: null };
  }

  const sampleSize = Number(draft.sampleSize);
  if (!Number.isInteger(sampleSize) || sampleSize < 2) {
    return {
      request: null,
      error: "Enter the study sample size (an integer of at least 2) for a covariance or correlation matrix.",
    };
  }
  return { request: { dataKind: draft.dataKind, sampleSize, missingMarkers }, error: null };
}

export function normalizeNativeDataImportRequest(value: unknown): NativeDataImportRequest {
  if (!value || typeof value !== "object") return { ...DEFAULT_NATIVE_DATA_IMPORT_REQUEST, missingMarkers: [...DEFAULT_NATIVE_DATA_IMPORT_REQUEST.missingMarkers] };
  const candidate = value as Partial<NativeDataImportRequest>;
  const dataKind: NativeDataKind = candidate.dataKind === "covariance" || candidate.dataKind === "correlation" ? candidate.dataKind : "raw";
  const markers = Array.isArray(candidate.missingMarkers)
    ? candidate.missingMarkers.filter((marker): marker is string => typeof marker === "string")
    : DEFAULT_NATIVE_DATA_IMPORT_REQUEST.missingMarkers;
  const missingMarkers = ["", ...new Set(markers.map((marker) => marker.trim()).filter(Boolean))];
  const sampleSize = Number(candidate.sampleSize);
  return {
    dataKind,
    ...(dataKind !== "raw" && Number.isInteger(sampleSize) && sampleSize >= 2 ? { sampleSize } : {}),
    missingMarkers,
  };
}
