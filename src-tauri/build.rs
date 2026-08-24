use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

const AUTHORITY_ENV: &str = "QPLS_MULTIMOD_BUILD_CANDIDATE_AUTHORITY_V1";
const MANIFEST_SET_ENV: &str = "QPLS_MULTIMOD_BUILD_PREPACKAGE_MANIFEST_SET_V1";
const EMBEDDED_AUTHORITY_FILE: &str = "qpls_multimod_embedded_candidate_authority_v1.json";
const EMBEDDED_MANIFEST_SET_FILE: &str = "qpls_multimod_embedded_prepackage_manifest_set_v1.json";
const LABS_SENTINEL: &str = "{\"schema_version\":1,\"authority_kind\":\"qpls_multimod_embedded_candidate_authority_v1\",\"state\":\"labs_only\",\"binding\":null,\"authority_binding_sha256\":null}\n";
const EMPTY_MANIFEST_SENTINEL: &str = "{\"schema_version\":1,\"manifest_set_id\":\"qpls.v256.multimod.prepackage-authority-set.v1\",\"stage\":\"labs_preview\",\"state\":\"absent\",\"exact_profile_cells\":[]}\n";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateAuthorityDocumentV1 {
    schema_version: u32,
    authority_kind: String,
    state: String,
    binding: Option<CandidateAuthorityBindingV1>,
    authority_binding_sha256: Option<String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct CandidateAuthorityBindingV1 {
    candidate_commit_sha: String,
    candidate_version: String,
    qualification_plan_sha256: String,
    gate_binding_sha256: String,
    capability_index_sha256: String,
    prepackage_manifest_set_sha256: String,
    exact_profile_cells: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepackageAuthoritySetV1 {
    schema_version: u32,
    manifest_set_id: String,
    stage: String,
    state: String,
    surface: String,
    promotion_allowed: bool,
    candidate_commit_sha: String,
    candidate_version: String,
    plan_sha256: String,
    gate_binding_sha256: String,
    capability_index_sha256: String,
    generated_at_utc: String,
    non_circular_binding: NonCircularAuthorityBindingV1,
    manifests: Vec<PrepackageManifestRowV1>,
    exact_profile_cells: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NonCircularAuthorityBindingV1 {
    candidate_source_contains_evidence: bool,
    authority_generated_after_candidate_commit: bool,
    build_consumes_external_prepackage_set: bool,
    artifact_paths_relative_to_campaign_root: bool,
    campaign_root_ancestor_depth: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepackageManifestRowV1 {
    family_id: String,
    path: String,
    sha256: String,
    template_path: String,
    template_sha256: String,
    exact_profile_cells: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CapabilityIndexV1 {
    schema_version: u32,
    candidate_release: String,
    policy: CapabilityIndexPolicyV1,
    families: Vec<CapabilityIndexFamilyV1>,
}

#[derive(Debug, Deserialize)]
struct CapabilityIndexPolicyV1 {
    surface: String,
    evidence_state: String,
    promotion_allowed: bool,
}

#[derive(Debug, Deserialize)]
struct CapabilityIndexFamilyV1 {
    family_id: String,
    manifest: String,
    profiles: Vec<String>,
}

#[derive(Debug, Clone)]
struct TrackedCapabilityFamilyV1 {
    manifest: String,
    profiles: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct TrackedManifestProfileV1 {
    method_version: String,
    exact_profile_cells: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct TrackedManifestAuthorityV1 {
    source_paths: BTreeSet<String>,
    profiles: BTreeMap<String, TrackedManifestProfileV1>,
}

#[derive(Debug, Clone)]
struct GateStepAuthorityV1 {
    step_id: String,
    uses_cargo: bool,
    maximum_seconds: u64,
    expected_output_count: usize,
}

#[derive(Debug, Clone)]
struct GateBindingAuthorityV1 {
    profiles: Value,
    covered_evidence_cells: Value,
    probable_root_component: String,
    input_artifacts: Vec<String>,
    steps: Vec<GateStepAuthorityV1>,
}

#[derive(Debug, Clone)]
struct CampaignGateStateAuthorityV1 {
    receipt_path: String,
    receipt_sha256: String,
    input_digest: String,
    seed: u64,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn lower_sha(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn exact_cell(value: &str) -> bool {
    if value.is_empty()
        || value.trim() != value
        || value
            .chars()
            .any(|character| matches!(character, '*' | '?' | '[' | ']') || character.is_control())
    {
        return false;
    }
    let mut pieces = value.split("::");
    matches!((pieces.next(), pieces.next(), pieces.next()), (Some(profile), Some(procedure), None) if !profile.is_empty() && !procedure.is_empty())
}

fn exact_sorted_cells(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells.iter().all(|cell| exact_cell(cell))
        && cells.windows(2).all(|pair| pair[0] < pair[1])
}

fn star_pattern_matches(pattern: &str, value: &str) -> bool {
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();
    let mut previous = vec![false; value.len() + 1];
    previous[0] = true;
    for token in pattern {
        let mut current = vec![false; value.len() + 1];
        if *token == b'*' {
            current[0] = previous[0];
            for index in 1..=value.len() {
                current[index] = previous[index] || current[index - 1];
            }
        } else {
            for index in 1..=value.len() {
                current[index] = previous[index - 1] && *token == value[index - 1];
            }
        }
        previous = current;
    }
    previous[value.len()]
}

const PREPACKAGE_GLOBAL_GATES: [&str; 12] = [
    "contracts.schemas.source_identity",
    "core.project.compile",
    "typescript.contracts.typecheck",
    "estimation.point.kernels",
    "resampling.shared_ledgers",
    "runner.persistence.integration",
    "metamorphic.global",
    "archive.sidecar.integrity",
    "exports.semantic.readback",
    "legacy.continuous_and_serialization",
    "native.workflow.accessibility",
    "performance.maximum_profiles",
];

fn expected_prepackage_gate_ids(family_id: &str, profile_id: &str) -> Vec<&'static str> {
    let mut gates = PREPACKAGE_GLOBAL_GATES.to_vec();
    let profile_gates: &[&str] = match family_id {
        "qpls.multimod.mga_multigroup_v1" => &[
            "mga.group_matrix",
            "mga.inference.matrix",
            "mga.label_reversal",
        ],
        "qpls.multimod.pls_heterogeneity_v2" if profile_id.starts_with("fimix.") => &[
            "fimix.recovery",
            "fimix.collapse.boundaries",
            "heterogeneity.bootstrap",
        ],
        "qpls.multimod.pls_heterogeneity_v2" => &[
            "pos.recovery",
            "pos.common_metric",
            "heterogeneity.bootstrap",
        ],
        "qpls.multimod.general_sem_conditional_process_v2" => {
            gates.extend(["conditional.profile_matrix", "conditional.probes"]);
            if matches!(
                profile_id,
                "conditional.multi_two_way_bca.v2" | "conditional.studentized.v2"
            ) {
                gates.push("conditional.bca_studentized");
            }
            if matches!(
                profile_id,
                "conditional.multiple_hoc_percentile.v2"
                    | "conditional.grouped_percentile.v2"
                    | "conditional.case_weighted_percentile.v2"
                    | "conditional.frequency_weighted_percentile.v2"
            ) {
                gates.push("conditional.hoc_group_weight");
            }
            return gates;
        }
        "qpls.multimod.interventional_causal_mediation_v1" => {
            &["causal.known_targets", "causal.assumption_failures"]
        }
        _ => panic!("unsupported prepackage family/profile: {family_id}/{profile_id}"),
    };
    gates.extend(profile_gates.iter().copied());
    gates
}

fn read(path: &Path, label: &str) -> Vec<u8> {
    fs::read(path)
        .unwrap_or_else(|error| panic!("failed to read {label} {}: {error}", path.display()))
}

fn parse<T: for<'de> Deserialize<'de>>(bytes: &[u8], label: &str) -> T {
    serde_json::from_slice(bytes).unwrap_or_else(|error| panic!("invalid {label} JSON: {error}"))
}

fn git_head(repository: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap_or_else(|error| panic!("failed to execute git for candidate authority: {error}"));
    if !output.status.success() {
        panic!("git could not resolve the candidate HEAD for build-time authority");
    }
    String::from_utf8(output.stdout)
        .expect("git HEAD must be UTF-8")
        .trim()
        .to_owned()
}

fn git_worktree_is_clean(repository: &Path) -> bool {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(["status", "--porcelain=v1", "--untracked-files=all"])
        .output()
        .unwrap_or_else(|error| panic!("failed to inspect candidate worktree: {error}"));
    output.status.success() && output.stdout.is_empty()
}

fn object<'a>(value: &'a Value, label: &str) -> &'a serde_json::Map<String, Value> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("{label} must be a JSON object"))
}

fn array<'a>(value: &'a Value, label: &str) -> &'a Vec<Value> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("{label} must be a JSON array"))
}

fn text<'a>(value: Option<&'a Value>, label: &str) -> &'a str {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.trim() == *value)
        .unwrap_or_else(|| panic!("{label} must be nonempty text"))
}

fn boolean(value: Option<&Value>, label: &str) -> bool {
    value
        .and_then(Value::as_bool)
        .unwrap_or_else(|| panic!("{label} must be boolean"))
}

fn safe_existing_file(root: &Path, relative: &str, label: &str) -> PathBuf {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        panic!("{label} path escapes its authority root: {relative}");
    }
    let canonical_root = root
        .canonicalize()
        .unwrap_or_else(|error| panic!("failed to resolve {label} root: {error}"));
    let candidate = root.join(relative);
    let canonical = candidate
        .canonicalize()
        .unwrap_or_else(|error| panic!("failed to resolve {label} {relative}: {error}"));
    if !canonical.starts_with(&canonical_root) || !canonical.is_file() {
        panic!("{label} is not one regular file below its authority root: {relative}");
    }
    canonical
}

fn safe_absolute_campaign_file(root: &Path, raw: &str, label: &str) -> PathBuf {
    let path = Path::new(raw);
    if !path.is_absolute() {
        panic!("{label} must be an absolute campaign path: {raw}");
    }
    let canonical_root = root
        .canonicalize()
        .unwrap_or_else(|error| panic!("failed to resolve campaign root: {error}"));
    let canonical = path
        .canonicalize()
        .unwrap_or_else(|error| panic!("failed to resolve {label} {raw}: {error}"));
    if !canonical.starts_with(&canonical_root) || !canonical.is_file() {
        panic!("{label} is not one regular file below the campaign root: {raw}");
    }
    canonical
}

fn validate_tracked_manifest_template(
    repository: &Path,
    template: &serde_json::Map<String, Value>,
    expected_family: &TrackedCapabilityFamilyV1,
) -> TrackedManifestAuthorityV1 {
    let family_id = text(
        template.get("family_id"),
        "tracked manifest template.family_id",
    );
    if text(template.get("surface"), "tracked manifest template.surface") != "labs"
        || text(
            template.get("declared_evidence_state"),
            "tracked manifest template.declared_evidence_state",
        ) != "absent"
        || boolean(
            template.get("promotion_allowed"),
            "tracked manifest template.promotion_allowed",
        )
    {
        panic!("tracked manifest template must remain Labs/absent: {family_id}");
    }
    let source = object(
        template
            .get("source_binding")
            .expect("tracked manifest template source_binding"),
        "tracked manifest template.source_binding",
    );
    if text(
        source.get("status"),
        "tracked manifest template.source_binding.status",
    ) != "pending"
        || !source
            .get("candidate_commit_sha")
            .is_some_and(Value::is_null)
    {
        panic!("tracked manifest source binding must remain pending: {family_id}");
    }
    let mut source_paths = BTreeSet::new();
    for row in array(
        source
            .get("source_artifacts")
            .expect("tracked manifest source artifacts"),
        "tracked manifest source artifacts",
    ) {
        let row = object(row, "tracked manifest source artifact");
        let path = text(row.get("path"), "tracked manifest source artifact.path");
        if !row.get("sha256").is_some_and(Value::is_null) || !source_paths.insert(path.to_owned()) {
            panic!("tracked manifest source row is prebound or duplicated: {family_id}/{path}");
        }
        safe_existing_file(repository, path, "tracked manifest source artifact");
    }
    if source_paths.is_empty() {
        panic!("tracked manifest declares no source artifacts: {family_id}");
    }

    let method_versions = array(
        template
            .get("method_versions")
            .expect("tracked manifest method_versions"),
        "tracked manifest method_versions",
    )
    .iter()
    .map(|value| {
        value
            .as_str()
            .filter(|value| !value.is_empty() && value.trim() == *value)
            .unwrap_or_else(|| panic!("tracked method version is invalid: {family_id}"))
            .to_owned()
    })
    .collect::<BTreeSet<_>>();
    if method_versions.is_empty() {
        panic!("tracked manifest declares no method versions: {family_id}");
    }

    let mut profiles = BTreeMap::new();
    for profile in array(
        template
            .get("profile_matrix")
            .expect("tracked manifest profile_matrix"),
        "tracked manifest profile_matrix",
    ) {
        let profile = object(profile, "tracked manifest profile");
        let profile_id = text(profile.get("profile_id"), "tracked profile.profile_id");
        let method_version = text(
            profile.get("method_version"),
            "tracked profile.method_version",
        );
        if !expected_family.profiles.contains(profile_id)
            || !method_versions.contains(method_version)
            || text(
                profile.get("coverage_state"),
                "tracked profile.coverage_state",
            ) != "absent"
            || text(
                profile.get("evidence_state"),
                "tracked profile.evidence_state",
            ) != "absent"
            || text(profile.get("surface"), "tracked profile.surface") != "labs"
        {
            panic!("tracked profile is not Labs/absent or indexed: {profile_id}");
        }
        let mut exact_profile_cells = BTreeSet::new();
        for procedure in array(
            profile
                .get("procedure_cells")
                .expect("tracked profile procedure_cells"),
            "tracked profile procedure_cells",
        ) {
            let procedure = object(procedure, "tracked procedure cell");
            let procedure_id = text(
                procedure.get("procedure_id"),
                "tracked procedure.procedure_id",
            );
            let identity = format!("{profile_id}::{procedure_id}");
            if !exact_cell(&identity)
                || text(
                    procedure.get("evidence_identity_template"),
                    "tracked procedure.evidence_identity_template",
                ) != identity.as_str()
                || text(
                    procedure.get("evidence_state"),
                    "tracked procedure.evidence_state",
                ) != "absent"
                || text(procedure.get("gate_state"), "tracked procedure.gate_state") != "pending"
                || !procedure.get("report_path").is_some_and(Value::is_null)
                || !procedure.get("report_sha256").is_some_and(Value::is_null)
                || !exact_profile_cells.insert(identity.clone())
            {
                panic!("tracked procedure is not an exact absent cell: {identity}");
            }
        }
        if exact_profile_cells.is_empty()
            || profiles
                .insert(
                    profile_id.to_owned(),
                    TrackedManifestProfileV1 {
                        method_version: method_version.to_owned(),
                        exact_profile_cells,
                    },
                )
                .is_some()
        {
            panic!("tracked profile identity is empty or duplicated: {profile_id}");
        }
    }
    if profiles.keys().cloned().collect::<BTreeSet<_>>() != expected_family.profiles {
        panic!("tracked manifest profile matrix differs from capability index: {family_id}");
    }
    TrackedManifestAuthorityV1 {
        source_paths,
        profiles,
    }
}

fn gate_binding_authorities(bytes: &[u8]) -> BTreeMap<String, GateBindingAuthorityV1> {
    let catalog: Value = parse(bytes, "gate binding catalog");
    let catalog = object(&catalog, "gate binding catalog");
    let gates = array(
        catalog.get("gates").expect("gate binding catalog.gates"),
        "gate binding catalog.gates",
    );
    let mut authorities = BTreeMap::new();
    for gate in gates {
        let gate = object(gate, "gate binding");
        let gate_id = text(gate.get("gate_id"), "gate binding.gate_id").to_owned();
        let profiles = gate
            .get("profiles")
            .filter(|value| value.is_array())
            .cloned()
            .unwrap_or_else(|| panic!("gate binding.profiles must be an array: {gate_id}"));
        let covered_evidence_cells = gate
            .get("covered_evidence_cells")
            .filter(|value| value.is_array())
            .cloned()
            .unwrap_or_else(|| {
                panic!("gate binding.covered_evidence_cells must be an array: {gate_id}")
            });
        let input_artifacts = array(
            gate.get("input_artifacts")
                .expect("gate binding.input_artifacts"),
            "gate binding.input_artifacts",
        )
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.is_empty() && value.trim() == *value)
                .unwrap_or_else(|| panic!("gate binding input artifact is invalid: {gate_id}"))
                .to_owned()
        })
        .collect::<Vec<_>>();
        if input_artifacts.is_empty()
            || input_artifacts.iter().collect::<BTreeSet<_>>().len() != input_artifacts.len()
        {
            panic!("gate binding input artifacts are empty or duplicated: {gate_id}");
        }
        let steps = array(
            gate.get("steps").expect("gate binding.steps"),
            "gate binding.steps",
        )
        .iter()
        .map(|value| {
            let step = object(value, "gate binding step");
            GateStepAuthorityV1 {
                step_id: text(step.get("step_id"), "gate binding step.step_id").to_owned(),
                uses_cargo: boolean(step.get("uses_cargo"), "gate binding step.uses_cargo"),
                maximum_seconds: step
                    .get("maximum_seconds")
                    .and_then(Value::as_u64)
                    .filter(|value| *value > 0)
                    .expect("gate binding step.maximum_seconds must be positive"),
                expected_output_count: array(
                    step.get("expected_outputs")
                        .expect("gate binding step.expected_outputs"),
                    "gate binding step.expected_outputs",
                )
                .len(),
            }
        })
        .collect::<Vec<_>>();
        if authorities
            .insert(
                gate_id.clone(),
                GateBindingAuthorityV1 {
                    profiles,
                    covered_evidence_cells,
                    probable_root_component: text(
                        gate.get("probable_root_component"),
                        "gate binding.probable_root_component",
                    )
                    .to_owned(),
                    input_artifacts,
                    steps,
                },
            )
            .is_some()
        {
            panic!("gate binding identity is duplicated: {gate_id}");
        }
    }
    authorities
}

