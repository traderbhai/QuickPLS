# QuickPLS v2.13.0 Real Dataset Protocol Entrypoints

Status: validated after the v2.13.0 gate passes.

This frontend-only milestone makes the v2.12 real dataset review protocol discoverable from the desktop app. It does not change statistical engines, method validation, result schemas, project format, or numerical fingerprints.

## Problem

The privacy-safe review protocol existed as documentation and templates, but a user testing QuickPLS with a real SEM dataset needed an obvious in-app place to learn:

- do not commit raw private datasets;
- do not store value-revealing screenshots;
- separate UI/product issues from statistical evidence gaps;
- use anonymized issue notes and synthetic reproductions.

## Completed Scope

- Added a Trust Center section named `Real dataset review protocol`.
- Added a Settings section summarizing the private dataset review rules.
- Added a Home notice for users reviewing private datasets, with a direct handoff to the Trust Center.
- Kept the v2.12 protocol and anonymized template as the source of truth.
- Added smoke and audit evidence proving the entrypoints are wired and the milestone remains frontend/product-only.

## Boundary

Private datasets are still manual review inputs only. Automated gates continue to use bundled, generated, or static evidence. Any product issue discovered from a private dataset must be anonymized or reproduced synthetically before implementation.
