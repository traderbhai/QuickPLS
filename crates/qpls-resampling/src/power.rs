use std::{collections::BTreeMap, fmt::Write as _};

use qpls_core::{
    AnalysisMethod, AnalysisRecipe, AnalysisSettings, ModelSpec, Preprocessing,
    ValidatedExecutionRecipe, WeightingScheme,
};
use qpls_data::{ImportOptions, import_delimited_bytes};
use qpls_estimation::{PLS_METHOD_VERSION, estimate_pls_validated_with_control};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use statrs::distribution::{ContinuousCDF, Normal};
use thiserror::Error;

use crate::{RESAMPLING_METHOD_VERSION, bootstrap_pls_normal_reference_validated};

pub const PLS_SAMPLE_SIZE_POWER_METHOD_VERSION: &str = "pls_sample_size_power_v1";
pub const PLS_SAMPLE_SIZE_POWER_METHOD_VERSION_V2: &str = "pls_sample_size_power_v2";
pub const PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID: &str = "qpls3.pls.sample_size_power";
pub const PLS_SAMPLE_SIZE_POWER_RECIPE_SCHEMA_VERSION: u32 = 1;
pub const PLS_SAMPLE_SIZE_POWER_RECIPE_SCHEMA_VERSION_V2: u32 = 2;
pub const PLS_SAMPLE_SIZE_POWER_RESULT_SCHEMA_VERSION: u32 = 1;
pub const PLS_SAMPLE_SIZE_POWER_RESULT_SCHEMA_VERSION_V2: u32 = 2;
pub const PLS_SAMPLE_SIZE_POWER_STREAM_DOMAIN: &str =
    "quickpls/pls_sample_size_power_v1/monte_carlo";
pub const PLS_SAMPLE_SIZE_POWER_STREAM_DOMAIN_V2: &str =
    "quickpls/pls_sample_size_power_v2/monte_carlo";
pub const PLS_SAMPLE_SIZE_POWER_FAILURE_POLICY: &str =
    "failed_replicates_count_as_non_rejections_v1";
pub const PLS_SAMPLE_SIZE_POWER_INTERVAL_METHOD: &str = "wilson_score_two_sided_v1";
pub const PLS_SAMPLE_SIZE_POWER_INFERENCE_METHOD: &str =
    "pls_pm_case_bootstrap_normal_reference_two_sided_v1";
pub const PLS_SAMPLE_SIZE_POWER_INFERENCE_METHOD_V2: &str =
    "pls_pm_case_bootstrap_null_centered_two_sided_plus_one_v2";

