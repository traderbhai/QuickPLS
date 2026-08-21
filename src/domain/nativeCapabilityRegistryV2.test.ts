import { describe, expect, it } from "vitest";
import rawCapabilityRegistryV2 from "../../validation/capabilities/capability_registry_v2.json";
import { parseNativeCapabilityRegistryV2 } from "./nativeCapabilityRegistryV2";

async function sha256Hex(value: string): Promise<string> {
  const digest = await globalThis.crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function wire(source: unknown = rawCapabilityRegistryV2) {
  const sourceJson = JSON.stringify(source);
  const capabilities = (source as typeof rawCapabilityRegistryV2).capabilities;
  return {
    response_schema_version: 1,
    registry_schema_version: 2,
    registry_id: "quickpls.capability_registry.v2",
    registry_version: rawCapabilityRegistryV2.registry_version,
    source_sha256: await sha256Hex(sourceJson),
    capability_row_count: capabilities.length,
    active_row_count: capabilities.filter((row) => row.official_lifecycle === "active").length,
    option_cell_count: capabilities.reduce((count, row) => count + row.option_cells.length, 0),
    source_json: sourceJson,
  };
}

describe("native Capability Registry V2 bridge", () => {
  it("verifies the source digest and exact frontend rows and option cells", async () => {
    const parsed = await parseNativeCapabilityRegistryV2(await wire());

    expect(parsed).toMatchObject({
      response_schema_version: 1,
      registry_schema_version: 2,
      registry_id: "quickpls.capability_registry.v2",
      registry_version: rawCapabilityRegistryV2.registry_version,
      capability_row_count: 45,
      active_row_count: 43,
      option_cell_count: 59,
    });
    expect(parsed.registry.capabilities).toHaveLength(45);
    expect(Object.isFrozen(parsed)).toBe(true);
    expect(Object.isFrozen(parsed.registry)).toBe(true);
  });

  it("fails closed on digest, count, and unknown-envelope-field tampering", async () => {
    const digestTamper = await wire();
    digestTamper.source_sha256 = "0".repeat(64);
    await expect(parseNativeCapabilityRegistryV2(digestTamper)).rejects.toThrow(/does not match source_json/);

    const countTamper = await wire();
    countTamper.option_cell_count += 1;
    await expect(parseNativeCapabilityRegistryV2(countTamper)).rejects.toThrow(/option_cell_count.*does not match/);

    const unknown = { ...await wire(), standard_available: true };
    await expect(parseNativeCapabilityRegistryV2(unknown)).rejects.toThrow(/must contain exactly/);
  });

  it("rejects structurally valid executable row drift even with a matching digest", async () => {
    const drift = JSON.parse(JSON.stringify(rawCapabilityRegistryV2));
    drift.capabilities[0].official_method = "PLS-SEM Algorithm revised";
    drift.capabilities[0].legacy_row.official_method = "PLS-SEM Algorithm revised";

    await expect(parseNativeCapabilityRegistryV2(await wire(drift))).rejects.toThrow(
      /capability rows or option cells differ/,
    );
  });
});
