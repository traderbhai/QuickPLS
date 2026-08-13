# QuickPLS 3 Commercial Readiness

## Purpose

This document defines the minimum release gate for positioning QuickPLS 3 as an
independently implemented, fully offline Windows competitor for the workflows
that QuickPLS explicitly documents as supported. It does not authorize claims
of complete SmartPLS parity, numerical identity, SmartPLS project
compatibility, or affiliation.

The current contract is intentionally pending. A build, certificate, legal
review, clean-machine test, or external beta result is evidence only after the
corresponding record exists and has been reviewed; planned work is never
counted as completed work.

## Product Decisions

- **Platform:** Windows x64 is the QuickPLS 3 stable target. macOS may follow
  demonstrated demand; Linux remains a later experimental option.
- **License:** the launch product remains free proprietary software. Accounts,
  activation, and a time-limited trial are not required.
- **Offline and privacy:** calculations, projects, documentation, and exports
  remain local. Product telemetry is disabled by default.
- **Updates:** update checks are user-initiated by default. Offline users can
  install a complete signed package without an updater service.
- **Positioning:** publish an evidence-linked comparison matrix using
  `Supported`, `Bounded`, `Experimental`, and `Not supported` states.

SmartPLS currently establishes a commercial benchmark through Windows and
macOS distribution, an experimental Linux build, a full-feature trial,
seat/floating licensing, documented release notes, support tickets, tutorials,
and training resources. QuickPLS does not need to copy that business model, but
its stable distribution, support, and claims must be equally unambiguous. See
the official [downloads](https://smartpls.com/downloads/),
[license options](https://smartpls.com/faq/license-and-installation/license-options/),
[release notes](https://www.smartpls.com/release_notes/),
[support](https://smartpls.com/support/), and
[documentation](https://www.smartpls.com/documentation/).

## Release Channels

| Channel | Audience | Policy |
| --- | --- | --- |
| Internal | Maintainers | CI/nightly evidence; never advertised or offered as publication-ready. |
| Beta | Named external testers | Signed prerelease, visible beta label, separate update manifest, known-issue disclosure. |
| Stable | Public users | Exact artifacts that passed this contract; immutable tag, notes, checksums, SBOM, and provenance. |

An institutional or long-term-support channel is a later maturity feature. A
stable release is never rebuilt after acceptance: test one candidate, sign it,
record its identity, and promote those exact hashes.

## Minimum Competitor-Grade Gate

Every item below is release-blocking.

### 1. Signing and trusted distribution

- Obtain an organization-appropriate CA-trusted Authenticode certificate or a
  managed Windows signing identity.
- Sign and RFC 3161 timestamp the desktop executable, CLI executable, and final
  NSIS installer with SHA-256.
- Verify publisher, chain, timestamp, and all signatures with Windows
  SignTool. Keep signing credentials outside the repository and require
  human-reviewed release authorization.
- Produce checksums only after signing. Preserve the exact signed hashes used
  by clean-machine acceptance.

### 2. Installer lifecycle

- Prove clean offline installation and first launch on the oldest supported
  Windows build and the current fully patched Windows release, under standard
  and administrator accounts.
- Prove import/sample, calculation, export, save, close, and reopen on the
  installed build.
- Prove N-1 upgrade, settings and project preservation, failed-update recovery,
  uninstall cleanup, user-project preservation, and portable/install
  coexistence.
- Bundle or otherwise explicitly satisfy WebView2 for the documented offline
  installation promise.
- Document and test unattended installation and deterministic exit codes for
  university deployment.

### 3. Updates and recovery

- Use separately protected Tauri updater signing keys and distinct beta/stable
  manifests.
- Require package signature and hash validation, prohibit automatic downgrade,
  retain a full-installer fallback, and test interrupted-update recovery.
- Do not perform a network request at launch unless the user has explicitly
  enabled it. Disclose that update hosts can receive ordinary server metadata
  such as IP address and request time.

### 4. Support, diagnostics, and documentation

- Publish one monitored support channel, severity definitions, response
  targets, supported-version policy, known issues, rollback instructions, and
  a private security-reporting route.
- Provide a user-initiated diagnostic bundle with rotating local logs. Exclude
  dataset contents, project contents, and full personal paths by default.
- Keep system requirements, installation, update/uninstall, method scope,
  tutorials, citation guidance, release notes, and troubleshooting versioned.

### 5. Privacy, security, legal, and supply chain

- Verify the packaged application's network behavior and publish a privacy
  notice for the offline product, manual update checks, website logs, and
  support submissions.
- Threat-model hostile CSV/XLSX/project archives, archive traversal and bombs,
  formula injection, unsafe HTML/SVG export, file paths, Tauri IPC, updater
  compromise, and release-key compromise.
- Run dependency/advisory, secret, and license checks; produce an SBOM and
  release provenance/attestation from a protected clean build.
- Obtain legal review of the EULA, proprietary source-available license,
  third-party notices, privacy notice, trademark position, comparative claims,
  warranty/liability terms, and institutional distribution terms.

### 6. Independent scientific review

Before external beta exit, commission independent scientific review of the
target release's high-risk method families. Reviewers must have recorded
relevant methodological qualifications, disclose conflicts, and document the
exact version, method scope, evidence examined, findings, and disposition.
Every scientifically material finding must be closed, treated as
release-blocking, or resolved by narrowing the supported claim. Independent
review complements but never replaces executable reference, boundary,
persistence, and packaged-workflow evidence.

### 7. External beta

Use a closed cohort containing both experienced SmartPLS users and researchers
new to SEM software. The initial acceptance target is 15-25 participants from
at least five independent institutions or research groups completing at least
30 privacy-safe real workflows.

Beta exit requires:

- no unresolved P0 or P1 defect;
- no reproducible data loss or archive corruption;
- all scientifically material discrepancies fixed or explicitly explained;
- at least 90% completion of designated core journeys without developer
  intervention; and
- a final signing and clean-machine lifecycle rerun after beta fixes.

The cohort is product-acceptance evidence, not a substitute for scientific
method qualification.

## Required Evidence Artifacts

The machine-readable contract is
[`validation/quickpls_3_release_readiness.json`](../validation/quickpls_3_release_readiness.json).
Each passed requirement must cite its own repository-relative JSON evidence
record. One record cannot be reused for another requirement. URLs and external
attestation identifiers are supplementary only and cannot cause a requirement
to pass.

Evidence JSON is strict: duplicate keys, `NaN`, and infinities are rejected.
The record must bind `requirement_id`, `target_release`, `passed: true`, a
timezone-aware `performed_at`, nonempty `scope` and `summary`, and the artifact
or reviewer identity required for that requirement. Evidence older than 365
days or materially future-dated is rejected. Each contract acceptance
criterion has a stable `acceptance_check_ids` entry; evidence must contain one
successful, nonempty `criterion_results` row for every exact check ID.

Artifact evidence does not trust a declared checksum. Its identity is an exact
lowercase 64-hex SHA-256 and its `{path, size, sha256}` descriptor is verified
against the repository bytes. Signing, installer, updater, SBOM, and provenance
gates additionally bind the same `candidate_id` and candidate-manifest
descriptor. The strict manifest identifies exactly one desktop, CLI, installer,
updater bundle, SBOM, and provenance artifact set, and the validator rehashes
every listed file. `candidate_id` is the canonical SHA-256 identity of the
target release plus the desktop, CLI, installer, and updater-bundle sizes and
digests; the manifest has its own separately verified descriptor. Cross-
candidate evidence is rejected.

Candidate semantics are also validated. Desktop, CLI, and installer files must
be x64 PE32+ executables with bounded headers and a nonempty embedded
WIN_CERTIFICATE table. For every distinct candidate PE referenced by passed
candidate-scoped evidence, the production validator locates Windows SignTool
and reruns `verify /pa /all /v /tw` at validation time. A missing tool, nonzero
exit, untrusted result, warning, absent timestamp, absent publisher, file change,
or mismatch between the normalized live output and its strict JSON report fails
closed. The report binds the exact PE and candidate hashes, command, exit code,
full normalized verification output and its SHA-256, publisher, and timestamp.
There is no contract field or command-line option that bypasses this live trust
check. The all-pending baseline remains runnable without SignTool because it
contains no passed candidate evidence and therefore makes no authenticity claim.

The updater must be a bounded, traversal-safe ZIP containing only the exact
signed installer payload. SBOM and provenance must be strict JSON bound to the
release, candidate ID, distribution hashes, build identity/times, source commit,
and SBOM digest. Release reports must be produced from the trusted signing
service and the validation-time SignTool run, never hand-authored.

Expected records include:

- Authenticode and timestamp verification report;
- signed artifact manifest with post-signing SHA-256 identities;
- clean install, upgrade, recovery, uninstall, and unattended-install reports;
- updater signature/channel/rollback report;
- support, version-support, privacy, security, and legal review records;
- SBOM, dependency/license audit, and build-provenance attestation;
- independent scientific-review qualification, conflict, scope, disposition,
  and material-finding closure record;
- external-beta cohort, journey, discrepancy, and defect-closure report; and
- final release-decision record naming the approver and approval time.

Artifact-bound evidence uses:

```json
{
  "schema_version": 1,
  "requirement_id": "signing.artifacts",
  "target_release": "3.0.0",
  "passed": true,
  "performed_at": "2026-08-13T12:00:00+05:30",
  "scope": "Desktop, CLI, installer, timestamp, and trust-chain verification",
  "summary": "All exact release-candidate binaries passed SignTool verification.",
  "criterion_results": [
    {
      "check_id": "signing.artifacts.signatures_timestamped",
      "passed": true,
      "result": {
        "summary": "Desktop, CLI, and installer signatures and RFC 3161 timestamps verified.",
        "measurements": {"verified_files": 3, "failures": 0}
      }
    },
    {
      "check_id": "signing.artifacts.signtool_verified",
      "passed": true,
      "result": {
        "summary": "SignTool trust and timestamp verification completed without warnings.",
        "measurements": {"exit_code": 0, "warning_count": 0}
      }
    },
    {
      "check_id": "signing.artifacts.post_signing_hashes",
      "passed": true,
      "result": {
        "summary": "Accepted hashes match the signed clean-machine test artifacts.",
        "measurements": {"hash_mismatches": 0}
      }
    }
  ],
  "artifact_identity": {
    "name": "QuickPLS 3 signing verification report",
    "identifier": "<lowercase-64-hex-report-sha256>",
    "artifact": {
      "path": "validation/results/quickpls_3_signing_verification_output.json",
      "size": 1234,
      "sha256": "<same-lowercase-64-hex-report-sha256>"
    }
  },
  "candidate_id": "<lowercase-64-hex-distribution-set-sha256>",
  "candidate_manifest": {
    "path": "validation/results/quickpls_3_candidate_manifest.json",
    "size": 2345,
    "sha256": "<lowercase-64-hex-manifest-file-sha256>"
  }
}
```

Human-review requirements replace `artifact_identity` with
`reviewer_identity`. It must bind nonempty `name`, `role`, `organization`,
`independence`, `conflict_disclosure`, `disposition`, `reviewed_scope`, and a
stable `record_id`. Independent scientific review specifically requires
`independence: "independent"`; a passed review disposition can only be
`approved` or `approved_with_closed_findings`. The validator assigns the
required identity type per requirement so a generic file or the wrong kind of
review cannot satisfy the gate.

Final approval is also strict JSON. It must occur at or after the newest
evidence timestamp and bind the stable channel, complete requirement set,
unique evidence record paths and hashes, shared candidate ID, and a canonical
digest of the contract inputs used to compute `competitor_ready`.

Confidential source documents may stay outside the repository, but the checked
in evidence record must identify the reviewer, date, scope, disposition, and a
stable internal reference without exposing secrets or personal data.

## Validation

The repository and CI command validates the contract and reports pending work
without pretending that the product is competitor-ready:

```powershell
python validation/quickpls_3_release_readiness.py
python -m unittest validation.test_quickpls_3_release_readiness
```

The final launch gate adds `--require-ready` and deliberately returns a nonzero
exit code until every mandatory requirement is passed with existing evidence
and the release decision is approved:

```powershell
python validation/quickpls_3_release_readiness.py --require-ready
```

## Safe Public Claim After The Gate

> QuickPLS is an independently implemented, fully offline Windows competitor
> to SmartPLS for the analytical workflows documented as supported. It does
> not claim complete feature parity, identical undocumented behavior, or
> SmartPLS project compatibility.

Broader claims remain prohibited until the method compatibility ledger and the
commercial-readiness contract both support them.
