//! Observed-variable interventional mediation V1.
//!
//! This is not a PLS estimator and it does not turn an associational SEM result
//! into a causal result.  The kernel fits explicitly supplied observed-variable
//! linear equations, checks a declared identification/positivity contract, and
//! evaluates a joint mediator-distribution mean intervention by parametric
//! g-computation.  Results retain deliberately cautious interpretation text.

use arrow::array::{Array, BooleanArray, Float64Array, Int64Array};
use qpls_core::{
    CausalIdentificationChecklistV1 as CoreCausalIdentificationChecklistV1,
    CausalPositivityPolicyV1 as CoreCausalPositivityPolicyV1,
    InterventionalCausalMediationConfigV1 as CoreInterventionalCausalMediationConfigV1,
    ObservedCausalPathV1 as CoreObservedCausalPathV1,
    ObservedTreatmentContrastV1 as CoreObservedTreatmentContrastV1, ResolvedObservedCausalRoleV1,
    SemDataBindingV4, SemModelV4, canonicalize_interventional_causal_config_v1,
    resolve_observed_causal_role_v1,
};
use qpls_data::Dataset;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const INTERVENTIONAL_CAUSAL_MEDIATION_METHOD_VERSION_V1: &str =
    "interventional_causal_mediation_v1";
pub const INTERVENTIONAL_MEDIATION_INTERPRETATION_V1: &str =
    "assumption-dependent interventional estimate; causality is not established by the calculation";
