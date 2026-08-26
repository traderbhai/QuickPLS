import { describe, expect, it } from "vitest";
import type {
  MultiModResultAttachmentV1,
  MultimodResultSidecarDescriptorV1,
} from "../domain/multimodContractsV1";
import {
  NATIVE_MULTIMOD_LABS_ACCESS_V1,
  NATIVE_MULTIMOD_STANDARD_ACCESS_V1,
} from "./nativeMultiModJobV1";
import { buildNativeMultiModRawSidecarExportRequestV1 } from "./nativeMultiModRawSidecarExportV1";

const attachment = {
  result_id: "result-mga-v1",
  identity_sha256: "1".repeat(64),
} as MultiModResultAttachmentV1;

const descriptor = {
  entry_name: "results/result-mga-v1/mga-ledger.arrow",
  sha256: "2".repeat(64),
} as MultimodResultSidecarDescriptorV1;

const authority = {
  archivePath: "D:\\projects\\study.qpls",
  archiveSha256: "3".repeat(64),
  projectId: "00000000-0000-0000-0000-000000000101",
};

describe("native MultiMod raw-sidecar export access", () => {
  it("carries the exact Standard(false) or Labs(true) discriminator", () => {
    const standard = buildNativeMultiModRawSidecarExportRequestV1(
      { ...authority, access: NATIVE_MULTIMOD_STANDARD_ACCESS_V1 },
      attachment,
      descriptor,
      "D:\\exports\\mga-ledger.arrow",
    );
    const labs = buildNativeMultiModRawSidecarExportRequestV1(
      { ...authority, access: NATIVE_MULTIMOD_LABS_ACCESS_V1 },
      attachment,
      descriptor,
      "D:\\exports\\mga-ledger.arrow",
    );

    expect(standard).toMatchObject({
      surface: "standard_multimod_v1",
      experimentalLabsEnabled: false,
    });
    expect(labs).toMatchObject({
      surface: "internal_labs_multimod_v1",
      experimentalLabsEnabled: true,
    });
  });
});
