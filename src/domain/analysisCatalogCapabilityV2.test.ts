import { describe, expect, it } from "vitest";
import { methods } from "../data/sample";
import type { AnalysisUiSettings, MethodDefinition } from "../types";
import {
  analysisCatalogCapabilityCountsV2,
  analysisCatalogCapabilityEntriesV2,
  visibleAnalysisCatalogCapabilityEntriesV2,
} from "./analysisCatalogCapabilityV2";

const settings: AnalysisUiSettings = {
  method: "pls_pm",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 7,
  workers: 1,
  confidenceLevel: 0.95,
  cbsemBootstrapSamples: 0,
  groupMethods: "micom,mga_permutation",
  regressionType: "ols",
};

describe("analysis catalogue Capability Registry V2 projection", () => {
  it("shows the sixteen release-qualified scoped methods in Standard", () => {
    expect(visibleAnalysisCatalogCapabilityEntriesV2(methods, settings, {
      experimentalLabsEnabled: false,
    }).map((entry) => entry.method.id)).toEqual([
      "pls_pm",
      "bootstrap",
      "permutation",
      "pls_sample_size_power",
      "plsc",
      "wpls",
      "cca",
      "cta_pls",
      "predict",
      "mga",
      "ipma",
      "cbsem",
      "pca",
      "gsca",
      "regression",
      "nca",
    ]);
  });

  it("keeps sixteen Standard methods and adds only executable Labs methods when opted in", () => {
    const entries = analysisCatalogCapabilityEntriesV2(methods, settings, {
      experimentalLabsEnabled: true,
    });
    const visible = entries.filter((entry) => entry.availability.selectable);

    expect(visible.map((entry) => entry.method.id)).toEqual([
      "pls_pm",
      "bootstrap",
      "permutation",
      "pls_sample_size_power",
      "plsc",
      "wpls",
      "cca",
      "cta_pls",
      "nonlinear_effects",
      "moderated_mediation",
      "predict",
      "mga",
      "ipma",
      "cbsem",
      "pca",
      "gsca",
      "regression",
      "nca",
    ]);
    expect(entries.find((entry) => entry.method.id === "pls_pm")?.availability).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
    });
    expect(entries.find((entry) => entry.method.id === "wpls")?.availability).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
    });
    expect(entries.find((entry) => entry.method.id === "plsc")?.availability).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
    });
    expect(entries.find((entry) => entry.method.id === "permutation")?.availability).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
    });
    expect(entries.find((entry) => entry.method.id === "mga")?.availability).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
    });
    expect(analysisCatalogCapabilityCountsV2(entries)).toEqual({
      standard: 16,
      experimental: 2,
      hidden: methods.length - 18,
    });
  });

  it("exposes the registered exact-CFA bootstrap add-on as Standard", () => {
    const entries = analysisCatalogCapabilityEntriesV2(methods, {
      ...settings,
      cbsemModelType: "cfa",
      cbsemBootstrapSamples: 1_000,
    }, { experimentalLabsEnabled: false });
    const cbsem = entries.find((entry) => entry.method.id === "cbsem");

    expect(cbsem?.availability).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
      required_capability_ids: ["smartpls.cfa", "smartpls.cbsem_bootstrapping"],
    });
  });

  it("fails closed on duplicate legacy definitions", () => {
    const duplicate = [...methods, methods[0] as MethodDefinition];
    expect(() => analysisCatalogCapabilityEntriesV2(duplicate, settings, {
      experimentalLabsEnabled: true,
    })).toThrow("Duplicate analysis method definition");
  });
});
