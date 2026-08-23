# QuickPLS 2.55.3 Release Notes

QuickPLS 2.55.3 adds editable routing for construct–indicator measurement
connectors. A connector can use Straight, Curved, Orthogonal, or Polyline
routing, and routing can be changed for one indicator or all indicators on a
construct. Polyline bend points can be edited, undone, and saved with the
diagram.

These settings are presentation-only. They do not change indicator assignment,
measurement direction, construct identity, structural paths, estimates, or
stored analytical results. Completed-result and publication diagrams remain
read-only.

This release also retains the 2.55.2 Results → Edit Model fix, which returns the
source model to an editable SEM canvas, and the Corporate Reputation sample
introduced in 2.55.1.

The source authorities and embedded Windows version metadata are coordinated at
2.55.3. An unsigned local NSIS installer was built at
`target/release/bundle/nsis/QuickPLS_2.55.3_x64-setup.exe` (217.85 MiB; SHA-256
`bd88a2d15a5ebeacb91279095c806b92c2b7eda79234bda3d59a9cbde52978d1`).
It passed the source, frontend, native sample, and packaging checks documented
in the development handoff. It has not undergone the separate installed and
portable smoke/evidence workflow required for a formally qualified public
candidate, and it is not code-signed.
