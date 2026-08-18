# Packaged Windows Acceptance V2

The packaged Windows gate is defined by
`validation/capabilities/packaged_windows_acceptance_v2.manifest.json`.
The manifest owns every required check ID, grouped by the full run and focused
stages. A numeric total is derived for receipts and display only; matching the
total cannot satisfy the gate when an ID is missing or replaced.

`validation/packaged_windows_acceptance_v2.py` validates the manifest, exposes
the exact check set to Python release adapters, and can compare a packaged
report with the required IDs. The cumulative PowerShell supervisor reads the
same manifest directly. Its schema-v2 receipt records the manifest identity,
version, required count, and file SHA-256.

This design means:

- adding or removing a packaged obligation requires an explicit manifest edit;
- duplicate, missing, and unexpected check IDs fail closed;
- Phase-2 method checks must be a subset of the complete manifest;
- all release adapters consume the same derived contract; and
- an old receipt cannot qualify a run after the contract changes.

Run the source and current-report checks with:

```powershell
python validation\packaged_windows_acceptance_v2.py
python validation\packaged_windows_acceptance_v2.py --report validation\results\v247_tauri_native_acceptance.json
python -m unittest validation.test_packaged_windows_acceptance_v2 validation.test_v247_cumulative_native_acceptance_supervisor
```

The checked-in report predates the schema-v2 receipt contract. It remains
useful as a fixture for the exact check-ID set, but a new packaged execution is
required before any future evidence refresh. Product-finalization work does not
reuse the older numeric-only receipt.
