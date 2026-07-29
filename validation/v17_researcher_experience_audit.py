from v17_researcher_experience_common import aggregate

ARTIFACTS = [
    "v170_confidence_scope_audit.json",
    "v171_workflow_audit.json",
    "v172_sem_designer_audit.json",
    "v173_reportability_audit.json",
    "v174_research_tables_audit.json",
    "v175_publication_pack_audit.json",
    "v176_samples_guided_audit.json",
]

raise SystemExit(0 if aggregate(ARTIFACTS, "v17_researcher_experience_audit.json") else 1)
