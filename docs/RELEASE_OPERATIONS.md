# QuickPLS Release Operations

## Status And Claim Boundary

QuickPLS now has executable foundations for dependency, secret, and license
scanning; unsigned SBOM/provenance review; identity-bound runtime-evidence
transport; and a protected signed-candidate workflow contract. These controls do
not make an unsigned build commercially ready. Beta and stable remain blocked
until the approved certificate, protected environment, exact-candidate tests,
external reviews, beta evidence, and final release approval exist.

## Routine Security Scans

The `qpls:security:scan` command performs the following checks against committed
inputs:

- full npm dependency graph audit, including build and test dependencies;
- `pip-audit` against the hash-pinned validation requirements;
- `cargo-audit` for the Windows x64 target;
- tracked-source secret scanning;
- npm and Windows-targeted Cargo license inventory validation.

The scan writes a new report and refuses to overwrite an existing report. Known
vulnerabilities, undisposed Rust warnings, secret findings, and unknown license
identifiers fail closed. Informational Rust findings require a checked-in owner,
rationale, review date, and expiry in
`validation/security_scan_dispositions.json`. Passing automation does not replace
qualified security or legal review, and the report records that boundary.

The scheduled security workflow runs these checks weekly and on relevant
dependency, policy, and workflow changes. The workflow uploads the report even
when the gate fails so the exact findings remain reviewable.

The workflow actions are pinned to full commit identities. Python scan tools and
their transitive dependencies are installed from
`validation/security-tools-requirements.txt` with pip hash enforcement. The
Cargo scanner is locked to `cargo-audit 0.22.2`; automation verifies both the
executable version and SHA-256 of the crates.io source archive retained by
Cargo. A tool update therefore requires an intentional lock, hash, workflow,
and contract-test change.

## Release-Evidence Transport

Strict method validation uses runtime evidence such as packaged binaries, saved
projects, XLSX exports, and acceptance screenshots. Those files are transported
between a generating workflow and a clean validation checkout with
`validation/release_evidence_bundle.py`.

Evidence generation and packing use this exact two-stage source-freeze flow:

1. Generate the runtime artifacts and their `*.identity.json` reports, then
   review and commit every tracked source and report change.
2. Record the full committed identity with `git rev-parse HEAD`. Do not
   regenerate or edit tracked files after this point. The ignored runtime bytes
   may remain in the same workspace.
3. Confirm `git status --porcelain=v1 --untracked-files=all` is empty, then run
   `npm run qpls:release:evidence-pack -- --bundle release/evidence/<name>.zip
   --expected-commit <40-hex-commit>` from that exact checkout. The destination
   must not already exist.
4. Publish the ZIP only as an immutable workflow artifact tied to that source
   run. Provide its exact artifact name and source run ID to `evidence-verify.yml`
   and, later, the protected signed-candidate workflow.

Packing intentionally refuses a dirty tracked source state or a HEAD different
from `--expected-commit`. The tool derives the complete runtime allowlist from
tracked `*.identity.json` reports, verifies the current bytes of every tracked
source descriptor, then verifies every required runtime file by
repository-relative path, byte count, and SHA-256 before creating the ZIP. The
manifest binds the exact source commit and the hashes of all current identity
reports.

Restoration accepts only that manifest and its exact allowlisted payloads. It
rejects duplicate or unexpected entries, traversal, links, source-commit drift,
identity-report drift, size/hash drift, unsafe ancestors, and any overwrite.
Payloads are staged and fully hashed before any destination is created. A failed
restore removes files created during that attempt.

The bundle is transport, not evidence authority. Restored bytes must still pass
the strict method factory. Bundle verification cannot promote a capability or
authorize a commercial claim.

## Protected Signed-Candidate Workflow

The manual `release.yml` workflow is intentionally unavailable until repository
maintainers configure a protected `quickpls-signing` environment and update the
signing identity record after purchasing and independently approving the exact
leaf certificate.

Before enabling it:

1. Protect `main` and require reviewed, passing changes.
2. Protect the `quickpls-signing` environment with required reviewers.
3. Provision the approved hardware-backed or managed signer to the Windows
   runner without exporting a PFX, password, or private key to the repository.
4. Populate `validation/quickpls_signing_identity.json` with the exact approved
   subject and SHA-1 thumbprint.
5. Confirm the protected runner exposes SignTool and the approved certificate in
   the current-user certificate store for detached CMS signing.
6. Generate and publish the identity-bound runtime-evidence bundle for the exact
   source commit using the reviewed evidence workflow.
7. Configure the signed-candidate workflow to download that exact artifact and
   restore it before strict validation.

The workflow accepts only beta or stable. Before signing, the structural program
must pass while `competitor_ready` remains explicitly false. It then builds
locked inputs, performs the full security scan, runs the project test gates,
signs the desktop and CLI, builds and signs the installer, and invokes the
existing fail-closed signed-candidate factory. Only after exact signed-candidate
evidence exists does the stable path re-run commercial readiness and the
aggregate competitor program with `--require-ready`. The workflow uploads a
review artifact only; it has read-only repository-content permission and never
creates a public GitHub release automatically.

## Updater And Recovery Operations

Each signed candidate contains the exact full installer plus a detached-CMS
signed channel manifest. The manifest binds channel, target release, payload ID,
approved signing identity, minimum installed version, no-downgrade policy,
installer hash, and offline full-installer recovery hash. Update checks remain
manual by default.

Before external beta, rehearse signature failure, corrupted manifest, interrupted
download, interrupted install, retry, N-1 upgrade preservation, rollback, and
offline full-installer recovery on the exact signed candidate. Preserve project
copies before lifecycle tests. An older application must not rewrite a newer
unsupported project schema. Follow `RELEASE_ROLLBACK_AND_RECOVERY.md` for channel
freeze, user notification, immutable replacement candidate, and requalification.

## Diagnostics And Support Operations

The desktop diagnostic bundle is local, user-initiated, preview-before-save, and
does not upload. Before external beta, execute the packaged acceptance harness for
preview, cancel, save, archive/redaction boundaries, keyboard use, local path edge
cases, and observed application/page network behavior under both standard and
administrator accounts. Retain the generated build receipt and the saved bundle
hash with the candidate evidence.

Support owners must monitor the published issue route and private security route,
record primary and backup coverage, exercise escalation, and review response-target
performance. External legal and security dispositions use stable confidential
record references; privileged or secret material must not be checked in.

## Remaining External Gates

The following cannot be completed by repository automation alone:

- certificate purchase, identity approval, protected key custody, and live signing;
- clean Windows lifecycle/recovery matrices on supported VMs and real accounts;
- qualified security, privacy, license, trademark, comparative-claims, and EULA
  reviews;
- monitored support ownership and measured response performance;
- external beta cohort execution and exit review;
- release-manager approval of the exact tested signed hashes and immutable
  promotion record.

Until those gates pass, use only internal or unsigned-preview labels and retain
the documented functional-offline claim boundary for Microsoft-managed WebView2
runtime traffic.
