use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const DEVELOPMENT_SLICES_JSON: &str =
    include_str!("../../../validation/development_slices.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SliceRegistry {
    pub schema_version: u32,
    pub program: String,
    pub active_goal: String,
    pub current_stage: String,
    pub policy: SlicePolicy,
    pub tracks: Vec<String>,
    pub slices: Vec<DevelopmentSlice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlicePolicy {
    pub promotion_order: Vec<SliceStatus>,
    pub deterministic_tolerance: f64,
    pub stable_export_rule: String,
    pub validation_rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DevelopmentSlice {
    pub id: String,
    pub release: String,
    pub family: String,
    pub name: String,
    pub status: SliceStatus,
    pub stable_output: bool,
    pub owner_track: String,
    pub priority: u32,
    pub summary: String,
    pub gates: Vec<SliceGate>,
    pub next_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SliceGate {
    pub track: String,
    pub name: String,
    pub status: GateStatus,
    pub evidence: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SliceStatus {
    Unsupported,
    Experimental,
    Validated,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Passed,
    Open,
    Blocked,
    NotApplicable,
}

impl DevelopmentSlice {
    pub fn open_gates(&self) -> impl Iterator<Item = &SliceGate> {
        self.gates
            .iter()
            .filter(|gate| gate.status == GateStatus::Open || gate.status == GateStatus::Blocked)
    }

    pub fn gate_summary(&self) -> GateSummary {
        let mut summary = GateSummary::default();
        for gate in &self.gates {
            match gate.status {
                GateStatus::Passed => summary.passed += 1,
                GateStatus::Open => summary.open += 1,
                GateStatus::Blocked => summary.blocked += 1,
                GateStatus::NotApplicable => summary.not_applicable += 1,
            }
        }
        summary
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateSummary {
    pub passed: usize,
    pub open: usize,
    pub blocked: usize,
    pub not_applicable: usize,
}

pub fn development_slice_registry() -> Result<SliceRegistry, serde_json::Error> {
    serde_json::from_str(DEVELOPMENT_SLICES_JSON)
}

pub fn validate_slice_registry(registry: &SliceRegistry) -> Vec<String> {
    let mut errors = Vec::new();
    if registry.schema_version == 0 {
        errors.push("schema_version must be positive".to_owned());
    }
    if registry.policy.promotion_order
        != vec![
            SliceStatus::Unsupported,
            SliceStatus::Experimental,
            SliceStatus::Validated,
        ]
    {
        errors.push("promotion_order must be unsupported -> experimental -> validated".to_owned());
    }
    if registry.policy.deterministic_tolerance <= 0.0 {
        errors.push("deterministic_tolerance must be positive".to_owned());
    }
    let tracks = registry.tracks.iter().collect::<HashSet<_>>();
    let mut ids = HashSet::new();
    let mut priorities = HashSet::new();
    for slice in &registry.slices {
        if !ids.insert(&slice.id) {
            errors.push(format!("duplicate slice id {}", slice.id));
        }
        if !priorities.insert(slice.priority) {
            errors.push(format!("duplicate priority {}", slice.priority));
        }
        if !tracks.contains(&slice.owner_track) {
            errors.push(format!(
                "slice {} has unknown owner track {}",
                slice.id, slice.owner_track
            ));
        }
        if slice.gates.is_empty() {
            errors.push(format!("slice {} has no gates", slice.id));
        }
        for gate in &slice.gates {
            if !tracks.contains(&gate.track) {
                errors.push(format!(
                    "slice {} gate {} has unknown track {}",
                    slice.id, gate.name, gate.track
                ));
            }
            if gate.evidence.trim().is_empty() {
                errors.push(format!(
                    "slice {} gate {} has empty evidence",
                    slice.id, gate.name
                ));
            }
        }
        let has_open_gate = slice.open_gates().next().is_some();
        if slice.status == SliceStatus::Validated && has_open_gate {
            errors.push(format!("validated slice {} still has open gates", slice.id));
        }
        if slice.stable_output && slice.status != SliceStatus::Validated {
            errors.push(format!(
                "slice {} exposes stable output before validation",
                slice.id
            ));
        }
        if slice.status != SliceStatus::Validated && slice.next_actions.is_empty() {
            errors.push(format!("slice {} needs next_actions", slice.id));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_registry_is_valid() {
        let registry = development_slice_registry().unwrap();
        let errors = validate_slice_registry(&registry);
        assert!(errors.is_empty(), "{errors:#?}");
    }

    #[test]
    fn registry_keeps_current_stage_explicit() {
        let registry = development_slice_registry().unwrap();
        assert_eq!(registry.current_stage, "v1_2_method_promotion_program");
        assert!(
            registry
                .slices
                .iter()
                .any(|slice| slice.id == "v0_4_assessment_reliability"
                    && slice.status == SliceStatus::Validated
                    && slice.stable_output)
        );
        assert!(
            registry
                .slices
                .iter()
                .any(|slice| slice.id == "v0_4_inference_resampling"
                    && slice.status == SliceStatus::Validated
                    && slice.stable_output)
        );
        assert!(
            registry
                .slices
                .iter()
                .any(|slice| slice.id == "v0_5_extended_pls"
                    && slice.status == SliceStatus::Experimental)
        );
        assert!(
            registry
                .slices
                .iter()
                .any(|slice| slice.id == "v0_8_extended_methods"
                    && slice.status == SliceStatus::Experimental)
        );
        assert!(
            registry
                .slices
                .iter()
                .any(|slice| slice.id == "publication_ready_v0_1_to_v0_8"
                    && slice.status == SliceStatus::Unsupported)
        );
        assert!(
            registry
                .slices
                .iter()
                .any(|slice| slice.id == "v1_2_method_promotion_program"
                    && slice.status == SliceStatus::Validated
                    && slice.stable_output)
        );
    }

    #[test]
    fn gate_summary_counts_open_work() {
        let registry = development_slice_registry().unwrap();
        let inference = registry
            .slices
            .iter()
            .find(|slice| slice.id == "v0_4_inference_resampling")
            .unwrap();
        let inference_summary = inference.gate_summary();
        assert!(inference_summary.passed > 0);
        assert_eq!(inference_summary.open, 0);

        let extended_methods = registry
            .slices
            .iter()
            .find(|slice| slice.id == "v0_8_extended_methods")
            .unwrap();
        let extended_summary = extended_methods.gate_summary();
        assert!(extended_summary.passed > 0);
        assert_eq!(extended_summary.open, 0);
    }
}
