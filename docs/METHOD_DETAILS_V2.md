# Method Details V2

Method Details is the customer-facing explanation layer for the exact option
cells selected by a calculation recipe. It is not a static list of methods and
does not infer availability from a family-level label.

## Resolution

`methodDetailsForSettingsV2` first resolves current `AnalysisUiSettings`
through the method-to-capability bridge. When Method Details is opened from
Results, the dialog instead resolves the selected completed run through
`nativeCapabilityRequirementsForRunV2`; it never substitutes the current
workspace settings for a historical run. Every item must match the full
four-field registry identity:

- registry schema version;
- capability ID;
- option-cell ID;
- capability version.

The live desktop dialog then shows one card per required option cell. This is
important for combined workflows: a usable point estimator and an unavailable
resampling or group add-on are explained independently.

Calculate exposes Method Details beside the selected setup, and Results exposes
it beside the selected completed run. The Results entry is run-bound: changing
today's workspace settings cannot change the method explanation for an
immutable historical result.

## Customer template

Every card contains the same nine sections:

1. What this method answers.
2. When to use it.
3. Required model and data.
4. Main settings and defaults.
5. Outputs.
6. Assumptions and cautions.
7. Interpretation guidance.
8. Method references.
9. Advanced technical details.

Standard and Experimental availability comes from Capability Registry V2.
Internal development and evidence states are never rendered. When Experimental
Labs is disabled, the dialog tells the user where to enable it. An option that
has no executable implementation remains unavailable even when Labs is on.

## Boundaries

The current implementation uses the official registry predicates as the base
requirements and provides method-family explanations for the current Calculate
surface. As each option reaches complete parity, its registry predicates,
settings schema, result schema, and customer explanation must be revised
together. Method references are available without being required for an
offline calculation; bundled offline help remains a later product-finalization
task.

Relevant source:

- `src/domain/methodDetailsV2.ts`
- `src/domain/methodDetailsV2.test.ts`
- `src/native/NativeUtilityDialog.tsx`
- `src/native/NativeUtilityDialog.test.tsx`
- `src/domain/capabilityRegistryV2.ts`
- `src/domain/methodCapabilityRegistryV2.ts`
