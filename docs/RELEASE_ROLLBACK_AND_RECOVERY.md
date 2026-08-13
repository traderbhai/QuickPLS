# QuickPLS Release Rollback And Recovery

## Purpose

This runbook governs withdrawal, rollback, and recovery for a defective QuickPLS
release. It protects user projects before restoring application availability.

## Rollback Triggers

The release manager stops promotion and evaluates withdrawal when any of the
following is confirmed:

- a P0 or P1 defect;
- reproducible project loss, corruption, or incompatible rewrite;
- a materially incorrect result inside advertised supported scope;
- a compromised or incorrectly signed artifact or updater manifest;
- an installer, upgrade, or uninstall defect without a safe recovery path;
- an undisclosed launch-time network request or material privacy regression.

## Immediate Containment

1. Freeze the affected channel and record the release version, tag, candidate ID,
   checksums, updater manifest, discovery time, and decision owner.
2. Remove the affected updater manifest from promotion. Do not replace files under
   an existing version or reuse an accepted filename.
3. Preserve the affected artifacts and logs for investigation; do not rebuild them
   in place.
4. Publish a concise advisory identifying affected versions, symptoms, user action,
   and whether users should stop opening or saving projects.
5. Direct security-sensitive communication to the private advisory route.

## User Data Protection

Application rollback never deletes user projects. Before opening a project with a
different release, the user should copy the `.qpls` project and its exports to a
separate location. Recovery guidance must distinguish application binaries from
user-owned data, settings, autosave state, and exported results.

An older version must not overwrite a project written by a newer unsupported
schema. If compatibility is not proven, the recovery procedure is export-only or
copy-first and the limitation must be explicit.

## Recovery Paths

Use the first safe path that has evidence for the affected release:

1. Install a corrected patch built from a new commit and new immutable candidate.
2. Use the last known-good full installer when N-1 compatibility has passed.
3. Use the portable last known-good build against a copied project when installed
   state is suspected, without sharing mutable settings with the defective build.
4. Restore from the user's original project, autosave, or backup only after the
   recovery copy has been preserved.

Updater failure must always retain a manually downloadable full-installer fallback.
Each beta/stable updater archive carries that installer together with the exact
detached-CMS signed channel manifest. Recovery verifies the frozen QuickPLS leaf
signer, manifest and installer hashes, channel, version floor, and no-downgrade
policy before use. Automatic downgrade is prohibited.

## Requalification

A replacement build is a new candidate. It must repeat all affected numerical,
archive, installer, updater, network, signing, and packaged-workflow checks. If any
binary changes, its candidate ID, hashes, signatures, SBOM, provenance, and clean-
machine evidence must also change.

## Closure Record

The incident closes only after the record identifies root cause, affected versions,
user impact, recovery validation, fixed candidate, regression coverage, remaining
limitations, disclosure decision, and release-manager approval. A rollback rehearsal
against the exact candidate is required before external beta exit.
