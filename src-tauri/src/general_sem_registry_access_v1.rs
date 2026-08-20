//! Exact-cell product-surface authorization for General SEM execution.
//!
//! Capability Registry V2 is the sole authority for whether a qualified cell
//! belongs to Standard or Labs. Callers may request a surface, but cannot use
//! that request to promote, downgrade, or make an unavailable cell executable.

use qpls_core::{
    cbsem_general_sem_ml_capability_cell_v1, cbsem_recursive_sem_bootstrap_capability_cell_v1,
    pls_general_bootstrap_capability_cell_v1,
    pls_general_higher_order_bootstrap_capability_cell_v1,
    pls_general_higher_order_point_capability_cell_v1,
    pls_general_multiple_moderation_bootstrap_capability_cell_v1,
    pls_general_multiple_moderation_point_capability_cell_v1,
    pls_general_recursive_effects_capability_cell_v1,
    pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1,
    CapabilityCellReferenceV2, CapabilityRegistryV2, GeneralSemConfigV1, GeneralSemInferenceV1,
    SemCapabilityDecisionV1, SemDerivedTermV4, SemModelV4,
};

pub(crate) const GENERAL_SEM_STANDARD_SURFACE: &str = "standard";
pub(crate) const GENERAL_SEM_INTERNAL_LABS_SURFACE: &str = "internal_labs";
pub(crate) const GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1: &str =
    "native_general_sem_pls_labs_v1";
pub(crate) const GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1: &str =
    "native_general_sem_pls_standard_v1";
pub(crate) const GENERAL_SEM_CBSEM_LABS_RECIPE_EXECUTION_SURFACE_V1: &str =
    "native_general_sem_cbsem_labs_v1";

pub(crate) fn general_sem_recipe_execution_surface_v1(surface: &str) -> Option<&'static str> {
    match surface {
        GENERAL_SEM_STANDARD_SURFACE => Some(GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1),
        GENERAL_SEM_INTERNAL_LABS_SURFACE => Some(GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1),
        _ => None,
    }
}

pub(crate) fn general_sem_cbsem_recipe_execution_surface_v1(
    surface: &str,
) -> Option<&'static str> {
    (surface == GENERAL_SEM_INTERNAL_LABS_SURFACE)
        .then_some(GENERAL_SEM_CBSEM_LABS_RECIPE_EXECUTION_SURFACE_V1)
}

/// Bounded PLS General SEM execution inventory. The base PLS dependency is
/// intentionally excluded because it cannot own General SEM persistence.
pub(crate) fn is_pls_general_sem_execution_cell_v1(cell: &CapabilityCellReferenceV2) -> bool {
    matches!(
        (
            cell.registry_schema_version,
            cell.capability_id.as_str(),
            cell.cell_id.as_str(),
            cell.capability_version.as_str(),
        ),
        (
            2,
            "smartpls.mediation",
            "qpls3.pls.mediation",
            "pls_mediation_v1"
        ) | (
            2,
            "smartpls.mediation",
            "qpls3.pls.general_sem_multiple_mediation_bootstrap",
            "general_sem_pls_full_model_case_bootstrap_v1"
        ) | (
            2,
            "smartpls.moderation",
            "qpls3.pls.general_sem_multiple_two_way_moderation_point",
            "general_sem_pls_multiple_two_way_moderation_point_v1"
        ) | (
            2,
            "smartpls.moderation",
            "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
            "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1"
        ) | (
            2,
            "smartpls.higher_order_models",
            "qpls3.pls.general_sem_higher_order_point",
            "general_sem_pls_higher_order_point_v1"
        ) | (
            2,
            "smartpls.higher_order_models",
            "qpls3.pls.general_sem_higher_order_full_model_case_bootstrap",
            "general_sem_pls_higher_order_full_model_case_bootstrap_v1"
        ) | (
            2,
            "smartpls.mediation",
            "qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap",
            "general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1"
        )
    )
}

/// Bounded Rank-3 CB-SEM V3 execution inventory. These cells remain Labs-only
/// until their independent qualification evidence is complete.
pub(crate) fn is_rank3_general_sem_cbsem_execution_cell_v1(
    cell: &CapabilityCellReferenceV2,
) -> bool {
    *cell == cbsem_general_sem_ml_capability_cell_v1()
        || *cell == cbsem_recursive_sem_bootstrap_capability_cell_v1()
}

