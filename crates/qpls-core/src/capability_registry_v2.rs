use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fmt};
use thiserror::Error;

use crate::CapabilityCellReferenceV2;

pub const CAPABILITY_REGISTRY_V2_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../validation/capabilities/capability_registry_v2.json"
));

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStateV2 {
    Full,
    Partial,
    Absent,
    IntentionallyExcluded,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStateV2 {
    Absent,
    EngineOnly,
    ArchiveQualified,
    NativeQualified,
    ReleaseQualified,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ProductSurfaceV2 {
    Standard,
    Labs,
    Legacy,
    Internal,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OfficialLifecycleV2 {
    Active,
    Legacy,
}

macro_rules! display_debug_lower {
    ($($kind:ty),+ $(,)?) => {$(
        impl fmt::Display for $kind {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                let value = serde_json::to_value(self).map_err(|_| fmt::Error)?;
                formatter.write_str(value.as_str().ok_or(fmt::Error)?)
            }
        }
    )+};
}

display_debug_lower!(
    CoverageStateV2,
    EvidenceStateV2,
    ProductSurfaceV2,
    OfficialLifecycleV2
);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CapabilityOptionCellV2 {
    pub capability_id: String,
    pub cell_id: String,
    pub capability_version: String,
    pub coverage_state: CoverageStateV2,
    pub evidence_state: EvidenceStateV2,
    pub surface: ProductSurfaceV2,
    pub qualification_spec: CapabilityQualificationSpecV2,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CapabilityQualificationSpecV2 {
    pub links: Vec<CapabilityCellReferenceV2>,
}

impl CapabilityOptionCellV2 {
    pub fn standard_available(&self) -> bool {
        self.surface == ProductSurfaceV2::Standard
            && matches!(
                self.coverage_state,
                CoverageStateV2::Full | CoverageStateV2::Partial
            )
            && self.evidence_state == EvidenceStateV2::ReleaseQualified
    }

    pub fn labs_available(&self) -> bool {
        self.surface == ProductSurfaceV2::Labs
            && matches!(
                self.coverage_state,
                CoverageStateV2::Full | CoverageStateV2::Partial
            )
            && self.evidence_state != EvidenceStateV2::Absent
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CapabilityRegistryRowV2 {
    pub catalogue_position: usize,
    pub capability_id: String,
    pub official_family: String,
    pub official_method: String,
    pub official_lifecycle: OfficialLifecycleV2,
    pub official_url: String,
    pub coverage_state: CoverageStateV2,
    pub evidence_state: EvidenceStateV2,
    pub surface: ProductSurfaceV2,
    pub option_cells: Vec<CapabilityOptionCellV2>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CapabilityCatalogueSnapshotV2 {
    pub provider: String,
    pub product: String,
    pub official_catalogue_url: String,
    pub source_snapshot_date: String,
    pub source_reverified_on: String,
    pub capability_row_count: usize,
    pub active_row_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CapabilityRegistryV2 {
    pub registry_schema_version: u32,
    pub registry_id: String,
    pub registry_version: String,
    pub frozen_on: String,
    pub catalogue_snapshot: CapabilityCatalogueSnapshotV2,
    pub capabilities: Vec<CapabilityRegistryRowV2>,
    #[serde(skip_deserializing)]
    pub source_sha256: String,
}

#[derive(Debug, Error)]
pub enum CapabilityRegistryV2Error {
    #[error("Capability Registry V2 JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Capability Registry V2 contract failed: {0}")]
    Contract(String),
}

impl CapabilityRegistryV2 {
    pub fn embedded() -> Result<Self, CapabilityRegistryV2Error> {
        Self::from_json(CAPABILITY_REGISTRY_V2_JSON)
    }

    pub fn from_json(source: &str) -> Result<Self, CapabilityRegistryV2Error> {
        let mut registry: Self = serde_json::from_str(source)?;
        registry.source_sha256 = format!("{:x}", Sha256::digest(source.as_bytes()));
        registry.validate()?;
        Ok(registry)
    }

    pub fn option_cells(&self) -> impl Iterator<Item = &CapabilityOptionCellV2> {
        self.capabilities
            .iter()
            .flat_map(|row| row.option_cells.iter())
    }

    pub fn validate(&self) -> Result<(), CapabilityRegistryV2Error> {
        let fail = |message: String| CapabilityRegistryV2Error::Contract(message);
        if self.registry_schema_version != 2
            || self.registry_id != "quickpls.capability_registry.v2"
        {
            return Err(fail(
                "the registry identity must be schema 2 quickpls.capability_registry.v2".into(),
            ));
        }
        if self.registry_version.trim().is_empty() || self.frozen_on.trim().is_empty() {
            return Err(fail(
                "registry_version and frozen_on must be nonempty".into(),
            ));
        }
        if self.catalogue_snapshot.capability_row_count != self.capabilities.len() {
            return Err(fail(
                "catalogue row count does not match capabilities".into(),
            ));
        }
        let active_count = self
            .capabilities
            .iter()
            .filter(|row| row.official_lifecycle == OfficialLifecycleV2::Active)
            .count();
        if self.catalogue_snapshot.active_row_count != active_count {
            return Err(fail(
                "active row count does not match capability lifecycles".into(),
            ));
        }

        let expected_positions: Vec<_> = (1..=self.capabilities.len()).collect();
        let actual_positions: Vec<_> = self
            .capabilities
            .iter()
            .map(|row| row.catalogue_position)
            .collect();
        if actual_positions != expected_positions {
            return Err(fail(
                "catalogue positions must be unique and contiguous in source order".into(),
            ));
        }

        let mut row_ids = BTreeSet::new();
        let mut cell_ids = BTreeSet::new();
        for row in &self.capabilities {
            if !row_ids.insert(row.capability_id.as_str()) {
                return Err(fail(format!(
                    "duplicate capability row {}",
                    row.capability_id
                )));
            }
            if row.option_cells.is_empty() {
                return Err(fail(format!("{} has no option cells", row.capability_id)));
            }
            if !row.official_url.starts_with("https://smartpls.com/") {
                return Err(fail(format!(
                    "{} has a non-official URL",
                    row.capability_id
                )));
            }
            for cell in &row.option_cells {
                if cell.capability_id != row.capability_id {
                    return Err(fail(format!(
                        "{} contains an option cell owned by {}",
                        row.capability_id, cell.capability_id
                    )));
                }
                let identity = (
                    cell.capability_id.as_str(),
                    cell.cell_id.as_str(),
                    cell.capability_version.as_str(),
                );
                if !cell_ids.insert(identity) {
                    return Err(fail(format!(
                        "duplicate option-cell identity {}::{}::{}",
                        identity.0, identity.1, identity.2
                    )));
                }
                let Some(link) = cell.qualification_spec.links.as_slice().first() else {
                    return Err(fail(format!("{} has no qualification link", cell.cell_id)));
                };
                if cell.qualification_spec.links.len() != 1
                    || link.registry_schema_version != 2
                    || link.capability_id != cell.capability_id
                    || link.cell_id != cell.cell_id
                    || link.capability_version != cell.capability_version
                {
                    return Err(fail(format!(
                        "{} does not own one exact four-field qualification link",
                        cell.cell_id
                    )));
                }
                if cell.surface == ProductSurfaceV2::Standard && !cell.standard_available() {
                    return Err(fail(format!(
                        "{} is labelled Standard without documented coverage and release evidence",
                        cell.cell_id
                    )));
                }
                if row.official_lifecycle == OfficialLifecycleV2::Legacy
                    && (cell.coverage_state != CoverageStateV2::IntentionallyExcluded
                        || cell.evidence_state != EvidenceStateV2::Absent
                        || cell.surface != ProductSurfaceV2::Legacy)
                {
                    return Err(fail(format!(
                        "legacy capability {} must remain an evidence-absent intentional exclusion",
                        row.capability_id
                    )));
                }
                if row.official_lifecycle == OfficialLifecycleV2::Active
                    && cell.coverage_state == CoverageStateV2::IntentionallyExcluded
                {
                    return Err(fail(format!(
                        "active capability {} cannot be intentionally excluded",
                        row.capability_id
                    )));
                }
            }

            let projection = derive_row_projection(&row.option_cells)?;
            if (row.coverage_state, row.evidence_state, row.surface) != projection {
                return Err(fail(format!(
                    "{} row state is not the conservative option-cell projection",
                    row.capability_id
                )));
            }
        }
        Ok(())
    }
}

fn derive_row_projection(
    cells: &[CapabilityOptionCellV2],
) -> Result<(CoverageStateV2, EvidenceStateV2, ProductSurfaceV2), CapabilityRegistryV2Error> {
    let intentional = cells
        .iter()
        .filter(|cell| cell.coverage_state == CoverageStateV2::IntentionallyExcluded)
        .count();
    if intentional > 0 && intentional != cells.len() {
        return Err(CapabilityRegistryV2Error::Contract(
            "active and intentionally excluded option cells cannot share a row".into(),
        ));
    }
    let coverage = if intentional == cells.len() {
        CoverageStateV2::IntentionallyExcluded
    } else if cells
        .iter()
        .any(|cell| cell.coverage_state == CoverageStateV2::Absent)
    {
        CoverageStateV2::Absent
    } else if cells
        .iter()
        .any(|cell| cell.coverage_state == CoverageStateV2::Partial)
    {
        CoverageStateV2::Partial
    } else {
        CoverageStateV2::Full
    };
    let evidence = cells
        .iter()
        .map(|cell| cell.evidence_state)
        .min()
        .ok_or_else(|| {
            CapabilityRegistryV2Error::Contract("option-cell projection is empty".into())
        })?;
    let surface = if cells
        .iter()
        .all(|cell| cell.surface == ProductSurfaceV2::Legacy)
    {
        ProductSurfaceV2::Legacy
    } else if cells
        .iter()
        .all(|cell| cell.surface == ProductSurfaceV2::Internal)
    {
        ProductSurfaceV2::Internal
    } else if cells
        .iter()
        .all(|cell| cell.surface == ProductSurfaceV2::Standard)
    {
        ProductSurfaceV2::Standard
    } else {
        ProductSurfaceV2::Labs
    };
    Ok((coverage, evidence, surface))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value() -> serde_json::Value {
        serde_json::from_str(CAPABILITY_REGISTRY_V2_JSON).unwrap()
    }

    #[test]
    fn embedded_registry_is_the_exact_option_cell_authority() {
        let registry = CapabilityRegistryV2::embedded().unwrap();
        assert_eq!(registry.capabilities.len(), 45);
        assert_eq!(registry.catalogue_snapshot.active_row_count, 43);
        assert_eq!(registry.source_sha256.len(), 64);
        assert!(registry.option_cells().count() >= registry.capabilities.len());
        let standard_cells: Vec<_> = registry
            .option_cells()
            .filter(|cell| cell.standard_available())
            .collect();
        assert_eq!(standard_cells.len(), 33);
        assert!(standard_cells.iter().all(|cell| {
            matches!(
                cell.cell_id.as_str(),
                "qpls3.pls.algorithm"
                    | "qpls3.pls.posthoc_technical_minimum_sample_size"
                    | "qpls3.pls.sample_size_power"
                    | "qpls3.pls.consistent"
                    | "qpls3.pls.weighted"
                    | "qpls3.assessment.cca_residuals"
                    | "qpls3.assessment.cta_pls"
                    | "qpls3.assessment.ipma"
                    | "qpls3.inference.bootstrap"
                    | "qpls3.inference.consistent_bootstrap"
                    | "qpls3.pls.higher_order_two_stage"
                    | "qpls3.pls.mediation"
                    | "qpls3.pls.general_sem_multiple_mediation_bootstrap"
                    | "qpls3.pls.general_sem_multiple_two_way_moderation_point"
                    | "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap"
                    | "qpls3.groups.micom_permutation_mga"
                    | "qpls3.inference.structural_path_randomization"
                    | "qpls3.prediction.plspredict_cvpat"
                    | "qpls3.standalone.pca"
                    | "qpls3.gsca.als"
                    | "qpls3.standalone.nca"
                    | "qpls3.standalone.logistic"
                    | "qpls3.standalone.ols"
                    | "qpls3.standalone.regression_bootstrap"
                    | "qpls3.standalone.process"
                    | "qpls3.cbsem.ml"
                    | "qpls3.cbsem.bootstrap"
            ) && cell.coverage_state == CoverageStateV2::Partial
        }));
        assert_eq!(
            standard_cells
                .iter()
                .filter(|cell| cell.cell_id == "qpls3.pls.consistent")
                .count(),
            1
        );
        assert_eq!(
            standard_cells
                .iter()
                .filter(|cell| cell.cell_id == "qpls3.gsca.als")
                .count(),
            1
        );
        assert_eq!(
            standard_cells
                .iter()
                .filter(|cell| cell.cell_id == "qpls3.assessment.cca_residuals")
                .count(),
            1
        );
        assert_eq!(
            standard_cells
                .iter()
                .filter(|cell| cell.cell_id == "qpls3.assessment.cta_pls")
                .count(),
            1
        );
        assert_eq!(
            standard_cells
                .iter()
                .filter(|cell| cell.cell_id == "qpls3.assessment.ipma")
                .count(),
            1
        );
        assert_eq!(
            standard_cells
                .iter()
                .filter(|cell| cell.cell_id == "qpls3.inference.bootstrap")
                .count(),
            1
        );
        assert_eq!(
            standard_cells
                .iter()
                .filter(|cell| cell.cell_id == "qpls3.pls.higher_order_two_stage")
                .count(),
            1
        );
        assert_eq!(
            standard_cells
                .iter()
                .filter(|cell| cell.cell_id == "qpls3.groups.micom_permutation_mga")
                .count(),
            3
        );
        assert_eq!(
            standard_cells
                .iter()
                .filter(|cell| cell.cell_id == "qpls3.inference.structural_path_randomization")
                .count(),
            1
        );
        assert_eq!(
            standard_cells
                .iter()
                .filter(|cell| cell.cell_id == "qpls3.prediction.plspredict_cvpat")
                .count(),
            2
        );
        assert_eq!(
            standard_cells
                .iter()
                .filter(|cell| cell.cell_id == "qpls3.standalone.process")
                .count(),
            2
        );
        assert_eq!(
            standard_cells
                .iter()
                .filter(|cell| cell.cell_id == "qpls3.cbsem.ml")
                .count(),
            2
        );
        assert_eq!(
            standard_cells
                .iter()
                .filter(|cell| cell.cell_id == "qpls3.cbsem.bootstrap")
                .count(),
            1
        );
        assert!(
            registry
                .option_cells()
                .any(CapabilityOptionCellV2::labs_available)
        );
    }

    #[test]
    fn registry_rejects_a_row_projection_that_overstates_its_cells() {
        let mut input = value();
        input["capabilities"][1]["coverage_state"] = serde_json::json!("full");
        let error = CapabilityRegistryV2::from_json(&input.to_string()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("conservative option-cell projection")
        );
    }

    #[test]
    fn registry_rejects_standard_without_release_evidence_and_duplicate_cell_identities() {
        let mut standard = value();
        standard["capabilities"][9]["surface"] = serde_json::json!("standard");
        standard["capabilities"][9]["option_cells"][0]["surface"] = serde_json::json!("standard");
        let error = CapabilityRegistryV2::from_json(&standard.to_string()).unwrap_err();
        assert!(error.to_string().contains("labelled Standard"));

        let mut duplicate = value();
        let first = duplicate["capabilities"][0]["option_cells"][0].clone();
        duplicate["capabilities"][0]["option_cells"]
            .as_array_mut()
            .unwrap()
            .push(first);
        let error = CapabilityRegistryV2::from_json(&duplicate.to_string()).unwrap_err();
        assert!(error.to_string().contains("duplicate option-cell identity"));
    }

    #[test]
    fn registry_rejects_a_drifted_or_non_exact_qualification_link() {
        let mut input = value();
        input["capabilities"][0]["option_cells"][0]["qualification_spec"]["links"][0]["capability_version"] =
            serde_json::json!("wrong_version");
        let error = CapabilityRegistryV2::from_json(&input.to_string()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("one exact four-field qualification link")
        );

        let mut extra = value();
        extra["capabilities"][0]["option_cells"][0]["qualification_spec"]["links"][0]["unexpected"] =
            serde_json::json!(true);
        assert!(matches!(
            CapabilityRegistryV2::from_json(&extra.to_string()),
            Err(CapabilityRegistryV2Error::Json(_))
        ));
    }
}