fn campaign_gate_state_authorities(
    campaign_root: &Path,
    binding: &CandidateAuthorityBindingV1,
) -> BTreeMap<String, CampaignGateStateAuthorityV1> {
    let state_path = safe_existing_file(campaign_root, "campaign_state.json", "campaign state");
    let state: Value = parse(&read(&state_path, "campaign state"), "campaign state");
    let state = object(&state, "campaign state");
    if text(
        state.get("candidate_commit_sha"),
        "campaign state.candidate_commit_sha",
    ) != binding.candidate_commit_sha
        || text(
            state.get("candidate_version"),
            "campaign state.candidate_version",
        ) != binding.candidate_version
        || text(state.get("plan_sha256"), "campaign state.plan_sha256")
            != binding.qualification_plan_sha256
        || text(state.get("binding_sha256"), "campaign state.binding_sha256")
            != binding.gate_binding_sha256
    {
        panic!("campaign state differs from candidate authority");
    }
    let mut authorities = BTreeMap::new();
    for gate in array(
        state.get("gates").expect("campaign state.gates"),
        "campaign state.gates",
    ) {
        let gate = object(gate, "campaign gate state");
        if text(gate.get("status"), "campaign gate state.status") != "passed"
            || !boolean(
                gate.get("evidence_valid"),
                "campaign gate state.evidence_valid",
            )
        {
            continue;
        }
        let gate_id = text(gate.get("gate_id"), "campaign gate state.gate_id").to_owned();
        let authority = CampaignGateStateAuthorityV1 {
            receipt_path: text(gate.get("receipt"), "campaign gate state.receipt").to_owned(),
            receipt_sha256: text(
                gate.get("receipt_sha256"),
                "campaign gate state.receipt_sha256",
            )
            .to_owned(),
            input_digest: text(gate.get("input_digest"), "campaign gate state.input_digest")
                .to_owned(),
            seed: gate
                .get("seed")
                .and_then(Value::as_u64)
                .expect("campaign gate state.seed must be an integer"),
        };
        if !lower_sha(&authority.receipt_sha256, 64)
            || !lower_sha(&authority.input_digest, 64)
            || authorities.insert(gate_id.clone(), authority).is_some()
        {
            panic!("campaign gate state authority is invalid or duplicated: {gate_id}");
        }
    }

    let issues_path = safe_existing_file(campaign_root, "issue_inventory.json", "issue inventory");
    let issues: Value = parse(&read(&issues_path, "issue inventory"), "issue inventory");
    let issues = object(&issues, "issue inventory");
    if text(
        issues.get("candidate_commit_sha"),
        "issue inventory.candidate_commit_sha",
    ) != binding.candidate_commit_sha
        || text(
            issues.get("candidate_version"),
            "issue inventory.candidate_version",
        ) != binding.candidate_version
        || text(issues.get("plan_sha256"), "issue inventory.plan_sha256")
            != binding.qualification_plan_sha256
        || text(
            issues.get("binding_sha256"),
            "issue inventory.binding_sha256",
        ) != binding.gate_binding_sha256
        || array(
            issues.get("issues").expect("issue inventory.issues"),
            "issue inventory.issues",
        )
        .iter()
        .any(|issue| {
            object(issue, "issue inventory row")
                .get("disposition")
                .and_then(Value::as_str)
                == Some("open")
        })
    {
        panic!("issue inventory is stale or contains an open issue");
    }
    authorities
}

