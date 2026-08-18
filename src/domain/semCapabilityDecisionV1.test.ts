import { describe, expect, it } from "vitest";
import {
  canonicalSemCapabilityDecisionV1Json,
  createSemCapabilityDecisionV1,
  parseSemCapabilityDecisionV1,
  type SemCapabilityDecisionV1,
} from "./semCapabilityDecisionV1";

function blockedWire(): SemCapabilityDecisionV1 {
  return {
    schema_version: 1,
    status: "blocked",
    status_label: "Blocked",
    estimator_id: "estimator:general_sem_ml_v1",
    capability_cells: [
      {
        registry_schema_version: 2,
        capability_id: "smartpls.cbsem_moderator",
        cell_id: "qpls3.cbsem.moderator.product_indicator",
        capability_version: "cbsem_product_indicator_v1",
      },
      {
        registry_schema_version: 2,
        capability_id: "smartpls.cbsem",
        cell_id: "qpls3.cbsem.ml",
        capability_version: "cbsem_ml_v1",
      },
    ],
    diagnostics: [
      {
        code: "sem.capability.qualification_missing",
        severity: "warning",
        subject: null,
        message: "Qualification evidence is incomplete.",
        corrections: ["Run the registered qualification suite."],
      },
      {
        code: "sem.capability.interaction_order_unsupported",
        severity: "error",
        subject: "term:x:m:w",
        message: "This estimator cannot execute the authored three-way interaction.",
        corrections: [
          "Reduce the interaction to two operands.",
          "Choose an estimator that supports three-way interactions.",
        ],
      },
    ],
    evidence: [
      {
        evidence_id: "qualification:moderation:v1",
        description: "The moderation qualification covers two-way interactions only.",
      },
      {
        evidence_id: "compiler:general_sem:v1",
        description: "The compiler preserved all authored operands.",
      },
    ],
    summary: "Blocked: the selected estimator cannot execute this model.",
    explanation: "The scientific model is preserved, but execution requires a qualified estimator for every exact capability cell.",
  };
}

