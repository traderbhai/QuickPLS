import { describe, expect, it, vi } from "vitest";
import { parseNativeSampleProjectId } from "../domain/bundledSampleCatalog";
import { launchNativeBundledSampleV1 } from "./nativeBundledSampleLaunch";

describe("native bundled sample launch", () => {
  it("routes the strict moderation sample through materialization and schema-6 open only", async () => {
    const sampleId = parseNativeSampleProjectId("organizational_identification_moderation");
    expect(sampleId).not.toBeNull();
    const openOrdinarySample = vi.fn();
    const materializeGeneralSemSample = vi.fn(async () => "D:\\QuickPLS\\samples\\oi-moderation.qpls");
    const openGeneralSemArchive = vi.fn(async () => undefined);

    await expect(launchNativeBundledSampleV1(sampleId!, {
      openOrdinarySample,
      materializeGeneralSemSample,
      openGeneralSemArchive,
    })).resolves.toBe("general_sem_v1");

    expect(materializeGeneralSemSample).toHaveBeenCalledExactlyOnceWith(sampleId);
    expect(openGeneralSemArchive).toHaveBeenCalledExactlyOnceWith("D:\\QuickPLS\\samples\\oi-moderation.qpls");
    expect(openOrdinarySample).not.toHaveBeenCalled();
  });

  it("preserves the ordinary demo-project route for existing samples", async () => {
    const sampleId = parseNativeSampleProjectId("organizational_identification_mediation");
    expect(sampleId).not.toBeNull();
    const openOrdinarySample = vi.fn(async () => undefined);
    const materializeGeneralSemSample = vi.fn();
    const openGeneralSemArchive = vi.fn();

    await expect(launchNativeBundledSampleV1(sampleId!, {
      openOrdinarySample,
      materializeGeneralSemSample,
      openGeneralSemArchive,
    })).resolves.toBe("ordinary_v1");

    expect(openOrdinarySample).toHaveBeenCalledExactlyOnceWith(sampleId);
    expect(materializeGeneralSemSample).not.toHaveBeenCalled();
    expect(openGeneralSemArchive).not.toHaveBeenCalled();
  });
});
