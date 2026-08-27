# Bundled sample projects v1

QuickPLS bundled samples are defined by one versioned manifest:

`src/data/bundledSampleProjects.v1.json`

The TypeScript launcher/onboarding surfaces and the Rust desktop project builders consume the same document. This prevents the sample names, IDs, model definitions, result settings, project authority, and acceptance counts from drifting across separate hard-coded lists.

## Runtime flow

1. The frontend validates the manifest and renders each `samples[]` entry as a launcher card.
2. The frontend reads each sample's validated `projectKind` (`ordinary_v1` by default or `general_sem_v1`) and dispatches the selected stable ID to the matching native builder.
3. Rust validates the same manifest, resolves the referenced embedded dataset, verifies its SHA-256 and optional QuickPLS fingerprint, and constructs the declared model.
4. Ordinary samples execute the declared run and append it through `Project::append_validated_result`; General SEM samples compile the exact promoted SemModelV4/RecipeV4 authority, execute the qualified capability cell, and append its canonical result to a fresh schema-6 archive.
5. Ordinary projects are hydrated as editable copies. A General SEM sample is strictly reopened from its freshly materialized archive, with the same immutable scientific-authority and Save As Revision lifecycle as any other schema-6 project.

The catalog may declare multiple runs per sample. Current entries use one deterministic point-estimate run, but the array avoids a future schema change for samples that need a point run plus qualified inference.

## Adding a sample

For a new model that reuses an existing bundled dataset:

1. Add one `samples[]` entry to the manifest.
2. Reuse an existing `constructSetId` or add a reusable construct set.
3. Declare `projectKind` when the model requires General SEM; otherwise omit it for the `ordinary_v1` default.
4. Declare stable model/recipe UUIDs, explicit settings, paths, interactions or HOC metadata, layout positions, and acceptance counts.
5. Add a small reference JSON when numerical parity is claimed.
6. Run the catalog-wide Rust and frontend tests, including strict archive materialization/reopen checks for General SEM samples.

No Rust or React card list must be edited for a sample that reuses an embedded dataset. New project-authority kinds still require one reviewed builder/launcher implementation before they can be declared in the manifest.

For a genuinely new dataset, also add one literal `include_bytes!` mapping in `src-tauri/src/sample_projects.rs`. Rust requires a compile-time literal path for an offline embedded asset. Do not duplicate the same CSV for several models; reference one dataset ID instead.

## Scientific and provenance rules

- An ordinary sample is an editable copy. A `general_sem_v1` sample is a newly materialized strict archive whose scientific authority is immutable; users edit it through the normal Save As Revision workflow.
- Model and recipe UUIDs, settings, seed, worker count, data hash, and reference precision are explicit.
- Runtime metadata records the catalog schema, sample/version IDs, dataset asset, fixture hash, and scientific template hash.
- Screenshot values are not silently treated as full-precision truth. Every reference states whether it proves displayed-value parity, QuickPLS-current reproducibility, or only a bounded comparison.
- Generated moderation/HOC indicators remain technical result artifacts; they are never added to the raw dataset or authored measurement model.
- A sample must pass the validation and persistence contract for its declared project kind. The catalog builders do not bypass unsupported capability guards or route a General SEM model through the ordinary estimator.

## Required checks

The backend catalog tests validate all entries, asset hashes, dimensions, identities, endpoints, generated-term declarations, completed results, numeric references, and ordinary or strict-schema-6 reopen round trips. Frontend tests validate runtime IDs, project-kind dispatch, and manifest-driven cards. Release acceptance reads the same catalog rather than maintaining a fixed list of sample IDs.