fn validate_gate_receipt(
    repository: &Path,
    campaign_root: &Path,
    row: &Value,
    expected_binding: &GateBindingAuthorityV1,
    expected_state: &CampaignGateStateAuthorityV1,
    commit: &str,
    version: &str,
    plan_sha256: &str,
    gate_binding_sha256: &str,
) {
    let row = object(row, "profile report gate receipt row");
    let expected_row_keys = [
        "gate_id",
        "input_artifacts",
        "input_digest",
        "path",
        "seed",
        "sha256",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if row.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected_row_keys {
        panic!("profile report gate receipt row has an unsupported shape");
    }
    let gate_id = text(row.get("gate_id"), "gate receipt row.gate_id");
    let path = text(row.get("path"), "gate receipt row.path");
    let expected_sha = text(row.get("sha256"), "gate receipt row.sha256");
    let expected_input_digest = text(row.get("input_digest"), "gate receipt row.input_digest");
    let expected_seed = row
        .get("seed")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| panic!("gate receipt row.seed must be a nonnegative integer"));
    if !lower_sha(expected_sha, 64) {
        panic!("gate receipt row has invalid SHA-256: {gate_id}");
    }
    if !lower_sha(expected_input_digest, 64) || expected_seed != 42 {
        panic!("gate receipt row has invalid input digest or seed: {gate_id}");
    }
    if path != expected_state.receipt_path
        || expected_sha != expected_state.receipt_sha256
        || expected_input_digest != expected_state.input_digest
        || expected_seed != expected_state.seed
    {
        panic!("gate receipt row differs from the hash-bound campaign state: {gate_id}");
    }
    let input_rows = array(
        row.get("input_artifacts")
            .expect("gate receipt row.input_artifacts"),
        "gate receipt row.input_artifacts",
    );
    if input_rows.len() != expected_binding.input_artifacts.len() {
        panic!("gate receipt row input inventory is incomplete: {gate_id}");
    }
    for (input, expected_path) in input_rows.iter().zip(&expected_binding.input_artifacts) {
        let input = object(input, "gate receipt input artifact");
        if input.keys().map(String::as_str).collect::<BTreeSet<_>>()
            != ["path", "sha256"].into_iter().collect::<BTreeSet<_>>()
        {
            panic!("gate receipt input artifact has an unsupported shape: {gate_id}");
        }
        let path = text(input.get("path"), "gate receipt input artifact.path");
        let expected_sha = text(input.get("sha256"), "gate receipt input artifact.sha256");
        let source_path = safe_existing_file(repository, path, "gate input artifact");
        if path != expected_path
            || !lower_sha(expected_sha, 64)
            || sha256(&read(&source_path, "gate input artifact")) != expected_sha
        {
            panic!("gate receipt input artifact is stale: {gate_id}/{path}");
        }
    }
    let receipt_path = safe_existing_file(campaign_root, path, "gate receipt");
    let bytes = read(&receipt_path, "gate receipt");
    if sha256(&bytes) != expected_sha {
        panic!("gate receipt hash is stale: {gate_id}");
    }
    let receipt: Value = parse(&bytes, "gate receipt");
    let receipt = object(&receipt, "gate receipt");
    if text(receipt.get("receipt_kind"), "gate receipt.receipt_kind")
        != "qpls_multimod_gate_receipt_v1"
        || text(receipt.get("gate_id"), "gate receipt.gate_id") != gate_id
        || text(receipt.get("status"), "gate receipt.status") != "passed"
        || text(
            receipt.get("coverage_binding_state"),
            "gate receipt.coverage_binding_state",
        ) != "executed_real_commands"
        || text(
            receipt.get("candidate_commit_sha"),
            "gate receipt.candidate_commit_sha",
        ) != commit
        || text(
            receipt.get("candidate_version"),
            "gate receipt.candidate_version",
        ) != version
        || text(receipt.get("plan_sha256"), "gate receipt.plan_sha256") != plan_sha256
        || text(receipt.get("binding_sha256"), "gate receipt.binding_sha256") != gate_binding_sha256
        || text(receipt.get("input_digest"), "gate receipt.input_digest") != expected_input_digest
        || receipt.get("seed").and_then(Value::as_u64) != Some(expected_seed)
        || receipt.get("profiles") != Some(&expected_binding.profiles)
        || receipt.get("covered_evidence_cells") != Some(&expected_binding.covered_evidence_cells)
        || text(
            receipt.get("probable_root_component"),
            "gate receipt.probable_root_component",
        ) != expected_binding.probable_root_component.as_str()
    {
        panic!("gate receipt is stale or failed: {gate_id}");
    }
    let steps = array(
        receipt.get("steps").expect("gate receipt.steps"),
        "gate receipt.steps",
    );
    if steps.len() != expected_binding.steps.len() {
        panic!("gate receipt step inventory is incomplete: {gate_id}");
    }
    for (step, expected_step) in steps.iter().zip(&expected_binding.steps) {
        let step = object(step, "gate receipt step");
        if text(step.get("step_id"), "gate receipt step.step_id") != expected_step.step_id.as_str()
            || text(step.get("status"), "gate receipt step.status") != "passed"
            || step.get("exit_code").and_then(Value::as_i64) != Some(0)
            || boolean(step.get("uses_cargo"), "gate receipt step.uses_cargo")
                != expected_step.uses_cargo
            || step.get("maximum_seconds").and_then(Value::as_u64)
                != Some(expected_step.maximum_seconds)
            || boolean(
                step.get("budget_exceeded"),
                "gate receipt step.budget_exceeded",
            )
            || boolean(
                step.get("timeout_terminated"),
                "gate receipt step.timeout_terminated",
            )
            || boolean(
                step.get("empty_cargo_test_rejected"),
                "gate receipt step.empty_cargo_test_rejected",
            )
            || !array(
                step.get("missing_outputs")
                    .expect("gate receipt step.missing_outputs"),
                "gate receipt step.missing_outputs",
            )
            .is_empty()
        {
            panic!("gate receipt step failed or differs from its binding: {gate_id}");
        }
        for stream in ["stdout", "stderr"] {
            let path_key = format!("{stream}_path");
            let sha_key = format!("{stream}_sha256");
            let size_key = format!("{stream}_size");
            let path = text(step.get(path_key.as_str()), path_key.as_str());
            let expected_sha = text(step.get(sha_key.as_str()), sha_key.as_str());
            let expected_size = step
                .get(size_key.as_str())
                .and_then(Value::as_u64)
                .expect("gate receipt stream size must be an integer");
            let file = safe_absolute_campaign_file(campaign_root, path, "gate log");
            if !lower_sha(expected_sha, 64)
                || sha256(&read(&file, "gate log")) != expected_sha
                || file.metadata().expect("gate log metadata").len() != expected_size
            {
                panic!("gate receipt log is stale: {gate_id}/{stream}");
            }
        }
        let outputs = array(
            step.get("expected_outputs")
                .expect("gate receipt step.expected_outputs"),
            "gate receipt step.expected_outputs",
        );
        if outputs.len() != expected_step.expected_output_count {
            panic!("gate receipt output inventory is incomplete: {gate_id}");
        }
        for output in outputs {
            let output = object(output, "gate receipt output");
            let path = text(output.get("path"), "gate receipt output.path");
            let expected_sha = text(output.get("sha256"), "gate receipt output.sha256");
            let expected_size = output
                .get("size")
                .and_then(Value::as_u64)
                .expect("gate receipt output.size must be an integer");
            let file = safe_absolute_campaign_file(campaign_root, path, "gate output");
            if !lower_sha(expected_sha, 64)
                || sha256(&read(&file, "gate output")) != expected_sha
                || file.metadata().expect("gate output metadata").len() != expected_size
            {
                panic!("gate receipt output is stale: {gate_id}/{path}");
            }
        }
    }
}

