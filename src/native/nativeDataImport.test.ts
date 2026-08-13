import { describe, expect, it } from "vitest";
import {
  normalizeNativeDataImportRequest,
  parseNativeMissingMarkers,
  validateNativeDataImportDraft,
} from "./nativeDataImport";

describe("native data import settings", () => {
  it("normalizes unique missing markers while always recognizing blank cells", () => {
    expect(parseNativeMissingMarkers(" NA, -99\nNA, . ")).toEqual(["", "NA", "-99", "."]);
  });

  it("accepts raw data without a declared sample size", () => {
    expect(validateNativeDataImportDraft({ dataKind: "raw", sampleSize: "not used", missingMarkers: "NA" })).toEqual({
      request: { dataKind: "raw", missingMarkers: ["", "NA"] },
      error: null,
    });
  });

  it("requires a valid study sample size for matrix data", () => {
    expect(validateNativeDataImportDraft({ dataKind: "correlation", sampleSize: "", missingMarkers: "NA" }).error).toContain("sample size");
    expect(validateNativeDataImportDraft({ dataKind: "covariance", sampleSize: "250", missingMarkers: "." })).toEqual({
      request: { dataKind: "covariance", sampleSize: 250, missingMarkers: ["", "."] },
      error: null,
    });
  });

  it("coerces untrusted command details to the supported import contract", () => {
    expect(normalizeNativeDataImportRequest({ dataKind: "other", sampleSize: -1, missingMarkers: ["NA", "NA", 3] })).toEqual({
      dataKind: "raw",
      missingMarkers: ["", "NA"],
    });
  });
});