describe("SemCapabilityDecisionV1", () => {
  it("strictly round-trips to canonical order with explicit textual status", () => {
    const parsed = parseSemCapabilityDecisionV1(blockedWire());
    expect(parsed).toMatchObject({
      schema_version: 1,
      status: "blocked",
      status_label: "Blocked",
      estimator_id: "estimator:general_sem_ml_v1",
      summary: expect.stringMatching(/^Blocked:/),
    });
    expect(parsed.capability_cells.map((cell) => cell.capability_id)).toEqual([
      "smartpls.cbsem",
      "smartpls.cbsem_moderator",
    ]);
    expect(parsed.diagnostics.map((diagnostic) => diagnostic.severity)).toEqual(["error", "warning"]);
    expect(parsed.diagnostics[0]?.corrections).toEqual([
      "Choose an estimator that supports three-way interactions.",
      "Reduce the interaction to two operands.",
    ]);
    expect(parsed.evidence.map((item) => item.evidence_id)).toEqual([
      "compiler:general_sem:v1",
      "qualification:moderation:v1",
    ]);
    expect(Object.isFrozen(parsed)).toBe(true);
    expect(Object.isFrozen(parsed.diagnostics[0]?.corrections)).toBe(true);

    const reopened = parseSemCapabilityDecisionV1(JSON.parse(JSON.stringify(parsed)));
    expect(reopened).toEqual(parsed);
    expect(canonicalSemCapabilityDecisionV1Json(reopened)).toBe(canonicalSemCapabilityDecisionV1Json(parsed));
  });

  it("canonicalizes declaration order without changing human-authored text", () => {
    const first = blockedWire();
    const reordered = structuredClone(first) as unknown as {
      capability_cells: unknown[];
      diagnostics: Array<{ corrections: unknown[] }>;
      evidence: unknown[];
    };
    reordered.capability_cells.reverse();
    reordered.diagnostics.reverse();
    reordered.diagnostics.forEach((diagnostic) => diagnostic.corrections.reverse());
    reordered.evidence.reverse();

    expect(canonicalSemCapabilityDecisionV1Json(reordered)).toBe(canonicalSemCapabilityDecisionV1Json(first));
    expect(parseSemCapabilityDecisionV1(reordered).explanation).toBe(first.explanation);
  });

  it("rejects unknown fields and invalid status or severity discriminators", () => {
    expect(() => parseSemCapabilityDecisionV1({ ...blockedWire(), color: "red" }))
      .toThrowError(expect.objectContaining({ code: "schema.unknown_field", subject: "decision.color" }));

    const nested = structuredClone(blockedWire()) as unknown as { diagnostics: Array<Record<string, unknown>> };
    nested.diagnostics[0]!.color = "red";
    expect(() => parseSemCapabilityDecisionV1(nested))
      .toThrowError(expect.objectContaining({ code: "schema.unknown_field", subject: "decision.diagnostics[0].color" }));

    expect(() => parseSemCapabilityDecisionV1({ ...blockedWire(), status: "available" }))
      .toThrowError(expect.objectContaining({ code: "schema.invalid_discriminator", subject: "decision.status" }));

    const severity = structuredClone(blockedWire()) as unknown as { diagnostics: Array<Record<string, unknown>> };
    severity.diagnostics[0]!.severity = "danger";
    expect(() => parseSemCapabilityDecisionV1(severity))
      .toThrowError(expect.objectContaining({ code: "schema.invalid_discriminator", subject: "decision.diagnostics[0].severity" }));

    const code = structuredClone(blockedWire()) as unknown as { diagnostics: Array<Record<string, unknown>> };
    code.diagnostics[0]!.code = "qualification_missing";
    expect(() => parseSemCapabilityDecisionV1(code))
      .toThrowError(expect.objectContaining({ code: "schema.diagnostic_code_invalid", subject: "decision.diagnostics[0].code" }));
  });

  it("rejects status and blocking-diagnostic contradictions", () => {
    for (const [status, statusLabel] of [["supported", "Supported"], ["experimental", "Experimental"]] as const) {
      expect(() => parseSemCapabilityDecisionV1({ ...blockedWire(), status, status_label: statusLabel }))
        .toThrowError(expect.objectContaining({ code: "decision.runnable_status_has_blocking_diagnostic" }));
    }

    const blockedWithoutError = structuredClone(blockedWire()) as unknown as {
      diagnostics: Array<Record<string, unknown>>;
    };
    blockedWithoutError.diagnostics.forEach((diagnostic) => { diagnostic.severity = "warning"; });
    expect(() => parseSemCapabilityDecisionV1(blockedWithoutError))
      .toThrowError(expect.objectContaining({ code: "decision.blocked_without_blocking_diagnostic" }));
  });

  it("requires exact unique cells, evidence, diagnostics, and actionable corrections", () => {
    expect(() => parseSemCapabilityDecisionV1({ ...blockedWire(), capability_cells: [] }))
      .toThrowError(expect.objectContaining({ code: "decision.capability_cells_empty" }));
    expect(() => parseSemCapabilityDecisionV1({
      ...blockedWire(),
      capability_cells: [blockedWire().capability_cells[0], blockedWire().capability_cells[0]],
    })).toThrowError(expect.objectContaining({ code: "decision.capability_cell_duplicate" }));

    expect(() => parseSemCapabilityDecisionV1({ ...blockedWire(), evidence: [] }))
      .toThrowError(expect.objectContaining({ code: "decision.evidence_empty" }));
    expect(() => parseSemCapabilityDecisionV1({
      ...blockedWire(),
      evidence: [blockedWire().evidence[0], blockedWire().evidence[0]],
    })).toThrowError(expect.objectContaining({ code: "decision.evidence_duplicate" }));

    expect(() => parseSemCapabilityDecisionV1({
      ...blockedWire(),
      diagnostics: [blockedWire().diagnostics[1], blockedWire().diagnostics[1]],
    })).toThrowError(expect.objectContaining({ code: "decision.diagnostic_duplicate" }));

    const noCorrection = structuredClone(blockedWire()) as unknown as {
      diagnostics: Array<Record<string, unknown>>;
    };
    noCorrection.diagnostics[1]!.corrections = [];
    expect(() => parseSemCapabilityDecisionV1(noCorrection))
      .toThrowError(expect.objectContaining({ code: "decision.correction_required" }));
  });

  it("requires accessible status, summary, explanation, and diagnostic messages", () => {
    expect(() => parseSemCapabilityDecisionV1({ ...blockedWire(), status_label: "Unavailable" }))
      .toThrowError(expect.objectContaining({ code: "decision.status_label_mismatch" }));
    expect(() => parseSemCapabilityDecisionV1({ ...blockedWire(), summary: "   " }))
      .toThrowError(expect.objectContaining({ code: "schema.text_invalid", subject: "decision.summary" }));
    expect(() => parseSemCapabilityDecisionV1({ ...blockedWire(), explanation: "" }))
      .toThrowError(expect.objectContaining({ code: "schema.text_invalid", subject: "decision.explanation" }));

    const blankMessage = structuredClone(blockedWire()) as unknown as {
      diagnostics: Array<Record<string, unknown>>;
    };
    blankMessage.diagnostics[0]!.message = "";
    expect(() => parseSemCapabilityDecisionV1(blankMessage))
      .toThrowError(expect.objectContaining({ code: "schema.text_invalid", subject: "decision.diagnostics[0].message" }));
  });

  it("derives the non-color status label through the constructor boundary", () => {
    const wire = blockedWire();
    const decision = createSemCapabilityDecisionV1({
      status: wire.status,
      estimator_id: wire.estimator_id,
      capability_cells: wire.capability_cells,
      diagnostics: wire.diagnostics,
      evidence: wire.evidence,
      summary: wire.summary,
      explanation: wire.explanation,
    });
    expect(decision).toMatchObject({ schema_version: 1, status: "blocked", status_label: "Blocked" });
  });
});
