# Legacy continuous-moderation V1 qualification

This regression gate protects the existing continuous-moderation V1 behavior;
it does not migrate, widen, or reinterpret that behavior as a MultiMod V2
profile.

The immutable input is
`validation/fixtures/v255/archives/continuous-moderation-legacy-result.qpls`
(28,542 bytes; SHA-256
`283185ebc4f4c51fc5643c0b64c77a9099c7a5231683d07f10f3848845bc514d`).
The gate binds all of these identities:

- result `fa5ac33f-4b8a-458b-a889-7b005d95b403`;
- recipe `fd7ae1a4-fbd2-4b2a-b021-6ee590fa38a2` (legacy recipe schema 3);
- dataset `103c2aea-494c-4273-b85b-33c7fbc189a3`, fingerprint
  `v2:4461c776997c9e4723276a54dc1f5ac95c1380fc78047271f28fdfe52006d1f7`;
  and
- persisted result table `moderation_simple_slopes`, backed by exactly one
  moderation estimate with three simple-slope probes.

The candidate CLI is explicitly rebuilt from the frozen candidate commit. The
gate first reuses `validation/v255_named_archive_identity.py` to project and
validate the archived table identity. It then runs the public command, without
an Experimental or internal-qualification switch:

```text
qpls run <archive.qpls> --recipe-id fd7ae1a4-fbd2-4b2a-b021-6ee590fa38a2 --output <result.json>
```

The verifier compares the complete scientific `payload`: object fields,
arrays, ordering, types and nonnumeric values must be exact, while finite
numeric values have an absolute tolerance of `1e-12`. It also requires exact
recipe ID, dataset fingerprint, method, method version, seed and settings.
Only the newly generated result ID, engine version and execution timestamps are
excluded. A missing CLI, changed archive, identity mismatch, failed replay,
shape difference or out-of-tolerance value fails the gate.

Passing this gate is regression evidence only. Its serialization evidence is
bounded to the frozen V5 result, the tested absent-MultiMod Recipe V4 wire
shape, the tested empty additive inventory in a pre-MultiMod V6 document, and
the frozen pre-MultiMod canonical-export hashes. It does not qualify the wider
unbound legacy archive universe or any new MGA, latent-segmentation,
conditional-process or causal-mediation profile.