pub const INTERVENTIONAL_MEDIATION_MINIMUM_OBSERVATIONS_V1: usize = 20;
pub const INTERVENTIONAL_MEDIATION_MAX_PATH_EDGES_V1: usize = 4;
pub const INTERVENTIONAL_MEDIATION_MAX_TERM_FACTORS_V1: usize = 3;
const OLS_RELATIVE_RANK_TOLERANCE_V1: f64 = 1.0e-10;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservedTreatmentKindV1 {
    Binary,
    ContinuousContrast,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservedTreatmentContrastV1 {
    pub kind: ObservedTreatmentKindV1,
    pub x0: f64,
    pub x1: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservedNumericColumnV1 {
    pub variable_id: String,
    pub values: Vec<f64>,
}

/// A design-matrix term is the row-wise product of one to three factors.
/// Products may contain at most one post-treatment mediator.  Treatment and
/// baseline-variable factors are fixed under an intervention, so replacing a
/// mediator by its conditional intervention mean remains exact for this linear
/// expectation model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservedLinearTermV1 {
    pub term_id: String,
    pub factor_variable_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservedLinearEquationV1 {
    pub equation_id: String,
    pub outcome_variable_id: String,
    pub terms: Vec<ObservedLinearTermV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InterventionalIdentificationChecklistV1 {
    pub temporal_order_reviewed: bool,
    pub consistency_reviewed: bool,
    pub treatment_outcome_exchangeability_reviewed: bool,
    pub treatment_mediator_exchangeability_reviewed: bool,
    pub mediator_outcome_exchangeability_reviewed: bool,
    pub no_exposure_induced_mediator_outcome_confounder_reviewed: bool,
    pub no_recanting_witness_reviewed: bool,
    pub linear_model_specification_reviewed: bool,
}

impl InterventionalIdentificationChecklistV1 {
    fn missing_items(&self) -> Vec<&'static str> {
        [
            ("temporal_order", self.temporal_order_reviewed),
            ("consistency", self.consistency_reviewed),
            (
                "treatment_outcome_exchangeability",
                self.treatment_outcome_exchangeability_reviewed,
            ),
            (
                "treatment_mediator_exchangeability",
                self.treatment_mediator_exchangeability_reviewed,
            ),
            (
                "mediator_outcome_exchangeability",
                self.mediator_outcome_exchangeability_reviewed,
            ),
            (
                "no_exposure_induced_mediator_outcome_confounder",
                self.no_exposure_induced_mediator_outcome_confounder_reviewed,
            ),
            ("no_recanting_witness", self.no_recanting_witness_reviewed),
            (
                "linear_model_specification",
                self.linear_model_specification_reviewed,
            ),
        ]
        .into_iter()
        .filter_map(|(item, reviewed)| (!reviewed).then_some(item))
        .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterventionalPositivityPolicyV1 {
    pub minimum_binary_arm_count: usize,
    pub maximum_binary_arm_ratio: f64,
    pub positivity_strata_variable_ids: Vec<String>,
    pub minimum_count_per_binary_stratum_arm: usize,
    /// Continuous contrasts require observations within this fraction of the
    /// observed treatment range on each side of x0 and x1.
    pub continuous_neighborhood_fraction_of_range: f64,
    pub minimum_continuous_neighborhood_count: usize,
}

impl Default for InterventionalPositivityPolicyV1 {
    fn default() -> Self {
        Self {
            minimum_binary_arm_count: 10,
            maximum_binary_arm_ratio: 10.0,
            positivity_strata_variable_ids: Vec::new(),
            minimum_count_per_binary_stratum_arm: 1,
            continuous_neighborhood_fraction_of_range: 0.05,
            minimum_continuous_neighborhood_count: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct UnsupportedInterventionalFeatureRequestV1 {
    pub latent_composite_or_hoc_roles: bool,
    pub groups: bool,
    pub weights: bool,
    pub natural_or_cross_world_effects: bool,
    pub exposure_induced_mediator_outcome_confounder_present: bool,
    pub recanting_witness_present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterventionalCausalMediationInputV1 {
    pub analysis_id: String,
    pub columns: Vec<ObservedNumericColumnV1>,
    pub treatment_variable_id: String,
    /// One to three ordered observed mediators, producing a two- to four-edge
    /// path from treatment through the final outcome.
    pub ordered_mediator_variable_ids: Vec<String>,
    pub outcome_variable_id: String,
    pub adjustment_covariate_variable_ids: Vec<String>,
    pub baseline_moderator_variable_ids: Vec<String>,
    /// Optional fixed baseline moderator values.  Moderators without an entry
    /// are averaged over their empirical baseline distribution.
    pub baseline_moderator_intervention_values: BTreeMap<String, f64>,
    /// Equations must be ordered exactly as the mediators and then the outcome.
    pub equations: Vec<ObservedLinearEquationV1>,
    pub treatment_contrast: ObservedTreatmentContrastV1,
    pub identification_checklist: InterventionalIdentificationChecklistV1,
    pub positivity_policy: InterventionalPositivityPolicyV1,
    #[serde(default)]
    pub unsupported_features: UnsupportedInterventionalFeatureRequestV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InterventionalExcludedRowV1 {
    pub source_row_index: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedInterventionalDatasetPathV1 {
    pub path_id: String,
    /// Source-row positions in exact fit order. Positions may repeat for a
    /// case-bootstrap refit and are never silently deduplicated.
    pub source_row_indices: Vec<usize>,
    pub excluded_rows: Vec<InterventionalExcludedRowV1>,
    pub input: InterventionalCausalMediationInputV1,
}

#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "code", content = "detail", rename_all = "snake_case")]
pub enum InterventionalDatasetPreparationErrorV1 {
    #[error("causal dataset preparation requires the exact raw dataset bound by SemModelV4")]
    DatasetAuthority,
    #[error("causal role {0} is not a uniquely bound observed variable")]
    ObservedRoleBinding(String),
    #[error("source column {0} is missing, duplicated, or nonnumeric")]
    SourceColumn(String),
    #[error("source row {0} is outside the dataset")]
    SourceRowOutOfRange(usize),
    #[error("no complete finite rows remain after applying the shared causal role mask")]
    NoUsableRows,
    #[error("causal path {0} is absent from the selected configuration")]
    PathAuthority(String),
    #[error("causal configuration is invalid after canonical observed-role resolution: {0}")]
    InvalidConfiguration(String),
}

fn observed_role_bindings_v1(
    model: &SemModelV4,
    required_references: &BTreeSet<String>,
) -> Result<BTreeMap<String, ResolvedObservedCausalRoleV1>, InterventionalDatasetPreparationErrorV1>
{
    let mut resolved = BTreeMap::new();
    for reference in required_references {
        let binding = resolve_observed_causal_role_v1(model, reference).ok_or_else(|| {
            InterventionalDatasetPreparationErrorV1::ObservedRoleBinding(reference.clone())
        })?;
        resolved.insert(reference.clone(), binding);
    }
    Ok(resolved)
}

fn canonical_observed_id_v1<'a>(
    bindings: &'a BTreeMap<String, ResolvedObservedCausalRoleV1>,
    reference: &str,
) -> Result<&'a str, InterventionalDatasetPreparationErrorV1> {
    bindings
        .get(reference)
        .map(|binding| binding.variable_id.as_str())
        .ok_or_else(|| {
            InterventionalDatasetPreparationErrorV1::ObservedRoleBinding(reference.to_owned())
        })
}

fn numeric_observed_cell_v1(array: &dyn Array, row: usize) -> Option<f64> {
    if array.is_null(row) {
        return None;
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        Some(values.value(row))
    } else if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        Some(values.value(row) as f64)
    } else {
        array
            .as_any()
            .downcast_ref::<BooleanArray>()
            .map(|values| if values.value(row) { 1.0 } else { 0.0 })
    }
}

fn core_identification_v1(
    checklist: &CoreCausalIdentificationChecklistV1,
) -> InterventionalIdentificationChecklistV1 {
    InterventionalIdentificationChecklistV1 {
        temporal_order_reviewed: checklist.temporal_order_declared,
        consistency_reviewed: checklist.consistency_assumption_acknowledged,
        treatment_outcome_exchangeability_reviewed: checklist
            .no_unmeasured_treatment_outcome_confounding_acknowledged,
        treatment_mediator_exchangeability_reviewed: checklist
            .no_unmeasured_treatment_mediator_confounding_acknowledged,
        mediator_outcome_exchangeability_reviewed: checklist
            .no_unmeasured_mediator_outcome_confounding_acknowledged,
        no_exposure_induced_mediator_outcome_confounder_reviewed: checklist
            .no_exposure_induced_mediator_outcome_confounder_confirmed,
        no_recanting_witness_reviewed: checklist.no_recanting_witness_confirmed,
        linear_model_specification_reviewed: checklist.linear_model_specification_reviewed,
    }
}

fn core_positivity_policy_v1(
    policy: &CoreCausalPositivityPolicyV1,
) -> InterventionalPositivityPolicyV1 {
    InterventionalPositivityPolicyV1 {
        minimum_binary_arm_count: policy.minimum_binary_arm_count as usize,
        maximum_binary_arm_ratio: policy.maximum_binary_arm_ratio,
        positivity_strata_variable_ids: policy.positivity_strata_variable_ids.clone(),
        minimum_count_per_binary_stratum_arm: policy.minimum_count_per_binary_stratum_arm as usize,
        continuous_neighborhood_fraction_of_range: policy.continuous_neighborhood_fraction_of_range,
        minimum_continuous_neighborhood_count: policy.minimum_continuous_neighborhood_count
            as usize,
    }
}

fn core_treatment_contrast_v1(
    contrast: &CoreObservedTreatmentContrastV1,
) -> ObservedTreatmentContrastV1 {
    match contrast {
        CoreObservedTreatmentContrastV1::Binary { control, treated } => {
            ObservedTreatmentContrastV1 {
                kind: ObservedTreatmentKindV1::Binary,
                x0: *control,
                x1: *treated,
            }
        }
        CoreObservedTreatmentContrastV1::Continuous { x0, x1 } => ObservedTreatmentContrastV1 {
            kind: ObservedTreatmentKindV1::ContinuousContrast,
            x0: *x0,
            x1: *x1,
        },
    }
}

fn canonical_positivity_policy_v1(
    policy: &CoreCausalPositivityPolicyV1,
    bindings: &BTreeMap<String, ResolvedObservedCausalRoleV1>,
) -> Result<InterventionalPositivityPolicyV1, InterventionalDatasetPreparationErrorV1> {
    let mut canonical = core_positivity_policy_v1(policy);
    canonical.positivity_strata_variable_ids = policy
        .positivity_strata_variable_ids
        .iter()
        .map(|reference| canonical_observed_id_v1(bindings, reference).map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(canonical)
}

fn core_equations_v1(
    path: &CoreObservedCausalPathV1,
    bindings: &BTreeMap<String, ResolvedObservedCausalRoleV1>,
) -> Result<Vec<ObservedLinearEquationV1>, InterventionalDatasetPreparationErrorV1> {
    path.equations
        .iter()
        .map(|equation| {
            let terms = equation
                .terms
                .iter()
                .map(|term| {
                    let factor_variable_ids = term
                        .factor_variable_ids
                        .iter()
                        .map(|reference| {
                            canonical_observed_id_v1(bindings, reference).map(str::to_owned)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(ObservedLinearTermV1 {
                        term_id: term.term_id.clone(),
                        factor_variable_ids,
                    })
                })
                .collect::<Result<Vec<_>, InterventionalDatasetPreparationErrorV1>>()?;
            Ok(ObservedLinearEquationV1 {
                equation_id: equation.equation_id.clone(),
                outcome_variable_id: canonical_observed_id_v1(
                    bindings,
                    &equation.outcome_variable_id,
                )?
                .to_owned(),
                terms,
            })
        })
        .collect()
}

/// Builds the exact observed-data inputs selected by the public V1 contract.
/// A single complete-case mask over every declared causal role is used for all
/// selected paths, so path totals and their shared bootstrap ledger cannot be
/// based on silently different respondent sets.
pub fn prepare_interventional_causal_inputs_from_dataset_v1(
    dataset: &Dataset,
    model: &SemModelV4,
    config: &CoreInterventionalCausalMediationConfigV1,
    requested_source_rows: Option<&[usize]>,
) -> Result<Vec<PreparedInterventionalDatasetPathV1>, InterventionalDatasetPreparationErrorV1> {
    match &model.data_binding {
        SemDataBindingV4::Raw { dataset_id, .. } if dataset_id == &dataset.id.to_string() => {}
        _ => return Err(InterventionalDatasetPreparationErrorV1::DatasetAuthority),
    }
    let canonical_config =
        canonicalize_interventional_causal_config_v1(model, config).map_err(|error| {
            InterventionalDatasetPreparationErrorV1::InvalidConfiguration(error.to_string())
        })?;
    canonical_config.ensure_valid().map_err(|error| {
        InterventionalDatasetPreparationErrorV1::InvalidConfiguration(format!(
            "{}: {}",
            error.code, error.message
        ))
    })?;
    let config = &canonical_config;

    let mut required_references =
        BTreeSet::from([config.treatment.clone(), config.outcome.clone()]);
    required_references.extend(config.mediators.iter().cloned());
    required_references.extend(config.baseline_moderators.iter().cloned());
    required_references.extend(config.adjustment_covariates.iter().cloned());
    required_references.extend(
        config
            .positivity_policy
            .positivity_strata_variable_ids
            .iter()
            .cloned(),
    );
    for path in &config.paths {
        required_references.extend(path.ordered_variable_ids.iter().cloned());
        for equation in &path.equations {
            required_references.insert(equation.outcome_variable_id.clone());
            for term in &equation.terms {
                required_references.extend(term.factor_variable_ids.iter().cloned());
            }
        }
    }
    let bindings = observed_role_bindings_v1(model, &required_references)?;
    let source_columns = bindings
        .values()
        .map(|binding| (binding.variable_id.clone(), binding.source_column.clone()))
        .collect::<BTreeMap<_, _>>();
    let required_variable_ids = source_columns.keys().cloned().collect::<BTreeSet<_>>();
    let mut positions = BTreeMap::new();
    for (variable_id, source_column) in &source_columns {
        let matches = dataset
            .batch
            .schema()
            .fields()
            .iter()
            .enumerate()
            .filter_map(|(index, field)| (field.name() == source_column).then_some(index))
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(InterventionalDatasetPreparationErrorV1::SourceColumn(
                source_column.clone(),
            ));
        }
        let array = dataset.batch.column(matches[0]).as_ref();
        if !array.as_any().is::<Float64Array>()
            && !array.as_any().is::<Int64Array>()
            && !array.as_any().is::<BooleanArray>()
        {
            return Err(InterventionalDatasetPreparationErrorV1::SourceColumn(
                source_column.clone(),
            ));
        }
        positions.insert(variable_id.clone(), matches[0]);
    }

    let requested_rows = requested_source_rows
        .map(<[usize]>::to_vec)
        .unwrap_or_else(|| (0..dataset.batch.num_rows()).collect());
    let mut complete_rows = Vec::with_capacity(requested_rows.len());
    let mut excluded_rows = Vec::new();
    for source_row_index in requested_rows {
        if source_row_index >= dataset.batch.num_rows() {
            return Err(
                InterventionalDatasetPreparationErrorV1::SourceRowOutOfRange(source_row_index),
            );
        }
        let unusable = required_variable_ids.iter().find_map(|variable_id| {
            let position = positions[variable_id];
            let value =
                numeric_observed_cell_v1(dataset.batch.column(position).as_ref(), source_row_index);
            match value {
                None => Some(format!("missing:{variable_id}")),
                Some(value) if !value.is_finite() => Some(format!("nonfinite:{variable_id}")),
                Some(_) => None,
            }
        });
        if let Some(reason) = unusable {
            excluded_rows.push(InterventionalExcludedRowV1 {
                source_row_index,
                reason,
            });
        } else {
            complete_rows.push(source_row_index);
        }
    }
    if complete_rows.is_empty() {
        return Err(InterventionalDatasetPreparationErrorV1::NoUsableRows);
    }

    let columns = required_variable_ids
        .iter()
        .map(|variable_id| ObservedNumericColumnV1 {
            variable_id: variable_id.clone(),
            values: complete_rows
                .iter()
                .map(|row| {
                    numeric_observed_cell_v1(
                        dataset.batch.column(positions[variable_id]).as_ref(),
                        *row,
                    )
                    .expect("complete finite causal row")
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let identification_checklist = core_identification_v1(&config.identification);
    let positivity_policy = canonical_positivity_policy_v1(&config.positivity_policy, &bindings)?;
    let treatment_contrast = core_treatment_contrast_v1(&config.treatment_contrast);
    let canonical_treatment = canonical_observed_id_v1(&bindings, &config.treatment)?.to_owned();
    let canonical_outcome = canonical_observed_id_v1(&bindings, &config.outcome)?.to_owned();
    let canonical_mediators = config
        .mediators
        .iter()
        .map(|reference| canonical_observed_id_v1(&bindings, reference).map(str::to_owned))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let canonical_adjustments = config
        .adjustment_covariates
        .iter()
        .map(|reference| canonical_observed_id_v1(&bindings, reference).map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    let canonical_moderators = config
        .baseline_moderators
        .iter()
        .map(|reference| canonical_observed_id_v1(&bindings, reference).map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    config
        .paths
        .iter()
        .map(|path| {
            let ordered_mediator_variable_ids = path.ordered_variable_ids
                [1..path.ordered_variable_ids.len() - 1]
                .iter()
                .map(|reference| canonical_observed_id_v1(&bindings, reference).map(str::to_owned))
                .collect::<Result<Vec<_>, _>>()?;
            if ordered_mediator_variable_ids
                .iter()
                .any(|mediator| !canonical_mediators.contains(mediator))
            {
                return Err(InterventionalDatasetPreparationErrorV1::PathAuthority(
                    path.path_id.clone(),
                ));
            }
            Ok(PreparedInterventionalDatasetPathV1 {
                path_id: path.path_id.clone(),
                source_row_indices: complete_rows.clone(),
                excluded_rows: excluded_rows.clone(),
                input: InterventionalCausalMediationInputV1 {
                    analysis_id: path.path_id.clone(),
                    columns: columns.clone(),
                    treatment_variable_id: canonical_treatment.clone(),
                    ordered_mediator_variable_ids,
                    outcome_variable_id: canonical_outcome.clone(),
                    adjustment_covariate_variable_ids: canonical_adjustments.clone(),
                    baseline_moderator_variable_ids: canonical_moderators.clone(),
                    baseline_moderator_intervention_values: BTreeMap::new(),
                    equations: core_equations_v1(path, &bindings)?,
                    treatment_contrast: treatment_contrast.clone(),
                    identification_checklist: identification_checklist.clone(),
                    positivity_policy: positivity_policy.clone(),
                    unsupported_features: UnsupportedInterventionalFeatureRequestV1::default(),
                },
            })
        })
        .collect()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterventionalMediationBlockerCodeV1 {
    EmptyOrDuplicateIdentity,
    ObservationShape,
    NonFiniteObservedValue,
    UnsupportedRoleOrFeature,
    PathLengthOutsideProfile,
    EquationOrderOrCount,
    UnsupportedEquationTerm,
    MissingStrongHierarchyTerm,
    MissingAdjustmentCovariate,
    MissingSelectedPathRelation,
    IdentificationChecklistIncomplete,
    InvalidTreatmentContrast,
    PositivityFailure,
    RankDeficientEquation,
    NonFiniteFit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InterventionalMediationBlockerV1 {
    pub code: InterventionalMediationBlockerCodeV1,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservedEquationCoefficientV1 {
    pub parameter_id: String,
    pub term_id: String,
    pub factor_variable_ids: Vec<String>,
    pub estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FittedObservedEquationV1 {
    pub equation_id: String,
    pub outcome_variable_id: String,
    pub coefficients: Vec<ObservedEquationCoefficientV1>,
    pub residual_standard_deviation: f64,
    pub rank: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterventionalPositivityReceiptV1 {
    pub screen_version: String,
    pub treatment_kind: ObservedTreatmentKindV1,
    pub observed_treatment_minimum: f64,
    pub observed_treatment_maximum: f64,
    pub x0_support_count: usize,
    pub x1_support_count: usize,
    pub binary_strata_checked: usize,
    pub wording: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterventionalMediationResultV1 {
    pub method_version: String,
    pub target_id: String,
    pub analysis_id: String,
    pub interpretation: String,
    pub observation_count: usize,
    pub treatment_variable_id: String,
    pub ordered_mediator_variable_ids: Vec<String>,
    pub outcome_variable_id: String,
    pub adjustment_covariate_variable_ids: Vec<String>,
    pub baseline_moderator_variable_ids: Vec<String>,
    pub x0: f64,
    pub x1: f64,
    pub baseline_moderator_intervention_values: BTreeMap<String, f64>,
    pub identification_checklist: InterventionalIdentificationChecklistV1,
    pub fitted_equations: Vec<FittedObservedEquationV1>,
    pub positivity: InterventionalPositivityReceiptV1,
    /// E[Y(x0, G_x0)] under the fitted observed-data model.
    pub outcome_mean_x0_g_x0: f64,
    /// E[Y(x1, G_x0)] under the fitted observed-data model.
    pub outcome_mean_x1_g_x0: f64,
    /// E[Y(x1, G_x1)] under the fitted observed-data model.
    pub outcome_mean_x1_g_x1: f64,
    pub interventional_direct_effect: f64,
    pub joint_interventional_indirect_effect: f64,
    pub total_interventional_contrast: f64,
    pub additive_decomposition_residual: f64,
}

fn blocker(
    blockers: &mut Vec<InterventionalMediationBlockerV1>,
    code: InterventionalMediationBlockerCodeV1,
    detail: impl Into<String>,
) {
    blockers.push(InterventionalMediationBlockerV1 {
        code,
        detail: detail.into(),
    });
}

fn is_non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn has_duplicate_or_empty(values: impl IntoIterator<Item = String>) -> bool {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .any(|value| !is_non_empty(&value) || !seen.insert(value))
}

fn canonical_factor_ids(term: &ObservedLinearTermV1) -> Vec<String> {
    let mut factors = term.factor_variable_ids.clone();
    factors.sort();
    factors
}

fn validate_identification_and_roles(
    input: &InterventionalCausalMediationInputV1,
    blockers: &mut Vec<InterventionalMediationBlockerV1>,
) {
    if !is_non_empty(&input.analysis_id)
        || !is_non_empty(&input.treatment_variable_id)
        || !is_non_empty(&input.outcome_variable_id)
        || has_duplicate_or_empty(
            input
                .columns
                .iter()
                .map(|column| column.variable_id.clone()),
        )
        || has_duplicate_or_empty(input.ordered_mediator_variable_ids.clone())
        || has_duplicate_or_empty(input.adjustment_covariate_variable_ids.clone())
        || has_duplicate_or_empty(input.baseline_moderator_variable_ids.clone())
        || has_duplicate_or_empty(
            input
                .positivity_policy
                .positivity_strata_variable_ids
                .clone(),
        )
        || has_duplicate_or_empty(
            input
                .equations
                .iter()
                .map(|equation| equation.equation_id.clone()),
        )
    {
        blocker(
            blockers,
            InterventionalMediationBlockerCodeV1::EmptyOrDuplicateIdentity,
            "analysis, variable, column, mediator, and equation identities must be non-empty and unique",
        );
    }

    let mut roles = BTreeSet::new();
    let roles_unique = roles.insert(input.treatment_variable_id.clone())
        && input
            .ordered_mediator_variable_ids
            .iter()
            .all(|id| roles.insert(id.clone()))
        && roles.insert(input.outcome_variable_id.clone());
    if !roles_unique
        || input
            .adjustment_covariate_variable_ids
            .iter()
            .chain(&input.baseline_moderator_variable_ids)
            .any(|id| roles.contains(id))
        || input
            .adjustment_covariate_variable_ids
            .iter()
            .any(|id| input.baseline_moderator_variable_ids.contains(id))
    {
        blocker(
            blockers,
            InterventionalMediationBlockerCodeV1::UnsupportedRoleOrFeature,
            "treatment, mediators, outcome, and baseline roles must not overlap",
        );
    }
    if input
        .positivity_policy
        .positivity_strata_variable_ids
        .iter()
        .any(|id| {
            !input.adjustment_covariate_variable_ids.contains(id)
                && !input.baseline_moderator_variable_ids.contains(id)
        })
    {
        blocker(
            blockers,
            InterventionalMediationBlockerCodeV1::UnsupportedRoleOrFeature,
            "positivity strata must be declared baseline covariates or baseline moderators",
        );
    }

    let unsupported = &input.unsupported_features;
    if unsupported.latent_composite_or_hoc_roles
        || unsupported.groups
        || unsupported.weights
        || unsupported.natural_or_cross_world_effects
        || unsupported.exposure_induced_mediator_outcome_confounder_present
        || unsupported.recanting_witness_present
    {
        blocker(
            blockers,
            InterventionalMediationBlockerCodeV1::UnsupportedRoleOrFeature,
            "V1 accepts observed roles only and excludes groups, weights, natural/cross-world effects, exposure-induced mediator-outcome confounding, and recanting-witness settings",
        );
    }

    if input.ordered_mediator_variable_ids.is_empty()
        || input.ordered_mediator_variable_ids.len() + 1
            > INTERVENTIONAL_MEDIATION_MAX_PATH_EDGES_V1
    {
        blocker(
            blockers,
            InterventionalMediationBlockerCodeV1::PathLengthOutsideProfile,
            "V1 requires one to three ordered mediators (a two- to four-edge path)",
        );
    }

    let missing_checklist = input.identification_checklist.missing_items();
    if !missing_checklist.is_empty() {
        blocker(
            blockers,
            InterventionalMediationBlockerCodeV1::IdentificationChecklistIncomplete,
            format!(
                "unreviewed identification items: {}",
                missing_checklist.join(", ")
            ),
        );
    }
}

fn validate_observations(
    input: &InterventionalCausalMediationInputV1,
    blockers: &mut Vec<InterventionalMediationBlockerV1>,
) -> (usize, BTreeMap<String, Vec<f64>>) {
    let observation_count = input
        .columns
        .first()
        .map(|column| column.values.len())
        .unwrap_or(0);
    if observation_count < INTERVENTIONAL_MEDIATION_MINIMUM_OBSERVATIONS_V1
        || input
            .columns
            .iter()
            .any(|column| column.values.len() != observation_count)
    {
        blocker(
            blockers,
            InterventionalMediationBlockerCodeV1::ObservationShape,
            "all observed columns must have the same length and contain at least 20 complete cases",
        );
    }
    for column in &input.columns {
        if column.values.iter().any(|value| !value.is_finite()) {
            blocker(
                blockers,
                InterventionalMediationBlockerCodeV1::NonFiniteObservedValue,
                format!("column {} contains a non-finite value", column.variable_id),
            );
        }
    }
    let columns = input
        .columns
        .iter()
        .map(|column| (column.variable_id.clone(), column.values.clone()))
        .collect::<BTreeMap<_, _>>();
    let requested_ids = std::iter::once(&input.treatment_variable_id)
        .chain(input.ordered_mediator_variable_ids.iter())
        .chain(std::iter::once(&input.outcome_variable_id))
        .chain(input.adjustment_covariate_variable_ids.iter())
        .chain(input.baseline_moderator_variable_ids.iter())
        .chain(
            input
                .positivity_policy
                .positivity_strata_variable_ids
                .iter(),
        );
    for id in requested_ids {
        if !columns.contains_key(id) {
            blocker(
                blockers,
                InterventionalMediationBlockerCodeV1::ObservationShape,
                format!("required observed column {id} is unavailable"),
            );
        }
    }
    (observation_count, columns)
}

fn validate_equations(
    input: &InterventionalCausalMediationInputV1,
    available_columns: &BTreeMap<String, Vec<f64>>,
    blockers: &mut Vec<InterventionalMediationBlockerV1>,
) {
    let expected_outcomes = input
        .ordered_mediator_variable_ids
        .iter()
        .chain(std::iter::once(&input.outcome_variable_id))
        .cloned()
        .collect::<Vec<_>>();
    let actual_outcomes = input
        .equations
        .iter()
        .map(|equation| equation.outcome_variable_id.clone())
        .collect::<Vec<_>>();
    if expected_outcomes != actual_outcomes {
        blocker(
            blockers,
            InterventionalMediationBlockerCodeV1::EquationOrderOrCount,
            "equations must appear exactly in ordered-mediator order followed by the outcome equation",
        );
    }

    let baseline_ids = input
        .adjustment_covariate_variable_ids
        .iter()
        .chain(&input.baseline_moderator_variable_ids)
        .cloned()
        .collect::<BTreeSet<_>>();
    let mediator_ids = input
        .ordered_mediator_variable_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    for (equation_index, equation) in input.equations.iter().enumerate() {
        if !is_non_empty(&equation.equation_id)
            || !is_non_empty(&equation.outcome_variable_id)
            || has_duplicate_or_empty(equation.terms.iter().map(|term| term.term_id.clone()))
        {
            blocker(
                blockers,
                InterventionalMediationBlockerCodeV1::EmptyOrDuplicateIdentity,
                format!(
                    "equation {} has an invalid or duplicate identity",
                    equation.equation_id
                ),
            );
        }
        let allowed_prior_mediators = input
            .ordered_mediator_variable_ids
            .iter()
            .take(equation_index.min(input.ordered_mediator_variable_ids.len()))
            .cloned()
            .collect::<BTreeSet<_>>();
        let allowed_factors = baseline_ids
            .iter()
            .cloned()
            .chain(std::iter::once(input.treatment_variable_id.clone()))
            .chain(allowed_prior_mediators.iter().cloned())
            .collect::<BTreeSet<_>>();

        let main_factors = equation
            .terms
            .iter()
            .filter(|term| term.factor_variable_ids.len() == 1)
            .map(|term| term.factor_variable_ids[0].clone())
            .collect::<BTreeSet<_>>();
        for term in &equation.terms {
            let canonical = canonical_factor_ids(term);
            let unique_factor_count = canonical.iter().collect::<BTreeSet<_>>().len();
            let post_treatment_factor_count = canonical
                .iter()
                .filter(|factor| mediator_ids.contains(*factor))
                .count();
            if term.factor_variable_ids.is_empty()
                || term.factor_variable_ids.len() > INTERVENTIONAL_MEDIATION_MAX_TERM_FACTORS_V1
                || unique_factor_count != term.factor_variable_ids.len()
                || term.factor_variable_ids.iter().any(|factor| {
                    !allowed_factors.contains(factor) || !available_columns.contains_key(factor)
                })
                || post_treatment_factor_count > 1
            {
                blocker(
                    blockers,
                    InterventionalMediationBlockerCodeV1::UnsupportedEquationTerm,
                    format!(
                        "term {} in equation {} is nonrecursive, unavailable, repeated, too high-order, or contains multiple post-treatment variables",
                        term.term_id, equation.equation_id
                    ),
                );
            }
            if term.factor_variable_ids.len() > 1
                && term
                    .factor_variable_ids
                    .iter()
                    .any(|factor| !main_factors.contains(factor))
            {
                blocker(
                    blockers,
                    InterventionalMediationBlockerCodeV1::MissingStrongHierarchyTerm,
                    format!(
                        "term {} in equation {} lacks a constituent main effect",
                        term.term_id, equation.equation_id
                    ),
                );
            }
        }

        for adjustment in &input.adjustment_covariate_variable_ids {
            if !main_factors.contains(adjustment) {
                blocker(
                    blockers,
                    InterventionalMediationBlockerCodeV1::MissingAdjustmentCovariate,
                    format!(
                        "adjustment covariate {adjustment} is not a main effect in equation {}",
                        equation.equation_id
                    ),
                );
            }
        }
        let required_predecessor = if equation_index == 0 {
            &input.treatment_variable_id
        } else if equation_index <= input.ordered_mediator_variable_ids.len() {
            &input.ordered_mediator_variable_ids[equation_index - 1]
        } else {
            &input.treatment_variable_id
        };
        if !main_factors.contains(required_predecessor) {
            blocker(
                blockers,
                InterventionalMediationBlockerCodeV1::MissingSelectedPathRelation,
                format!(
                    "equation {} lacks the selected path predecessor {required_predecessor}",
                    equation.equation_id
                ),
            );
        }
    }

    for (moderator_id, value) in &input.baseline_moderator_intervention_values {
        if !input.baseline_moderator_variable_ids.contains(moderator_id) || !value.is_finite() {
            blocker(
                blockers,
                InterventionalMediationBlockerCodeV1::UnsupportedRoleOrFeature,
                format!("moderator intervention {moderator_id} is undeclared or non-finite"),
            );
        }
    }
}

fn approximately_equal(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 1.0e-12 * scale
}

fn assess_positivity(
    input: &InterventionalCausalMediationInputV1,
    columns: &BTreeMap<String, Vec<f64>>,
    blockers: &mut Vec<InterventionalMediationBlockerV1>,
) -> Option<InterventionalPositivityReceiptV1> {
    let contrast = &input.treatment_contrast;
    if !contrast.x0.is_finite()
        || !contrast.x1.is_finite()
        || approximately_equal(contrast.x0, contrast.x1)
    {
        blocker(
            blockers,
            InterventionalMediationBlockerCodeV1::InvalidTreatmentContrast,
            "x0 and x1 must be distinct finite treatment values",
        );
        return None;
    }
    let policy = &input.positivity_policy;
    if policy.minimum_binary_arm_count == 0
        || !policy.maximum_binary_arm_ratio.is_finite()
        || policy.maximum_binary_arm_ratio < 1.0
        || policy.minimum_count_per_binary_stratum_arm == 0
        || !policy.continuous_neighborhood_fraction_of_range.is_finite()
        || !(0.0..=0.5).contains(&policy.continuous_neighborhood_fraction_of_range)
        || policy.minimum_continuous_neighborhood_count == 0
    {
        blocker(
            blockers,
            InterventionalMediationBlockerCodeV1::PositivityFailure,
            "positivity policy parameters are invalid",
        );
        return None;
    }
    let treatment = columns.get(&input.treatment_variable_id)?;
    let minimum = treatment.iter().copied().min_by(f64::total_cmp)?;
    let maximum = treatment.iter().copied().max_by(f64::total_cmp)?;
    let (x0_support_count, x1_support_count, strata_checked) = match contrast.kind {
        ObservedTreatmentKindV1::Binary => {
            let mut x0_count = 0_usize;
            let mut x1_count = 0_usize;
            let mut unexpected = 0_usize;
            for value in treatment {
                if approximately_equal(*value, contrast.x0) {
                    x0_count += 1;
                } else if approximately_equal(*value, contrast.x1) {
                    x1_count += 1;
                } else {
                    unexpected += 1;
                }
            }
            if unexpected > 0
                || x0_count < policy.minimum_binary_arm_count
                || x1_count < policy.minimum_binary_arm_count
                || (x0_count.max(x1_count) as f64 / x0_count.min(x1_count).max(1) as f64)
                    > policy.maximum_binary_arm_ratio
            {
                blocker(
                    blockers,
                    InterventionalMediationBlockerCodeV1::PositivityFailure,
                    format!(
                        "binary support failed: x0={x0_count}, x1={x1_count}, unexpected={unexpected}"
                    ),
                );
            }

            let mut strata = BTreeMap::<Vec<u8>, (usize, usize)>::new();
            if !policy.positivity_strata_variable_ids.is_empty() {
                for row in 0..treatment.len() {
                    let mut key = Vec::new();
                    let mut valid_strata = true;
                    for stratum_id in &policy.positivity_strata_variable_ids {
                        let Some(value) =
                            columns.get(stratum_id).and_then(|values| values.get(row))
                        else {
                            valid_strata = false;
                            break;
                        };
                        if approximately_equal(*value, 0.0) {
                            key.push(0);
                        } else if approximately_equal(*value, 1.0) {
                            key.push(1);
                        } else {
                            valid_strata = false;
                            break;
                        }
                    }
                    if !valid_strata {
                        blocker(
                            blockers,
                            InterventionalMediationBlockerCodeV1::PositivityFailure,
                            "declared positivity strata must be binary observed variables",
                        );
                        break;
                    }
                    let counts = strata.entry(key).or_default();
                    if approximately_equal(treatment[row], contrast.x0) {
                        counts.0 += 1;
                    } else if approximately_equal(treatment[row], contrast.x1) {
                        counts.1 += 1;
                    }
                }
            }
            if strata.values().any(|(x0, x1)| {
                *x0 < policy.minimum_count_per_binary_stratum_arm
                    || *x1 < policy.minimum_count_per_binary_stratum_arm
            }) {
                blocker(
                    blockers,
                    InterventionalMediationBlockerCodeV1::PositivityFailure,
                    "at least one declared baseline stratum lacks both treatment arms",
                );
            }
            (x0_count, x1_count, strata.len())
        }
        ObservedTreatmentKindV1::ContinuousContrast => {
            if contrast.x0 < minimum
                || contrast.x0 > maximum
                || contrast.x1 < minimum
                || contrast.x1 > maximum
                || approximately_equal(minimum, maximum)
            {
                blocker(
                    blockers,
                    InterventionalMediationBlockerCodeV1::PositivityFailure,
                    "continuous treatment contrast lies outside observed support",
                );
            }
            let radius =
                (maximum - minimum).abs() * policy.continuous_neighborhood_fraction_of_range;
            let x0_count = treatment
                .iter()
                .filter(|value| (**value - contrast.x0).abs() <= radius)
                .count();
            let x1_count = treatment
                .iter()
                .filter(|value| (**value - contrast.x1).abs() <= radius)
                .count();
            if x0_count < policy.minimum_continuous_neighborhood_count
                || x1_count < policy.minimum_continuous_neighborhood_count
            {
                blocker(
                    blockers,
                    InterventionalMediationBlockerCodeV1::PositivityFailure,
                    format!(
                        "continuous local support failed: x0 neighborhood={x0_count}, x1 neighborhood={x1_count}"
                    ),
                );
            }
            (x0_count, x1_count, 0)
        }
    };
    Some(InterventionalPositivityReceiptV1 {
        screen_version: "observed_treatment_support_screen_v1".to_owned(),
        treatment_kind: contrast.kind,
        observed_treatment_minimum: minimum,
        observed_treatment_maximum: maximum,
        x0_support_count,
        x1_support_count,
        binary_strata_checked: strata_checked,
        wording: "This observed-support screen is necessary but does not prove causal positivity or exchangeability.".to_owned(),
    })
}

fn evaluate_observed_term(
    term: &ObservedLinearTermV1,
    columns: &BTreeMap<String, Vec<f64>>,
    row: usize,
) -> Option<f64> {
    term.factor_variable_ids
        .iter()
        .try_fold(1.0, |product, id| {
            columns
                .get(id)
                .and_then(|values| values.get(row))
                .map(|value| product * value)
        })
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn fit_equation(
    equation: &ObservedLinearEquationV1,
    columns: &BTreeMap<String, Vec<f64>>,
    observation_count: usize,
) -> Result<FittedObservedEquationV1, InterventionalMediationBlockerV1> {
    let outcome = columns.get(&equation.outcome_variable_id).ok_or_else(|| {
        InterventionalMediationBlockerV1 {
            code: InterventionalMediationBlockerCodeV1::ObservationShape,
            detail: format!("outcome {} is unavailable", equation.outcome_variable_id),
        }
    })?;
    let mut design_columns = Vec::with_capacity(equation.terms.len() + 1);
    design_columns.push(vec![1.0; observation_count]);
    for term in &equation.terms {
        let mut values = Vec::with_capacity(observation_count);
        for row in 0..observation_count {
            values.push(evaluate_observed_term(term, columns, row).ok_or_else(|| {
                InterventionalMediationBlockerV1 {
                    code: InterventionalMediationBlockerCodeV1::ObservationShape,
                    detail: format!("term {} references unavailable data", term.term_id),
                }
            })?);
        }
        design_columns.push(values);
    }

    let parameter_count = design_columns.len();
    if observation_count <= parameter_count {
        return Err(InterventionalMediationBlockerV1 {
            code: InterventionalMediationBlockerCodeV1::RankDeficientEquation,
            detail: format!(
                "equation {} has {parameter_count} parameters but only {observation_count} observations",
                equation.equation_id
            ),
        });
    }
    let mut q_columns = Vec::<Vec<f64>>::with_capacity(parameter_count);
    let mut upper = vec![vec![0.0; parameter_count]; parameter_count];
    for column_index in 0..parameter_count {
        let original_norm =
            dot(&design_columns[column_index], &design_columns[column_index]).sqrt();
        let mut residual = design_columns[column_index].clone();
        for prior in 0..column_index {
            upper[prior][column_index] = dot(&q_columns[prior], &residual);
            for row in 0..observation_count {
                residual[row] -= upper[prior][column_index] * q_columns[prior][row];
            }
        }
        let residual_norm = dot(&residual, &residual).sqrt();
        if !residual_norm.is_finite()
            || residual_norm <= OLS_RELATIVE_RANK_TOLERANCE_V1 * original_norm.max(1.0)
        {
            return Err(InterventionalMediationBlockerV1 {
                code: InterventionalMediationBlockerCodeV1::RankDeficientEquation,
                detail: format!(
                    "equation {} is rank deficient at parameter index {column_index}",
                    equation.equation_id
                ),
            });
        }
        upper[column_index][column_index] = residual_norm;
        for value in &mut residual {
            *value /= residual_norm;
        }
        q_columns.push(residual);
    }
    let projected = q_columns
        .iter()
        .map(|column| dot(column, outcome))
        .collect::<Vec<_>>();
    let mut coefficients = vec![0.0; parameter_count];
    for row in (0..parameter_count).rev() {
        let remainder = ((row + 1)..parameter_count)
            .map(|column| upper[row][column] * coefficients[column])
            .sum::<f64>();
        coefficients[row] = (projected[row] - remainder) / upper[row][row];
    }
    if coefficients
        .iter()
        .any(|coefficient| !coefficient.is_finite())
    {
        return Err(InterventionalMediationBlockerV1 {
            code: InterventionalMediationBlockerCodeV1::NonFiniteFit,
            detail: format!(
                "equation {} produced a non-finite coefficient",
                equation.equation_id
            ),
        });
    }
    let residual_sum_of_squares = (0..observation_count)
        .map(|row| {
            let fitted = coefficients
                .iter()
                .zip(&design_columns)
                .map(|(coefficient, column)| coefficient * column[row])
                .sum::<f64>();
            (outcome[row] - fitted).powi(2)
        })
        .sum::<f64>();
    let degrees_of_freedom = observation_count - parameter_count;
    let residual_standard_deviation = (residual_sum_of_squares / degrees_of_freedom as f64).sqrt();
    if !residual_standard_deviation.is_finite() {
        return Err(InterventionalMediationBlockerV1 {
            code: InterventionalMediationBlockerCodeV1::NonFiniteFit,
            detail: format!(
                "equation {} produced a non-finite residual scale",
                equation.equation_id
            ),
        });
    }
    let mut result_coefficients = Vec::with_capacity(parameter_count);
    result_coefficients.push(ObservedEquationCoefficientV1 {
        parameter_id: format!("{}::intercept", equation.equation_id),
        term_id: "intercept".to_owned(),
        factor_variable_ids: Vec::new(),
        estimate: coefficients[0],
    });
    for (index, term) in equation.terms.iter().enumerate() {
        result_coefficients.push(ObservedEquationCoefficientV1 {
            parameter_id: format!("{}::{}", equation.equation_id, term.term_id),
            term_id: term.term_id.clone(),
            factor_variable_ids: term.factor_variable_ids.clone(),
            estimate: coefficients[index + 1],
        });
    }
    Ok(FittedObservedEquationV1 {
        equation_id: equation.equation_id.clone(),
        outcome_variable_id: equation.outcome_variable_id.clone(),
        coefficients: result_coefficients,
        residual_standard_deviation,
        rank: parameter_count,
    })
}

fn predict_from_context(
    equation: &FittedObservedEquationV1,
    context: &BTreeMap<String, f64>,
) -> Option<f64> {
    equation
        .coefficients
        .iter()
        .try_fold(0.0, |sum, coefficient| {
            let term_value = coefficient
                .factor_variable_ids
                .iter()
                .try_fold(1.0, |product, factor| {
                    context.get(factor).map(|value| product * value)
                })?;
            Some(sum + coefficient.estimate * term_value)
        })
}

fn row_baseline_context(
    input: &InterventionalCausalMediationInputV1,
    columns: &BTreeMap<String, Vec<f64>>,
    row: usize,
) -> Option<BTreeMap<String, f64>> {
    let mut context = BTreeMap::new();
    for id in input
        .adjustment_covariate_variable_ids
        .iter()
        .chain(&input.baseline_moderator_variable_ids)
    {
        let value = input
            .baseline_moderator_intervention_values
            .get(id)
            .copied()
            .or_else(|| columns.get(id).and_then(|values| values.get(row)).copied())?;
        context.insert(id.clone(), value);
    }
    Some(context)
}

fn g_computation_mean(
    input: &InterventionalCausalMediationInputV1,
    columns: &BTreeMap<String, Vec<f64>>,
    fitted_equations: &[FittedObservedEquationV1],
    outcome_treatment_value: f64,
    mediator_distribution_treatment_value: f64,
    observation_count: usize,
) -> Option<f64> {
    let mediator_equations = &fitted_equations[..input.ordered_mediator_variable_ids.len()];
    let outcome_equation = fitted_equations.last()?;
    let mut total = 0.0;
    for row in 0..observation_count {
        let mut mediator_context = row_baseline_context(input, columns, row)?;
        mediator_context.insert(
            input.treatment_variable_id.clone(),
            mediator_distribution_treatment_value,
        );
        for equation in mediator_equations {
            let conditional_interventional_mean =
                predict_from_context(equation, &mediator_context)?;
            if !conditional_interventional_mean.is_finite() {
                return None;
            }
            mediator_context.insert(
                equation.outcome_variable_id.clone(),
                conditional_interventional_mean,
            );
        }
        mediator_context.insert(input.treatment_variable_id.clone(), outcome_treatment_value);
        let outcome_mean = predict_from_context(outcome_equation, &mediator_context)?;
        if !outcome_mean.is_finite() {
            return None;
        }
        total += outcome_mean;
    }
    Some(total / observation_count as f64)
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct InterventionalTargetMaterialV1<'a> {
    method_version: &'static str,
    analysis_id: &'a str,
    treatment_variable_id: &'a str,
    mediator_ids: &'a [String],
    outcome_variable_id: &'a str,
    contrast: &'a ObservedTreatmentContrastV1,
    moderator_interventions: &'a BTreeMap<String, f64>,
    equations: &'a [ObservedLinearEquationV1],
    observed_data_sha256: String,
}

fn canonical_equations(equations: &[ObservedLinearEquationV1]) -> Vec<ObservedLinearEquationV1> {
    let mut canonical = equations.to_vec();
    for equation in &mut canonical {
        for term in &mut equation.terms {
            term.factor_variable_ids.sort();
        }
        equation.terms.sort_by(|left, right| {
            left.term_id
                .cmp(&right.term_id)
                .then_with(|| left.factor_variable_ids.cmp(&right.factor_variable_ids))
        });
    }
    canonical
}

fn observed_data_digest(
    columns: &BTreeMap<String, Vec<f64>>,
) -> Result<String, InterventionalMediationBlockerV1> {
    let bytes = serde_json::to_vec(columns).map_err(|error| InterventionalMediationBlockerV1 {
        code: InterventionalMediationBlockerCodeV1::NonFiniteFit,
        detail: format!("observed data digest serialization failed: {error}"),
    })?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn target_id(
    input: &InterventionalCausalMediationInputV1,
    columns: &BTreeMap<String, Vec<f64>>,
    canonical_equations: &[ObservedLinearEquationV1],
) -> Result<String, InterventionalMediationBlockerV1> {
    let material = InterventionalTargetMaterialV1 {
        method_version: INTERVENTIONAL_CAUSAL_MEDIATION_METHOD_VERSION_V1,
        analysis_id: &input.analysis_id,
        treatment_variable_id: &input.treatment_variable_id,
        mediator_ids: &input.ordered_mediator_variable_ids,
        outcome_variable_id: &input.outcome_variable_id,
        contrast: &input.treatment_contrast,
        moderator_interventions: &input.baseline_moderator_intervention_values,
        equations: canonical_equations,
        observed_data_sha256: observed_data_digest(columns)?,
    };
    let bytes =
        serde_json::to_vec(&material).map_err(|error| InterventionalMediationBlockerV1 {
            code: InterventionalMediationBlockerCodeV1::NonFiniteFit,
            detail: format!("target identity serialization failed: {error}"),
        })?;
    Ok(format!("qpls.icm.v1.{:x}", Sha256::digest(bytes)))
}

/// Fits the declared equations and computes an assumption-dependent joint
/// interventional indirect effect.  All blockers are returned before any
/// result may be published.
pub fn estimate_interventional_mediation_v1(
    input: &InterventionalCausalMediationInputV1,
) -> Result<InterventionalMediationResultV1, Vec<InterventionalMediationBlockerV1>> {
    let mut blockers = Vec::new();
    validate_identification_and_roles(input, &mut blockers);
    let (observation_count, columns) = validate_observations(input, &mut blockers);
    validate_equations(input, &columns, &mut blockers);
    let positivity = assess_positivity(input, &columns, &mut blockers);
    if !blockers.is_empty() {
        return Err(blockers);
    }
    let positivity = positivity.expect("positivity exists when validation has no blockers");

    let compiled_equations = canonical_equations(&input.equations);
    let mut fitted_equations = Vec::with_capacity(compiled_equations.len());
    for equation in &compiled_equations {
        match fit_equation(equation, &columns, observation_count) {
            Ok(fitted) => fitted_equations.push(fitted),
            Err(fit_blocker) => blockers.push(fit_blocker),
        }
    }
    if !blockers.is_empty() {
        return Err(blockers);
    }

    let x0 = input.treatment_contrast.x0;
    let x1 = input.treatment_contrast.x1;
    let outcome_mean_x0_g_x0 = g_computation_mean(
        input,
        &columns,
        &fitted_equations,
        x0,
        x0,
        observation_count,
    );
    let outcome_mean_x1_g_x0 = g_computation_mean(
        input,
        &columns,
        &fitted_equations,
        x1,
        x0,
        observation_count,
    );
    let outcome_mean_x1_g_x1 = g_computation_mean(
        input,
        &columns,
        &fitted_equations,
        x1,
        x1,
        observation_count,
    );
    let (Some(y00), Some(y10), Some(y11)) = (
        outcome_mean_x0_g_x0,
        outcome_mean_x1_g_x0,
        outcome_mean_x1_g_x1,
    ) else {
        return Err(vec![InterventionalMediationBlockerV1 {
            code: InterventionalMediationBlockerCodeV1::NonFiniteFit,
            detail: "g-computation produced an unavailable or non-finite mean".to_owned(),
        }]);
    };
    let direct = y10 - y00;
    let indirect = y11 - y10;
    let total = y11 - y00;
    let decomposition_residual = total - direct - indirect;
    let result_target_id = match target_id(input, &columns, &compiled_equations) {
        Ok(id) => id,
        Err(target_blocker) => return Err(vec![target_blocker]),
    };

    Ok(InterventionalMediationResultV1 {
        method_version: INTERVENTIONAL_CAUSAL_MEDIATION_METHOD_VERSION_V1.to_owned(),
        target_id: result_target_id,
        analysis_id: input.analysis_id.clone(),
        interpretation: INTERVENTIONAL_MEDIATION_INTERPRETATION_V1.to_owned(),
        observation_count,
        treatment_variable_id: input.treatment_variable_id.clone(),
        ordered_mediator_variable_ids: input.ordered_mediator_variable_ids.clone(),
        outcome_variable_id: input.outcome_variable_id.clone(),
        adjustment_covariate_variable_ids: input.adjustment_covariate_variable_ids.clone(),
        baseline_moderator_variable_ids: input.baseline_moderator_variable_ids.clone(),
        x0,
        x1,
        baseline_moderator_intervention_values: input
            .baseline_moderator_intervention_values
            .clone(),
        identification_checklist: input.identification_checklist.clone(),
        fitted_equations,
        positivity,
        outcome_mean_x0_g_x0: y00,
        outcome_mean_x1_g_x0: y10,
        outcome_mean_x1_g_x1: y11,
        interventional_direct_effect: direct,
        joint_interventional_indirect_effect: indirect,
        total_interventional_contrast: total,
        additive_decomposition_residual: decomposition_residual,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use qpls_core::{
        CausalIdentificationChecklistV1, CausalLinearEquationV1, CausalLinearTermV1,
        CausalPositivityPolicyV1, MissingDataPolicyV4, ObservedCausalPathV1, ObservedRoleV4,
        ObservedScaleV4, SemGroupV4, SemPresentationV4, SemVariableV4,
    };

    fn main_term(id: &str) -> ObservedLinearTermV1 {
        ObservedLinearTermV1 {
            term_id: id.to_owned(),
            factor_variable_ids: vec![id.to_owned()],
        }
    }

    fn reviewed_checklist() -> InterventionalIdentificationChecklistV1 {
        InterventionalIdentificationChecklistV1 {
            temporal_order_reviewed: true,
            consistency_reviewed: true,
            treatment_outcome_exchangeability_reviewed: true,
            treatment_mediator_exchangeability_reviewed: true,
            mediator_outcome_exchangeability_reviewed: true,
            no_exposure_induced_mediator_outcome_confounder_reviewed: true,
            no_recanting_witness_reviewed: true,
            linear_model_specification_reviewed: true,
        }
    }

    fn linear_fixture() -> InterventionalCausalMediationInputV1 {
        let mut x = Vec::new();
        let mut c = Vec::new();
        let mut m = Vec::new();
        let mut y = Vec::new();
        for row in 0..40 {
            let treatment = (row % 2) as f64;
            let covariate = ((row * 7) % 13) as f64 / 6.0 - 1.0;
            // Rows separated by 26 have identical treatment and covariate
            // values, so these paired disturbances are orthogonal to the
            // mediator design while keeping it non-degenerate in y_model.
            let mediator_disturbance = if row < 14 {
                0.125
            } else if row >= 26 {
                -0.125
            } else {
                0.0
            };
            let mediator = 1.0 + 2.0 * treatment + 0.5 * covariate + mediator_disturbance;
            let outcome = 3.0 + treatment + 4.0 * mediator + 0.25 * covariate;
            x.push(treatment);
            c.push(covariate);
            m.push(mediator);
            y.push(outcome);
        }
        InterventionalCausalMediationInputV1 {
            analysis_id: "known_linear_dgp".to_owned(),
            columns: vec![
                ObservedNumericColumnV1 {
                    variable_id: "x".to_owned(),
                    values: x,
                },
                ObservedNumericColumnV1 {
                    variable_id: "c".to_owned(),
                    values: c,
                },
                ObservedNumericColumnV1 {
                    variable_id: "m".to_owned(),
                    values: m,
                },
                ObservedNumericColumnV1 {
                    variable_id: "y".to_owned(),
                    values: y,
                },
            ],
            treatment_variable_id: "x".to_owned(),
            ordered_mediator_variable_ids: vec!["m".to_owned()],
            outcome_variable_id: "y".to_owned(),
            adjustment_covariate_variable_ids: vec!["c".to_owned()],
            baseline_moderator_variable_ids: Vec::new(),
            baseline_moderator_intervention_values: BTreeMap::new(),
            equations: vec![
                ObservedLinearEquationV1 {
                    equation_id: "m_model".to_owned(),
                    outcome_variable_id: "m".to_owned(),
                    terms: vec![main_term("x"), main_term("c")],
                },
                ObservedLinearEquationV1 {
                    equation_id: "y_model".to_owned(),
                    outcome_variable_id: "y".to_owned(),
                    terms: vec![main_term("x"), main_term("m"), main_term("c")],
                },
            ],
            treatment_contrast: ObservedTreatmentContrastV1 {
                kind: ObservedTreatmentKindV1::Binary,
                x0: 0.0,
                x1: 1.0,
            },
            identification_checklist: reviewed_checklist(),
            positivity_policy: InterventionalPositivityPolicyV1::default(),
            unsupported_features: UnsupportedInterventionalFeatureRequestV1::default(),
        }
    }

    #[test]
    fn observed_ols_g_computation_recovers_known_linear_interventional_effects() {
        let result = estimate_interventional_mediation_v1(&linear_fixture()).unwrap();
        assert_abs_diff_eq!(result.interventional_direct_effect, 1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(
            result.joint_interventional_indirect_effect,
            8.0,
            epsilon = 1e-10
        );
        assert_abs_diff_eq!(result.total_interventional_contrast, 9.0, epsilon = 1e-10);
        assert_abs_diff_eq!(result.additive_decomposition_residual, 0.0, epsilon = 1e-12);
        assert!(result.interpretation.contains("assumption-dependent"));
        assert!(
            result
                .interpretation
                .contains("causality is not established")
        );
        assert_eq!(result.positivity.x0_support_count, 20);
        assert_eq!(result.positivity.x1_support_count, 20);
    }

    #[test]
    fn target_identity_is_deterministic_and_data_bound() {
        let first = estimate_interventional_mediation_v1(&linear_fixture()).unwrap();
        let same = estimate_interventional_mediation_v1(&linear_fixture()).unwrap();
        assert_eq!(first.target_id, same.target_id);

        let mut reordered_input = linear_fixture();
        reordered_input.equations[1].terms.reverse();
        let reordered = estimate_interventional_mediation_v1(&reordered_input).unwrap();
        assert_eq!(first.target_id, reordered.target_id);
        assert_abs_diff_eq!(
            first.joint_interventional_indirect_effect,
            reordered.joint_interventional_indirect_effect,
            epsilon = 1e-10
        );

        let mut changed_input = linear_fixture();
        changed_input
            .columns
            .iter_mut()
            .find(|column| column.variable_id == "y")
            .unwrap()
            .values[0] += 0.01;
        let changed = estimate_interventional_mediation_v1(&changed_input).unwrap();
        assert_ne!(first.target_id, changed.target_id);
    }

    #[test]
    fn exactly_collinear_outcome_equation_fails_closed() {
        let mut input = linear_fixture();
        let treatment = input
            .columns
            .iter()
            .find(|column| column.variable_id == "x")
            .unwrap()
            .values
            .clone();
        let covariate = input
            .columns
            .iter()
            .find(|column| column.variable_id == "c")
            .unwrap()
            .values
            .clone();
        let mediator = &mut input
            .columns
            .iter_mut()
            .find(|column| column.variable_id == "m")
            .unwrap()
            .values;
        for row in 0..mediator.len() {
            mediator[row] = 1.0 + 2.0 * treatment[row] + 0.5 * covariate[row];
        }

        let blockers = estimate_interventional_mediation_v1(&input).unwrap_err();
        assert!(blockers.iter().any(|blocker| {
            blocker.code == InterventionalMediationBlockerCodeV1::RankDeficientEquation
                && blocker.detail.contains("y_model")
        }));
    }

    #[test]
    fn incomplete_assumptions_and_unsupported_natural_effects_fail_closed() {
        let mut input = linear_fixture();
        input.identification_checklist.consistency_reviewed = false;
        input.unsupported_features.natural_or_cross_world_effects = true;
        let blockers = estimate_interventional_mediation_v1(&input).unwrap_err();
        assert!(blockers.iter().any(|value| {
            value.code == InterventionalMediationBlockerCodeV1::IdentificationChecklistIncomplete
        }));
        assert!(blockers.iter().any(|value| {
            value.code == InterventionalMediationBlockerCodeV1::UnsupportedRoleOrFeature
        }));
    }

    #[test]
    fn binary_positivity_failure_is_a_blocker_not_a_warning() {
        let mut input = linear_fixture();
        let treatment = input
            .columns
            .iter_mut()
            .find(|column| column.variable_id == "x")
            .unwrap();
        for value in treatment.values.iter_mut().skip(5) {
            *value = 1.0;
        }
        let blockers = estimate_interventional_mediation_v1(&input).unwrap_err();
        assert!(blockers.iter().any(|value| {
            value.code == InterventionalMediationBlockerCodeV1::PositivityFailure
        }));
    }

    #[test]
    fn nonrecursive_mediator_term_is_rejected() {
        let mut input = linear_fixture();
        input.equations[0].terms.push(main_term("y"));
        let blockers = estimate_interventional_mediation_v1(&input).unwrap_err();
        assert!(blockers.iter().any(|value| {
            value.code == InterventionalMediationBlockerCodeV1::UnsupportedEquationTerm
        }));
    }

    fn observed_model_variable(
        id: &str,
        source_column: &str,
        scale: ObservedScaleV4,
    ) -> SemVariableV4 {
        SemVariableV4::Observed {
            id: id.into(),
            label: id.into(),
            source_column: source_column.into(),
            scale,
            role: ObservedRoleV4::Structural,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        }
    }

    fn core_term(term_id: &str, factors: &[&str]) -> CausalLinearTermV1 {
        CausalLinearTermV1 {
            term_id: term_id.into(),
            factor_variable_ids: factors.iter().map(|factor| (*factor).into()).collect(),
        }
    }

    #[test]
    fn raw_preparation_resolves_source_aliases_to_canonical_observed_ids_everywhere() {
        let dataset = qpls_data::import_delimited_bytes(
            b"x_col,m_col,y_col,z_col,c_col\n0,1,2,0,3\n1,2,4,1,5\n",
            "causal-aliases.csv",
            b',',
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let model = SemModelV4 {
            schema_version: qpls_core::SEM_MODEL_V4_SCHEMA_VERSION,
            id: "causal-alias-model".into(),
            name: "causal aliases".into(),
            variables: vec![
                observed_model_variable("x_id", "x_col", ObservedScaleV4::Binary),
                observed_model_variable("m_id", "m_col", ObservedScaleV4::Continuous),
                observed_model_variable("y_id", "y_col", ObservedScaleV4::Continuous),
                observed_model_variable("z_id", "z_col", ObservedScaleV4::Binary),
                observed_model_variable("c_id", "c_col", ObservedScaleV4::Continuous),
            ],
            relations: Vec::new(),
            parameters: Vec::new(),
            constraints: Vec::new(),
            derived_terms: Vec::new(),
            group: SemGroupV4::SingleGroup,
            data_binding: SemDataBindingV4::Raw {
                dataset_id: dataset.id.to_string(),
                missing_data: MissingDataPolicyV4::ListwiseDeletion,
                weight: None,
                cluster_variable: None,
                strata_variable: None,
            },
            annotations: Vec::new(),
            presentation: SemPresentationV4::default(),
        };
        let config = CoreInterventionalCausalMediationConfigV1 {
            schema_version: 1,
            treatment: "x_col".into(),
            treatment_contrast: CoreObservedTreatmentContrastV1::Binary {
                control: 0.0,
                treated: 1.0,
            },
            outcome: "y_col".into(),
            mediators: vec!["m_col".into()],
            baseline_moderators: vec!["z_col".into()],
            adjustment_covariates: vec!["c_col".into()],
            paths: vec![ObservedCausalPathV1 {
                path_id: "alias-path".into(),
                ordered_variable_ids: vec!["x_col".into(), "m_col".into(), "y_col".into()],
                equations: vec![
                    CausalLinearEquationV1 {
                        equation_id: "mediator".into(),
                        outcome_variable_id: "m_col".into(),
                        terms: vec![
                            core_term("x", &["x_col"]),
                            core_term("z", &["z_col"]),
                            core_term("c", &["c_col"]),
                        ],
                    },
                    CausalLinearEquationV1 {
                        equation_id: "outcome".into(),
                        outcome_variable_id: "y_col".into(),
                        terms: vec![
                            core_term("x", &["x_col"]),
                            core_term("m", &["m_col"]),
                            core_term("z", &["z_col"]),
                            core_term("c", &["c_col"]),
                            core_term("x_by_z", &["x_col", "z_col"]),
                        ],
                    },
                ],
            }],
            positivity_policy: CausalPositivityPolicyV1 {
                positivity_strata_variable_ids: vec!["z_col".into()],
                ..CausalPositivityPolicyV1::default()
            },
            identification: CausalIdentificationChecklistV1 {
                temporal_order_declared: true,
                adjustment_set_justified: true,
                consistency_assumption_acknowledged: true,
                no_unmeasured_treatment_outcome_confounding_acknowledged: true,
                no_unmeasured_treatment_mediator_confounding_acknowledged: true,
                no_unmeasured_mediator_outcome_confounding_acknowledged: true,
                no_exposure_induced_mediator_outcome_confounder_confirmed: true,
                no_recanting_witness_confirmed: true,
                linear_model_specification_reviewed: true,
                positivity_reviewed: true,
            },
            bootstrap_resamples: 500,
            seed: 42,
            confidence_level: 0.95,
        };

        let prepared =
            prepare_interventional_causal_inputs_from_dataset_v1(&dataset, &model, &config, None)
                .unwrap();
        let input = &prepared[0].input;
        assert_eq!(input.treatment_variable_id, "x_id");
        assert_eq!(input.ordered_mediator_variable_ids, vec!["m_id"]);
        assert_eq!(input.outcome_variable_id, "y_id");
        assert_eq!(input.adjustment_covariate_variable_ids, vec!["c_id"]);
        assert_eq!(input.baseline_moderator_variable_ids, vec!["z_id"]);
        assert_eq!(
            input.positivity_policy.positivity_strata_variable_ids,
            vec!["z_id"]
        );
        assert_eq!(input.equations[0].outcome_variable_id, "m_id");
        assert!(input.equations[1].terms.iter().any(|term| {
            term.term_id == "x_by_z"
                && term.factor_variable_ids == vec!["x_id".to_owned(), "z_id".to_owned()]
        }));
        assert_eq!(
            input
                .columns
                .iter()
                .map(|column| column.variable_id.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["c_id", "m_id", "x_id", "y_id", "z_id"])
        );

        let mut overlapped = config;
        overlapped.baseline_moderators = vec!["c_id".into()];
        let overlap_error = prepare_interventional_causal_inputs_from_dataset_v1(
            &dataset,
            &model,
            &overlapped,
            None,
        )
        .unwrap_err();
        assert!(matches!(
            overlap_error,
            InterventionalDatasetPreparationErrorV1::InvalidConfiguration(message)
                if message.contains("interventional_causal_mediation_v1.role_overlap")
        ));
    }
}
