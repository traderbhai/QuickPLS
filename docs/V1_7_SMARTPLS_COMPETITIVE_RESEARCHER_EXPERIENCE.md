# QuickPLS v1.7 SmartPLS-Competitive Researcher Experience

QuickPLS v1.7 is a product and frontend program. It improves researcher trust, workflow continuity, SEM diagram usability, result reportability, research tables, publication packaging, and sample workflows without changing statistical engines, result schemas, project archives, or numerical fingerprints.

## Scope

- v1.7.0 adds method-scope transparency and run confidence panels.
- v1.7.1 strengthens the Data -> Model -> Calculate -> Results -> Report workflow.
- v1.7.2 adds Focus Diagram mode and publication-oriented diagram checks.
- v1.7.3 adds a canonical PLS-SEM reportability checklist.
- v1.7.4 adds reusable research-table infrastructure and table affordance checks.
- v1.7.5 adds Reviewer Pack export workflow support.
- v1.7.6 adds sample-project gallery and guided Start from dataset flow.

## Product Rules

- QuickPLS remains offline and proprietary.
- R/Rscript and external engines remain validation-only.
- No SmartPLS project import or SmartPLS equivalence claim is made.
- Threshold colors are guidance, not universal pass/fail rules.
- Default numeric exports remain clean; interpretation notes are explicit opt-in or Reviewer Pack output.
- SVG remains the audited figure export path.

## Evidence

The v1.7 program is gated by static product audits under `validation/results/`:

- `v170_confidence_scope_audit.json`
- `v171_workflow_audit.json`
- `v172_sem_designer_audit.json`
- `v173_reportability_audit.json`
- `v174_research_tables_audit.json`
- `v175_publication_pack_audit.json`
- `v176_samples_guided_audit.json`
- `v17_researcher_experience_audit.json`

Final gate:

```powershell
cargo run -p qpls-cli -- gate v1_7_smartpls_competitive_researcher_experience
```
