# QuickPLS Supply-Chain And Release Evidence Policy

## Reproducible Inputs

- JavaScript and TypeScript dependencies use `package-lock.json` and `npm ci` in CI.
- Rust dependencies use the committed workspace `Cargo.lock`; release builds use
  locked resolution.
- Python validation dependencies use the hash-pinned
  `validation/requirements.txt`.
- Release workflows use reviewed, version-pinned actions and record Node, npm,
  Rust, Cargo, Python, Tauri, NSIS, Windows SDK, and SignTool versions.

Lockfiles provide deterministic resolution; they do not prove that dependencies are
safe or legally compatible.

## Required Scans

Every release candidate must run dependency-advisory, secret, and license checks for
the shipped application and its build pipeline. Critical and high advisories require
a fix, a release-blocking disposition, or a documented reviewed determination that
the vulnerable path is not present in the shipped scope. Suppression identifiers,
rationale, reviewer, and expiry must be recorded.

Generated dependencies, installers, binaries, source maps, and archives must not
contain signing material, access tokens, private keys, credentials, or unintended
development paths.

## SBOM

The exact candidate has a machine-readable CycloneDX SBOM containing the QuickPLS
application, shipped runtime components, versions, package identifiers where
available, licenses, and dependency relationships. It binds the target release,
candidate ID, and final distribution hashes. Third-party notices are reviewed
against the SBOM; neither one substitutes for the other.

## Provenance

Signed release provenance is an in-toto Statement v1 with a SLSA provenance v1
predicate. It records the clean source commit, protected main-branch workflow and
run identity, toolchain, lockfile hashes, build start and finish times, distribution
artifact subjects, CycloneDX hash, approved signing identity, signed protected-build
attestation, and final candidate ID. The exact tested and signed hashes are promoted
without rebuilding or overwriting an existing version.

## Signing And Secret Custody

QuickPLS 3 freezes one approved leaf release signer into every candidate. It signs
the Authenticode PEs and detached-CMS channel/build-attestation documents; every use
is leaf-subject and thumbprint bound. Private material is never stored in the
repository, frontend bundle, build log, test fixture, or release archive. Stable
signing requires hardware-backed or managed least-privilege access and human-
reviewed authorization.
Checksums, SBOM, provenance, and candidate manifests are finalized after the release
binaries have reached their accepted signed identity.

Unsigned artifacts are labeled internal or unsigned preview and cannot satisfy the
commercial signing, provenance, or promotion gates.

## Evidence And Review

For the target release, preserve scan outputs, dispositions, license inventory,
SBOM, provenance, candidate manifest, signing verification, checksums, workflow run,
and immutable tag/release identifiers. Each machine-readable artifact is hashed and
bound to the same candidate. Qualified legal review of licenses and notices remains
separate and mandatory.

## Implementation Commands

- `npm run qpls:release:artifacts` preserves the exact internal or unsigned-preview
  binaries under unique names and records their byte identities.
- `npm run qpls:release:supply-chain` rehashes those preserved files and produces a
  CycloneDX 1.6 SBOM, machine-readable license inventory, source/build provenance,
  and a binding report. It refuses beta or stable input because those channels must
  use the signed-candidate factory.
- Dirty-source and unsigned status are recorded, never hidden. Evidence generated
  before signing is review material only and cannot satisfy the commercial SBOM,
  provenance, signing, or competitor-claim gates.
- `python validation/quickpls_signed_candidate.py ...` is the protected beta/stable
  path. It produces an actual CycloneDX 1.6 SBOM with package URLs and a complete
  dependency graph, an in-toto/SLSA provenance statement, a signed protected-build
  attestation, and a signed channel manifest. It fails while the approved signer or
  protected workflow is absent.
