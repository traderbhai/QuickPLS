# QuickPLS 3 External-Beta Operations Kit

## Current status

This is a preparation package, not a completed beta. The committed beta program
is `planned`, its decision is `pending`, it contains no participants or workflow
results, and `beta_ready` is false. Passing the kit dry run means only that the
forms, privacy boundaries, scoring rules, and fail-closed placeholders are
internally consistent.

Do not recruit participants or distribute a candidate until the signed-candidate,
Windows lifecycle, independent-review, privacy, and support entry gates have
real owners and accepted records.

## Roles and operating flow

The beta coordinator controls candidate identity, recruitment, consent records,
access, retention, deletion requests, and the final evidence assembly. An
observer records journey outcomes without completing work for the participant.
The defect owner triages product failures; the scientific owner separately
disposes numerical discrepancies. An independent release approver makes the exit
decision only after the post-beta candidate lifecycle rerun.

A participant does the following on the exact approved beta candidate:

1. Confirms consent through the external consent system and receives a random
   program-scoped participant ID.
2. Uses a privacy-safe real dataset without giving QuickPLS staff or the evidence
   repository raw rows, direct identifiers, private project files, or
   value-revealing screenshots.
3. Performs all six journeys: import, author, calculate, interpret, export, and
   save/reopen.
4. Reports workflow friction, defects, and possible scientific discrepancies
   through the designated routes.
5. Uses `privacy@quickpls.org` for access, correction, withdrawal, or deletion
   requests.

For every observed workflow, the coordinator does the following:

1. Confirms the workflow and participant pseudonyms, exact candidate SHA-256,
   consent record reference, and privacy attestation.
2. Scores each journey as `completed`, `incomplete`, `blocked`, or
   `not_attempted`; records assistance separately as `none`, `prompted`,
   `guided`, or `developer_intervention`.
3. Counts a journey as unassisted only when it is completed and assistance is
   `none`. Time alone never establishes success.
4. Opens a defect for product behavior and a scientific discrepancy for a
   numerical or interpretation concern. The same observation may require both.
5. Stops candidate promotion immediately for reproducible data loss, archive
   corruption, a P0/P1 defect, or a material release-blocking discrepancy.

## Kit contents

The manifest is
`validation/beta_kit/quickpls_beta_operations_kit.json`. Its seven frozen forms
cover:

- participant enrollment and external consent references;
- a six-journey workflow observation rubric;
- access, correction, withdrawal, and deletion handling;
- product-defect and scientific-discrepancy registers;
- an inert signed-candidate/lifecycle placeholder; and
- the independent exit-decision checklist.

The committed forms are pristine templates. Do not fill them in place. Make
working copies in an access-controlled beta evidence store. Copy only the
privacy-safe normalized fields required by `validation/quickpls_external_beta.py`
into a protected beta-contract copy. Names, email addresses, institution names,
raw datasets, direct-identifier mappings, consent text, and raw privacy requests
must never enter this repository.

## Pseudonyms, consent, retention, and deletion

Generate participant and institution tokens with a cryptographically secure
random generator outside the repository. Do not hash a name, email address, or
institution name: predictable hashes remain identifying. IDs are stable only
within this beta program. Keep the token-to-person mapping and consent record in
the access-controlled external system, with separate access from product
evidence.

Before enrollment, the consent record must state the candidate scope, evidence
collected, purpose, retention period, withdrawal route, pseudonymous audit use,
and any limits of version-controlled retention. Privacy/legal reviewers must
approve that language and the retention design before real enrollment.

For a privacy request, the coordinator must:

1. open a request using a new request ID and the pseudonymous participant ID;
2. verify identity only in the external system;
3. pause further evidence use when withdrawal or deletion is requested;
4. complete the approved deletion or correction across the identity map,
   consent system, working evidence store, backups, and any protected contract;
5. retain only the permitted pseudonymous request ID and external completion
   record reference; and
6. recompute cohort and workflow thresholds. The program returns to not-ready if
   evidence drops below any threshold.

Repository history is not an erasure mechanism. Do not commit filled participant
records until the approved privacy/retention design explicitly covers the chosen
private repository and purge process. A deletion problem is a release blocker,
not an administrative exception.

## Defects and scientific discrepancies

Product defects use P0-P3 severity and retain the exact candidate ID, a
privacy-safe summary, and preferably a synthetic reproduction. Closed defects
need a closure evidence record. Exit permits no unresolved P0 or P1, no
reproducible data loss, and no archive corruption.

Scientific discrepancies remain separate from defects. A material discrepancy
must be fixed, explained against the bounded method contract, resolved by
narrowing the claim, or remain explicitly release-blocking. Beta observations
cannot replace independent numerical references, simulations, boundary tests,
or method-factory promotion.

## Signed candidate and lifecycle evidence

The lifecycle form deliberately contains null candidate, signer, environment,
phase, and result fields and declares itself ineligible as release evidence. A
SHA-like string, `signed: true`, screenshot, or opaque record ID cannot activate
it. Replace the placeholder only through the exact candidate manifest and
artifact descriptors accepted by the canonical external-beta validator.

After the last beta activity and every resulting fix, rerun all ten lifecycle
phases on the exact final signed candidate. The report must bind the candidate
manifest, signed artifact hashes, signature-report hashes, approved signing
identity, disconnected Windows environment, and post-beta timestamp.

## Exit decision

The independent approver reviews the nine checklist items in the exit template.
Every item needs an immutable evidence record. Approval must occur after the
post-beta lifecycle rerun and must bind that report and the exact candidate. A
coordinator cannot self-approve by changing template statuses or setting
`beta_ready`; the canonical validator recomputes readiness from the underlying
records.

## Dry run

Run the structural preparation check from the repository root:

```powershell
python validation/quickpls_beta_operations_kit.py
python -m unittest validation.test_quickpls_beta_operations_kit
```

Expected dry-run output includes `passed: true`, `dry_run_ready: true`,
`program_status: planned`, zero participants and workflows, and
`beta_ready: false`. The following command is a fail-closed canary and must exit
nonzero while this preparation-only kit is in use:

```powershell
python validation/quickpls_beta_operations_kit.py --require-ready
```

When genuine protected evidence exists, validate that copy with the canonical
gate; do not alter the templates to make them pass:

```powershell
python validation/quickpls_external_beta.py --contract <protected-beta-contract.json> --require-ready
```

## External blockers

Repository automation cannot supply any of the following:

- purchase and independent approval of the code-signing certificate and signer;
- an exact signed/timestamped beta candidate and Windows lifecycle entry record;
- scientific, security, privacy, legal, license, trademark,
  comparative-claims, and EULA review dispositions;
- named support coverage and exercised escalation routes;
- 15-25 genuinely consented participants from at least five institutions,
  including the required experience mix;
- at least 30 privacy-safe real workflows over all six journeys;
- disposition of beta defects, data-integrity findings, and scientific
  discrepancies;
- the post-beta lifecycle rerun on the exact final signed candidate; and
- independent, dated exit approval binding the immutable candidate evidence.

Until all of those records exist and pass the canonical gate, beta readiness and
competitor-grade commercial positioning remain blocked.
