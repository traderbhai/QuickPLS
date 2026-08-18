import { describe, expect, it } from "vitest";
import {
  parseStandardSemModelV4AuthorityCasOutcomeV1,
  parseStandardSemModelV4AuthorityCasRequestV1,
  parseStandardSemModelV4AuthorityResolveOutcomeV1,
  parseStandardSemModelV4AuthorityResolveRequestV1,
} from "./standardSemModelV4AuthorityCas";
import {
  canonicalizeSemModelV4,
  convertLegacyBasicModelV4,
  validateSemModelV4,
  type LegacyBasicModelV4Input,
  type SemModelV4,
} from "./semModelV4";

const SOURCE_DIGEST = "a".repeat(64);
const CANDIDATE_DIGEST = "b".repeat(64);
const SCIENTIFIC_DIGEST = "c".repeat(64);

function readyModel(): SemModelV4 {
  const legacy: LegacyBasicModelV4Input = {
    id: "standard-model",
    name: "Standard CAS fixture",
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

function request(candidate = readyModel()) {
  return parseStandardSemModelV4AuthorityCasRequestV1({
    expectedSourceModelDocumentSha256: SOURCE_DIGEST,
    sourceModel: readyModel(),
    candidate,
  });
}

function readyOutcome(candidate = readyModel()) {
  return {
    status: "ok",
    value: {
      schemaVersion: 1,
      sourceModelDocumentSha256: SOURCE_DIGEST,
      canonicalCandidate: canonicalizeSemModelV4(candidate),
      candidateModelDocumentSha256: CANDIDATE_DIGEST,
      candidateScientificSha256: SCIENTIFIC_DIGEST,
      readiness: "ready",
      authoringIssues: [],
      readinessIssues: [],
    },
  };
}

describe("Standard SemModelV4 native document-digest CAS wire", () => {
  it("accepts a canonical ready candidate bound to the exact source digest", () => {
    const exactRequest = request();
    expect(parseStandardSemModelV4AuthorityCasOutcomeV1(
      readyOutcome(exactRequest.candidate),
      exactRequest,
    )).toMatchObject({
      status: "ok",
      value: {
        sourceModelDocumentSha256: SOURCE_DIGEST,
        candidateModelDocumentSha256: CANDIDATE_DIGEST,
        candidateScientificSha256: SCIENTIFIC_DIGEST,
        readiness: "ready",
        authoringIssues: [],
        readinessIssues: [],
      },
    });
  });

  it("keeps authoring validity separate from readiness and scientific identity", () => {
    const draft = readyModel();
    const factor = draft.variables.find((variable) => variable.kind === "common_factor");
    if (factor?.kind !== "common_factor") throw new Error("Expected a common-factor fixture.");
    factor.identification = { kind: "fixed_variance" };
    const exactRequest = request(draft);
    const readinessIssues = validateSemModelV4(draft);
    expect(readinessIssues.map((issue) => issue.code))
      .toContain("identification.fixed_variance.missing");
    const ready = readyOutcome(draft);
    const outcome = {
      ...ready,
      value: {
        ...ready.value,
        readiness: "authoring_only",
        candidateScientificSha256: null,
        readinessIssues,
      },
    };

    const parsed = parseStandardSemModelV4AuthorityCasOutcomeV1(outcome, exactRequest);
    expect(parsed).toMatchObject({
      status: "ok",
      value: {
        readiness: "authoring_only",
        candidateScientificSha256: null,
        authoringIssues: [],
        readinessIssues,
      },
    });
  });

  it("rejects response drift and preserves exact blocked issue arrays", () => {
    const exactRequest = request();
    const drifted = readyOutcome(exactRequest.candidate);
    drifted.value.canonicalCandidate = {
      ...drifted.value.canonicalCandidate,
      name: "Different candidate",
    };
    expect(() => parseStandardSemModelV4AuthorityCasOutcomeV1(drifted, exactRequest))
      .toThrowError(expect.objectContaining({ code: "standard_sem_model_v4_authority.candidate_mismatch" }));

    const issue = { code: "relation.parameter.unknown", subject: "effect", message: "Unknown parameter" };
    expect(parseStandardSemModelV4AuthorityCasOutcomeV1({
      status: "blocked",
      diagnostic: {
        code: "standard_sem_model_v4_authority.candidate_authoring_invalid",
        message: "Candidate failed authoring integrity.",
        correctiveAction: "Correct the issue.",
        authoringIssues: [issue],
        readinessIssues: [issue],
      },
    }, exactRequest)).toEqual({
      status: "blocked",
      diagnostic: {
        code: "standard_sem_model_v4_authority.candidate_authoring_invalid",
        message: "Candidate failed authoring integrity.",
        correctiveAction: "Correct the issue.",
        authoringIssues: [issue],
        readinessIssues: [issue],
      },
    });
  });
});

describe("Standard SemModelV4 native authority bootstrap wire", () => {
  it("accepts only the native canonical model with coherent ready digests", () => {
    const exactRequest = parseStandardSemModelV4AuthorityResolveRequestV1({ model: readyModel() });
    expect(parseStandardSemModelV4AuthorityResolveOutcomeV1({
      status: "ok",
      value: {
        schemaVersion: 1,
        canonicalModel: canonicalizeSemModelV4(exactRequest.model),
        modelDocumentSha256: SOURCE_DIGEST,
        scientificSha256: SCIENTIFIC_DIGEST,
        readiness: "ready",
        authoringIssues: [],
        readinessIssues: [],
      },
    }, exactRequest)).toMatchObject({
      status: "ok",
      value: {
        modelDocumentSha256: SOURCE_DIGEST,
        scientificSha256: SCIENTIFIC_DIGEST,
        readiness: "ready",
      },
    });
  });

  it("rejects a resolved model that is not the requested native canonical identity", () => {
    const exactRequest = parseStandardSemModelV4AuthorityResolveRequestV1({ model: readyModel() });
    expect(() => parseStandardSemModelV4AuthorityResolveOutcomeV1({
      status: "ok",
      value: {
        schemaVersion: 1,
        canonicalModel: { ...canonicalizeSemModelV4(exactRequest.model), name: "Drifted" },
        modelDocumentSha256: SOURCE_DIGEST,
        scientificSha256: SCIENTIFIC_DIGEST,
        readiness: "ready",
        authoringIssues: [],
        readinessIssues: [],
      },
    }, exactRequest)).toThrowError(expect.objectContaining({
      code: "standard_sem_model_v4_authority.resolved_model_mismatch",
    }));
  });
});