const MIN_SAMPLE_SIZE: u32 = 30;
const MAX_SAMPLE_SIZE: u32 = 5_000;
const MIN_MONTE_CARLO_REPLICATES: u32 = 100;
const MAX_MONTE_CARLO_REPLICATES: u32 = 10_000;
const MIN_BOOTSTRAP_REPLICATES: u32 = 99;
const MAX_BOOTSTRAP_REPLICATES: u32 = 1_999;
const MAX_GRID_POINTS: usize = 16;
const MAX_WORKERS: usize = 64;
const MAX_ESTIMATED_PLS_FITS: u64 = 250_000;
const MAX_ESTIMATED_PLS_CASE_FITS: u64 = 100_000_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlsPowerDistributionV1 {
    StandardNormal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlsPowerMissingDataV1 {
    None,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlsPowerInferenceV1 {
    CaseBootstrapNormalReferenceTwoSided,
    CaseBootstrapNullCenteredTwoSidedPlusOne,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReflectiveGaussianPathDesignV1 {
    pub predictor_construct: String,
    pub outcome_construct: String,
    pub predictor_indicator_loadings: Vec<f64>,
    pub outcome_indicator_loadings: Vec<f64>,
    pub population_path: f64,
    pub exogenous_distribution: PlsPowerDistributionV1,
    pub structural_disturbance_distribution: PlsPowerDistributionV1,
    pub indicator_error_distribution: PlsPowerDistributionV1,
    pub missing_data: PlsPowerMissingDataV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsPowerEstimatorSettingsV1 {
    pub weighting_scheme: WeightingScheme,
    pub preprocessing: Preprocessing,
    pub tolerance: f64,
    pub max_iterations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsSampleSizePowerRecipeV1 {
    pub schema_version: u32,
    pub capability_id: String,
    pub method_version: String,
    pub scenario_identity: String,
    pub design: ReflectiveGaussianPathDesignV1,
    pub estimator: PlsPowerEstimatorSettingsV1,
    pub inference: PlsPowerInferenceV1,
    pub sample_size_grid: Vec<u32>,
    pub alpha: f64,
    pub target_power: f64,
    pub confidence_level: f64,
    pub monte_carlo_replicates: u32,
    pub bootstrap_replicates: u32,
    pub master_seed: u64,
    pub workers: usize,
}

/// V2 intentionally keeps the bounded design fields stable while changing the
/// inferential identity.  The alias avoids duplicating the scientific design
/// model; schema/method/inference validation below still distinguishes every
/// v1 and v2 payload exactly.
pub type PlsSampleSizePowerRecipeV2 = PlsSampleSizePowerRecipeV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlsPowerContractIdentity {
    result_schema_version: u32,
    method_version: &'static str,
    stream_domain: &'static str,
    inference_method: &'static str,
    archive_domain: &'static str,
    version_label: &'static str,
}

const PLS_POWER_V1_IDENTITY: PlsPowerContractIdentity = PlsPowerContractIdentity {
    result_schema_version: PLS_SAMPLE_SIZE_POWER_RESULT_SCHEMA_VERSION,
    method_version: PLS_SAMPLE_SIZE_POWER_METHOD_VERSION,
    stream_domain: PLS_SAMPLE_SIZE_POWER_STREAM_DOMAIN,
    inference_method: PLS_SAMPLE_SIZE_POWER_INFERENCE_METHOD,
    archive_domain: "pls_sample_size_power_archive_v1",
    version_label: "v1",
};

const PLS_POWER_V2_IDENTITY: PlsPowerContractIdentity = PlsPowerContractIdentity {
    result_schema_version: PLS_SAMPLE_SIZE_POWER_RESULT_SCHEMA_VERSION_V2,
    method_version: PLS_SAMPLE_SIZE_POWER_METHOD_VERSION_V2,
    stream_domain: PLS_SAMPLE_SIZE_POWER_STREAM_DOMAIN_V2,
    inference_method: PLS_SAMPLE_SIZE_POWER_INFERENCE_METHOD_V2,
    archive_domain: "pls_sample_size_power_archive_v2",
    version_label: "v2",
};

fn contract_identity(
    recipe: &PlsSampleSizePowerRecipeV1,
) -> Result<PlsPowerContractIdentity, PlsSampleSizePowerError> {
    match (
        recipe.schema_version,
        recipe.method_version.as_str(),
        recipe.inference,
    ) {
        (
            PLS_SAMPLE_SIZE_POWER_RECIPE_SCHEMA_VERSION,
            PLS_SAMPLE_SIZE_POWER_METHOD_VERSION,
            PlsPowerInferenceV1::CaseBootstrapNormalReferenceTwoSided,
        ) => Ok(PLS_POWER_V1_IDENTITY),
        (
            PLS_SAMPLE_SIZE_POWER_RECIPE_SCHEMA_VERSION_V2,
            PLS_SAMPLE_SIZE_POWER_METHOD_VERSION_V2,
            PlsPowerInferenceV1::CaseBootstrapNullCenteredTwoSidedPlusOne,
        ) => Ok(PLS_POWER_V2_IDENTITY),
        _ => Err(invalid(
            "method_version",
            "schema_version, method_version, and inference must identify the exact historical v1 or null-calibrated v2 contract",
        )),
    }
}

impl PlsSampleSizePowerRecipeV1 {
    pub fn validate(&self) -> Result<PlsPowerWorkload, PlsSampleSizePowerError> {
        let identity = contract_identity(self)?;
        if self.capability_id != PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID {
            return Err(invalid(
                "capability_id",
                "does not identify the frozen sample-size/power capability",
            ));
        }
        validate_identity("scenario_identity", &self.scenario_identity)?;
        validate_identity(
            "design.predictor_construct",
            &self.design.predictor_construct,
        )?;
        validate_identity("design.outcome_construct", &self.design.outcome_construct)?;
        if self.design.predictor_construct == self.design.outcome_construct {
            return Err(invalid(
                "design.outcome_construct",
                "must differ from the predictor construct",
            ));
        }
        validate_loadings(
            "design.predictor_indicator_loadings",
            &self.design.predictor_indicator_loadings,
        )?;
        validate_loadings(
            "design.outcome_indicator_loadings",
            &self.design.outcome_indicator_loadings,
        )?;
        if !self.design.population_path.is_finite() || self.design.population_path.abs() > 0.80 {
            return Err(invalid(
                "design.population_path",
                "must be finite and between -0.80 and 0.80",
            ));
        }
        if self.estimator.weighting_scheme != WeightingScheme::Path {
            return Err(unsupported(
                "estimator.weighting_scheme",
                &format!("{} supports path weighting only", identity.version_label),
            ));
        }
        if self.estimator.preprocessing != Preprocessing::Standardized {
            return Err(unsupported(
                "estimator.preprocessing",
                &format!(
                    "{} supports standardized indicators only",
                    identity.version_label
                ),
            ));
        }
        if !self.estimator.tolerance.is_finite()
            || !(1e-10..=1e-3).contains(&self.estimator.tolerance)
        {
            return Err(invalid(
                "estimator.tolerance",
                "must be finite and between 1e-10 and 1e-3",
            ));
        }
        if !(100..=10_000).contains(&self.estimator.max_iterations) {
            return Err(invalid(
                "estimator.max_iterations",
                "must be between 100 and 10000",
            ));
        }
        if !(2..=MAX_GRID_POINTS).contains(&self.sample_size_grid.len()) {
            return Err(invalid(
                "sample_size_grid",
                "must contain between 2 and 16 values",
            ));
        }
        let mut previous = None;
        for sample_size in &self.sample_size_grid {
            if !(MIN_SAMPLE_SIZE..=MAX_SAMPLE_SIZE).contains(sample_size) {
                return Err(invalid(
                    "sample_size_grid",
                    "every value must be between 30 and 5000",
                ));
            }
            if previous.is_some_and(|value| *sample_size <= value) {
                return Err(invalid(
                    "sample_size_grid",
                    "values must be strictly increasing and unique",
                ));
            }
            previous = Some(*sample_size);
        }
        if !self.alpha.is_finite() || !(0.001..=0.10).contains(&self.alpha) {
            return Err(invalid("alpha", "must be between 0.001 and 0.10"));
        }
        if !self.target_power.is_finite() || !(0.50..=0.99).contains(&self.target_power) {
            return Err(invalid("target_power", "must be between 0.50 and 0.99"));
        }
        if !self.confidence_level.is_finite() || !(0.80..=0.999).contains(&self.confidence_level) {
            return Err(invalid(
                "confidence_level",
                "must be between 0.80 and 0.999",
            ));
        }
        if !(MIN_MONTE_CARLO_REPLICATES..=MAX_MONTE_CARLO_REPLICATES)
            .contains(&self.monte_carlo_replicates)
        {
            return Err(invalid(
                "monte_carlo_replicates",
                "must be between 100 and 10000",
            ));
        }
        if !(MIN_BOOTSTRAP_REPLICATES..=MAX_BOOTSTRAP_REPLICATES)
            .contains(&self.bootstrap_replicates)
            || self.bootstrap_replicates % 2 == 0
        {
            return Err(invalid(
                "bootstrap_replicates",
                "must be an odd number between 99 and 1999",
            ));
        }
        if !(1..=MAX_WORKERS).contains(&self.workers) {
            return Err(invalid("workers", "must be between 1 and 64"));
        }

        let best_possible = wilson_interval(
            self.monte_carlo_replicates,
            self.monte_carlo_replicates,
            self.confidence_level,
        )
        .0;
        if best_possible + f64::EPSILON < self.target_power {
            return Err(invalid(
                "monte_carlo_replicates",
                "cannot make the Wilson lower bound reach target_power even if every replicate rejects",
            ));
        }

        let grid_points = self.sample_size_grid.len() as u64;
        let planned_datasets = grid_points
            .checked_mul(self.monte_carlo_replicates as u64)
            .ok_or_else(|| invalid("sample_size_grid", "planned dataset count overflowed"))?;
        // The prospective test uses the production case-bootstrap primary
        // kernel but intentionally excludes BCa/delete-one work that is not
        // consumed by the normal-reference rejection rule.
        let fits_per_dataset = 1_u64
            .checked_add(self.bootstrap_replicates as u64)
            .ok_or_else(|| invalid("bootstrap_replicates", "estimated workload overflowed"))?;
        let estimated_pls_fits = planned_datasets
            .checked_mul(fits_per_dataset)
            .ok_or_else(|| invalid("sample_size_grid", "estimated workload overflowed"))?;
        if estimated_pls_fits > MAX_ESTIMATED_PLS_FITS {
            return Err(invalid(
                "sample_size_grid",
                "estimated PLS workload exceeds the 250000-fit desktop execution limit",
            ));
        }
        let estimated_pls_case_fits = self
            .sample_size_grid
            .iter()
            .try_fold(0_u64, |total, sample_size| {
                total.checked_add(
                    (*sample_size as u64)
                        .checked_mul(self.monte_carlo_replicates as u64)?
                        .checked_mul(fits_per_dataset)?,
                )
            })
            .ok_or_else(|| invalid("sample_size_grid", "estimated case-fit workload overflowed"))?;
        if estimated_pls_case_fits > MAX_ESTIMATED_PLS_CASE_FITS {
            return Err(invalid(
                "sample_size_grid",
                "estimated case-fit workload exceeds the 100000000-row desktop execution limit",
            ));
        }
        Ok(PlsPowerWorkload {
            grid_points,
            planned_datasets,
            estimated_pls_fits,
            estimated_pls_case_fits,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlsPowerWorkload {
    pub grid_points: u64,
    pub planned_datasets: u64,
    pub estimated_pls_fits: u64,
    pub estimated_pls_case_fits: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlsPowerPhase {
    Simulating,
    Summarizing,
}

impl PlsPowerPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Simulating => "pls_power_simulating",
            Self::Summarizing => "pls_power_summarizing",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlsPowerProgress {
    pub phase: PlsPowerPhase,
    pub completed_replicates: u64,
    pub total_replicates: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsPowerReplicateOutcomeV1 {
    pub sample_size: u32,
    pub replicate_index: u32,
    pub stream_identity: String,
    pub attempted: bool,
    pub successful: bool,
    pub converged: bool,
    pub target_estimate: Option<f64>,
    pub p_value_two_sided: Option<f64>,
    /// V2-only exact accounting for the null-centered plus-one decision.
    /// Historical v1 ledgers deserialize with these fields absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap_requested_replicates: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap_usable_replicates: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap_failed_replicates: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap_two_sided_exceedances: Option<u32>,
    pub rejected: bool,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsPowerRowV1 {
    pub sample_size: u32,
    pub requested_replicates: u32,
    pub attempted_replicates: u32,
    pub successful_replicates: u32,
    pub failed_replicates: u32,
    pub rejections: u32,
    pub achieved_power: f64,
    pub confidence_lower: f64,
    pub confidence_upper: f64,
    pub qualifies: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlsPowerGridDecisionV1 {
    Reached { sample_size: u32 },
    NotReached,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsSampleSizePowerResultV1 {
    pub schema_version: u32,
    pub capability_id: String,
    pub method_version: String,
    pub recipe_digest: String,
    pub stream_domain: String,
    pub failure_policy: String,
    pub interval_method: String,
    pub inference_method: String,
    pub pls_method_version: String,
    pub resampling_method_version: String,
    pub workload: PlsPowerWorkload,
    pub rows: Vec<PlsPowerRowV1>,
    pub outcomes: Vec<PlsPowerReplicateOutcomeV1>,
    pub outcome_digest: String,
    pub decision: PlsPowerGridDecisionV1,
    pub monotonicity_violations: u32,
    pub warnings: Vec<String>,
    pub exclusions: Vec<String>,
}

pub type PlsSampleSizePowerResultV2 = PlsSampleSizePowerResultV1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsSampleSizePowerArchiveV1 {
    pub recipe: PlsSampleSizePowerRecipeV1,
    pub result: PlsSampleSizePowerResultV1,
    pub checksum_sha256: String,
}

pub type PlsSampleSizePowerArchiveV2 = PlsSampleSizePowerArchiveV1;

impl PlsSampleSizePowerArchiveV1 {
    pub fn seal(
        recipe: PlsSampleSizePowerRecipeV1,
        result: PlsSampleSizePowerResultV1,
    ) -> Result<Self, PlsSampleSizePowerError> {
        validate_result(&recipe, &result)?;
        let checksum_sha256 = archive_checksum(&recipe, &result)?;
        Ok(Self {
            recipe,
            result,
            checksum_sha256,
        })
    }

    pub fn verify(&self) -> Result<(), PlsSampleSizePowerError> {
        validate_result(&self.recipe, &self.result)?;
        let expected = archive_checksum(&self.recipe, &self.result)?;
        if self.checksum_sha256 != expected {
            return Err(PlsSampleSizePowerError::Tamper(
                "sample-size/power archive checksum does not match its typed payload".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlsPowerExportTableV1 {
    pub name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlsPowerExportBundleV1 {
    pub capability_id: String,
    pub method_version: String,
    pub recipe_digest: String,
    pub outcome_digest: String,
    pub tables: Vec<PlsPowerExportTableV1>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlsSampleSizePowerError {
    #[error("invalid sample-size/power recipe field {field}: {message}")]
    InvalidRecipe { field: String, message: String },
    #[error("unsupported sample-size/power design field {field}: {message}")]
    UnsupportedDesign { field: String, message: String },
    #[error("cannot create sample-size/power worker pool: {0}")]
    WorkerPool(String),
    #[error("sample-size/power calculation was cancelled")]
    Cancelled,
    #[error("sample-size/power result is inconsistent: {0}")]
    InconsistentResult(String),
    #[error("sample-size/power archive failed strict validation: {0}")]
    Tamper(String),
    #[error("sample-size/power payload serialization failed: {0}")]
    Serialization(String),
}

pub fn run_pls_sample_size_power_v1(
    recipe: &PlsSampleSizePowerRecipeV1,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(PlsPowerProgress) + Sync,
) -> Result<PlsSampleSizePowerResultV1, PlsSampleSizePowerError> {
    if contract_identity(recipe)? != PLS_POWER_V1_IDENTITY {
        return Err(invalid(
            "method_version",
            "run_pls_sample_size_power_v1 accepts only the historical v1 identity",
        ));
    }
    run_pls_sample_size_power(recipe, is_cancelled, report_progress)
}

pub fn run_pls_sample_size_power_v2(
    recipe: &PlsSampleSizePowerRecipeV2,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(PlsPowerProgress) + Sync,
) -> Result<PlsSampleSizePowerResultV2, PlsSampleSizePowerError> {
    if contract_identity(recipe)? != PLS_POWER_V2_IDENTITY {
        return Err(invalid(
            "method_version",
            "run_pls_sample_size_power_v2 accepts only the null-calibrated v2 identity",
        ));
    }
    run_pls_sample_size_power(recipe, is_cancelled, report_progress)
}

fn run_pls_sample_size_power(
    recipe: &PlsSampleSizePowerRecipeV1,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(PlsPowerProgress) + Sync,
) -> Result<PlsSampleSizePowerResultV1, PlsSampleSizePowerError> {
    let identity = contract_identity(recipe)?;
    let workload = recipe.validate()?;
    if is_cancelled() {
        return Err(PlsSampleSizePowerError::Cancelled);
    }
    let tasks = recipe
        .sample_size_grid
        .iter()
        .flat_map(|sample_size| {
            (0..recipe.monte_carlo_replicates)
                .map(move |replicate_index| (*sample_size, replicate_index))
        })
        .collect::<Vec<_>>();
    let total = tasks.len() as u64;
    let mut outcomes = Vec::with_capacity(tasks.len());
    // Use one Rayon layer: the production case-bootstrap scheduler. Parallel
    // Monte Carlo workers around that scheduler would nest a fresh bootstrap
    // pool per outer worker and can exhaust the Windows worker stack. Indexed
    // tasks remain sequentially collected, while recipe.workers controls the
    // production bootstrap pool and therefore remains execution-only.
    for (index, (sample_size, replicate_index)) in tasks.into_iter().enumerate() {
        if is_cancelled() {
            return Err(PlsSampleSizePowerError::Cancelled);
        }
        outcomes.push(execute_replicate(
            recipe,
            sample_size,
            replicate_index,
            &is_cancelled,
        ));
        report_progress(PlsPowerProgress {
            phase: PlsPowerPhase::Simulating,
            completed_replicates: index as u64 + 1,
            total_replicates: total,
        });
    }
    if is_cancelled() {
        return Err(PlsSampleSizePowerError::Cancelled);
    }
    report_progress(PlsPowerProgress {
        phase: PlsPowerPhase::Summarizing,
        completed_replicates: total,
        total_replicates: total,
    });
    let rows = summarize_rows(recipe, &outcomes);
    let decision =
        rows.iter()
            .find(|row| row.qualifies)
            .map_or(PlsPowerGridDecisionV1::NotReached, |row| {
                PlsPowerGridDecisionV1::Reached {
                    sample_size: row.sample_size,
                }
            });
    let monotonicity_violations = rows
        .windows(2)
        .filter(|pair| pair[1].achieved_power + 1e-12 < pair[0].achieved_power)
        .count() as u32;
    let recipe_digest = scientific_recipe_digest(recipe)?;
    let outcome_digest = digest_json(&outcomes)?;
    let warnings = expected_warnings(identity, &workload, monotonicity_violations);
    let exclusions = expected_exclusions(identity);
    let result = PlsSampleSizePowerResultV1 {
        schema_version: identity.result_schema_version,
        capability_id: PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID.into(),
        method_version: identity.method_version.into(),
        recipe_digest,
        stream_domain: identity.stream_domain.into(),
        failure_policy: PLS_SAMPLE_SIZE_POWER_FAILURE_POLICY.into(),
        interval_method: PLS_SAMPLE_SIZE_POWER_INTERVAL_METHOD.into(),
        inference_method: identity.inference_method.into(),
        pls_method_version: PLS_METHOD_VERSION.into(),
        resampling_method_version: RESAMPLING_METHOD_VERSION.into(),
        workload,
        rows,
        outcomes,
        outcome_digest,
        decision,
        monotonicity_violations,
        warnings,
        exclusions,
    };
    validate_result(recipe, &result)?;
    Ok(result)
}

pub fn validate_result(
    recipe: &PlsSampleSizePowerRecipeV1,
    result: &PlsSampleSizePowerResultV1,
) -> Result<(), PlsSampleSizePowerError> {
    let identity = contract_identity(recipe)?;
    let workload = recipe.validate()?;
    if result.schema_version != identity.result_schema_version
        || result.capability_id != PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID
        || result.method_version != identity.method_version
        || result.stream_domain != identity.stream_domain
        || result.failure_policy != PLS_SAMPLE_SIZE_POWER_FAILURE_POLICY
        || result.interval_method != PLS_SAMPLE_SIZE_POWER_INTERVAL_METHOD
        || result.inference_method != identity.inference_method
        || result.pls_method_version != PLS_METHOD_VERSION
        || result.resampling_method_version != RESAMPLING_METHOD_VERSION
    {
        return Err(PlsSampleSizePowerError::InconsistentResult(
            format!(
                "stable method or provenance identity differs from the frozen {} contract",
                identity.version_label
            ),
        ));
    }
    if result.workload != workload {
        return Err(PlsSampleSizePowerError::InconsistentResult(
            "stored workload differs from the validated recipe".into(),
        ));
    }
    if result.recipe_digest != scientific_recipe_digest(recipe)? {
        return Err(PlsSampleSizePowerError::InconsistentResult(
            "recipe digest differs from the typed recipe".into(),
        ));
    }
    if result.outcome_digest != digest_json(&result.outcomes)? {
        return Err(PlsSampleSizePowerError::InconsistentResult(
            "outcome digest differs from the ordered replicate ledger".into(),
        ));
    }
    let expected_count = recipe.sample_size_grid.len() * recipe.monte_carlo_replicates as usize;
    if result.outcomes.len() != expected_count {
        return Err(PlsSampleSizePowerError::InconsistentResult(
            "ordered replicate ledger has the wrong length".into(),
        ));
    }
    for (position, outcome) in result.outcomes.iter().enumerate() {
        let grid_index = position / recipe.monte_carlo_replicates as usize;
        let replicate_index = (position % recipe.monte_carlo_replicates as usize) as u32;
        let sample_size = recipe.sample_size_grid[grid_index];
        if outcome.sample_size != sample_size
            || outcome.replicate_index != replicate_index
            || outcome.stream_identity != stream_identity(recipe, sample_size, replicate_index)
            || !outcome.attempted
            || outcome.successful != outcome.failure_code.is_none()
            || outcome.successful != outcome.failure_message.is_none()
            || (!outcome.successful
                && (outcome
                    .failure_code
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                    || outcome
                        .failure_message
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())))
            || outcome.rejected
                != outcome
                    .p_value_two_sided
                    .is_some_and(|p_value| p_value <= recipe.alpha)
            || (outcome.successful
                && (!outcome.converged
                    || outcome.target_estimate.is_none()
                    || outcome.p_value_two_sided.is_none()))
            || (!outcome.successful
                && (outcome.converged
                    || outcome.target_estimate.is_some()
                    || outcome.p_value_two_sided.is_some()
                    || outcome.rejected))
        {
            return Err(PlsSampleSizePowerError::InconsistentResult(format!(
                "replicate ledger row {position} violates its indexed outcome contract"
            )));
        }
        if outcome
            .target_estimate
            .is_some_and(|value| !value.is_finite())
            || outcome
                .p_value_two_sided
                .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            return Err(PlsSampleSizePowerError::InconsistentResult(format!(
                "replicate ledger row {position} contains a non-finite analytical value"
            )));
        }
        match recipe.inference {
            PlsPowerInferenceV1::CaseBootstrapNormalReferenceTwoSided => {
                if outcome.bootstrap_requested_replicates.is_some()
                    || outcome.bootstrap_usable_replicates.is_some()
                    || outcome.bootstrap_failed_replicates.is_some()
                    || outcome.bootstrap_two_sided_exceedances.is_some()
                {
                    return Err(PlsSampleSizePowerError::InconsistentResult(format!(
                        "historical v1 replicate ledger row {position} contains injected v2 tail accounting"
                    )));
                }
            }
            PlsPowerInferenceV1::CaseBootstrapNullCenteredTwoSidedPlusOne => {
                if outcome.successful {
                    let Some(requested) = outcome.bootstrap_requested_replicates else {
                        return Err(PlsSampleSizePowerError::InconsistentResult(format!(
                            "v2 replicate ledger row {position} omits requested bootstrap accounting"
                        )));
                    };
                    let Some(usable) = outcome.bootstrap_usable_replicates else {
                        return Err(PlsSampleSizePowerError::InconsistentResult(format!(
                            "v2 replicate ledger row {position} omits usable bootstrap accounting"
                        )));
                    };
                    let Some(failed) = outcome.bootstrap_failed_replicates else {
                        return Err(PlsSampleSizePowerError::InconsistentResult(format!(
                            "v2 replicate ledger row {position} omits failed bootstrap accounting"
                        )));
                    };
                    let Some(exceedances) = outcome.bootstrap_two_sided_exceedances else {
                        return Err(PlsSampleSizePowerError::InconsistentResult(format!(
                            "v2 replicate ledger row {position} omits tail exceedance accounting"
                        )));
                    };
                    let expected_probability =
                        (f64::from(exceedances) + 1.0) / (f64::from(usable) + 1.0);
                    if requested != recipe.bootstrap_replicates
                        || usable.saturating_add(failed) != requested
                        || usable < ((f64::from(requested) * 0.9).ceil() as u32).max(2)
                        || exceedances > usable
                        || outcome
                            .p_value_two_sided
                            .is_none_or(|value| value.to_bits() != expected_probability.to_bits())
                    {
                        return Err(PlsSampleSizePowerError::InconsistentResult(format!(
                            "v2 replicate ledger row {position} has inconsistent null-centered plus-one accounting"
                        )));
                    }
                } else if outcome.bootstrap_requested_replicates.is_some()
                    || outcome.bootstrap_usable_replicates.is_some()
                    || outcome.bootstrap_failed_replicates.is_some()
                    || outcome.bootstrap_two_sided_exceedances.is_some()
                {
                    return Err(PlsSampleSizePowerError::InconsistentResult(format!(
                        "failed v2 replicate ledger row {position} contains successful tail accounting"
                    )));
                }
            }
        }
    }
    let rows = summarize_rows(recipe, &result.outcomes);
    if result.rows != rows {
        return Err(PlsSampleSizePowerError::InconsistentResult(
            "power rows do not reproduce from the ordered replicate ledger".into(),
        ));
    }
    let decision =
        rows.iter()
            .find(|row| row.qualifies)
            .map_or(PlsPowerGridDecisionV1::NotReached, |row| {
                PlsPowerGridDecisionV1::Reached {
                    sample_size: row.sample_size,
                }
            });
    if result.decision != decision {
        return Err(PlsSampleSizePowerError::InconsistentResult(
            "minimum-grid decision does not reproduce from Wilson lower bounds".into(),
        ));
    }
    let monotonicity_violations = rows
        .windows(2)
        .filter(|pair| pair[1].achieved_power + 1e-12 < pair[0].achieved_power)
        .count() as u32;
    if result.monotonicity_violations != monotonicity_violations {
        return Err(PlsSampleSizePowerError::InconsistentResult(
            "monotonicity diagnostic does not reproduce from the power rows".into(),
        ));
    }
    if result.warnings != expected_warnings(identity, &workload, monotonicity_violations) {
        return Err(PlsSampleSizePowerError::InconsistentResult(
            "warnings do not reproduce from the validated workload and power rows".into(),
        ));
    }
    if result.exclusions != expected_exclusions(identity) {
        return Err(PlsSampleSizePowerError::InconsistentResult(
            format!(
                "scope exclusions differ from the frozen {} contract",
                identity.version_label
            ),
        ));
    }
    Ok(())
}

pub fn power_export_bundle(
    recipe: &PlsSampleSizePowerRecipeV1,
    result: &PlsSampleSizePowerResultV1,
) -> Result<PlsPowerExportBundleV1, PlsSampleSizePowerError> {
    validate_result(recipe, result)?;
    let power_rows = result
        .rows
        .iter()
        .map(|row| {
            vec![
                row.sample_size.to_string(),
                row.requested_replicates.to_string(),
                row.attempted_replicates.to_string(),
                row.successful_replicates.to_string(),
                row.failed_replicates.to_string(),
                row.rejections.to_string(),
                format_float(row.achieved_power),
                format_float(row.confidence_lower),
                format_float(row.confidence_upper),
                row.qualifies.to_string(),
            ]
        })
        .collect();
    let failure_rows = result
        .outcomes
        .iter()
        .filter(|outcome| !outcome.successful)
        .map(|outcome| {
            vec![
                outcome.sample_size.to_string(),
                outcome.replicate_index.to_string(),
                outcome.stream_identity.clone(),
                outcome.failure_code.clone().unwrap_or_default(),
                outcome.failure_message.clone().unwrap_or_default(),
            ]
        })
        .collect();
    let mut assumptions = BTreeMap::<String, String>::new();
    assumptions.insert("scenario_identity".into(), recipe.scenario_identity.clone());
    assumptions.insert(
        "target_path".into(),
        format!(
            "{} -> {}",
            recipe.design.predictor_construct, recipe.design.outcome_construct
        ),
    );
    assumptions.insert(
        "population_path".into(),
        format_float(recipe.design.population_path),
    );
    assumptions.insert(
        "predictor_indicator_loadings".into(),
        format_float_list(&recipe.design.predictor_indicator_loadings),
    );
    assumptions.insert(
        "outcome_indicator_loadings".into(),
        format_float_list(&recipe.design.outcome_indicator_loadings),
    );
    assumptions.insert("distribution".into(), "standard_normal".into());
    assumptions.insert("missing_data".into(), "none".into());
    assumptions.insert("alpha".into(), format_float(recipe.alpha));
    assumptions.insert("target_power".into(), format_float(recipe.target_power));
    assumptions.insert(
        "confidence_level".into(),
        format_float(recipe.confidence_level),
    );
    assumptions.insert(
        "monte_carlo_replicates".into(),
        recipe.monte_carlo_replicates.to_string(),
    );
    assumptions.insert(
        "bootstrap_replicates".into(),
        recipe.bootstrap_replicates.to_string(),
    );
    let assumption_rows = assumptions
        .into_iter()
        .map(|(key, value)| vec![key, value])
        .collect();
    let decision = match result.decision {
        PlsPowerGridDecisionV1::Reached { sample_size } => sample_size.to_string(),
        PlsPowerGridDecisionV1::NotReached => "not_reached_on_evaluated_grid".into(),
    };
    let provenance_rows = vec![
        vec!["capability_id".into(), result.capability_id.clone()],
        vec!["method_version".into(), result.method_version.clone()],
        vec!["recipe_digest".into(), result.recipe_digest.clone()],
        vec!["outcome_digest".into(), result.outcome_digest.clone()],
        vec!["stream_domain".into(), result.stream_domain.clone()],
        vec!["failure_policy".into(), result.failure_policy.clone()],
        vec!["interval_method".into(), result.interval_method.clone()],
        vec!["inference_method".into(), result.inference_method.clone()],
        vec![
            "pls_method_version".into(),
            result.pls_method_version.clone(),
        ],
        vec![
            "resampling_method_version".into(),
            result.resampling_method_version.clone(),
        ],
        vec!["grid_decision".into(), decision],
    ];
    Ok(PlsPowerExportBundleV1 {
        capability_id: result.capability_id.clone(),
        method_version: result.method_version.clone(),
        recipe_digest: result.recipe_digest.clone(),
        outcome_digest: result.outcome_digest.clone(),
        tables: vec![
            PlsPowerExportTableV1 {
                name: "Power by sample size".into(),
                columns: vec![
                    "sample_size".into(),
                    "requested_replicates".into(),
                    "attempted_replicates".into(),
                    "successful_replicates".into(),
                    "failed_replicates".into(),
                    "rejections".into(),
                    "achieved_power".into(),
                    "confidence_lower".into(),
                    "confidence_upper".into(),
                    "qualifies".into(),
                ],
                rows: power_rows,
            },
            PlsPowerExportTableV1 {
                name: "Simulation failures".into(),
                columns: vec![
                    "sample_size".into(),
                    "replicate_index".into(),
                    "stream_identity".into(),
                    "failure_code".into(),
                    "failure_message".into(),
                ],
                rows: failure_rows,
            },
            PlsPowerExportTableV1 {
                name: "Design assumptions".into(),
                columns: vec!["assumption".into(), "value".into()],
                rows: assumption_rows,
            },
            PlsPowerExportTableV1 {
                name: "Run provenance".into(),
                columns: vec!["field".into(), "value".into()],
                rows: provenance_rows,
            },
        ],
    })
}

fn execute_replicate(
    recipe: &PlsSampleSizePowerRecipeV1,
    sample_size: u32,
    replicate_index: u32,
    is_cancelled: &(impl Fn() -> bool + Sync),
) -> PlsPowerReplicateOutcomeV1 {
    let stream_identity = stream_identity(recipe, sample_size, replicate_index);
    let failure = |code: &str, message: String| PlsPowerReplicateOutcomeV1 {
        sample_size,
        replicate_index,
        stream_identity: stream_identity.clone(),
        attempted: true,
        successful: false,
        converged: false,
        target_estimate: None,
        p_value_two_sided: None,
        bootstrap_requested_replicates: None,
        bootstrap_usable_replicates: None,
        bootstrap_failed_replicates: None,
        bootstrap_two_sided_exceedances: None,
        rejected: false,
        failure_code: Some(code.into()),
        failure_message: Some(message),
    };
    let csv = match generate_dataset_csv(recipe, sample_size, replicate_index, is_cancelled) {
        Ok(csv) => csv,
        Err(error) => return failure("data_generation_failed", error),
    };
    if is_cancelled() {
        return failure(
            "cancelled",
            "calculation cancelled during data generation".into(),
        );
    }
    let dataset = match import_delimited_bytes(
        &csv,
        "pls_sample_size_power_generated.csv",
        b',',
        &ImportOptions::default(),
    ) {
        Ok(dataset) => dataset,
        Err(error) => return failure("generated_data_import_failed", error.to_string()),
    };
    let model = model_spec(recipe);
    let mut settings = AnalysisSettings {
        method: AnalysisMethod::PlsPm,
        weighting_scheme: recipe.estimator.weighting_scheme.clone(),
        tolerance: recipe.estimator.tolerance,
        max_iterations: recipe.estimator.max_iterations,
        bootstrap_samples: recipe.bootstrap_replicates,
        seed: derived_bootstrap_seed(recipe, sample_size, replicate_index),
        workers: 1,
        confidence_level: 1.0 - recipe.alpha,
        preprocessing: recipe.estimator.preprocessing.clone(),
        ..AnalysisSettings::default()
    };
    settings.studentized_inner_samples = 0;
    settings.permutation_samples = 0;
    let mut analysis_recipe = AnalysisRecipe::new(&csv, model, settings);
    analysis_recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
    let resampling_recipe =
        match ValidatedExecutionRecipe::for_dataset(&analysis_recipe, &dataset.fingerprint.0) {
            Ok(recipe) => recipe,
            Err(error) => return failure("analysis_recipe_invalid", error.to_string()),
        };
    let point_recipe = match resampling_recipe.without_outer_resampling() {
        Ok(recipe) => recipe,
        Err(error) => return failure("point_recipe_invalid", error.to_string()),
    };
    let original =
        match estimate_pls_validated_with_control(&dataset, &point_recipe, |_| !is_cancelled()) {
            Ok(result) if result.converged => result,
            Ok(_) => {
                return failure(
                    "pls_not_converged",
                    "PLS point estimate did not converge".into(),
                );
            }
            Err(error) => return failure("pls_estimation_failed", error.to_string()),
        };
    let target_estimate = original
        .paths
        .iter()
        .find(|path| {
            path.source == recipe.design.predictor_construct
                && path.target == recipe.design.outcome_construct
        })
        .map(|path| path.coefficient);
    let Some(target_estimate) = target_estimate else {
        return failure(
            "target_path_missing",
            "PLS point result omitted the declared target path".into(),
        );
    };
    let bootstrap = match bootstrap_pls_normal_reference_validated(
        &dataset,
        &resampling_recipe,
        &original,
        recipe.workers,
        || is_cancelled(),
        |_| {},
    ) {
        Ok(result) => result,
        Err(error) => return failure("bootstrap_inference_failed", error.to_string()),
    };
    let parameter = serde_json::to_string(&(
        "path",
        [
            recipe.design.predictor_construct.as_str(),
            recipe.design.outcome_construct.as_str(),
        ],
    ))
    .expect("target path identity is serializable");
    let (p_value, accounting) = match recipe.inference {
        PlsPowerInferenceV1::CaseBootstrapNormalReferenceTwoSided => (
            bootstrap
                .inference
                .parameters
                .iter()
                .find(|candidate| candidate.parameter == parameter)
                .and_then(|candidate| candidate.p_value_two_sided),
            None,
        ),
        PlsPowerInferenceV1::CaseBootstrapNullCenteredTwoSidedPlusOne => {
            let tail = bootstrap
                .test_tail_inference
                .parameters
                .iter()
                .find(|candidate| candidate.parameter == parameter);
            (
                tail.map(|candidate| candidate.p_value_two_sided),
                tail.map(|candidate| {
                    (
                        bootstrap.plan.replicates,
                        candidate.usable_replicates,
                        bootstrap
                            .plan
                            .replicates
                            .saturating_sub(candidate.usable_replicates),
                        candidate.two_sided_exceedances,
                    )
                }),
            )
        }
    };
    let Some(p_value_two_sided) = p_value.filter(|value| value.is_finite()) else {
        return failure(
            "target_inference_unavailable",
            "case-bootstrap inference did not yield a finite target-path p value".into(),
        );
    };
    let (
        bootstrap_requested_replicates,
        bootstrap_usable_replicates,
        bootstrap_failed_replicates,
        bootstrap_two_sided_exceedances,
    ) = accounting.map_or((None, None, None, None), |(requested, usable, failed, exceedances)| {
        (
            Some(requested),
            Some(usable),
            Some(failed),
            Some(exceedances),
        )
    });
    PlsPowerReplicateOutcomeV1 {
        sample_size,
        replicate_index,
        stream_identity,
        attempted: true,
        successful: true,
        converged: true,
        target_estimate: Some(target_estimate),
        p_value_two_sided: Some(p_value_two_sided),
        bootstrap_requested_replicates,
        bootstrap_usable_replicates,
        bootstrap_failed_replicates,
        bootstrap_two_sided_exceedances,
        rejected: p_value_two_sided <= recipe.alpha,
        failure_code: None,
        failure_message: None,
    }
}

fn generate_dataset_csv(
    recipe: &PlsSampleSizePowerRecipeV1,
    sample_size: u32,
    replicate_index: u32,
    is_cancelled: &(impl Fn() -> bool + Sync),
) -> Result<Vec<u8>, String> {
    let seed = stream_digest(recipe, sample_size, replicate_index, b"generated_data");
    let mut rng = ChaCha20Rng::from_seed(seed);
    let predictor_names = indicator_names(
        &recipe.design.predictor_construct,
        recipe.design.predictor_indicator_loadings.len(),
    );
    let outcome_names = indicator_names(
        &recipe.design.outcome_construct,
        recipe.design.outcome_indicator_loadings.len(),
    );
    let mut csv = String::with_capacity(
        sample_size as usize * (predictor_names.len() + outcome_names.len()) * 22,
    );
    csv.push_str(&predictor_names.join(","));
    csv.push(',');
    csv.push_str(&outcome_names.join(","));
    csv.push('\n');
    let beta = recipe.design.population_path;
    let disturbance_scale = (1.0 - beta * beta).sqrt();
    for row in 0..sample_size {
        if row % 32 == 0 && is_cancelled() {
            return Err("calculation cancelled".into());
        }
        let predictor = standard_normal(&mut rng);
        let outcome = beta * predictor + disturbance_scale * standard_normal(&mut rng);
        let mut first = true;
        for loading in &recipe.design.predictor_indicator_loadings {
            if !first {
                csv.push(',');
            }
            first = false;
            let indicator =
                loading * predictor + (1.0 - loading * loading).sqrt() * standard_normal(&mut rng);
            write!(&mut csv, "{indicator:.17}").map_err(|error| error.to_string())?;
        }
        for loading in &recipe.design.outcome_indicator_loadings {
            csv.push(',');
            let indicator =
                loading * outcome + (1.0 - loading * loading).sqrt() * standard_normal(&mut rng);
            write!(&mut csv, "{indicator:.17}").map_err(|error| error.to_string())?;
        }
        csv.push('\n');
    }
    Ok(csv.into_bytes())
}

fn model_spec(recipe: &PlsSampleSizePowerRecipeV1) -> ModelSpec {
    let predictor = &recipe.design.predictor_construct;
    let outcome = &recipe.design.outcome_construct;
    let json = serde_json::json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": "PLS sample-size/power generated design",
        "constructs": [
            {
                "id": predictor,
                "name": predictor,
                "short_name": predictor,
                "mode": "reflective",
                "indicators": indicator_names(predictor, recipe.design.predictor_indicator_loadings.len()),
            },
            {
                "id": outcome,
                "name": outcome,
                "short_name": outcome,
                "mode": "reflective",
                "indicators": indicator_names(outcome, recipe.design.outcome_indicator_loadings.len()),
            }
        ],
        "paths": [{"source": predictor, "target": outcome}],
        "controls": [],
        "higher_order_constructs": [],
        "interactions": [],
    });
    serde_json::from_value(json).expect("validated bounded power design creates a valid model")
}

fn summarize_rows(
    recipe: &PlsSampleSizePowerRecipeV1,
    outcomes: &[PlsPowerReplicateOutcomeV1],
) -> Vec<PlsPowerRowV1> {
    recipe
        .sample_size_grid
        .iter()
        .map(|sample_size| {
            let rows = outcomes
                .iter()
                .filter(|outcome| outcome.sample_size == *sample_size)
                .collect::<Vec<_>>();
            let attempted = rows.iter().filter(|outcome| outcome.attempted).count() as u32;
            let successful = rows.iter().filter(|outcome| outcome.successful).count() as u32;
            let rejections = rows.iter().filter(|outcome| outcome.rejected).count() as u32;
            let failed = recipe.monte_carlo_replicates.saturating_sub(successful);
            let achieved_power = rejections as f64 / recipe.monte_carlo_replicates as f64;
            let (confidence_lower, confidence_upper) = wilson_interval(
                rejections,
                recipe.monte_carlo_replicates,
                recipe.confidence_level,
            );
            PlsPowerRowV1 {
                sample_size: *sample_size,
                requested_replicates: recipe.monte_carlo_replicates,
                attempted_replicates: attempted,
                successful_replicates: successful,
                failed_replicates: failed,
                rejections,
                achieved_power,
                confidence_lower,
                confidence_upper,
                qualifies: confidence_lower >= recipe.target_power,
            }
        })
        .collect()
}

fn expected_warnings(
    identity: PlsPowerContractIdentity,
    workload: &PlsPowerWorkload,
    monotonicity_violations: u32,
) -> Vec<String> {
    let mut warnings = vec![
        "Power is conditional on the declared two-construct reflective Gaussian data-generating design and target path.".into(),
        "Failed or inadmissible replicates remain in the denominator as non-rejections.".into(),
        "The selected value is the first evaluated grid point whose Wilson lower confidence bound reaches target power; no interpolation or extrapolation is performed.".into(),
        format!(
            "Planned desktop workload: {} simulated datasets, {} PLS fits, and {} row-fits; actual duration depends on the model, grid, and hardware.",
            workload.planned_datasets,
            workload.estimated_pls_fits,
            workload.estimated_pls_case_fits
        ),
        "Each stream_identity identifies the deterministic indexed random-number stream for one (sample size, replicate) pair; the scientific recipe digest, not stream_identity, binds the declared path, loadings, and full data-generating design.".into(),
    ];
    if identity == PLS_POWER_V2_IDENTITY {
        warnings.insert(
            1,
            "Each target-path decision uses the exact null-centered, two-sided plus-one tail probability from the indexed case-bootstrap plan; the historical v1 normal-reference rule is not reused.".into(),
        );
    }
    if monotonicity_violations > 0 {
        warnings.push(format!(
            "Monte Carlo power decreased at {monotonicity_violations} adjacent grid step(s); rows are reported unsmoothed and the conservative first-qualified-grid rule is unchanged."
        ));
    }
    warnings
}

fn expected_exclusions(identity: PlsPowerContractIdentity) -> Vec<String> {
    vec![
        format!(
            "Formative, higher-order, interaction, nonlinear, mixture, multigroup, endogeneity, weighted, clustered, ordinal, matrix-only, and missing-data designs are outside {}.",
            identity.version_label
        ),
        "Retrospective observed power and heuristic sample-size rules are not part of this calculation.".into(),
    ]
}

pub fn wilson_interval(successes: u32, trials: u32, confidence_level: f64) -> (f64, f64) {
    if trials == 0
        || successes > trials
        || !confidence_level.is_finite()
        || !(0.0..1.0).contains(&confidence_level)
    {
        return (f64::NAN, f64::NAN);
    }
    let n = trials as f64;
    let proportion = successes as f64 / n;
    let tail = (1.0 - confidence_level) / 2.0;
    let z = Normal::standard().inverse_cdf(1.0 - tail);
    let z_squared = z * z;
    let denominator = 1.0 + z_squared / n;
    let center = (proportion + z_squared / (2.0 * n)) / denominator;
    let half_width = z * ((proportion * (1.0 - proportion) / n + z_squared / (4.0 * n * n)).sqrt())
        / denominator;
    (
        (center - half_width).max(0.0),
        (center + half_width).min(1.0),
    )
}

fn validate_identity(field: &str, value: &str) -> Result<(), PlsSampleSizePowerError> {
    if value.is_empty()
        || value.len() > 80
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(invalid(
            field,
            "must be 1-80 ASCII letters, digits, dots, underscores, or hyphens",
        ));
    }
    Ok(())
}

fn validate_loadings(field: &str, loadings: &[f64]) -> Result<(), PlsSampleSizePowerError> {
    if !(3..=10).contains(&loadings.len()) {
        return Err(invalid(field, "must contain between 3 and 10 loadings"));
    }
    if loadings
        .iter()
        .any(|loading| !loading.is_finite() || !(0.50..=0.95).contains(loading))
    {
        return Err(invalid(
            field,
            "every loading must be finite and between 0.50 and 0.95",
        ));
    }
    Ok(())
}

fn indicator_names(construct: &str, count: usize) -> Vec<String> {
    (1..=count)
        .map(|index| format!("{construct}_indicator_{index}"))
        .collect()
}

fn standard_normal(rng: &mut ChaCha20Rng) -> f64 {
    let u1 = loop {
        let candidate = rng.random::<f64>();
        if candidate > f64::MIN_POSITIVE {
            break candidate;
        }
    };
    let u2 = rng.random::<f64>();
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

fn stream_identity(
    recipe: &PlsSampleSizePowerRecipeV1,
    sample_size: u32,
    replicate_index: u32,
) -> String {
    hex(&stream_digest(
        recipe,
        sample_size,
        replicate_index,
        b"identity",
    ))
}

fn stream_digest(
    recipe: &PlsSampleSizePowerRecipeV1,
    sample_size: u32,
    replicate_index: u32,
    subdomain: &[u8],
) -> [u8; 32] {
    let mut digest = Sha256::new();
    let stream_domain = contract_identity(recipe)
        .map(|identity| identity.stream_domain)
        .unwrap_or("quickpls/pls_sample_size_power_invalid/monte_carlo");
    digest.update(stream_domain.as_bytes());
    digest.update([0]);
    digest.update(recipe.method_version.as_bytes());
    digest.update([0]);
    digest.update(recipe.scenario_identity.as_bytes());
    digest.update([0]);
    digest.update(recipe.master_seed.to_le_bytes());
    digest.update(sample_size.to_le_bytes());
    digest.update(replicate_index.to_le_bytes());
    digest.update(subdomain);
    digest.finalize().into()
}

fn derived_bootstrap_seed(
    recipe: &PlsSampleSizePowerRecipeV1,
    sample_size: u32,
    replicate_index: u32,
) -> u64 {
    let digest = stream_digest(recipe, sample_size, replicate_index, b"bootstrap_inference");
    u64::from_le_bytes(digest[..8].try_into().expect("SHA-256 prefix has 8 bytes"))
}

fn digest_json<T: Serialize>(value: &T) -> Result<String, PlsSampleSizePowerError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| PlsSampleSizePowerError::Serialization(error.to_string()))?;
    Ok(hex(&Sha256::digest(encoded)))
}

fn scientific_recipe_digest(
    recipe: &PlsSampleSizePowerRecipeV1,
) -> Result<String, PlsSampleSizePowerError> {
    // Worker count is an execution-only scheduling choice. The sealed archive
    // checksum still covers the unmodified typed recipe (including workers),
    // while this analytical identity stays stable across deterministic worker
    // counts.
    let mut normalized = recipe.clone();
    normalized.workers = 1;
    digest_json(&normalized)
}

fn archive_checksum(
    recipe: &PlsSampleSizePowerRecipeV1,
    result: &PlsSampleSizePowerResultV1,
) -> Result<String, PlsSampleSizePowerError> {
    let identity = contract_identity(recipe)?;
    digest_json(&(identity.archive_domain, recipe, result))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn format_float(value: f64) -> String {
    format!("{value:.12}")
}

fn format_float_list(values: &[f64]) -> String {
    values
        .iter()
        .map(|value| format_float(*value))
        .collect::<Vec<_>>()
        .join(";")
}

fn invalid(field: &str, message: &str) -> PlsSampleSizePowerError {
    PlsSampleSizePowerError::InvalidRecipe {
        field: field.into(),
        message: message.into(),
    }
}

fn unsupported(field: &str, message: &str) -> PlsSampleSizePowerError {
    PlsSampleSizePowerError::UnsupportedDesign {
        field: field.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe() -> PlsSampleSizePowerRecipeV1 {
        PlsSampleSizePowerRecipeV1 {
            schema_version: 1,
            capability_id: PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID.into(),
            method_version: PLS_SAMPLE_SIZE_POWER_METHOD_VERSION.into(),
            scenario_identity: "two_construct_signal".into(),
            design: ReflectiveGaussianPathDesignV1 {
                predictor_construct: "x".into(),
                outcome_construct: "y".into(),
                predictor_indicator_loadings: vec![0.8, 0.8, 0.8],
                outcome_indicator_loadings: vec![0.8, 0.8, 0.8],
                population_path: 0.30,
                exogenous_distribution: PlsPowerDistributionV1::StandardNormal,
                structural_disturbance_distribution: PlsPowerDistributionV1::StandardNormal,
                indicator_error_distribution: PlsPowerDistributionV1::StandardNormal,
                missing_data: PlsPowerMissingDataV1::None,
            },
            estimator: PlsPowerEstimatorSettingsV1 {
                weighting_scheme: WeightingScheme::Path,
                preprocessing: Preprocessing::Standardized,
                tolerance: 1e-7,
                max_iterations: 3_000,
            },
            inference: PlsPowerInferenceV1::CaseBootstrapNormalReferenceTwoSided,
            sample_size_grid: vec![50, 100],
            alpha: 0.05,
            target_power: 0.80,
            confidence_level: 0.95,
            monte_carlo_replicates: 100,
            bootstrap_replicates: 99,
            master_seed: 20_260_813,
            workers: 1,
        }
    }

    fn recipe_v2() -> PlsSampleSizePowerRecipeV2 {
        let mut input = recipe();
        input.schema_version = PLS_SAMPLE_SIZE_POWER_RECIPE_SCHEMA_VERSION_V2;
        input.method_version = PLS_SAMPLE_SIZE_POWER_METHOD_VERSION_V2.into();
        input.inference = PlsPowerInferenceV1::CaseBootstrapNullCenteredTwoSidedPlusOne;
        input
    }

    #[test]
    fn validates_the_bounded_explicit_design_and_workload() {
        let workload = recipe().validate().unwrap();
        assert_eq!(workload.grid_points, 2);
        assert_eq!(workload.planned_datasets, 200);
        assert_eq!(workload.estimated_pls_fits, 20_000);
        assert_eq!(workload.estimated_pls_case_fits, 1_500_000);
    }

    #[test]
    fn rejects_heuristic_or_underspecified_substitutions() {
        let mut input = recipe();
        input.design.predictor_indicator_loadings.clear();
        assert!(matches!(
            input.validate(),
            Err(PlsSampleSizePowerError::InvalidRecipe { field, .. })
                if field == "design.predictor_indicator_loadings"
        ));
        let mut input = recipe();
        input.estimator.weighting_scheme = WeightingScheme::Factor;
        assert!(matches!(
            input.validate(),
            Err(PlsSampleSizePowerError::UnsupportedDesign { field, .. })
                if field == "estimator.weighting_scheme"
        ));
    }

    #[test]
    fn rejects_non_increasing_grids_and_infeasible_precision() {
        let mut input = recipe();
        input.sample_size_grid = vec![100, 100];
        assert!(input.validate().is_err());
        let mut input = recipe();
        input.target_power = 0.99;
        assert!(matches!(
            input.validate(),
            Err(PlsSampleSizePowerError::InvalidRecipe { field, .. })
                if field == "monte_carlo_replicates"
        ));
    }

    #[test]
    fn wilson_interval_matches_known_binomial_values() {
        let (lower, upper) = wilson_interval(80, 100, 0.95);
        // These full-precision values are also frozen in the native projection
        // tests. They prevent a lower-precision inverse-normal implementation
        // in another runtime from rejecting a valid Rust-authored result.
        assert!((lower - 0.711_170_834_406_841_1).abs() < 1e-15);
        assert!((upper - 0.866_633_066_668_967_6).abs() < 1e-15);
    }

    #[test]
    fn stream_plan_is_indexed_and_domain_separated() {
        let input = recipe();
        assert_eq!(
            stream_identity(&input, 50, 0),
            stream_identity(&input, 50, 0)
        );
        assert_ne!(
            stream_identity(&input, 50, 0),
            stream_identity(&input, 50, 1)
        );
        assert_ne!(
            stream_identity(&input, 50, 0),
            stream_identity(&input, 100, 0)
        );
        assert_ne!(
            stream_digest(&input, 50, 0, b"generated_data"),
            stream_digest(&input, 50, 0, b"bootstrap_inference")
        );
        let v2 = recipe_v2();
        assert_ne!(
            stream_identity(&input, 50, 0),
            stream_identity(&v2, 50, 0)
        );
    }

    #[test]
    fn v2_uses_exact_null_centered_plus_one_accounting_without_relabeling_v1() {
        let input = recipe_v2();
        assert_eq!(contract_identity(&input).unwrap(), PLS_POWER_V2_IDENTITY);
        assert!(matches!(
            run_pls_sample_size_power_v1(&input, || true, |_| {}),
            Err(PlsSampleSizePowerError::InvalidRecipe { field, .. }) if field == "method_version"
        ));

        let outcome = execute_replicate(&input, 50, 0, &|| false);
        assert!(outcome.successful, "unexpected v2 failure: {outcome:#?}");
        assert_eq!(
            outcome.bootstrap_requested_replicates,
            Some(input.bootstrap_replicates)
        );
        let usable = outcome.bootstrap_usable_replicates.unwrap();
        let failed = outcome.bootstrap_failed_replicates.unwrap();
        let exceedances = outcome.bootstrap_two_sided_exceedances.unwrap();
        assert_eq!(usable + failed, input.bootstrap_replicates);
        assert!(exceedances <= usable);
        let expected = (f64::from(exceedances) + 1.0) / (f64::from(usable) + 1.0);
        assert_eq!(
            outcome.p_value_two_sided.unwrap().to_bits(),
            expected.to_bits()
        );
        assert_eq!(outcome.rejected, expected <= input.alpha);
    }

    #[test]
    fn point_estimation_is_resampling_free_before_production_bootstrap() {
        let input = recipe();
        let first = execute_replicate(&input, 50, 0, &|| false);
        let repeated = execute_replicate(&input, 50, 0, &|| false);
        let second = execute_replicate(&input, 50, 1, &|| false);

        for outcome in [&first, &repeated, &second] {
            assert!(
                outcome.successful,
                "unexpected replicate failure: {outcome:#?}"
            );
            assert!(outcome.converged);
            assert!(outcome.target_estimate.is_some_and(f64::is_finite));
            assert!(outcome.p_value_two_sided.is_some_and(f64::is_finite));
            assert_eq!(outcome.failure_code, None);
            assert_eq!(outcome.failure_message, None);
        }
        assert_eq!(first, repeated);
        assert_ne!(first.stream_identity, second.stream_identity);
    }

    #[test]
    fn cancellation_stops_before_the_next_indexed_replicate() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        let cancelled = Arc::new(AtomicBool::new(false));
        let cancellation_probe = Arc::clone(&cancelled);
        let progress_cancellation = Arc::clone(&cancelled);
        let result = run_pls_sample_size_power_v1(
            &recipe(),
            move || cancellation_probe.load(Ordering::Relaxed),
            move |progress| {
                if progress.phase == PlsPowerPhase::Simulating && progress.completed_replicates == 1
                {
                    progress_cancellation.store(true, Ordering::Relaxed);
                }
            },
        );

        assert_eq!(result, Err(PlsSampleSizePowerError::Cancelled));
    }

    #[test]
    fn strict_archive_rejects_changed_outcomes_and_decisions() {
        let input = recipe();
        let outcomes = input
            .sample_size_grid
            .iter()
            .flat_map(|sample_size| {
                (0..input.monte_carlo_replicates).map(|replicate_index| {
                    let rejected = if *sample_size == 50 {
                        replicate_index < 95
                    } else {
                        replicate_index < 90
                    };
                    PlsPowerReplicateOutcomeV1 {
                        sample_size: *sample_size,
                        replicate_index,
                        stream_identity: stream_identity(&input, *sample_size, replicate_index),
                        attempted: true,
                        successful: true,
                        converged: true,
                        target_estimate: Some(0.3),
                        p_value_two_sided: Some(if rejected { 0.01 } else { 0.2 }),
                        bootstrap_requested_replicates: None,
                        bootstrap_usable_replicates: None,
                        bootstrap_failed_replicates: None,
                        bootstrap_two_sided_exceedances: None,
                        rejected,
                        failure_code: None,
                        failure_message: None,
                    }
                })
            })
            .collect::<Vec<_>>();
        let rows = summarize_rows(&input, &outcomes);
        let result = PlsSampleSizePowerResultV1 {
            schema_version: 1,
            capability_id: PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID.into(),
            method_version: PLS_SAMPLE_SIZE_POWER_METHOD_VERSION.into(),
            recipe_digest: scientific_recipe_digest(&input).unwrap(),
            stream_domain: PLS_SAMPLE_SIZE_POWER_STREAM_DOMAIN.into(),
            failure_policy: PLS_SAMPLE_SIZE_POWER_FAILURE_POLICY.into(),
            interval_method: PLS_SAMPLE_SIZE_POWER_INTERVAL_METHOD.into(),
            inference_method: PLS_SAMPLE_SIZE_POWER_INFERENCE_METHOD.into(),
            pls_method_version: PLS_METHOD_VERSION.into(),
            resampling_method_version: RESAMPLING_METHOD_VERSION.into(),
            workload: input.validate().unwrap(),
            outcome_digest: digest_json(&outcomes).unwrap(),
            outcomes,
            decision: PlsPowerGridDecisionV1::Reached { sample_size: 50 },
            monotonicity_violations: 1,
            rows,
            warnings: expected_warnings(PLS_POWER_V1_IDENTITY, &input.validate().unwrap(), 1),
            exclusions: expected_exclusions(PLS_POWER_V1_IDENTITY),
        };
        let archive = PlsSampleSizePowerArchiveV1::seal(input, result).unwrap();
        archive.verify().unwrap();
        let mut tampered = archive.clone();
        tampered.result.rows[1].rejections -= 1;
        assert!(tampered.verify().is_err());
        let mut tampered = archive;
        tampered.result.decision = PlsPowerGridDecisionV1::NotReached;
        assert!(tampered.verify().is_err());
    }

    #[test]
    fn strict_archive_rejects_removed_nonmonotonic_warning_and_changed_outer_workers() {
        let input = recipe();
        let outcomes = input
            .sample_size_grid
            .iter()
            .flat_map(|sample_size| {
                (0..input.monte_carlo_replicates).map(|replicate_index| {
                    let rejected = if *sample_size == 50 {
                        replicate_index < 95
                    } else {
                        replicate_index < 90
                    };
                    PlsPowerReplicateOutcomeV1 {
                        sample_size: *sample_size,
                        replicate_index,
                        stream_identity: stream_identity(&input, *sample_size, replicate_index),
                        attempted: true,
                        successful: true,
                        converged: true,
                        target_estimate: Some(0.3),
                        p_value_two_sided: Some(if rejected { 0.01 } else { 0.2 }),
                        bootstrap_requested_replicates: None,
                        bootstrap_usable_replicates: None,
                        bootstrap_failed_replicates: None,
                        bootstrap_two_sided_exceedances: None,
                        rejected,
                        failure_code: None,
                        failure_message: None,
                    }
                })
            })
            .collect::<Vec<_>>();
        let rows = summarize_rows(&input, &outcomes);
        let workload = input.validate().unwrap();
        let result = PlsSampleSizePowerResultV1 {
            schema_version: 1,
            capability_id: PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID.into(),
            method_version: PLS_SAMPLE_SIZE_POWER_METHOD_VERSION.into(),
            recipe_digest: scientific_recipe_digest(&input).unwrap(),
            stream_domain: PLS_SAMPLE_SIZE_POWER_STREAM_DOMAIN.into(),
            failure_policy: PLS_SAMPLE_SIZE_POWER_FAILURE_POLICY.into(),
            interval_method: PLS_SAMPLE_SIZE_POWER_INTERVAL_METHOD.into(),
            inference_method: PLS_SAMPLE_SIZE_POWER_INFERENCE_METHOD.into(),
            pls_method_version: PLS_METHOD_VERSION.into(),
            resampling_method_version: RESAMPLING_METHOD_VERSION.into(),
            workload: workload.clone(),
            outcome_digest: digest_json(&outcomes).unwrap(),
            outcomes,
            decision: PlsPowerGridDecisionV1::Reached { sample_size: 50 },
            monotonicity_violations: 1,
            rows,
            warnings: expected_warnings(PLS_POWER_V1_IDENTITY, &workload, 1),
            exclusions: expected_exclusions(PLS_POWER_V1_IDENTITY),
        };
        let archive = PlsSampleSizePowerArchiveV1::seal(input, result).unwrap();

        let mut changed_warning = archive.clone();
        changed_warning.result.warnings.pop();
        changed_warning.checksum_sha256 =
            archive_checksum(&changed_warning.recipe, &changed_warning.result).unwrap();
        assert!(matches!(
            changed_warning.verify(),
            Err(PlsSampleSizePowerError::InconsistentResult(message))
                if message.contains("warnings")
        ));

        let mut changed_workers = archive;
        changed_workers.recipe.workers = 4;
        assert!(matches!(
            changed_workers.verify(),
            Err(PlsSampleSizePowerError::Tamper(_))
        ));

        let mut empty_failure = changed_workers;
        let outcome = &mut empty_failure.result.outcomes[0];
        outcome.successful = false;
        outcome.converged = false;
        outcome.target_estimate = None;
        outcome.p_value_two_sided = None;
        outcome.rejected = false;
        outcome.failure_code = Some(String::new());
        outcome.failure_message = Some("   ".into());
        empty_failure.result.outcome_digest = digest_json(&empty_failure.result.outcomes).unwrap();
        empty_failure.checksum_sha256 =
            archive_checksum(&empty_failure.recipe, &empty_failure.result).unwrap();
        assert!(matches!(
            empty_failure.verify(),
            Err(PlsSampleSizePowerError::InconsistentResult(message))
                if message.contains("indexed outcome contract")
        ));
    }

    #[test]
    fn worker_counts_preserve_results_and_multiworker_progress_is_monotonic() {
        use std::sync::{Arc, Mutex};

        let mut multiworker_recipe = recipe();
        multiworker_recipe.sample_size_grid = vec![30, 31];
        multiworker_recipe.workers = 4;
        let progress = Arc::new(Mutex::new(Vec::<PlsPowerProgress>::new()));
        let captured = Arc::clone(&progress);
        let multiworker = run_pls_sample_size_power_v1(
            &multiworker_recipe,
            || false,
            move |event| captured.lock().unwrap().push(event),
        )
        .unwrap();
        let progress = progress.lock().unwrap();
        let total = multiworker_recipe.validate().unwrap().planned_datasets;
        assert_eq!(progress.len(), total as usize + 1);
        assert!(
            progress[..total as usize]
                .iter()
                .enumerate()
                .all(|(index, event)| event.phase == PlsPowerPhase::Simulating
                    && event.completed_replicates == index as u64 + 1
                    && event.total_replicates == total)
        );
        assert_eq!(
            progress.last(),
            Some(&PlsPowerProgress {
                phase: PlsPowerPhase::Summarizing,
                completed_replicates: total,
                total_replicates: total,
            })
        );
        drop(progress);

        let mut single_worker_recipe = multiworker_recipe.clone();
        single_worker_recipe.workers = 1;
        let single_worker =
            run_pls_sample_size_power_v1(&single_worker_recipe, || false, |_| {}).unwrap();
        assert_eq!(multiworker, single_worker);
        assert_eq!(
            scientific_recipe_digest(&multiworker_recipe).unwrap(),
            scientific_recipe_digest(&single_worker_recipe).unwrap()
        );
    }
}
