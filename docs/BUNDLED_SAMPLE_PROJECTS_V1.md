# Bundled sample projects v1

QuickPLS bundled samples are defined by one versioned manifest:

`src/data/bundledSampleProjects.v1.json`

The TypeScript launcher/onboarding surfaces and the Rust desktop project builder consume the same document. This prevents the sample names, IDs, model definitions, result settings, and acceptance counts from drifting across separate hard-coded lists.

## Runtime flow

1. The frontend validates the manifest and renders each `samples[]` entry as a launcher card.
2. The selected stable sample ID is sent to `open_demo_project`.
3. Rust validates the same manifest, resolves the referenced embedded dataset, verifies its SHA-256 and optional QuickPLS fingerprint, and constructs the declared model.
4. Every declared run is executed by the current engine and appended through `Project::append_validated_result`.
5. The project is hydrated as an ordinary editable QuickPLS project with dataset lineage, layout, recipe, provenance, and completed results.

The catalog may declare multiple runs per sample. Current entries use one deterministic point-estimate run, but the array avoids a future schema change for samples that need a point run plus qualified inference.

## Adding a sample

For a new model that reuses an existing bundled dataset:

1. Add one `samples[]` entry to the manifest.
2. Reuse an existing `constructSetId` or add a reusable construct set.
3. Declare stable model/recipe UUIDs, explicit settings, paths, interactions or HOC metadata, layout positions, and acceptance counts.
4. Add a small reference JSON when numerical parity is claimed.
5. Run the catalog-wide Rust and frontend tests.

No Rust or React list must be edited for a sample that reuses an embedded dataset.

For a genuinely new dataset, also add one literal `include_bytes!` mapping in `src-tauri/src/sample_projects.rs`. Rust requires a compile-time literal path for an offline embedded asset. Do not duplicate the same CSV for several models; reference one dataset ID instead.

## Scientific and provenance rules

- A sample is an editable copy, not a frozen `.qpls` archive.
- Model and recipe UUIDs, settings, seed, worker count, data hash, and reference precision are explicit.
- Runtime metadata records the catalog schema, sample/version IDs, dataset asset, fixture hash, and scientific template hash.
- Screenshot values are not silently treated as full-precision truth. Every reference states whether it proves displayed-value parity, QuickPLS-current reproducibility, or only a bounded comparison.
- Generated moderation/HOC indicators remain technical result artifacts; they are never added to the raw dataset or authored measurement model.
- A sample must pass current project validation. The catalog builder does not bypass unsupported capability guards.

## Required checks

The backend catalog tests validate all entries, asset hashes, dimensions, identities, endpoints, generated-term declarations, completed results, numeric references, and save/reopen round trips. Frontend tests validate runtime IDs and manifest-driven cards. Release acceptance reads the same catalog rather than maintaining a fixed list of sample IDs.
