# QuickPLS v1.5.2 Data Workspace Hardening

QuickPLS v1.5.2 reorganizes the Data workspace around the researcher workflow: import a data source, inspect data quality, edit column metadata, and continue to the model designer.

The change is frontend-only. It does not alter statistical engines, dataset schemas, analysis recipes, result payloads, project numerical fingerprints, or method validation status.

## User-Facing Changes

- The Data screen now has clear Import Source, Data Quality, and Preview And Metadata zones.
- The old duplicate validation fixture actions are replaced by one `Load Sample Dataset` action.
- Raw, covariance, and correlation import modes have separate guidance and readiness text.
- Matrix import modes keep the existing dataset preview visibly labeled as the current loaded dataset.
- Data quality cards summarize rows, variables, missing cells, nonnumeric variables, constant columns, header issues, and sample-size readiness.
- The preview table supports variable search and metadata/quality filters.
- The selected-column metadata editor is grouped into Essentials, Bounds, and Actions.
- Prefix groups can be created from the Data workspace through `Create Constructs From Prefixes`.

## Evidence

- `validation/v152_data_workspace_smoke.mjs`
- `validation/v152_data_workspace_audit.py`
- `validation/results/v152_data_workspace_smoke.json`
- `validation/results/screens/v152/data-workspace/`

## Verification

```powershell
npm test -- --run
npm run build
npm run qpls:v152:data-smoke
npm run qpls:v152:data-audit
cargo run -p qpls-cli -- gate v1_5_2_data_workspace_hardening
```
