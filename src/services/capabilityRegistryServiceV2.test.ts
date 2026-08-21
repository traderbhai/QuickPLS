import { beforeEach, describe, expect, it, vi } from "vitest";
import rawCapabilityRegistryV2 from "../../validation/capabilities/capability_registry_v2.json";
import { getNativeCapabilityRegistryV2 } from "./projectService";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));

async function sha256Hex(value: string): Promise<string> {
  const digest = await globalThis.crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

describe("Capability Registry V2 project service", () => {
  beforeEach(() => mocks.invoke.mockReset());

  it("invokes the read-only command and returns only a verified registry snapshot", async () => {
    const sourceJson = JSON.stringify(rawCapabilityRegistryV2);
    mocks.invoke.mockResolvedValue({
      response_schema_version: 1,
      registry_schema_version: 2,
      registry_id: "quickpls.capability_registry.v2",
      registry_version: rawCapabilityRegistryV2.registry_version,
      source_sha256: await sha256Hex(sourceJson),
      capability_row_count: rawCapabilityRegistryV2.capabilities.length,
      active_row_count: rawCapabilityRegistryV2.capabilities.filter(
        (row) => row.official_lifecycle === "active",
      ).length,
      option_cell_count: rawCapabilityRegistryV2.capabilities.reduce(
        (count, row) => count + row.option_cells.length,
        0,
      ),
      source_json: sourceJson,
    });

    const snapshot = await getNativeCapabilityRegistryV2();

    expect(mocks.invoke).toHaveBeenCalledWith("capability_registry_v2");
    expect(snapshot).toMatchObject({
      registry_id: "quickpls.capability_registry.v2",
      capability_row_count: 45,
      active_row_count: 43,
      option_cell_count: 56,
    });
    expect(snapshot.registry.capabilities).toHaveLength(45);
  });
});