pub(crate) fn is_general_sem_execution_cell_v1(cell: &CapabilityCellReferenceV2) -> bool {
    is_pls_general_sem_execution_cell_v1(cell)
        || is_rank3_general_sem_cbsem_execution_cell_v1(cell)
}

/// Selects the exact Rank-3 execution owner from resident Recipe V4 inference.
/// Point estimation owns non-bootstrap runs; the recursive SEM bootstrap cell
/// owns runs that include exact case-bootstrap inference.
pub(crate) fn selected_general_sem_cbsem_execution_cell_v1(
    config: &GeneralSemConfigV1,
) -> CapabilityCellReferenceV2 {
    match config.inference {
        GeneralSemInferenceV1::None => cbsem_general_sem_ml_capability_cell_v1(),
        GeneralSemInferenceV1::CaseBootstrap { .. } => {
            cbsem_recursive_sem_bootstrap_capability_cell_v1()
        }
    }
}

/// Selects the one cell that owns this exact point-or-bootstrap operation.
/// Capability decisions canonically sort their declared cells, so collection
/// position cannot encode primary-versus-supplemental execution ownership.
pub(crate) fn selected_general_sem_execution_cell_v1(
    model: &SemModelV4,
    config: &GeneralSemConfigV1,
) -> CapabilityCellReferenceV2 {
    let has_two_way_interactions = model
        .derived_terms
        .iter()
        .any(|term| matches!(term, SemDerivedTermV4::InteractionV2 { .. }));
    let has_higher_order = model
        .derived_terms
        .iter()
        .any(|term| matches!(term, SemDerivedTermV4::HigherOrder { .. }));
    if has_higher_order {
        return match config.inference {
            GeneralSemInferenceV1::None => pls_general_higher_order_point_capability_cell_v1(),
            GeneralSemInferenceV1::CaseBootstrap { .. } => {
                pls_general_higher_order_bootstrap_capability_cell_v1()
            }
        };
    }
    if has_two_way_interactions
        && matches!(&config.inference, GeneralSemInferenceV1::CaseBootstrap { .. })
        && !config.requested_effect_estimands.is_empty()
    {
        return pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1();
    }
    selected_general_sem_execution_cell_for_topology_v1(has_two_way_interactions, &config.inference)
}

fn selected_general_sem_execution_cell_for_topology_v1(
    has_two_way_interactions: bool,
    inference: &GeneralSemInferenceV1,
) -> CapabilityCellReferenceV2 {
    match (has_two_way_interactions, inference) {
        (false, GeneralSemInferenceV1::None) => pls_general_recursive_effects_capability_cell_v1(),
        (false, GeneralSemInferenceV1::CaseBootstrap { .. }) => {
            pls_general_bootstrap_capability_cell_v1()
        }
        (true, GeneralSemInferenceV1::None) => {
            pls_general_multiple_moderation_point_capability_cell_v1()
        }
        (true, GeneralSemInferenceV1::CaseBootstrap { .. }) => {
            pls_general_multiple_moderation_bootstrap_capability_cell_v1()
        }
    }
}

