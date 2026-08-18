/** Product-surface policy for option-level Capability Registry V2 cells. */

export type CoverageStateV2 = "full" | "partial" | "absent" | "intentionally_excluded";
export type EvidenceStateV2 = "absent" | "engine_only" | "archive_qualified" | "native_qualified" | "release_qualified";
export type ProductSurfaceV2 = "standard" | "labs" | "legacy" | "internal";

export interface CapabilitySurfaceCellV2 {
  capability_id: string;
  cell_id: string;
  capability_version: string;
  coverage_state: CoverageStateV2;
  evidence_state: EvidenceStateV2;
  surface: ProductSurfaceV2;
}

export interface CapabilityAvailabilityV2 {
  visibility: "hidden" | "supported" | "experimental";
  selectable: boolean;
  customer_label: "Supported" | "Experimental" | null;
  reason:
    | "standard_ready"
    | "labs_ready"
    | "labs_disabled"
    | "incomplete_standard_cell"
    | "not_executable"
    | "legacy_only"
    | "internal_only"
    | "intentionally_excluded";
}

export interface MethodDetailsV2 {
  what_it_answers: string;
  when_to_use: string;
  required_model_and_data: string;
  settings_and_defaults: string;
  outputs: string;
  assumptions_and_cautions: string;
  interpretation_guidance: string;
  method_references: string[];
  advanced_technical_details: string;
}

export const EXPERIMENTAL_LABS_WARNING =
  "Experimental methods may change and should be independently checked before final reporting.";

const EXECUTABLE_EVIDENCE = new Set<EvidenceStateV2>([
  "engine_only",
  "archive_qualified",
  "native_qualified",
  "release_qualified",
]);

/**
 * Standard and Labs visibility is fail-closed. In particular, a legacy
 * release-qualified claim cannot make partial SmartPLS coverage appear in
 * Standard.
 */
export function capabilityAvailabilityV2(
  cell: CapabilitySurfaceCellV2,
  experimentalLabsEnabled: boolean,
): CapabilityAvailabilityV2 {
  if (cell.coverage_state === "intentionally_excluded") {
    return { visibility: "hidden", selectable: false, customer_label: null, reason: "intentionally_excluded" };
  }
  if (cell.surface === "legacy") {
    return { visibility: "hidden", selectable: false, customer_label: null, reason: "legacy_only" };
  }
  if (cell.surface === "internal") {
    return { visibility: "hidden", selectable: false, customer_label: null, reason: "internal_only" };
  }
  if (cell.surface === "standard") {
    if (
      (cell.coverage_state === "full" || cell.coverage_state === "partial")
      && cell.evidence_state === "release_qualified"
    ) {
      return { visibility: "supported", selectable: true, customer_label: "Supported", reason: "standard_ready" };
    }
    return { visibility: "hidden", selectable: false, customer_label: null, reason: "incomplete_standard_cell" };
  }
  if (!EXECUTABLE_EVIDENCE.has(cell.evidence_state) || cell.coverage_state === "absent") {
    return { visibility: "hidden", selectable: false, customer_label: null, reason: "not_executable" };
  }
  if (!experimentalLabsEnabled) {
    return { visibility: "hidden", selectable: false, customer_label: null, reason: "labs_disabled" };
  }
  return { visibility: "experimental", selectable: true, customer_label: "Experimental", reason: "labs_ready" };
}

export function capabilityCellSessionKey(cell: Pick<CapabilitySurfaceCellV2, "capability_id" | "cell_id" | "capability_version">): string {
  return `${cell.capability_id}::${cell.cell_id}::${cell.capability_version}`;
}

export function shouldShowExperimentalWarning(
  cell: CapabilitySurfaceCellV2,
  experimentalLabsEnabled: boolean,
  acknowledgedThisSession: ReadonlySet<string>,
): boolean {
  const availability = capabilityAvailabilityV2(cell, experimentalLabsEnabled);
  return availability.visibility === "experimental" && !acknowledgedThisSession.has(capabilityCellSessionKey(cell));
}

export function acknowledgeExperimentalWarning(
  cell: Pick<CapabilitySurfaceCellV2, "capability_id" | "cell_id" | "capability_version">,
  acknowledgedThisSession: ReadonlySet<string>,
): Set<string> {
  const next = new Set(acknowledgedThisSession);
  next.add(capabilityCellSessionKey(cell));
  return next;
}

export function validateMethodDetailsV2(details: MethodDetailsV2): string[] {
  const errors: string[] = [];
  const requiredText: Array<[keyof Omit<MethodDetailsV2, "method_references">, string]> = [
    ["what_it_answers", "What this method answers"],
    ["when_to_use", "When to use it"],
    ["required_model_and_data", "Required model and data"],
    ["settings_and_defaults", "Main settings and defaults"],
    ["outputs", "Outputs"],
    ["assumptions_and_cautions", "Assumptions and cautions"],
    ["interpretation_guidance", "Interpretation guidance"],
    ["advanced_technical_details", "Advanced technical details"],
  ];
  for (const [key, label] of requiredText) {
    if (!details[key].trim()) errors.push(`${label} must be nonempty`);
  }
  if (details.method_references.length === 0) errors.push("Method references must contain at least one reference");
  if (details.method_references.some((reference) => !reference.trim())) errors.push("Method references cannot contain blank entries");
  return errors;
}
