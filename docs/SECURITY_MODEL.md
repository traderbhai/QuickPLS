# QuickPLS Security Model

## Security Scope

QuickPLS is an offline-first Windows desktop application built with Rust, Tauri,
React, and TypeScript. The principal trust boundaries are user-selected files,
project archives, exported documents, local application state, Tauri commands,
optional update requests, build infrastructure, and signing credentials.

This document defines required controls and verification work. It is not an
independent security review, and it does not close `trust.security` or
`trust.legal` in the commercial-readiness contract.

## Protected Assets

- Original datasets and QuickPLS project files.
- Saved models, recipes, results, recovery state, and exports.
- Numerical integrity and the truthful supported-scope status of outputs.
- Local filesystem privacy and diagnostic data.
- Release artifacts, updater manifests, provenance, and signing keys.

## Trust Boundaries And Required Controls

| Boundary | Threats | Required controls | Release evidence still required |
| --- | --- | --- | --- |
| CSV, TSV, XLSX, SAV, matrix input | malformed input, excessive dimensions, formula payloads, resource exhaustion, misleading types | bounded size/dimensions, strict parser errors, finite-number checks, explicit type inference, no active-content rendering | hostile fixture suite and installed-build resource/boundary report |
| `.qpls` and ZIP archives | path traversal, absolute paths, symlinks, duplicate names, decompression bombs, schema confusion, tampering | traversal-safe entry names, count/size/compression bounds, strict schemas, versioned payloads, atomic save, unknown-field policy | archive mutation corpus, save/reopen, corruption and recovery evidence |
| CSV/XLSX export | spreadsheet formula injection and data leakage | escape formula-leading cells, explicit selected tables, user-selected destination, no hidden dataset attachment | exported-file inspection and hostile-cell tests |
| HTML/SVG export and previews | script execution, event attributes, unsafe links, external resources | no untrusted raw HTML, safe text encoding, active-content allowlist, no remote script/resource inclusion | XSS/active-export corpus and packaged inspection |
| File paths and dialogs | traversal, unintended overwrite, personal-path disclosure | native user selection, extension allowlist, canonical destination checks where applicable, atomic replace, path redaction in diagnostics | Windows path boundary tests and failure recovery |
| Tauri IPC | unintended privileged command access and untrusted payloads | minimum capability allowlist, typed request validation in Rust, no shell execution from renderer input, bounded payloads | command inventory review, malformed request tests, capability audit |
| Optional updater | manifest substitution, downgrade, interruption, metadata disclosure | separately protected updater key, signature/hash validation, distinct channels, no automatic downgrade, manual checks by default, full-installer fallback | exact-candidate updater and network-behavior tests |
| Build and dependencies | dependency compromise, leaked secret, non-reproducible release | lockfiles, clean protected build, advisory/license/secret scans, SBOM, provenance, immutable hashes | target-release SBOM, scan disposition, provenance and promotion record |
| Authenticode identity | private-key theft, unauthorized signing, timestamp or publisher mismatch | hardware-backed or service-managed key, least privilege, human release approval, RFC 3161 timestamp, live SignTool verification | CA identity and exact signed-candidate reports |

## Frontend Security Baseline

- Secrets and signing material must never enter frontend code, Vite variables,
  logs, fixtures, or the repository.
- User-controlled values are rendered through React escaping. Raw HTML insertion,
  string-to-code execution, and active uploaded HTML/SVG are prohibited unless a
  narrowly reviewed sanitizer and policy are introduced.
- External navigation and network destinations require an explicit protocol and
  host allowlist. QuickPLS must not make authenticated requests to user-controlled
  destinations.
- The production Tauri content-security policy and capability manifest must remain
  restrictive; `unsafe-eval`, broad remote script sources, and unnecessary IPC
  permissions are release blockers.

## Logging And Diagnostics

Logs must be local, bounded, rotating, and free of dataset values, project content,
secrets, and full personal paths by default. Diagnostic export is user-initiated,
shows a preview, applies redaction before serialization, and never transmits itself.
The detailed contract is in [`DIAGNOSTICS_AND_LOGGING.md`](DIAGNOSTICS_AND_LOGGING.md).

## Vulnerability Handling

Suspected vulnerabilities are reported through a private GitHub security advisory.
P0/P1 handling, disclosure, and escalation follow
[`SUPPORT_POLICY.md`](SUPPORT_POLICY.md). A release security review must record the
reviewer, scope, findings, disposition, dependency/advisory status, and closure of
every release-blocking finding.

## Verification Status

The threat model is defined, but release verification remains pending. The
commercial security gate passes only after an appropriately qualified reviewer
assesses the exact release scope and the target candidate passes the required
hostile-input, capability, dependency, secret, license, updater, and packaged-
network tests.
