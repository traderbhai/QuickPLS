import { beforeEach, describe, expect, it, vi } from "vitest";
import { convertLegacyBasicModelV4, type LegacyBasicModelV4Input } from "../domain/semModelV4";
import {
  compareAndSwapStandardSemModelV4Authority,
  resolveStandardSemModelV4Authority,
} from "./standardSemModelV4AuthorityService";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

function model() {
  const legacy: LegacyBasicModelV4Input = {
    id: "standard-model",
    name: "Standard service fixture",
    constructs: [{
      id: "factor",
      name: "Factor",
      short_name: "F",
      mode: "reflective",
      indicators: ["x1", "x2", "x3"],
    }],
    paths: [],
    controls: [],
    higher_order_constructs: [],
    interactions: [],
  };
  return convertLegacyBasicModelV4(legacy, "cbsem_common_factor");
}

const blocked = {
  status: "blocked",
  diagnostic: {
    code: "standard_sem_model_v4_authority.stale_source_digest",
    message: "Source digest changed.",
    correctiveAction: "Refresh and retry.",
    authoringIssues: [],
    readinessIssues: [],
  },
};

describe("Standard SemModelV4 authority CAS service", () => {
  beforeEach(() => mocks.invoke.mockReset());

  it("sends only the complete source, expected document digest, and complete candidate", async () => {
    mocks.invoke.mockResolvedValue(blocked);
    const source = model();
    const candidate = { ...source, name: "Renamed authority" };
    const digest = "a".repeat(64);

    await expect(compareAndSwapStandardSemModelV4Authority(source, digest, candidate))
      .resolves.toEqual(blocked);
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(mocks.invoke).toHaveBeenCalledWith(
      "compare_and_swap_standard_sem_model_v4_authority",
      {
        request: expect.objectContaining({
          expectedSourceModelDocumentSha256: digest,
          sourceModel: expect.objectContaining({
            id: source.id,
            name: source.name,
            annotations: source.annotations,
            presentation: source.presentation,
          }),
          candidate: expect.objectContaining({
            id: candidate.id,
            name: candidate.name,
            annotations: candidate.annotations,
            presentation: candidate.presentation,
          }),
        }),
      },
    );
    expect(mocks.invoke.mock.calls[0]?.[1]).not.toEqual(expect.objectContaining({
      graph: expect.anything(),
      project: expect.anything(),
      path: expect.anything(),
    }));
  });

  it("throws on malformed native output so callers cannot replace authority", async () => {
    mocks.invoke.mockResolvedValue({ status: "ok", value: { readiness: "ready" } });
    const source = model();

    await expect(compareAndSwapStandardSemModelV4Authority(
      source,
      "a".repeat(64),
      source,
    )).rejects.toMatchObject({ code: "standard_sem_model_v4_authority.field_missing" });
  });

  it("bootstraps only from a complete model and returns the native canonical authority identity", async () => {
    const source = model();
    const resolved = {
      status: "ok",
      value: {
        schemaVersion: 1,
        canonicalModel: source,
        modelDocumentSha256: "b".repeat(64),
        scientificSha256: "c".repeat(64),
        readiness: "ready",
        authoringIssues: [],
        readinessIssues: [],
      },
    };
    mocks.invoke.mockResolvedValue(resolved);

    await expect(resolveStandardSemModelV4Authority(source)).resolves.toMatchObject({
      status: "ok",
      value: {
        modelDocumentSha256: "b".repeat(64),
        scientificSha256: "c".repeat(64),
        readiness: "ready",
      },
    });
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(mocks.invoke).toHaveBeenCalledWith(
      "resolve_standard_sem_model_v4_authority",
      { request: { model: expect.objectContaining({ id: source.id, name: source.name }) } },
    );
  });
});