fn validate_prepackage_manifests(
    repository: &Path,
    manifest_set_path: &Path,
    manifest_set: &PrepackageAuthoritySetV1,
    binding: &CandidateAuthorityBindingV1,
    tracked_families: &BTreeMap<String, TrackedCapabilityFamilyV1>,
    gate_bindings: &BTreeMap<String, GateBindingAuthorityV1>,
    campaign_state: &BTreeMap<String, CampaignGateStateAuthorityV1>,
) {
    let campaign_root = manifest_set_path
        .parent()
        .and_then(Path::parent)
        .expect("prepackage manifest set must be two levels below its campaign root");
    let mut families = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut all_cells = BTreeSet::new();
    for row in &manifest_set.manifests {
        if row.family_id.trim().is_empty()
            || row.path.trim().is_empty()
            || row.path.trim() != row.path
            || row.template_path.trim().is_empty()
            || row.template_path.trim() != row.template_path
            || !families.insert(row.family_id.clone())
            || !paths.insert(row.path.clone())
            || !lower_sha(&row.sha256, 64)
            || !lower_sha(&row.template_sha256, 64)
            || !exact_sorted_cells(&row.exact_profile_cells)
        {
            panic!("prepackage manifest row identity is invalid or duplicated");
        }
        let tracked_family = tracked_families.get(&row.family_id).unwrap_or_else(|| {
            panic!(
                "prepackage manifest family is not tracked: {}",
                row.family_id
            )
        });
        if row.template_path != tracked_family.manifest {
            panic!(
                "prepackage manifest template differs from its tracked capability index: {}",
                row.family_id
            );
        }
        let manifest_path = safe_existing_file(campaign_root, &row.path, "live manifest");
        let manifest_bytes = read(&manifest_path, "live manifest");
        if sha256(&manifest_bytes) != row.sha256 {
            panic!("live manifest hash is stale: {}", row.family_id);
        }
        let template_path =
            safe_existing_file(repository, &row.template_path, "tracked manifest template");
        let template_bytes = read(&template_path, "tracked manifest template");
        if sha256(&template_bytes) != row.template_sha256 {
            panic!("tracked manifest template changed: {}", row.family_id);
        }
        let template: Value = parse(&template_bytes, "tracked manifest template");
        let template = object(&template, "tracked manifest template");
        if text(
            template.get("family_id"),
            "tracked manifest template.family_id",
        ) != row.family_id
        {
            panic!(
                "tracked manifest family identity changed: {}",
                row.family_id
            );
        }
        let tracked_manifest =
            validate_tracked_manifest_template(repository, template, tracked_family);
        let manifest: Value = parse(&manifest_bytes, "live manifest");
        let manifest = object(&manifest, "live manifest");
        let source_binding = object(
            manifest
                .get("source_binding")
                .expect("live manifest source_binding"),
            "live manifest.source_binding",
        );
        if text(manifest.get("family_id"), "live manifest.family_id") != row.family_id
            || text(
                manifest.get("declared_evidence_state"),
                "live manifest.declared_evidence_state",
            ) != "release_qualified"
            || text(manifest.get("surface"), "live manifest.surface") != "labs"
            || boolean(
                manifest.get("promotion_allowed"),
                "live manifest.promotion_allowed",
            )
            || text(
                source_binding.get("status"),
                "live manifest.source_binding.status",
            ) != "bound"
            || text(
                source_binding.get("candidate_commit_sha"),
                "live manifest.source_binding.candidate_commit_sha",
            ) != binding.candidate_commit_sha
        {
            panic!(
                "live manifest state or candidate binding is invalid: {}",
                row.family_id
            );
        }
        let sources = array(
            source_binding
                .get("source_artifacts")
                .expect("live manifest source artifacts"),
            "live manifest.source_binding.source_artifacts",
        );
        if sources.is_empty() {
            panic!("live manifest has no source artifacts: {}", row.family_id);
        }
        let mut live_source_paths = BTreeSet::new();
        for source in sources {
            let source = object(source, "live manifest source artifact");
            let path = text(source.get("path"), "source artifact.path");
            let expected = text(source.get("sha256"), "source artifact.sha256");
            let source_path = safe_existing_file(repository, path, "source artifact");
            if !live_source_paths.insert(path.to_owned())
                || !lower_sha(expected, 64)
                || sha256(&read(&source_path, "source artifact")) != expected
            {
                panic!("live manifest source hash is stale: {path}");
            }
        }
        if live_source_paths != tracked_manifest.source_paths {
            panic!(
                "live manifest source inventory differs from its tracked template: {}",
                row.family_id
            );
        }

        let mut manifest_cells = BTreeSet::new();
        let mut live_profiles = BTreeSet::new();
        for profile in array(
            manifest
                .get("profile_matrix")
                .expect("live manifest profiles"),
            "live manifest.profile_matrix",
        ) {
            let profile = object(profile, "live manifest profile");
            let profile_id = text(profile.get("profile_id"), "live manifest profile_id");
            let method_version = text(profile.get("method_version"), "profile.method_version");
            let tracked_profile = tracked_manifest
                .profiles
                .get(profile_id)
                .unwrap_or_else(|| panic!("live manifest profile is not tracked: {profile_id}"));
            if !live_profiles.insert(profile_id.to_owned())
                || method_version != tracked_profile.method_version.as_str()
                || text(profile.get("coverage_state"), "profile.coverage_state")
                    != "release_qualified"
                || text(profile.get("evidence_state"), "profile.evidence_state")
                    != "release_qualified"
                || text(profile.get("surface"), "profile.surface") != "labs"
            {
                panic!("live manifest profile is not release-qualified: {profile_id}");
            }
            for procedure in array(
                profile
                    .get("procedure_cells")
                    .expect("live manifest procedure cells"),
                "profile.procedure_cells",
            ) {
                let procedure = object(procedure, "live manifest procedure cell");
                let procedure_id = text(procedure.get("procedure_id"), "procedure.procedure_id");
                let identity = format!("{profile_id}::{procedure_id}");
                if !exact_cell(&identity)
                    || !tracked_profile.exact_profile_cells.contains(&identity)
                    || text(
                        procedure.get("evidence_identity_template"),
                        "procedure.evidence_identity_template",
                    ) != identity.as_str()
                    || text(procedure.get("evidence_state"), "procedure.evidence_state")
                        != "release_qualified"
                    || text(procedure.get("gate_state"), "procedure.gate_state") != "passed"
                    || !manifest_cells.insert(identity.clone())
                {
                    panic!("live manifest procedure cell is invalid: {identity}");
                }
                let report_path = text(procedure.get("report_path"), "procedure.report_path");
                let report_sha = text(procedure.get("report_sha256"), "procedure.report_sha256");
                let report_path = safe_existing_file(campaign_root, report_path, "profile report");
                let report_bytes = read(&report_path, "profile report");
                if !lower_sha(report_sha, 64) || sha256(&report_bytes) != report_sha {
                    panic!("profile report hash is stale: {identity}");
                }
                let report: Value = parse(&report_bytes, "profile report");
                let report = object(&report, "profile report");
                let expected_report_id =
                    format!("qpls.multimod.evidence.{profile_id}.{procedure_id}");
                if report.get("schema_version").and_then(Value::as_u64) != Some(1)
                    || text(report.get("report_kind"), "profile report.report_kind")
                        != "qpls_multimod_profile_procedure_evidence_v1"
                    || text(report.get("report_id"), "profile report.report_id")
                        != expected_report_id.as_str()
                    || text(
                        report.get("candidate_commit_sha"),
                        "profile report.candidate_commit_sha",
                    ) != binding.candidate_commit_sha
                    || text(report.get("plan_sha256"), "profile report.plan_sha256")
                        != binding.qualification_plan_sha256
                    || text(report.get("family_id"), "profile report.family_id") != row.family_id
                    || text(report.get("profile_id"), "profile report.profile_id") != profile_id
                    || text(report.get("procedure_id"), "profile report.procedure_id")
                        != procedure_id
                    || text(
                        report.get("method_version"),
                        "profile report.method_version",
                    ) != method_version
                    || text(
                        report.get("evidence_state"),
                        "profile report.evidence_state",
                    ) != "release_qualified"
                    || text(report.get("template_path"), "profile report.template_path")
                        != row.template_path
                    || text(
                        report.get("template_sha256"),
                        "profile report.template_sha256",
                    ) != row.template_sha256
                    || report.get("source_artifacts") != source_binding.get("source_artifacts")
                    || text(
                        report.get("generated_at_utc"),
                        "profile report.generated_at_utc",
                    )
                    .trim()
                    .is_empty()
                {
                    panic!("profile report binding is invalid: {identity}");
                }
                let receipt_rows = array(
                    report
                        .get("required_gate_receipts")
                        .expect("profile report required gate receipts"),
                    "profile report.required_gate_receipts",
                );
                let expected_gate_ids = expected_prepackage_gate_ids(&row.family_id, profile_id);
                let actual_gate_ids = receipt_rows
                    .iter()
                    .map(|receipt| {
                        text(
                            object(receipt, "profile report gate receipt row").get("gate_id"),
                            "gate receipt row.gate_id",
                        )
                    })
                    .collect::<Vec<_>>();
                if actual_gate_ids != expected_gate_ids {
                    panic!("profile report gate inventory is incomplete or reordered: {identity}");
                }
                let mut profile_specific_coverage = false;
                for (receipt_index, receipt) in receipt_rows.iter().enumerate() {
                    let receipt_row = object(receipt, "profile report gate receipt row");
                    let gate_id = text(receipt_row.get("gate_id"), "gate receipt row.gate_id");
                    let expected_binding = gate_bindings.get(gate_id).unwrap_or_else(|| {
                        panic!("profile report cites an unknown gate: {gate_id}")
                    });
                    let expected_state = campaign_state.get(gate_id).unwrap_or_else(|| {
                        panic!(
                            "profile report gate lacks passed campaign-state authority: {gate_id}"
                        )
                    });
                    if receipt_index >= PREPACKAGE_GLOBAL_GATES.len()
                        && array(
                            &expected_binding.covered_evidence_cells,
                            "gate binding.covered_evidence_cells",
                        )
                        .iter()
                        .any(|pattern| {
                            pattern
                                .as_str()
                                .is_some_and(|pattern| star_pattern_matches(pattern, &identity))
                        })
                    {
                        profile_specific_coverage = true;
                    }
                    validate_gate_receipt(
                        repository,
                        campaign_root,
                        receipt,
                        expected_binding,
                        expected_state,
                        &binding.candidate_commit_sha,
                        &binding.candidate_version,
                        &binding.qualification_plan_sha256,
                        &binding.gate_binding_sha256,
                    );
                }
                if !profile_specific_coverage {
                    panic!("no profile-specific passed gate covers exact cell: {identity}");
                }
            }
        }
        let row_cells = row
            .exact_profile_cells
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if manifest_cells != row_cells {
            panic!(
                "manifest row exact-cell inventory differs from its live manifest: {}",
                row.family_id
            );
        }
        all_cells.extend(manifest_cells);
    }
    let declared = manifest_set
        .exact_profile_cells
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if all_cells != declared || declared.len() != manifest_set.exact_profile_cells.len() {
        panic!("prepackage exact-cell inventory differs from its deeply verified manifests");
    }
}

