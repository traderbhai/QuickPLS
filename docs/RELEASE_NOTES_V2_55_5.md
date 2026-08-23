# QuickPLS 2.55.5 Release Notes

QuickPLS 2.55.5 expands the bundled sample gallery from four to seven editable
projects. The three additions reuse the existing 305-case Organizational
Identification dataset without embedding duplicate copies:

- **Organizational Identification - Mediation**
- **Organizational Identification - Moderated Mediation (Point Topology)**
- **Organizational Identification - Higher-Order**

Each sample opens as an ordinary editable project and includes a deterministic
completed current-engine result. The moderated-mediation sample records point
estimates only; conditional-effect inference requires a separately qualified
bootstrap workflow. The
higher-order sample uses the supported reflective-reflective disjoint two-stage
approach. Evidence metadata states these boundaries explicitly and does not
claim exact screenshot parity where the QuickPLS specification differs.

The sample gallery is now driven by one versioned JSON catalog shared by the
React launcher and Rust native project builder. The catalog declares dataset
identity, model topology, deterministic run settings, canvas layout,
provenance, and acceptance evidence once. Future samples that reuse an embedded
dataset no longer need separate frontend and backend registry entries.

The source authorities are coordinated at 2.55.5. A fresh
`QuickPLS_2.55.5_x64-setup.exe` installer and provenance-bound setup, portable,
CLI, checksum, install, and smoke evidence must be produced from this source.
No earlier QuickPLS installer qualifies as a 2.55.5 candidate.
