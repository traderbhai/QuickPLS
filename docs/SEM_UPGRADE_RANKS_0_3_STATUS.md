# QuickPLS SEM Upgrade Program Status

This document tracks implementation and qualification of the existing QuickPLS
General SEM project mode. It does not describe a separate application.

## Program order

1. Rank 0: qualify and promote the existing General SEM mediation and
   simultaneous two-way moderation cells.
2. Rank 1: add and qualify the bounded higher-order construct matrix.
3. Rank 2: add and qualify bounded two-way moderated mediation.
4. Rank 3: bind the existing CB-SEM/CFA science to schema-6 authority and add a
   bounded recursive-SEM case bootstrap.

Later-rank groundwork may be prepared in isolated branches, but promotion and
integration remain ordered. Every new capability is additive; existing project,
recipe, method, capability, and result identities remain readable unchanged.

## Completion rule

A rank is complete only when each promoted option cell independently has:

- a frozen QualificationSpec V2 contract;
- independent scientific and adversarial evidence;
- deterministic seed, declaration-order, row-order, and worker results;
- fail-closed cancellation, persistence, and tamper handling;
- canonical CSV, XLSX, HTML, PDF, SVG, and PNG export evidence where applicable;
- keyboard and screen-reader semantics plus the required Windows viewport and
  scaling matrix;
- accepted performance, memory, and ten-run soak receipts;
- installed and portable offline Windows fresh-process acceptance;
- current source-bound evidence generated after source freeze; and
- passing focused and full repository gates.

Promotion is cell-atomic. A failing cell stays in Labs and does not block an
independently qualified sibling point cell.

## Active checkpoint

Rank 0 started from commit `2b84071` on branch
`codx/sem-rank0-standard-qualification`.

The initial evidence audit found, and the governance-preparation pass now
records truthfully:

- the stale `archive_qualified` declaration for
  `qpls3.pls.mediation / pls_mediation_v1` was reset to `engine_only / labs`;
- the multiple-mediation bootstrap cell derives `engine_only`;
- the simultaneous two-way moderation point cell derives `engine_only`; and
- the simultaneous two-way moderation bootstrap cell derives `engine_only`.

All four exact method contracts now name the same six canonical publication
formats: CSV, XLSX, HTML, PDF, SVG, and PNG. This is implementation scope, not
qualification evidence; every cell remains non-Standard until its own strict
receipt set passes.

Rank 0 work is split into shared scientific qualification, canonical export,
Registry-driven Standard authorization, packaged Windows/performance evidence,
and final identity-bound promotion. Final evidence hashes are minted only after
all implementation sources are stable.

## Storage policy

Cargo and packaging output stay on drive D. Broad build work soft-stops when C
falls below 22 GB or D falls below 28 GB; hard floors are 20 GB and 25 GB.
Only explicitly resolved reproducible caches or superseded untracked build
artifacts may be removed. Accepted installers, receipts, reports, archives, and
source hashes are retained.