fn validate_and_embed_candidate(
    repository: &Path,
    package_version: &str,
    out_dir: &Path,
    authority_path: &Path,
    manifest_set_path: &Path,
) {
    if !authority_path.is_absolute() || !manifest_set_path.is_absolute() {
        panic!("MultiMod build-only authority paths must be absolute");
    }
    let authority_bytes = read(authority_path, "candidate authority");
    let manifest_bytes = read(manifest_set_path, "prepackage manifest set");
    let authority: CandidateAuthorityDocumentV1 = parse(&authority_bytes, "candidate authority");
    let manifest_set: PrepackageAuthoritySetV1 = parse(&manifest_bytes, "prepackage manifest set");
    if authority.schema_version != 1
        || authority.authority_kind != "qpls_multimod_embedded_candidate_authority_v1"
        || authority.state != "release_qualified_candidate"
    {
        panic!("candidate authority has an unsupported identity or state");
    }
    let binding = authority
        .binding
        .as_ref()
        .expect("release candidate authority requires one binding");
    let declared_binding_sha = authority
        .authority_binding_sha256
        .as_deref()
        .expect("release candidate authority requires its binding digest");
    let canonical_binding =
        serde_json::to_vec(binding).expect("serialize candidate authority binding");
    if !lower_sha(declared_binding_sha, 64) || sha256(&canonical_binding) != declared_binding_sha {
        panic!("candidate authority binding digest is invalid");
    }
    let head = git_head(repository);
    if !lower_sha(&head, 40)
        || binding.candidate_commit_sha != head
        || binding.candidate_version != package_version
    {
        panic!("candidate authority differs from Git HEAD or the package version");
    }
    if !git_worktree_is_clean(repository) {
        panic!("candidate authority can be embedded only from a clean Git worktree");
    }

    let plan_path = repository.join("validation/multimod/v256_multimod_qualification_plan_v1.json");
    let gate_binding_path = repository.join("validation/multimod/multimod_gate_bindings_v1.json");
    let capability_index_path =
        repository.join("validation/multimod/multimod_capability_index_v1.json");
    let plan_bytes = read(&plan_path, "qualification plan");
    let gate_binding_bytes = read(&gate_binding_path, "gate bindings");
    let capability_index_bytes = read(&capability_index_path, "capability index");
    if sha256(&plan_bytes) != binding.qualification_plan_sha256
        || sha256(&gate_binding_bytes) != binding.gate_binding_sha256
        || sha256(&capability_index_bytes) != binding.capability_index_sha256
        || sha256(&manifest_bytes) != binding.prepackage_manifest_set_sha256
    {
        panic!("candidate authority source or prepackage manifest digest is stale");
    }
    if !exact_sorted_cells(&binding.exact_profile_cells) {
        panic!("candidate authority cells must be exact, sorted, unique, and wildcard-free");
    }

    if manifest_set.schema_version != 1
        || manifest_set.manifest_set_id != "qpls.v256.multimod.prepackage-authority-set.v1"
        || manifest_set.stage != "prepackage_authority"
        || manifest_set.state != "release_qualified"
        || manifest_set.surface != "labs"
        || manifest_set.promotion_allowed
        || manifest_set.candidate_commit_sha != head
        || manifest_set.candidate_version != package_version
        || manifest_set.plan_sha256 != binding.qualification_plan_sha256
        || manifest_set.gate_binding_sha256 != binding.gate_binding_sha256
        || manifest_set.capability_index_sha256 != binding.capability_index_sha256
        || manifest_set.exact_profile_cells != binding.exact_profile_cells
        || manifest_set.generated_at_utc.trim().is_empty()
        || manifest_set.manifests.is_empty()
        || manifest_set
            .non_circular_binding
            .candidate_source_contains_evidence
        || !manifest_set
            .non_circular_binding
            .authority_generated_after_candidate_commit
        || !manifest_set
            .non_circular_binding
            .build_consumes_external_prepackage_set
        || !manifest_set
            .non_circular_binding
            .artifact_paths_relative_to_campaign_root
        || manifest_set
            .non_circular_binding
            .campaign_root_ancestor_depth
            != 2
    {
        panic!("prepackage authority set differs from its exact candidate binding");
    }
    let index: CapabilityIndexV1 = parse(&capability_index_bytes, "tracked capability index");
    if index.schema_version != 1
        || index.candidate_release != package_version
        || index.policy.surface != "labs"
        || index.policy.evidence_state != "absent"
        || index.policy.promotion_allowed
    {
        panic!("tracked MultiMod capability index must remain Labs/absent for this candidate");
    }
    let mut tracked_profiles = BTreeSet::new();
    let mut tracked_families = BTreeMap::new();
    for family in index.families {
        if family.family_id.trim().is_empty()
            || family.manifest.trim().is_empty()
            || family.manifest.trim() != family.manifest
            || family.profiles.is_empty()
        {
            panic!("tracked capability family identity is incomplete");
        }
        let mut family_profiles = BTreeSet::new();
        for profile in family.profiles {
            if !family_profiles.insert(profile.clone()) || !tracked_profiles.insert(profile) {
                panic!("tracked capability profiles must be globally unique");
            }
        }
        if tracked_families
            .insert(
                family.family_id,
                TrackedCapabilityFamilyV1 {
                    manifest: family.manifest,
                    profiles: family_profiles,
                },
            )
            .is_some()
        {
            panic!("tracked capability families must be unique");
        }
    }
    for (family_id, tracked_family) in &tracked_families {
        let path = safe_existing_file(
            repository,
            &tracked_family.manifest,
            "tracked capability manifest",
        );
        let template: Value = parse(
            &read(&path, "tracked capability manifest"),
            "tracked capability manifest",
        );
        let template = object(&template, "tracked capability manifest");
        if text(
            template.get("family_id"),
            "tracked capability manifest.family_id",
        ) != family_id
        {
            panic!("tracked capability index points to the wrong family manifest: {family_id}");
        }
        validate_tracked_manifest_template(repository, template, tracked_family);
    }
    let campaign_root = manifest_set_path
        .parent()
        .and_then(Path::parent)
        .expect("prepackage manifest set must be two levels below its campaign root");
    let gate_bindings = gate_binding_authorities(&gate_binding_bytes);
    let campaign_state = campaign_gate_state_authorities(campaign_root, binding);
    validate_prepackage_manifests(
        repository,
        manifest_set_path,
        &manifest_set,
        binding,
        &tracked_families,
        &gate_bindings,
        &campaign_state,
    );
    let authority_profiles = binding
        .exact_profile_cells
        .iter()
        .map(|cell| {
            cell.split_once("::")
                .expect("validated exact cell")
                .0
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    if !authority_profiles.is_subset(&tracked_profiles) {
        panic!("candidate authority contains an exact cell for an untracked MultiMod profile");
    }

    fs::write(out_dir.join(EMBEDDED_AUTHORITY_FILE), authority_bytes)
        .expect("write embedded MultiMod candidate authority");
    fs::write(out_dir.join(EMBEDDED_MANIFEST_SET_FILE), manifest_bytes)
        .expect("write embedded MultiMod prepackage manifest set");
}

fn main() {
    println!("cargo:rerun-if-env-changed={AUTHORITY_ENV}");
    println!("cargo:rerun-if-env-changed={MANIFEST_SET_ENV}");
    println!("cargo:rerun-if-changed=../Cargo.toml");
    println!(
        "cargo:rerun-if-changed=../validation/multimod/v256_multimod_qualification_plan_v1.json"
    );
    println!("cargo:rerun-if-changed=../validation/multimod/multimod_gate_bindings_v1.json");
    println!("cargo:rerun-if-changed=../validation/multimod/multimod_capability_index_v1.json");

    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let repository = manifest_dir
        .parent()
        .expect("desktop crate must be inside repository");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let authority = env::var_os(AUTHORITY_ENV).map(PathBuf::from);
    let manifest_set = env::var_os(MANIFEST_SET_ENV).map(PathBuf::from);
    match (authority, manifest_set) {
        (None, None) => {
            fs::write(out_dir.join(EMBEDDED_AUTHORITY_FILE), LABS_SENTINEL)
                .expect("write Labs-only MultiMod authority sentinel");
            fs::write(
                out_dir.join(EMBEDDED_MANIFEST_SET_FILE),
                EMPTY_MANIFEST_SENTINEL,
            )
            .expect("write Labs-only MultiMod manifest sentinel");
        }
        (Some(authority), Some(manifest_set)) => validate_and_embed_candidate(
            repository,
            &env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION"),
            &out_dir,
            &authority,
            &manifest_set,
        ),
        _ => panic!(
            "{AUTHORITY_ENV} and {MANIFEST_SET_ENV} are build-only and must be supplied together"
        ),
    }
    tauri_build::build()
}