/// Proves that the exact topology/config-selected owner is one of the cells
/// declared by the independently validated capability decision.
pub(crate) fn decision_declares_general_sem_execution_cell_v1(
    decision: &SemCapabilityDecisionV1,
    selected: &CapabilityCellReferenceV2,
) -> bool {
    decision.capability_cells().iter().any(|cell| {
        cell.registry_schema_version() == selected.registry_schema_version
            && cell.capability_id() == selected.capability_id.as_str()
            && cell.cell_id() == selected.cell_id.as_str()
            && cell.capability_version() == selected.capability_version.as_str()
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GeneralSemRegistryAccessErrorV1 {
    RegistryInvalid(String),
    CapabilityUnavailable,
    StandardSurfaceRequired,
    InternalLabsRequired,
}

pub(crate) fn authorize_general_sem_registry_access_v1(
    surface: &str,
    experimental_labs_enabled: bool,
    capability_cell: &CapabilityCellReferenceV2,
) -> Result<(), GeneralSemRegistryAccessErrorV1> {
    let registry = CapabilityRegistryV2::embedded()
        .map_err(|error| GeneralSemRegistryAccessErrorV1::RegistryInvalid(error.to_string()))?;
    authorize_general_sem_registry_access_with_v1(
        &registry,
        surface,
        experimental_labs_enabled,
        capability_cell,
    )
}

/// Read-only result reopening deliberately does not grant calculation or
/// mutation authority. A recognized Labs cell may be read without opt-in,
/// while its surface identity remains Labs and cannot be relabelled.
pub(crate) fn authorize_general_sem_registry_read_access_v1(
    surface: &str,
    capability_cell: &CapabilityCellReferenceV2,
) -> Result<(), GeneralSemRegistryAccessErrorV1> {
    let registry = CapabilityRegistryV2::embedded()
        .map_err(|error| GeneralSemRegistryAccessErrorV1::RegistryInvalid(error.to_string()))?;
    authorize_general_sem_registry_read_access_with_v1(&registry, surface, capability_cell)
}

fn authorize_general_sem_registry_read_access_with_v1(
    registry: &CapabilityRegistryV2,
    surface: &str,
    capability_cell: &CapabilityCellReferenceV2,
) -> Result<(), GeneralSemRegistryAccessErrorV1> {
    if capability_cell.registry_schema_version != 2 {
        return Err(GeneralSemRegistryAccessErrorV1::CapabilityUnavailable);
    }
    let matching_cells = registry
        .option_cells()
        .filter(|cell| {
            cell.capability_id == capability_cell.capability_id
                && cell.cell_id == capability_cell.cell_id
                && cell.capability_version == capability_cell.capability_version
        })
        .collect::<Vec<_>>();
    let [cell] = matching_cells.as_slice() else {
        return Err(GeneralSemRegistryAccessErrorV1::CapabilityUnavailable);
    };
    if cell.standard_available() {
        return if surface == GENERAL_SEM_STANDARD_SURFACE
            || surface == GENERAL_SEM_INTERNAL_LABS_SURFACE
        {
            Ok(())
        } else {
            Err(GeneralSemRegistryAccessErrorV1::StandardSurfaceRequired)
        };
    }
    if cell.labs_available() {
        return if surface == GENERAL_SEM_INTERNAL_LABS_SURFACE {
            Ok(())
        } else {
            Err(GeneralSemRegistryAccessErrorV1::InternalLabsRequired)
        };
    }
    Err(GeneralSemRegistryAccessErrorV1::CapabilityUnavailable)
}

fn authorize_general_sem_registry_access_with_v1(
    registry: &CapabilityRegistryV2,
    surface: &str,
    experimental_labs_enabled: bool,
    capability_cell: &CapabilityCellReferenceV2,
) -> Result<(), GeneralSemRegistryAccessErrorV1> {
    if capability_cell.registry_schema_version != 2 {
        return Err(GeneralSemRegistryAccessErrorV1::CapabilityUnavailable);
    }
    let matching_cells = registry
        .option_cells()
        .filter(|cell| {
            cell.capability_id == capability_cell.capability_id
                && cell.cell_id == capability_cell.cell_id
                && cell.capability_version == capability_cell.capability_version
        })
        .collect::<Vec<_>>();
    let [cell] = matching_cells.as_slice() else {
        return Err(GeneralSemRegistryAccessErrorV1::CapabilityUnavailable);
    };

    if cell.standard_available() {
        return if surface == GENERAL_SEM_STANDARD_SURFACE {
            Ok(())
        } else {
            Err(GeneralSemRegistryAccessErrorV1::StandardSurfaceRequired)
        };
    }
    if cell.labs_available() {
        return if surface == GENERAL_SEM_INTERNAL_LABS_SURFACE && experimental_labs_enabled {
            Ok(())
        } else {
            Err(GeneralSemRegistryAccessErrorV1::InternalLabsRequired)
        };
    }
    Err(GeneralSemRegistryAccessErrorV1::CapabilityUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::CAPABILITY_REGISTRY_V2_JSON;
    use serde_json::{Value, json};

    const CAPABILITY_ID: &str = "smartpls.mediation";
    const CELL_ID: &str = "qpls3.pls.mediation";
    const CAPABILITY_VERSION: &str = "pls_mediation_v1";

    fn cell() -> CapabilityCellReferenceV2 {
        CapabilityCellReferenceV2 {
            registry_schema_version: 2,
            capability_id: CAPABILITY_ID.into(),
            cell_id: CELL_ID.into(),
            capability_version: CAPABILITY_VERSION.into(),
        }
    }

    fn bootstrap_inference() -> GeneralSemInferenceV1 {
        GeneralSemInferenceV1::CaseBootstrap {
            resamples: 500,
            seed: 7,
            confidence_level: 0.95,
            interval: qpls_core::GeneralSemBootstrapIntervalV1::Percentile,
            tail: qpls_core::GeneralSemInferenceTailV1::TwoSided,
        }
    }

    #[test]
    fn execution_owner_selection_is_explicit_for_mediation_point_and_bootstrap() {
        assert_eq!(
            selected_general_sem_execution_cell_for_topology_v1(
                false,
                &GeneralSemInferenceV1::None,
            ),
            pls_general_recursive_effects_capability_cell_v1()
        );
        assert_eq!(
            selected_general_sem_execution_cell_for_topology_v1(false, &bootstrap_inference()),
            pls_general_bootstrap_capability_cell_v1()
        );
    }

    #[test]
    fn execution_owner_selection_is_explicit_for_moderation_point_and_bootstrap() {
        assert_eq!(
            selected_general_sem_execution_cell_for_topology_v1(true, &GeneralSemInferenceV1::None,),
            pls_general_multiple_moderation_point_capability_cell_v1()
        );
        assert_eq!(
            selected_general_sem_execution_cell_for_topology_v1(true, &bootstrap_inference()),
            pls_general_multiple_moderation_bootstrap_capability_cell_v1()
        );
    }

    fn registry_with_cell_state(surface: &str, evidence_state: &str) -> CapabilityRegistryV2 {
        let mut source: Value = serde_json::from_str(CAPABILITY_REGISTRY_V2_JSON).unwrap();
        let rows = source["capabilities"].as_array_mut().unwrap();
        let row = rows
            .iter_mut()
            .find(|row| {
                row["option_cells"].as_array().is_some_and(|cells| {
                    cells.iter().any(|candidate| {
                        candidate["capability_id"] == json!(CAPABILITY_ID)
                            && candidate["cell_id"] == json!(CELL_ID)
                            && candidate["capability_version"] == json!(CAPABILITY_VERSION)
                    })
                })
            })
            .unwrap();
        let target = row["option_cells"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|candidate| {
                candidate["capability_id"] == json!(CAPABILITY_ID)
                    && candidate["cell_id"] == json!(CELL_ID)
                    && candidate["capability_version"] == json!(CAPABILITY_VERSION)
            })
            .unwrap();
        target["surface"] = json!(surface);
        target["evidence_state"] = json!(evidence_state);
        row["surface"] = json!(if surface == "standard" {
            "standard"
        } else {
            "labs"
        });
        row["evidence_state"] = json!(evidence_state);
        CapabilityRegistryV2::from_json(&source.to_string()).unwrap()
    }

    #[test]
    fn standard_cell_runs_without_labs_opt_in_and_cannot_be_relabelled_as_labs() {
        let registry = registry_with_cell_state("standard", "release_qualified");
        assert_eq!(
            authorize_general_sem_registry_access_with_v1(
                &registry,
                GENERAL_SEM_STANDARD_SURFACE,
                false,
                &cell(),
            ),
            Ok(())
        );
        assert_eq!(
            authorize_general_sem_registry_access_with_v1(
                &registry,
                GENERAL_SEM_INTERNAL_LABS_SURFACE,
                true,
                &cell(),
            ),
            Err(GeneralSemRegistryAccessErrorV1::StandardSurfaceRequired)
        );
        assert_eq!(
            authorize_general_sem_registry_read_access_with_v1(
                &registry,
                GENERAL_SEM_INTERNAL_LABS_SURFACE,
                &cell(),
            ),
            Ok(())
        );
    }

    #[test]
    fn promotion_is_cell_atomic_when_standard_and_labs_cells_coexist() {
        let mut source: Value = serde_json::from_str(CAPABILITY_REGISTRY_V2_JSON).unwrap();
        let rows = source["capabilities"].as_array_mut().unwrap();
        for target in rows
            .iter_mut()
            .flat_map(|row| row["option_cells"].as_array_mut().unwrap())
        {
            let cell_id = target["cell_id"].as_str().unwrap_or_default();
            if cell_id == CELL_ID {
                target["surface"] = json!("standard");
                target["evidence_state"] = json!("release_qualified");
            } else if cell_id == "qpls3.pls.general_sem_multiple_two_way_moderation_point" {
                target["surface"] = json!("labs");
                target["evidence_state"] = json!("archive_qualified");
            }
        }
        let registry = CapabilityRegistryV2::from_json(&source.to_string()).unwrap();
        let moderation = CapabilityCellReferenceV2 {
            registry_schema_version: 2,
            capability_id: "smartpls.moderation".into(),
            cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_point".into(),
            capability_version: "general_sem_pls_multiple_two_way_moderation_point_v1".into(),
        };

        assert_eq!(
            authorize_general_sem_registry_access_with_v1(
                &registry,
                GENERAL_SEM_STANDARD_SURFACE,
                false,
                &cell(),
            ),
            Ok(())
        );
        assert_eq!(
            authorize_general_sem_registry_access_with_v1(
                &registry,
                GENERAL_SEM_STANDARD_SURFACE,
                false,
                &moderation,
            ),
            Err(GeneralSemRegistryAccessErrorV1::InternalLabsRequired)
        );
        assert_eq!(
            authorize_general_sem_registry_access_with_v1(
                &registry,
                GENERAL_SEM_INTERNAL_LABS_SURFACE,
                true,
                &moderation,
            ),
            Ok(())
        );
    }

    #[test]
    fn labs_cell_requires_the_labs_surface_and_explicit_opt_in() {
        let registry = registry_with_cell_state("labs", "archive_qualified");
        assert_eq!(
            authorize_general_sem_registry_access_with_v1(
                &registry,
                GENERAL_SEM_INTERNAL_LABS_SURFACE,
                false,
                &cell(),
            ),
            Err(GeneralSemRegistryAccessErrorV1::InternalLabsRequired)
        );
        assert_eq!(
            authorize_general_sem_registry_access_with_v1(
                &registry,
                GENERAL_SEM_STANDARD_SURFACE,
                false,
                &cell(),
            ),
            Err(GeneralSemRegistryAccessErrorV1::InternalLabsRequired)
        );
        assert_eq!(
            authorize_general_sem_registry_access_with_v1(
                &registry,
                GENERAL_SEM_INTERNAL_LABS_SURFACE,
                true,
                &cell(),
            ),
            Ok(())
        );
        assert_eq!(
            authorize_general_sem_registry_read_access_with_v1(
                &registry,
                GENERAL_SEM_INTERNAL_LABS_SURFACE,
                &cell(),
            ),
            Ok(())
        );
    }

    #[test]
    fn tampered_or_unavailable_exact_cells_fail_closed() {
        let registry = registry_with_cell_state("labs", "archive_qualified");
        let mut tampered = cell();
        tampered.capability_version.push_str(".tampered");
        assert_eq!(
            authorize_general_sem_registry_access_with_v1(
                &registry,
                GENERAL_SEM_INTERNAL_LABS_SURFACE,
                true,
                &tampered,
            ),
            Err(GeneralSemRegistryAccessErrorV1::CapabilityUnavailable)
        );

        // Internal is a valid registry state but is deliberately not executable
        // through either customer surface. Keeping the evidence unchanged also
        // preserves the row's conservative projection, so this reaches the
        // authorization branch instead of testing registry parsing again.
        let unavailable = registry_with_cell_state("internal", "archive_qualified");
        assert_eq!(
            authorize_general_sem_registry_access_with_v1(
                &unavailable,
                GENERAL_SEM_INTERNAL_LABS_SURFACE,
                true,
                &cell(),
            ),
            Err(GeneralSemRegistryAccessErrorV1::CapabilityUnavailable)
        );
    }
}
