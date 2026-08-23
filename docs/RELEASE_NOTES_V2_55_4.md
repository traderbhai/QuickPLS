# QuickPLS 2.55.4 Release Notes

QuickPLS 2.55.4 centers measurement loading and weight badges on the exact
construct–indicator connector path. This removes the small above/below offset
that could make a value appear detached from its line.

The correction applies to Straight, Curved, Orthogonal, and Polyline routes in
Model, Results, and Publication diagrams. The badge still masks a short section
of the connector so the number remains legible, while the line enters and
leaves the badge on the same path.

This is a presentation-only correction. It does not change loading or weight
values, reflective/formative direction, indicator assignment, construct or
model identity, structural-path labels, estimates, exported result data, or stored results.
The editable measurement-connector routing introduced in 2.55.3 is retained.

This source also adds **Organizational Identification Model** as the fourth
built-in sample. It contains all 305 supplied cases, 22 variables (21 modeled
indicators plus the unassigned `gender` variable), four reflective constructs,
three structural paths, and a completed PLS-SEM result. No missing-value or
duplicate-row treatment was applied. QuickPLS matches all 27 values visible in
the supplied model screenshot after rounding to three decimals.

The source authorities remain coordinated at 2.55.4. An earlier unsigned local
NSIS installer with SHA-256
`9380af48bf3ed847ce744e5d68560f296ba27ab88264015c171fed187899dce1`
was built before the fourth sample was added. It is retained only as historical
local evidence: it was a tested local preview of that earlier source state, not
the current source state. A fresh
`QuickPLS_2.55.4_x64-setup.exe` installer and the repository's provenance-bound
installation, smoke, evidence, and publication gates are required before this
source can be treated as a public candidate.
