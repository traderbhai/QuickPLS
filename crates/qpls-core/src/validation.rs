use crate::{
    AnalysisRecipe, HigherOrderMethod, InteractionMethod, MeasurementMode, MethodStatus,
    PROJECT_SCHEMA_VERSION, Preprocessing, WeightingScheme, method_status,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationIssue {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub subject: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IpmaTargetSelectionError {
    #[error("IPMA requires at least one endogenous target construct")]
    NoEndogenousTargets,
    #[error("IPMA target metadata keys ipma_targets and ipma.targets disagree")]
    ConflictingMetadata,
    #[error("duplicate IPMA target construct: {0}")]
    DuplicateTarget(String),
    #[error("unknown IPMA target construct: {0}")]
    UnknownTarget(String),
    #[error("IPMA target must be endogenous: {0}")]
    ExogenousTarget(String),
}

/// Resolves the bounded IPMA target list once for validation, estimation, and
/// persistence checks. Explicit targets retain their declared order; the
/// default is every endogenous construct in model order.
pub fn resolve_ipma_targets(
    recipe: &AnalysisRecipe,
) -> Result<Vec<String>, IpmaTargetSelectionError> {
    let parse = |value: &str| {
        value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    };
    let primary = recipe
        .metadata
        .get("ipma_targets")
        .map(|value| parse(value))
        .filter(|targets| !targets.is_empty());
    let alternate = recipe
        .metadata
        .get("ipma.targets")
        .map(|value| parse(value))
        .filter(|targets| !targets.is_empty());
    if primary
        .as_ref()
        .zip(alternate.as_ref())
        .is_some_and(|(left, right)| left != right)
    {
        return Err(IpmaTargetSelectionError::ConflictingMetadata);
    }

    let endogenous = recipe
        .model
        .constructs
        .iter()
        .filter(|construct| {
            recipe
                .model
                .paths
                .iter()
                .any(|path| path.target == construct.id)
        })
        .map(|construct| construct.id.clone())
        .collect::<Vec<_>>();
    if endogenous.is_empty() {
        return Err(IpmaTargetSelectionError::NoEndogenousTargets);
    }
    let endogenous_set = endogenous.iter().cloned().collect::<HashSet<_>>();
    let known = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.as_str())
        .collect::<HashSet<_>>();
    let targets = primary.or(alternate).unwrap_or(endogenous);
    let mut seen = HashSet::new();
    for target in &targets {
        if !seen.insert(target.as_str()) {
            return Err(IpmaTargetSelectionError::DuplicateTarget(target.clone()));
        }
        if !known.contains(target.as_str()) {
            return Err(IpmaTargetSelectionError::UnknownTarget(target.clone()));
        }
        if !endogenous_set.contains(target.as_str()) {
            return Err(IpmaTargetSelectionError::ExogenousTarget(target.clone()));
        }
    }
    Ok(targets)
}

/// Returns all direct and indirect structural predecessors of an IPMA target
/// in model order. The selected target and unrelated constructs are excluded.
pub fn ipma_predecessor_constructs(recipe: &AnalysisRecipe, target: &str) -> Vec<String> {
    let mut reaches_target = HashSet::from([target.to_owned()]);
    loop {
        let before = reaches_target.len();
        for path in &recipe.model.paths {
            if reaches_target.contains(path.target.as_str()) {
                reaches_target.insert(path.source.clone());
            }
        }
        if reaches_target.len() == before {
            break;
        }
    }
    recipe
        .model
        .constructs
        .iter()
        .filter(|construct| {
            construct.id != target && reaches_target.contains(construct.id.as_str())
        })
        .map(|construct| construct.id.clone())
        .collect()
}

pub fn validate_recipe(recipe: &AnalysisRecipe) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if recipe.schema_version > PROJECT_SCHEMA_VERSION {
        issues.push(issue(
            "schema.future",
            Severity::Error,
            "Recipe uses a newer schema version",
            None,
        ));
    }
    if !recipe.settings.tolerance.is_finite() || recipe.settings.tolerance <= 0.0 {
        issues.push(issue(
            "settings.tolerance",
            Severity::Error,
            "Tolerance must be finite and greater than zero",
            None,
        ));
    }
    if recipe.settings.max_iterations == 0 {
        issues.push(issue(
            "settings.iterations",
            Severity::Error,
            "Maximum iterations must be greater than zero",
            None,
        ));
    }
    if recipe.settings.workers == 0 || recipe.settings.workers > 64 {
        issues.push(issue(
            "settings.workers",
            Severity::Error,
            "Worker count must be between 1 and 64",
            None,
        ));
    }
    if recipe.settings.bootstrap_samples > 10_000 {
        issues.push(issue(
            "settings.bootstrap_samples",
            Severity::Error,
            "Bootstrap samples cannot exceed 10000",
            None,
        ));
    }
    if recipe.settings.studentized_inner_samples > 0
        && (!(99..=999).contains(&recipe.settings.studentized_inner_samples)
            || recipe.settings.studentized_inner_samples % 2 == 0
            || recipe.settings.bootstrap_samples < 999)
    {
        issues.push(issue(
            "settings.studentized_inner_samples",
            Severity::Error,
            "Studentized bootstrap requires an odd 99 to 999 inner samples and at least 999 primary bootstrap samples",
            None,
        ));
    }
    if recipe.settings.permutation_samples > 10_000
        || (recipe.settings.permutation_samples > 0 && recipe.settings.permutation_samples < 99)
    {
        issues.push(issue(
            "settings.permutation_samples",
            Severity::Error,
            "Permutation samples must be zero or between 99 and 10000",
            None,
        ));
    }
    if !recipe.settings.confidence_level.is_finite()
        || recipe.settings.confidence_level <= 0.0
        || recipe.settings.confidence_level >= 1.0
    {
        issues.push(issue(
            "settings.confidence_level",
            Severity::Error,
            "Confidence level must be finite and strictly between zero and one",
            None,
        ));
    }
    match method_status(recipe.settings.method.as_str()) {
        MethodStatus::Unsupported => issues.push(issue(
            "method.unsupported",
            Severity::Error,
            "Selected method has not passed its implementation gate",
            Some(recipe.settings.method.to_string()),
        )),
        MethodStatus::Experimental => issues.push(issue(
            "method.experimental",
            Severity::Warning,
            "Selected method is available only inside the documented QuickPLS v1.0.0 supported scope",
            Some(recipe.settings.method.to_string()),
        )),
        MethodStatus::Validated => {}
    }
    if recipe.settings.method == crate::AnalysisMethod::Plsc {
        if recipe.settings.weighting_scheme == crate::WeightingScheme::Pca {
            issues.push(issue(
                "plsc.pca_unsupported",
                Severity::Error,
                "PLSc requires path or factor weighting in the documented validated scope",
                None,
            ));
        }
        if recipe
            .model
            .constructs
            .iter()
            .any(|construct| construct.mode != MeasurementMode::Reflective)
        {
            issues.push(issue(
                "plsc.reflective_only",
                Severity::Error,
                "PLSc is limited to reflective constructs in the documented validated scope",
                None,
            ));
        }
        for construct in recipe
            .model
            .constructs
            .iter()
            .filter(|construct| construct.indicators.len() < 2)
        {
            issues.push(issue(
                "plsc.minimum_indicators",
                Severity::Error,
                "PLSc requires at least two indicators per construct in the documented validated scope",
                Some(construct.id.clone()),
            ));
        }
        if !recipe.model.interactions.is_empty() {
            issues.push(issue(
                "plsc.interactions_unsupported",
                Severity::Error,
                "PLSc does not yet support generated two-stage interaction constructs",
                None,
            ));
        }
        if !recipe.model.higher_order_constructs.is_empty() {
            issues.push(issue(
                "plsc.higher_order_unsupported",
                Severity::Error,
                "PLSc does not yet support higher-order construct expansion",
                None,
            ));
        }
        if recipe.settings.bootstrap_samples > 0
            || recipe.settings.studentized_inner_samples > 0
            || recipe.settings.permutation_samples > 0
        {
            issues.push(issue(
                "plsc.resampling_unsupported",
                Severity::Error,
                "PLSc bootstrap, studentized bootstrap, and permutation inference are outside the documented validated scope",
                None,
            ));
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::Wpls {
        if recipe.settings.weighting_scheme == crate::WeightingScheme::Pca {
            issues.push(issue(
                "wpls.pca_unsupported",
                Severity::Error,
                "WPLS requires path or factor weighting in the documented validated scope",
                None,
            ));
        }
        if recipe.settings.preprocessing != crate::Preprocessing::Standardized {
            issues.push(issue(
                "wpls.standardized_only",
                Severity::Error,
                "WPLS currently supports standardized preprocessing only",
                None,
            ));
        }
        match recipe.settings.case_weight_column.as_deref() {
            Some(column) if !column.trim().is_empty() => {}
            _ => issues.push(issue(
                "wpls.case_weight_required",
                Severity::Error,
                "WPLS requires settings.case_weight_column",
                None,
            )),
        }
        if recipe
            .model
            .constructs
            .iter()
            .any(|construct| construct.mode != MeasurementMode::Reflective)
        {
            issues.push(issue(
                "wpls.reflective_only",
                Severity::Error,
                "WPLS is limited to reflective constructs in the documented validated scope",
                None,
            ));
        }
        if !recipe.model.interactions.is_empty() {
            issues.push(issue(
                "wpls.interactions_unsupported",
                Severity::Error,
                "WPLS does not yet support generated two-stage interaction constructs",
                None,
            ));
        }
        if !recipe.model.higher_order_constructs.is_empty() {
            issues.push(issue(
                "wpls.higher_order_unsupported",
                Severity::Error,
                "WPLS does not yet support higher-order construct expansion",
                None,
            ));
        }
        if recipe.settings.bootstrap_samples > 0
            || recipe.settings.studentized_inner_samples > 0
            || recipe.settings.permutation_samples > 0
        {
            issues.push(issue(
                "wpls.resampling_unsupported",
                Severity::Error,
                "WPLS bootstrap, studentized bootstrap, and permutation inference are outside the documented validated scope",
                None,
            ));
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::Cca {
        if recipe.settings.weighting_scheme == crate::WeightingScheme::Pca {
            issues.push(issue(
                "cca.pca_unsupported",
                Severity::Error,
                "CCA composite residual diagnostics require path or factor weighting",
                None,
            ));
        }
        if recipe.settings.preprocessing != crate::Preprocessing::Standardized {
            issues.push(issue(
                "cca.standardized_required",
                Severity::Error,
                "CCA composite residual diagnostics require standardized preprocessing",
                None,
            ));
        }
        if recipe.settings.missing_data != crate::MissingDataPolicy::ListwiseDeletion {
            issues.push(issue(
                "cca.listwise_required",
                Severity::Error,
                "CCA composite residual diagnostics require listwise deletion",
                None,
            ));
        }
        if recipe.model.constructs.len() < 2 {
            issues.push(issue(
                "cca.constructs_required",
                Severity::Error,
                "CCA composite residual diagnostics require at least two constructs",
                None,
            ));
        }
        if recipe.model.paths.is_empty() {
            issues.push(issue(
                "cca.structural_path_required",
                Severity::Error,
                "CCA composite residual diagnostics require at least one structural path",
                None,
            ));
        }
        if recipe
            .model
            .constructs
            .iter()
            .any(|construct| construct.mode != MeasurementMode::Reflective)
        {
            issues.push(issue(
                "cca.reflective_only",
                Severity::Error,
                "CCA composite residual diagnostics are limited to reflective constructs",
                None,
            ));
        }
        for construct in recipe
            .model
            .constructs
            .iter()
            .filter(|construct| construct.indicators.is_empty())
        {
            issues.push(issue(
                "cca.observed_indicators_required",
                Severity::Error,
                "Every CCA construct requires at least one observed indicator",
                Some(construct.id.clone()),
            ));
        }
        if !recipe.model.interactions.is_empty() {
            issues.push(issue(
                "cca.interactions_unsupported",
                Severity::Error,
                "CCA does not yet support generated two-stage interaction constructs",
                None,
            ));
        }
        if !recipe.model.higher_order_constructs.is_empty() {
            issues.push(issue(
                "cca.higher_order_unsupported",
                Severity::Error,
                "CCA composite residual diagnostics do not support higher-order construct expansion",
                None,
            ));
        }
        if !recipe.model.controls.is_empty() {
            issues.push(issue(
                "cca.controls_unsupported",
                Severity::Error,
                "CCA composite residual diagnostics do not support control-path declarations",
                None,
            ));
        }
        if recipe.settings.case_weight_column.is_some() {
            issues.push(issue(
                "cca.case_weights_unsupported",
                Severity::Error,
                "CCA composite residual diagnostics do not support case weights",
                None,
            ));
        }
        if recipe.settings.bootstrap_samples > 0
            || recipe.settings.studentized_inner_samples > 0
            || recipe.settings.permutation_samples > 0
        {
            issues.push(issue(
                "cca.resampling_unsupported",
                Severity::Error,
                "CCA bootstrap, studentized bootstrap, and permutation inference are outside the bounded descriptive scope",
                None,
            ));
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::ModeratedMediation {
        if recipe.settings.weighting_scheme == crate::WeightingScheme::Pca {
            issues.push(issue(
                "moderated_mediation.pca_unsupported",
                Severity::Error,
                "Moderated mediation requires path or factor weighting in this experimental release",
                None,
            ));
        }
        if recipe.model.interactions.is_empty() {
            issues.push(issue(
                "moderated_mediation.interaction_required",
                Severity::Error,
                "Moderated mediation requires at least one interaction term in this experimental release",
                None,
            ));
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::CtaPls {
        if recipe.settings.weighting_scheme == crate::WeightingScheme::Pca {
            issues.push(issue(
                "cta_pls.pca_unsupported",
                Severity::Error,
                "CTA-PLS requires path or factor weighting in this experimental release",
                None,
            ));
        }
        if !recipe
            .model
            .constructs
            .iter()
            .any(|construct| construct.indicators.len() >= 4)
        {
            issues.push(issue(
                "cta_pls.tetrad_block_required",
                Severity::Error,
                "CTA-PLS requires at least one construct with four or more indicators",
                None,
            ));
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::Predict {
        if recipe.settings.case_weight_column.is_some() {
            issues.push(issue(
                "predict.case_weights_unsupported",
                Severity::Error,
                "PLSpredict indicator v2 does not support case weights",
                None,
            ));
        }
        if !recipe.model.interactions.is_empty() {
            issues.push(issue(
                "predict.interactions_unsupported",
                Severity::Error,
                "PLSpredict indicator v2 does not support generated interaction constructs",
                None,
            ));
        }
        if !recipe.model.higher_order_constructs.is_empty() {
            issues.push(issue(
                "predict.higher_order_unsupported",
                Severity::Error,
                "PLSpredict indicator v2 does not support higher-order construct expansion",
                None,
            ));
        }
        if recipe.model.paths.is_empty() {
            issues.push(issue(
                "predict.endogenous_required",
                Severity::Error,
                "PLSpredict indicator v2 requires at least one endogenous construct",
                None,
            ));
        }
        if recipe.settings.missing_data != crate::MissingDataPolicy::ListwiseDeletion {
            issues.push(issue(
                "predict.listwise_required",
                Severity::Error,
                "PLSpredict indicator v2 requires listwise deletion so every fold uses the same complete-case population",
                None,
            ));
        }
        if recipe.settings.bootstrap_samples > 0
            || recipe.settings.studentized_inner_samples > 0
            || recipe.settings.permutation_samples > 0
        {
            issues.push(issue(
                "predict.external_resampling_unsupported",
                Severity::Error,
                "PLSpredict indicator v2 owns its fixed seeded 10-fold by 10-repeat plan and does not accept bootstrap or permutation settings",
                None,
            ));
        }
        if (recipe.settings.confidence_level - 0.95).abs() > 1e-12 {
            issues.push(issue(
                "predict.confidence_fixed",
                Severity::Error,
                "PLSpredict indicator v2 uses a fixed one-sided 95% CVPAT confidence contract",
                Some(recipe.settings.confidence_level.to_string()),
            ));
        }
        for construct in recipe.model.constructs.iter().filter(|construct| {
            recipe
                .model
                .paths
                .iter()
                .any(|path| path.target == construct.id)
        }) {
            if construct.mode != crate::MeasurementMode::Reflective {
                issues.push(issue(
                    "predict.reflective_endogenous_required",
                    Severity::Error,
                    format!(
                        "PLSpredict indicator v2 requires endogenous construct '{}' to be reflective",
                        construct.id
                    ),
                    Some(construct.id.clone()),
                ));
            }
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::Mga {
        if recipe.settings.weighting_scheme != crate::WeightingScheme::Path {
            issues.push(issue(
                "mga.path_weighting_required",
                Severity::Error,
                "The validated MICOM and permutation-MGA v2 scope requires path weighting",
                None,
            ));
        }
        if recipe.settings.preprocessing != crate::Preprocessing::Standardized {
            issues.push(issue(
                "mga.standardized_required",
                Severity::Error,
                "The validated MICOM and permutation-MGA v2 scope requires standardized preprocessing",
                None,
            ));
        }
        if recipe.settings.missing_data != crate::MissingDataPolicy::ListwiseDeletion {
            issues.push(issue(
                "mga.listwise_required",
                Severity::Error,
                "The validated MICOM and permutation-MGA v2 scope requires listwise deletion",
                None,
            ));
        }
        if recipe.settings.case_weight_column.is_some() {
            issues.push(issue(
                "mga.case_weights_unsupported",
                Severity::Error,
                "Bounded MGA v1 does not yet support case weights",
                None,
            ));
        }
        if !recipe.model.interactions.is_empty() {
            issues.push(issue(
                "mga.interactions_unsupported",
                Severity::Error,
                "Bounded MGA v1 does not yet support generated interaction constructs",
                None,
            ));
        }
        if !recipe.model.higher_order_constructs.is_empty() {
            issues.push(issue(
                "mga.higher_order_unsupported",
                Severity::Error,
                "Bounded MGA v1 does not yet support higher-order construct expansion",
                None,
            ));
        }
        if recipe.model.paths.is_empty() {
            issues.push(issue(
                "mga.path_required",
                Severity::Error,
                "Bounded MGA v1 requires at least one structural path",
                None,
            ));
        }
        let group_column = recipe
            .metadata
            .get("mga_group_column")
            .or_else(|| recipe.metadata.get("mga.group_column"))
            .map(|value| value.trim())
            .filter(|value| !value.is_empty());
        if group_column.is_none() {
            issues.push(issue(
                "mga.group_column_required",
                Severity::Error,
                "Bounded MGA v1 requires metadata.mga_group_column naming a two-group column",
                None,
            ));
        }
        if group_column.is_some_and(|column| {
            recipe
                .model
                .constructs
                .iter()
                .flat_map(|construct| construct.indicators.iter())
                .any(|indicator| indicator == column)
        }) {
            issues.push(issue(
                "mga.group_column_is_indicator",
                Severity::Error,
                "The MGA grouping column cannot also be a model indicator",
                group_column.map(ToOwned::to_owned),
            ));
        }
        let group_a = recipe
            .metadata
            .get("mga_group_a")
            .or_else(|| recipe.metadata.get("mga.group_a"))
            .map(|value| value.trim())
            .filter(|value| !value.is_empty());
        let group_b = recipe
            .metadata
            .get("mga_group_b")
            .or_else(|| recipe.metadata.get("mga.group_b"))
            .map(|value| value.trim())
            .filter(|value| !value.is_empty());
        if group_a.is_none() {
            issues.push(issue(
                "mga.group_a_required",
                Severity::Error,
                "Bounded MGA v1 requires metadata.mga_group_a naming the selected Group A value",
                None,
            ));
        }
        if group_b.is_none() {
            issues.push(issue(
                "mga.group_b_required",
                Severity::Error,
                "Bounded MGA v1 requires metadata.mga_group_b naming the selected Group B value",
                None,
            ));
        }
        if group_a
            .zip(group_b)
            .is_some_and(|(left, right)| left == right)
        {
            issues.push(issue(
                "mga.groups_must_differ",
                Severity::Error,
                "Group A and Group B must select different observed values",
                None,
            ));
        }
        if recipe.settings.bootstrap_samples > 0
            || recipe.settings.studentized_inner_samples > 0
            || recipe.settings.permutation_samples > 0
        {
            issues.push(issue(
                "mga.generic_resampling_unsupported",
                Severity::Error,
                "Two-group MGA uses its dedicated permutation option; pooled bootstrap and permutation settings are not supported",
                None,
            ));
        }
        if let Some(methods) = metadata_list(recipe, "group_methods") {
            let normalized = methods
                .iter()
                .map(|method| method.to_ascii_lowercase())
                .collect::<Vec<_>>();
            let unique = normalized.iter().collect::<std::collections::HashSet<_>>();
            if normalized.len() != 2
                || unique.len() != 2
                || !normalized.iter().any(|method| method == "mga_permutation")
                || !normalized.iter().any(|method| method == "micom")
            {
                issues.push(issue(
                    "mga.group_methods_required",
                    Severity::Error,
                    "The current native group workflow requires exactly MICOM and two-group permutation MGA",
                    Some(methods.join(",")),
                ));
            }
            for method in &methods {
                if !method.eq_ignore_ascii_case("mga_permutation")
                    && !method.eq_ignore_ascii_case("micom")
                {
                    issues.push(issue(
                        "mga.group_method_unsupported",
                        Severity::Error,
                        format!("Unsupported MGA group method: {method}"),
                        Some(method.clone()),
                    ));
                }
            }
            match recipe
                .metadata
                .get("group_permutation_samples")
                .map(|value| value.trim().parse::<usize>())
            {
                Some(Ok(samples)) if (5_000..=10_000).contains(&samples) => {}
                _ => issues.push(issue(
                    "mga.permutation_samples",
                    Severity::Error,
                    "MICOM and permutation MGA require metadata.group_permutation_samples between 5000 and 10000",
                    None,
                )),
            }
            if !recipe
                .metadata
                .get("micom_configural_confirmed")
                .is_some_and(|value| value.eq_ignore_ascii_case("true"))
            {
                issues.push(issue(
                    "micom.configural_confirmation_required",
                    Severity::Error,
                    "MICOM requires explicit confirmation that indicator meaning, coding, treatment, model specification, and algorithm settings are equivalent across Group A and Group B",
                    None,
                ));
            }
        } else {
            issues.push(issue(
                "mga.group_methods_required",
                Severity::Error,
                "The current native group workflow requires exactly MICOM and two-group permutation MGA",
                None,
            ));
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::Ipma {
        if recipe.settings.weighting_scheme != crate::WeightingScheme::Path {
            issues.push(issue(
                "ipma.path_weighting_required",
                Severity::Error,
                "IPMA v1 is limited to path weighting in the documented validated scope",
                None,
            ));
        }
        if recipe.settings.preprocessing != crate::Preprocessing::Standardized {
            issues.push(issue(
                "ipma.standardized_required",
                Severity::Error,
                "IPMA v1 requires standardized indicator preprocessing",
                None,
            ));
        }
        if recipe.settings.missing_data != crate::MissingDataPolicy::ListwiseDeletion {
            issues.push(issue(
                "ipma.listwise_required",
                Severity::Error,
                "IPMA v1 requires listwise deletion before standardization",
                None,
            ));
        }
        if recipe.settings.case_weight_column.is_some() {
            issues.push(issue(
                "ipma.case_weights_unsupported",
                Severity::Error,
                "IPMA v1 does not yet support case weights",
                None,
            ));
        }
        if !recipe.model.interactions.is_empty() {
            issues.push(issue(
                "ipma.interactions_unsupported",
                Severity::Error,
                "IPMA v1 does not yet support generated interaction constructs",
                None,
            ));
        }
        if !recipe.model.higher_order_constructs.is_empty() {
            issues.push(issue(
                "ipma.higher_order_unsupported",
                Severity::Error,
                "IPMA v1 does not yet support higher-order construct expansion",
                None,
            ));
        }
        if recipe.settings.bootstrap_samples > 0
            || recipe.settings.studentized_inner_samples > 0
            || recipe.settings.permutation_samples > 0
        {
            issues.push(issue(
                "ipma.resampling_unsupported",
                Severity::Error,
                "IPMA v1 is a deterministic descriptive analysis; bootstrap and permutation inference are outside this contract",
                None,
            ));
        }
        if let Err(error) = resolve_ipma_targets(recipe) {
            let (code, subject) = match &error {
                IpmaTargetSelectionError::NoEndogenousTargets => ("ipma.target_required", None),
                IpmaTargetSelectionError::ConflictingMetadata => {
                    ("ipma.target_metadata_conflict", None)
                }
                IpmaTargetSelectionError::DuplicateTarget(target) => {
                    ("ipma.target_duplicate", Some(target.clone()))
                }
                IpmaTargetSelectionError::UnknownTarget(target) => {
                    ("ipma.target_unknown", Some(target.clone()))
                }
                IpmaTargetSelectionError::ExogenousTarget(target) => {
                    ("ipma.target_must_be_endogenous", Some(target.clone()))
                }
            };
            issues.push(issue(code, Severity::Error, error.to_string(), subject));
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::Cbsem {
        let model_type = recipe
            .metadata
            .get("cbsem_model_type")
            .map(|value| value.trim().to_ascii_lowercase())
            .unwrap_or_else(|| {
                if recipe.model.paths.is_empty() {
                    "cfa".into()
                } else {
                    "sem".into()
                }
            });
        if !matches!(model_type.as_str(), "cfa" | "sem") {
            issues.push(issue(
                "cbsem.model_type",
                Severity::Error,
                "CB-SEM/CFA model type must be cfa or sem",
                Some(model_type.clone()),
            ));
        }
        if model_type == "cfa" && !recipe.model.paths.is_empty() {
            issues.push(issue(
                "cbsem.cfa_paths_unsupported",
                Severity::Error,
                "Confirmatory factor analysis does not accept structural paths",
                None,
            ));
        }
        if model_type == "sem" && recipe.model.paths.is_empty() {
            issues.push(issue(
                "cbsem.sem_path_required",
                Severity::Error,
                "CB-SEM requires at least one recursive latent structural path",
                None,
            ));
        }
        if recipe.settings.weighting_scheme != WeightingScheme::Path {
            issues.push(issue(
                "cbsem.path_initialization_required",
                Severity::Error,
                "The validated CB-SEM/CFA scope uses path-weighted PLS initialization",
                None,
            ));
        }
        if recipe.settings.preprocessing != Preprocessing::Standardized {
            issues.push(issue(
                "cbsem.standardized_input_required",
                Severity::Error,
                "The validated CB-SEM/CFA scope uses listwise-standardized raw-data indicators",
                None,
            ));
        }
        if recipe.settings.missing_data != crate::MissingDataPolicy::ListwiseDeletion {
            issues.push(issue(
                "cbsem.listwise_required",
                Severity::Error,
                "The validated CB-SEM/CFA scope requires listwise deletion",
                None,
            ));
        }
        if recipe.settings.workers != 1 {
            issues.push(issue(
                "cbsem.workers_fixed",
                Severity::Error,
                "The bounded CB-SEM/CFA optimizer executes with one deterministic worker",
                Some(recipe.settings.workers.to_string()),
            ));
        }
        if recipe.settings.bootstrap_samples > 0
            || recipe.settings.studentized_inner_samples > 0
            || recipe.settings.permutation_samples > 0
        {
            issues.push(issue(
                "cbsem.resampling_unsupported",
                Severity::Error,
                "CB-SEM/CFA bootstrap and permutation inference remain outside the validated native single-group scope",
                None,
            ));
        }
        if recipe
            .metadata
            .get("cbsem_input")
            .is_none_or(|value| value.trim() != "raw")
        {
            issues.push(issue(
                "cbsem.raw_input_required",
                Severity::Error,
                "The validated CB-SEM/CFA scope requires raw case-level input",
                None,
            ));
        }
        if recipe
            .metadata
            .get("cbsem_estimator")
            .is_some_and(|value| !value.trim().eq_ignore_ascii_case("ml"))
        {
            issues.push(issue(
                "cbsem.ml_required",
                Severity::Error,
                "The validated CB-SEM/CFA scope uses maximum likelihood estimation",
                None,
            ));
        }
        if recipe
            .metadata
            .get("cbsem_mean_structure")
            .is_some_and(|value| !value.trim().eq_ignore_ascii_case("false"))
        {
            issues.push(issue(
                "cbsem.mean_structure_unsupported",
                Severity::Error,
                "Selectable CB-SEM/CFA mean structures remain outside the validated native scope",
                None,
            ));
        }
        if recipe.metadata.contains_key("cbsem_group_column")
            || recipe.metadata.contains_key("cbsem_invariance_steps")
        {
            issues.push(issue(
                "cbsem.multigroup_unsupported",
                Severity::Error,
                "CB-SEM/CFA multigroup and measurement-invariance workflows remain outside the validated native single-group scope",
                None,
            ));
        }
        if recipe
            .metadata
            .get("cbsem_bootstrap_samples")
            .and_then(|value| value.parse::<u32>().ok())
            .is_some_and(|samples| samples > 0)
        {
            issues.push(issue(
                "cbsem.bootstrap_unsupported",
                Severity::Error,
                "CB-SEM/CFA bootstrap remains outside the validated native single-group scope",
                None,
            ));
        }
        if recipe.settings.case_weight_column.is_some() {
            issues.push(issue(
                "cbsem.case_weights_unsupported",
                Severity::Error,
                "CB-SEM/CFA ML v1 does not yet support case weights",
                None,
            ));
        }
        if !recipe.model.interactions.is_empty() {
            issues.push(issue(
                "cbsem.interactions_unsupported",
                Severity::Error,
                "CB-SEM/CFA ML v1 does not yet support generated interaction constructs",
                None,
            ));
        }
        if !recipe.model.higher_order_constructs.is_empty() {
            issues.push(issue(
                "cbsem.higher_order_unsupported",
                Severity::Error,
                "CB-SEM/CFA ML v1 does not yet support higher-order construct expansion",
                None,
            ));
        }
        if !recipe.model.controls.is_empty() {
            issues.push(issue(
                "cbsem.controls_unsupported",
                Severity::Error,
                "The validated CB-SEM/CFA scope does not support typed control-path annotations",
                None,
            ));
        }
        if recipe
            .model
            .constructs
            .iter()
            .any(|construct| construct.mode != MeasurementMode::Reflective)
        {
            issues.push(issue(
                "cbsem.reflective_only",
                Severity::Error,
                "CB-SEM/CFA ML v1 is limited to reflective constructs",
                None,
            ));
        }
        for construct in &recipe.model.constructs {
            if construct.indicators.len() < 2 {
                issues.push(issue(
                    "cbsem.indicators_per_factor",
                    Severity::Error,
                    "CB-SEM/CFA ML v1 requires at least two observed indicators per latent factor",
                    Some(construct.id.clone()),
                ));
            }
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::Pca {
        let variables = metadata_list(recipe, "pca_variables")
            .or_else(|| metadata_list(recipe, "pca.variables"))
            .unwrap_or_default();
        if variables.len() < 2 {
            issues.push(issue(
                "pca.variables_required",
                Severity::Error,
                "Standalone PCA requires metadata.pca_variables with at least two numeric columns",
                None,
            ));
        }
        let unique_variables = variables.iter().collect::<HashSet<_>>();
        if unique_variables.len() != variables.len() {
            issues.push(issue(
                "pca.variables_unique",
                Severity::Error,
                "Standalone PCA requires distinct selected variables",
                None,
            ));
        }
        let component_rule = recipe
            .metadata
            .get("pca_component_rule")
            .map(String::as_str)
            .unwrap_or("kaiser");
        if !matches!(component_rule, "kaiser" | "fixed" | "variance_threshold") {
            issues.push(issue(
                "pca.component_rule",
                Severity::Error,
                "PCA component retention must be kaiser, fixed, or variance_threshold",
                Some(component_rule.to_owned()),
            ));
        }
        if component_rule == "fixed" {
            let components = recipe
                .metadata
                .get("pca_components")
                .and_then(|value| value.parse::<usize>().ok());
            if components
                .is_none_or(|components| components == 0 || components > variables.len().min(50))
            {
                issues.push(issue(
                    "pca.components",
                    Severity::Error,
                    "Fixed PCA retention requires 1 to min(selected variables, 50) components",
                    None,
                ));
            }
        }
        if component_rule == "variance_threshold" {
            let threshold = recipe
                .metadata
                .get("pca_variance_threshold")
                .and_then(|value| value.parse::<f64>().ok());
            if threshold.is_none_or(|threshold| {
                !threshold.is_finite() || !(0.01..=0.999).contains(&threshold)
            }) {
                issues.push(issue(
                    "pca.variance_threshold",
                    Severity::Error,
                    "Variance-threshold PCA requires pca_variance_threshold from 0.01 to 0.999",
                    None,
                ));
            }
        }
        if recipe.settings.weighting_scheme != crate::WeightingScheme::Path {
            issues.push(issue(
                "pca.weighting_sentinel",
                Severity::Error,
                "Standalone PCA uses path weighting only as a fixed non-SEM recipe sentinel",
                None,
            ));
        }
        if recipe.settings.preprocessing != crate::Preprocessing::Standardized {
            issues.push(issue(
                "pca.standardized_required",
                Severity::Error,
                "Standalone PCA requires standardized numeric variables and a correlation-matrix eigensystem",
                None,
            ));
        }
        if recipe.settings.missing_data != crate::MissingDataPolicy::ListwiseDeletion {
            issues.push(issue(
                "pca.listwise_required",
                Severity::Error,
                "Standalone PCA requires listwise deletion across selected variables",
                None,
            ));
        }
        if recipe.settings.case_weight_column.is_some() {
            issues.push(issue(
                "pca.case_weights_unsupported",
                Severity::Error,
                "Standalone PCA does not support case weights",
                None,
            ));
        }
        if recipe.settings.bootstrap_samples > 0
            || recipe.settings.studentized_inner_samples > 0
            || recipe.settings.permutation_samples > 0
        {
            issues.push(issue(
                "pca.resampling_unsupported",
                Severity::Error,
                "Standalone PCA does not support PLS bootstrap or permutation inference",
                None,
            ));
        }
        if !recipe.model.constructs.is_empty()
            || !recipe.model.paths.is_empty()
            || !recipe.model.controls.is_empty()
            || !recipe.model.higher_order_constructs.is_empty()
            || !recipe.model.interactions.is_empty()
        {
            issues.push(issue(
                "pca.empty_model_required",
                Severity::Error,
                "Standalone PCA consumes selected raw-data columns and requires an empty SEM model",
                None,
            ));
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::Regression {
        let regression_type = recipe
            .metadata
            .get("regression_type")
            .map(String::as_str)
            .unwrap_or("ols");
        if regression_type == "process"
            && recipe
                .metadata
                .get("process_model")
                .map(String::as_str)
                .unwrap_or("mediation")
                == "moderated_mediation"
        {
            issues.push(issue(
                "process.moderated_mediation.experimental",
                Severity::Warning,
                "PROCESS moderated mediation remains experimental; validated PROCESS v1.2.2 scope is limited to bounded mediation and moderation workflows",
                Some(regression_type.to_owned()),
            ));
        }
        if !matches!(regression_type, "ols" | "logistic" | "process") {
            issues.push(issue(
                "regression.type",
                Severity::Error,
                "regression_type must be ols, logistic, or process",
                Some(regression_type.to_owned()),
            ));
        }
        if !recipe.metadata.contains_key("regression_outcome") {
            issues.push(issue(
                "regression.outcome_required",
                Severity::Error,
                "Regression requires metadata.regression_outcome",
                None,
            ));
        }
        if metadata_list(recipe, "regression_predictors")
            .or_else(|| metadata_list(recipe, "regression.predictors"))
            .unwrap_or_default()
            .is_empty()
        {
            issues.push(issue(
                "regression.predictors_required",
                Severity::Error,
                "Regression requires metadata.regression_predictors",
                None,
            ));
        }
        if regression_type == "ols" {
            let outcome = recipe
                .metadata
                .get("regression_outcome")
                .map(|value| value.trim())
                .unwrap_or("");
            let predictors = metadata_list(recipe, "regression_predictors")
                .or_else(|| metadata_list(recipe, "regression.predictors"))
                .unwrap_or_default();
            let controls = metadata_list(recipe, "regression_controls")
                .or_else(|| metadata_list(recipe, "regression.controls"))
                .unwrap_or_default();
            let mut variables = Vec::with_capacity(1 + predictors.len() + controls.len());
            if !outcome.is_empty() {
                variables.push(outcome.to_owned());
            }
            variables.extend(predictors.iter().cloned());
            variables.extend(controls.iter().cloned());
            let unique = variables.iter().collect::<std::collections::BTreeSet<_>>();
            if unique.len() != variables.len() {
                issues.push(issue(
                    "regression.variables_distinct",
                    Severity::Error,
                    "OLS outcome, predictors, and controls must be distinct observed variables",
                    None,
                ));
            }
            if recipe.metadata.get("robust_se").map(|value| value.trim()) != Some("hc3") {
                issues.push(issue(
                    "regression.hc3_required",
                    Severity::Error,
                    "The validated native OLS scope requires HC3 heteroskedasticity-consistent standard errors",
                    None,
                ));
            }
            if recipe.settings.weighting_scheme != crate::WeightingScheme::Path {
                issues.push(issue(
                    "regression.weighting_sentinel",
                    Severity::Error,
                    "Standalone OLS uses path weighting only as a non-operative wire sentinel",
                    None,
                ));
            }
            if recipe.settings.preprocessing != crate::Preprocessing::Unstandardized {
                issues.push(issue(
                    "regression.raw_values_required",
                    Severity::Error,
                    "The validated native OLS scope fits unstandardized observed values",
                    None,
                ));
            }
            if recipe.settings.missing_data != crate::MissingDataPolicy::ListwiseDeletion {
                issues.push(issue(
                    "regression.listwise_required",
                    Severity::Error,
                    "The validated native OLS scope requires listwise deletion",
                    None,
                ));
            }
            if recipe.settings.case_weight_column.is_some() {
                issues.push(issue(
                    "regression.case_weights_unsupported",
                    Severity::Error,
                    "The validated native OLS scope does not support case weights",
                    None,
                ));
            }
            if recipe.settings.bootstrap_samples > 0
                || recipe.settings.studentized_inner_samples > 0
                || recipe.settings.permutation_samples > 0
            {
                issues.push(issue(
                    "regression.resampling_unsupported",
                    Severity::Error,
                    "The validated native OLS scope does not accept external resampling settings",
                    None,
                ));
            }
            if (recipe.settings.confidence_level - 0.95).abs() > 1e-12 {
                issues.push(issue(
                    "regression.confidence_fixed",
                    Severity::Error,
                    "The validated native OLS scope uses fixed two-sided 95% confidence intervals",
                    None,
                ));
            }
            if !recipe.model.constructs.is_empty()
                || !recipe.model.paths.is_empty()
                || !recipe.model.controls.is_empty()
                || !recipe.model.interactions.is_empty()
                || !recipe.model.higher_order_constructs.is_empty()
            {
                issues.push(issue(
                    "regression.empty_model_required",
                    Severity::Error,
                    "Standalone OLS consumes selected raw-data columns and requires an empty SEM model",
                    None,
                ));
            }
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::Nca {
        let nca_x = recipe
            .metadata
            .get("nca_x")
            .map(|value| value.trim())
            .unwrap_or("");
        let nca_y = recipe
            .metadata
            .get("nca_y")
            .map(|value| value.trim())
            .unwrap_or("");
        if nca_x.is_empty() || nca_y.is_empty() {
            issues.push(issue(
                "nca.variables_required",
                Severity::Error,
                "NCA requires metadata.nca_x and metadata.nca_y",
                None,
            ));
        }
        if !nca_x.is_empty() && nca_x == nca_y {
            issues.push(issue(
                "nca.variables_distinct",
                Severity::Error,
                "NCA requires different X and Y variables",
                Some(nca_x.to_owned()),
            ));
        }
        let ceiling = recipe
            .metadata
            .get("nca_ceiling")
            .map(|value| value.trim())
            .unwrap_or("both");
        if !matches!(ceiling, "ce_fdh" | "cr_fdh" | "both") {
            issues.push(issue(
                "nca.ceiling",
                Severity::Error,
                "NCA ceiling must be ce_fdh, cr_fdh, or both",
                Some(ceiling.to_owned()),
            ));
        }
        let permutation_samples = recipe
            .metadata
            .get("nca_permutation_samples")
            .map(|value| value.trim())
            .unwrap_or("999");
        if permutation_samples
            .parse::<usize>()
            .ok()
            .is_none_or(|samples| !(1..=10_000).contains(&samples))
        {
            issues.push(issue(
                "nca.permutation_samples",
                Severity::Error,
                "NCA requires 1 to 10,000 permutation samples",
                Some(permutation_samples.to_owned()),
            ));
        }
        if recipe.settings.preprocessing != crate::Preprocessing::Unstandardized {
            issues.push(issue(
                "nca.raw_values_required",
                Severity::Error,
                "NCA v2 requires unstandardized raw X and Y values",
                None,
            ));
        }
        if recipe.settings.weighting_scheme != crate::WeightingScheme::Path {
            issues.push(issue(
                "nca.weighting_sentinel",
                Severity::Error,
                "NCA v2 uses path weighting as the non-SEM settings sentinel",
                None,
            ));
        }
        if recipe.settings.case_weight_column.is_some() {
            issues.push(issue(
                "nca.case_weights_unsupported",
                Severity::Error,
                "NCA v2 does not support case weights",
                None,
            ));
        }
        if recipe.settings.bootstrap_samples > 0
            || recipe.settings.studentized_inner_samples > 0
            || recipe.settings.permutation_samples > 0
        {
            issues.push(issue(
                "nca.external_resampling_unsupported",
                Severity::Error,
                "NCA v2 uses only its dedicated metadata permutation plan",
                None,
            ));
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::Gsca {
        if recipe.settings.weighting_scheme != crate::WeightingScheme::Path {
            issues.push(issue(
                "gsca.weighting_sentinel",
                Severity::Error,
                "GSCA ALS v2 uses the path-weighting wire value only as a fixed non-PLS settings sentinel",
                None,
            ));
        }
        if recipe.settings.preprocessing != crate::Preprocessing::Standardized {
            issues.push(issue(
                "gsca.standardization_required",
                Severity::Error,
                "GSCA ALS v2 requires standardized raw indicators",
                None,
            ));
        }
        if recipe.settings.missing_data != crate::MissingDataPolicy::ListwiseDeletion {
            issues.push(issue(
                "gsca.listwise_required",
                Severity::Error,
                "GSCA ALS v2 requires listwise deletion",
                None,
            ));
        }
        if recipe.settings.case_weight_column.is_some() {
            issues.push(issue(
                "gsca.case_weights_unsupported",
                Severity::Error,
                "GSCA ALS v2 does not support case weights",
                None,
            ));
        }
        if recipe.settings.bootstrap_samples > 0
            || recipe.settings.studentized_inner_samples > 0
            || recipe.settings.permutation_samples > 0
        {
            issues.push(issue(
                "gsca.resampling_unsupported",
                Severity::Error,
                "GSCA ALS v2 does not expose inference until a method-specific resampling contract is validated",
                None,
            ));
        }
        if recipe.settings.workers != 1 {
            issues.push(issue(
                "gsca.single_worker_required",
                Severity::Error,
                "GSCA ALS v2 uses deterministic single-worker estimation",
                None,
            ));
        }
        if recipe.settings.max_iterations != 3_000 {
            issues.push(issue(
                "gsca.max_iterations_fixed",
                Severity::Error,
                "GSCA ALS v2 uses a fixed maximum of 3,000 iterations",
                None,
            ));
        }
        if (recipe.settings.tolerance - 1e-7).abs() > f64::EPSILON {
            issues.push(issue(
                "gsca.stop_criterion_fixed",
                Severity::Error,
                "GSCA ALS v2 uses a fixed relative objective stop criterion of 1e-7",
                None,
            ));
        }
        if !recipe.model.controls.is_empty() {
            issues.push(issue(
                "gsca.controls_unsupported",
                Severity::Error,
                "GSCA ALS v2 does not support control annotations",
                None,
            ));
        }
        if !recipe.model.interactions.is_empty() {
            issues.push(issue(
                "gsca.interactions_unsupported",
                Severity::Error,
                "GSCA ALS v2 does not support generated interaction constructs",
                None,
            ));
        }
        if !recipe.model.higher_order_constructs.is_empty() {
            issues.push(issue(
                "gsca.higher_order_unsupported",
                Severity::Error,
                "GSCA ALS v2 does not support higher-order construct expansion",
                None,
            ));
        }
        if recipe.model.constructs.len() < 2 {
            issues.push(issue(
                "gsca.constructs_required",
                Severity::Error,
                "GSCA ALS v2 requires at least two constructs",
                None,
            ));
        }
        if recipe.model.paths.is_empty() {
            issues.push(issue(
                "gsca.paths_required",
                Severity::Error,
                "GSCA ALS v2 requires at least one recursive structural path",
                None,
            ));
        }
        let connected = recipe
            .model
            .paths
            .iter()
            .flat_map(|path| [path.source.as_str(), path.target.as_str()])
            .collect::<HashSet<_>>();
        for construct in &recipe.model.constructs {
            if !connected.contains(construct.id.as_str()) {
                issues.push(issue(
                    "gsca.isolated_construct",
                    Severity::Error,
                    "GSCA ALS v2 does not support isolated constructs",
                    Some(construct.id.clone()),
                ));
            }
        }
    }

    let product_constructs = recipe
        .model
        .interactions
        .iter()
        .map(|interaction| interaction.product_construct.as_str())
        .collect::<HashSet<_>>();
    let higher_order_constructs = recipe
        .model
        .higher_order_constructs
        .iter()
        .map(|higher_order| higher_order.id.as_str())
        .collect::<HashSet<_>>();
    let mut construct_ids = HashSet::new();
    let mut indicator_owner: HashMap<&str, &str> = HashMap::new();
    for construct in &recipe.model.constructs {
        if !construct_ids.insert(construct.id.as_str()) {
            issues.push(issue(
                "construct.duplicate_id",
                Severity::Error,
                "Construct identifier must be unique",
                Some(construct.id.clone()),
            ));
        }
        if construct.name.trim().is_empty() {
            issues.push(issue(
                "construct.name",
                Severity::Error,
                "Construct name cannot be empty",
                Some(construct.id.clone()),
            ));
        }
        if construct.indicators.is_empty()
            && !product_constructs.contains(construct.id.as_str())
            && !higher_order_constructs.contains(construct.id.as_str())
        {
            issues.push(issue(
                "construct.indicators",
                Severity::Error,
                "Construct requires at least one indicator",
                Some(construct.id.clone()),
            ));
        } else if construct.indicators.is_empty()
            && product_constructs.contains(construct.id.as_str())
        {
            issues.push(issue(
                "interaction.product_indicator.generated",
                Severity::Warning,
                "Two-stage interaction product indicators are generated from stage-1 construct scores",
                Some(construct.id.clone()),
            ));
        } else if construct.indicators.is_empty()
            && higher_order_constructs.contains(construct.id.as_str())
        {
            issues.push(issue(
                "higher_order.indicators.generated",
                Severity::Warning,
                "Higher-order construct indicators are generated from lower-order components for supported HOC methods",
                Some(construct.id.clone()),
            ));
        }
        if construct.mode == MeasurementMode::Reflective && construct.indicators.len() == 1 {
            issues.push(issue(
                "construct.single_item",
                Severity::Warning,
                "Single-item reflective construct requires explicit justification",
                Some(construct.id.clone()),
            ));
        }
        for indicator in &construct.indicators {
            if let Some(owner) = indicator_owner.insert(indicator, &construct.id) {
                issues.push(issue(
                    "indicator.duplicate",
                    Severity::Error,
                    format!(
                        "Indicator {indicator} is assigned to both {owner} and {}",
                        construct.id
                    ),
                    Some(indicator.clone()),
                ));
            }
        }
    }
    let mut structural_paths = HashSet::new();
    for path in &recipe.model.paths {
        if path.source == path.target {
            issues.push(issue(
                "path.self",
                Severity::Error,
                "Structural paths cannot target the same construct",
                Some(path.source.clone()),
            ));
        }
        if !construct_ids.contains(path.source.as_str())
            || !construct_ids.contains(path.target.as_str())
        {
            issues.push(issue(
                "path.unknown_construct",
                Severity::Error,
                "Structural path references an unknown construct",
                Some(format!("{} -> {}", path.source, path.target)),
            ));
        }
        if !structural_paths.insert((path.source.as_str(), path.target.as_str())) {
            issues.push(issue(
                "path.duplicate",
                Severity::Error,
                "Structural path is duplicated",
                Some(format!("{} -> {}", path.source, path.target)),
            ));
        }
    }
    let mut control_paths = HashSet::new();
    for control in &recipe.model.controls {
        if control.source == control.target {
            issues.push(issue(
                "control.self",
                Severity::Error,
                "Control paths cannot target the same construct",
                Some(control.source.clone()),
            ));
        }
        if !construct_ids.contains(control.source.as_str())
            || !construct_ids.contains(control.target.as_str())
        {
            issues.push(issue(
                "control.unknown_construct",
                Severity::Error,
                "Control path references an unknown construct",
                Some(format!("{} -> {}", control.source, control.target)),
            ));
        }
        if !control_paths.insert((control.source.as_str(), control.target.as_str())) {
            issues.push(issue(
                "control.duplicate",
                Severity::Error,
                "Control path is duplicated",
                Some(format!("{} -> {}", control.source, control.target)),
            ));
        }
        if !structural_paths.contains(&(control.source.as_str(), control.target.as_str())) {
            issues.push(issue(
                "control.missing_structural_path",
                Severity::Error,
                "Control declaration must correspond to an existing structural path",
                Some(format!("{} -> {}", control.source, control.target)),
            ));
        }
        issues.push(issue(
            "method.controls.experimental",
            Severity::Warning,
            "Control-variable semantics are validated only inside the documented QuickPLS v1.0.0 supported scope",
            Some(format!("{} -> {}", control.source, control.target)),
        ));
    }
    let mut higher_order_ids = HashSet::new();
    let construct_indicator_counts = recipe
        .model
        .constructs
        .iter()
        .map(|construct| (construct.id.as_str(), construct.indicators.len()))
        .collect::<HashMap<_, _>>();
    for higher_order in &recipe.model.higher_order_constructs {
        if !higher_order_ids.insert(higher_order.id.as_str()) {
            issues.push(issue(
                "higher_order.duplicate_id",
                Severity::Error,
                "Higher-order construct identifier must be unique",
                Some(higher_order.id.clone()),
            ));
        }
        if !construct_ids.contains(higher_order.id.as_str()) {
            issues.push(issue(
                "higher_order.unknown_construct",
                Severity::Error,
                "Higher-order construct references an unknown construct",
                Some(higher_order.id.clone()),
            ));
        }
        if higher_order.components.len() < 2 {
            issues.push(issue(
                "higher_order.components",
                Severity::Error,
                "Higher-order constructs require at least two lower-order components",
                Some(higher_order.id.clone()),
            ));
        }
        let mut components = HashSet::new();
        for component in &higher_order.components {
            if component == &higher_order.id {
                issues.push(issue(
                    "higher_order.self_component",
                    Severity::Error,
                    "Higher-order constructs cannot include themselves as components",
                    Some(higher_order.id.clone()),
                ));
            }
            if !construct_ids.contains(component.as_str()) {
                issues.push(issue(
                    "higher_order.unknown_component",
                    Severity::Error,
                    "Higher-order construct component references an unknown construct",
                    Some(format!("{}:{component}", higher_order.id)),
                ));
            }
            if !components.insert(component.as_str()) {
                issues.push(issue(
                    "higher_order.duplicate_component",
                    Severity::Error,
                    "Higher-order construct components must be unique",
                    Some(format!("{}:{component}", higher_order.id)),
                ));
            }
        }
        if higher_order.method == HigherOrderMethod::Hybrid {
            for component in &higher_order.components {
                if construct_indicator_counts
                    .get(component.as_str())
                    .is_some_and(|count| *count < 2)
                {
                    issues.push(issue(
                        "higher_order.hybrid_component_indicators",
                        Severity::Error,
                        "Hybrid higher-order constructs require at least two indicators per component so indicators can be split between lower-order and higher-order blocks",
                        Some(format!("{}:{component}", higher_order.id)),
                    ));
                }
            }
        }
        match higher_order.method {
            HigherOrderMethod::RepeatedIndicators
            | HigherOrderMethod::TwoStage
            | HigherOrderMethod::Hybrid => issues.push(issue(
                "method.higher_order.experimental",
                Severity::Warning,
                "Higher-order construct semantics are validated only inside the documented QuickPLS v1.0.0 supported scope",
                Some(higher_order.id.clone()),
            )),
        }
    }
    let mut interaction_ids = HashSet::new();
    for interaction in &recipe.model.interactions {
        if !interaction_ids.insert(interaction.id.as_str()) {
            issues.push(issue(
                "interaction.duplicate_id",
                Severity::Error,
                "Interaction identifier must be unique",
                Some(interaction.id.clone()),
            ));
        }
        for (role, construct) in [
            ("predictor", &interaction.predictor),
            ("moderator", &interaction.moderator),
            ("product", &interaction.product_construct),
            ("outcome", &interaction.outcome),
        ] {
            if !construct_ids.contains(construct.as_str()) {
                issues.push(issue(
                    "interaction.unknown_construct",
                    Severity::Error,
                    format!("Interaction {role} references an unknown construct"),
                    Some(format!("{}:{construct}", interaction.id)),
                ));
            }
        }
        if interaction.predictor == interaction.moderator
            || interaction.product_construct == interaction.predictor
            || interaction.product_construct == interaction.moderator
            || interaction.product_construct == interaction.outcome
        {
            issues.push(issue(
                "interaction.invalid_roles",
                Severity::Error,
                "Interaction predictor, moderator, product construct, and outcome must be distinct where required",
                Some(interaction.id.clone()),
            ));
        }
        if !structural_paths.contains(&(
            interaction.product_construct.as_str(),
            interaction.outcome.as_str(),
        )) {
            issues.push(issue(
                "interaction.missing_product_path",
                Severity::Error,
                "Interaction product construct must have a structural path to the moderated outcome",
                Some(format!(
                    "{} -> {}",
                    interaction.product_construct, interaction.outcome
                )),
            ));
        }
        match interaction.method {
            InteractionMethod::TwoStageProductScore => issues.push(issue(
                "method.moderation.experimental",
                Severity::Warning,
                "Two-stage moderation is validated only inside the documented QuickPLS v1.0.0 supported scope",
                Some(interaction.id.clone()),
            )),
        }
    }
    if contains_directed_cycle(recipe) {
        issues.push(issue(
            "path.cycle",
            Severity::Error,
            "Structural model contains a directed cycle",
            None,
        ));
    }
    issues
}

fn contains_directed_cycle(recipe: &AnalysisRecipe) -> bool {
    let construct_ids = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.as_str())
        .collect::<HashSet<_>>();
    let mut indegree = construct_ids
        .iter()
        .map(|construct| (*construct, 0usize))
        .collect::<HashMap<_, _>>();
    let mut adjacency = HashMap::<&str, Vec<&str>>::new();
    for path in &recipe.model.paths {
        if path.source != path.target
            && construct_ids.contains(path.source.as_str())
            && construct_ids.contains(path.target.as_str())
        {
            adjacency
                .entry(path.source.as_str())
                .or_default()
                .push(path.target.as_str());
            *indegree.get_mut(path.target.as_str()).unwrap() += 1;
        }
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(construct, degree)| (*degree == 0).then_some(*construct))
        .collect::<Vec<_>>();
    let mut visited = 0;
    while let Some(construct) = ready.pop() {
        visited += 1;
        for target in adjacency.get(construct).into_iter().flatten() {
            let degree = indegree.get_mut(target).unwrap();
            *degree -= 1;
            if *degree == 0 {
                ready.push(target);
            }
        }
    }
    visited != construct_ids.len()
}

fn metadata_list(recipe: &AnalysisRecipe, key: &str) -> Option<Vec<String>> {
    recipe.metadata.get(key).map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    })
}

fn issue(
    code: &'static str,
    severity: Severity,
    message: impl Into<String>,
    subject: Option<String>,
) -> ValidationIssue {
    ValidationIssue {
        code,
        severity,
        message: message.into(),
        subject,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AnalysisSettings, Construct, ControlPath, HigherOrderConstruct, InteractionMethod,
        InteractionTerm, ModelSpec, StructuralPath,
    };
    use chrono::Utc;
    use std::collections::BTreeMap;
    use uuid::Uuid;

    fn valid_recipe() -> AnalysisRecipe {
        AnalysisRecipe {
            schema_version: 1,
            id: Uuid::nil(),
            created_at: Utc::now(),
            dataset_fingerprint: "abc".into(),
            model: ModelSpec {
                id: Uuid::nil(),
                name: "Fixture".into(),
                constructs: vec![
                    Construct {
                        id: "x".into(),
                        name: "Predictor".into(),
                        short_name: "X".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["x1".into(), "x2".into()],
                    },
                    Construct {
                        id: "y".into(),
                        name: "Outcome".into(),
                        short_name: "Y".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["y1".into(), "y2".into()],
                    },
                ],
                paths: vec![StructuralPath {
                    source: "x".into(),
                    target: "y".into(),
                }],
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            settings: AnalysisSettings::default(),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn implemented_pls_recipe_is_validated_for_base_estimation() {
        let issues = validate_recipe(&valid_recipe());
        assert!(issues.is_empty());
        assert_eq!(method_status("pls_pm"), MethodStatus::Validated);
    }

    #[test]
    fn cca_bounded_scope_accepts_standardized_reflective_path_and_factor_models() {
        for weighting_scheme in [crate::WeightingScheme::Path, crate::WeightingScheme::Factor] {
            let mut recipe = valid_recipe();
            recipe.settings.method = crate::AnalysisMethod::Cca;
            recipe.settings.weighting_scheme = weighting_scheme;
            recipe.model.constructs[0].indicators.truncate(1);

            assert!(
                validate_recipe(&recipe)
                    .iter()
                    .all(|item| item.severity != Severity::Error),
                "a bounded CCA recipe may contain a single observed indicator with a warning"
            );
        }
    }

    #[test]
    fn cca_bounded_scope_rejects_unsupported_settings_and_model_shapes() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Cca;
        recipe.settings.weighting_scheme = crate::WeightingScheme::Pca;
        recipe.settings.preprocessing = crate::Preprocessing::MeanCentered;
        recipe.settings.case_weight_column = Some("case_weight".into());
        recipe.settings.bootstrap_samples = 999;
        recipe.settings.studentized_inner_samples = 99;
        recipe.settings.permutation_samples = 99;
        recipe.model.constructs.truncate(1);
        recipe.model.constructs[0].mode = MeasurementMode::Formative;
        recipe.model.constructs[0].indicators.clear();
        recipe.model.paths.clear();
        recipe.model.controls.push(ControlPath {
            source: "x".into(),
            target: "y".into(),
            label: None,
        });
        recipe.model.interactions.push(InteractionTerm {
            id: "interaction".into(),
            predictor: "x".into(),
            moderator: "x".into(),
            product_construct: "product".into(),
            outcome: "y".into(),
            method: InteractionMethod::TwoStageProductScore,
        });
        recipe
            .model
            .higher_order_constructs
            .push(HigherOrderConstruct {
                id: "x".into(),
                components: vec!["x".into(), "y".into()],
                method: HigherOrderMethod::RepeatedIndicators,
                stage_one_recipe: None,
            });

        let issues = validate_recipe(&recipe);
        for expected in [
            "cca.pca_unsupported",
            "cca.standardized_required",
            "cca.constructs_required",
            "cca.structural_path_required",
            "cca.reflective_only",
            "cca.observed_indicators_required",
            "cca.interactions_unsupported",
            "cca.higher_order_unsupported",
            "cca.controls_unsupported",
            "cca.case_weights_unsupported",
            "cca.resampling_unsupported",
        ] {
            assert!(
                issues
                    .iter()
                    .any(|item| item.code == expected && item.severity == Severity::Error),
                "missing expected CCA validation issue {expected}: {issues:?}"
            );
        }
        assert!(
            !issues
                .iter()
                .any(|item| item.code == "cca.listwise_required"),
            "the only currently representable missing-data policy is listwise deletion"
        );
    }

    #[test]
    fn gsca_als_v2_accepts_the_bounded_recursive_component_model() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Gsca;
        recipe.settings.workers = 1;
        recipe.settings.max_iterations = 3_000;
        recipe.settings.tolerance = 1e-7;
        recipe.model.constructs[1].mode = MeasurementMode::Formative;

        let issues = validate_recipe(&recipe);
        assert!(
            issues.iter().all(|issue| issue.severity != Severity::Error),
            "bounded GSCA ALS v2 recipe should be valid: {issues:?}"
        );
        assert!(!issues.iter().any(|issue| issue.code == "gsca.experimental"));
    }

    #[test]
    fn gsca_als_v2_rejects_pls_settings_and_unsupported_model_shapes() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Gsca;
        recipe.settings.weighting_scheme = crate::WeightingScheme::Factor;
        recipe.settings.preprocessing = crate::Preprocessing::MeanCentered;
        recipe.settings.case_weight_column = Some("case_weight".into());
        recipe.settings.bootstrap_samples = 999;
        recipe.settings.permutation_samples = 99;
        recipe.settings.workers = 2;
        recipe.settings.max_iterations = 500;
        recipe.settings.tolerance = 1e-5;
        recipe.model.paths.clear();
        recipe.model.controls.push(ControlPath {
            source: "x".into(),
            target: "y".into(),
            label: None,
        });
        recipe.model.interactions.push(InteractionTerm {
            id: "interaction".into(),
            predictor: "x".into(),
            moderator: "x".into(),
            product_construct: "product".into(),
            outcome: "y".into(),
            method: InteractionMethod::TwoStageProductScore,
        });

        let issues = validate_recipe(&recipe);
        for expected in [
            "gsca.weighting_sentinel",
            "gsca.standardization_required",
            "gsca.case_weights_unsupported",
            "gsca.resampling_unsupported",
            "gsca.single_worker_required",
            "gsca.max_iterations_fixed",
            "gsca.stop_criterion_fixed",
            "gsca.controls_unsupported",
            "gsca.interactions_unsupported",
            "gsca.paths_required",
            "gsca.isolated_construct",
        ] {
            assert!(
                issues
                    .iter()
                    .any(|issue| issue.code == expected && issue.severity == Severity::Error),
                "missing expected GSCA validation issue {expected}: {issues:?}"
            );
        }
    }

    #[test]
    fn plsc_validated_scope_requires_two_reflective_indicators_per_construct() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Plsc;
        assert!(validate_recipe(&recipe).is_empty());

        recipe.model.constructs[0].indicators.pop();
        let issues = validate_recipe(&recipe);
        assert!(issues.iter().any(|item| {
            item.code == "plsc.minimum_indicators"
                && item.severity == Severity::Error
                && item.subject.as_deref() == Some("x")
        }));

        recipe.model.constructs[0].mode = MeasurementMode::Formative;
        assert!(validate_recipe(&recipe).iter().any(|item| {
            item.code == "plsc.reflective_only" && item.severity == Severity::Error
        }));
    }

    #[test]
    fn plsc_and_wpls_validated_scopes_reject_generated_constructs() {
        let mut higher_order = valid_recipe();
        higher_order.settings.method = crate::AnalysisMethod::Plsc;
        higher_order.model.constructs.push(Construct {
            id: "z".into(),
            name: "Second Component".into(),
            short_name: "Z".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["z1".into(), "z2".into()],
        });
        higher_order
            .model
            .higher_order_constructs
            .push(HigherOrderConstruct {
                id: "y".into(),
                components: vec!["x".into(), "z".into()],
                method: HigherOrderMethod::RepeatedIndicators,
                stage_one_recipe: None,
            });
        assert!(validate_recipe(&higher_order).iter().any(|item| {
            item.code == "plsc.higher_order_unsupported" && item.severity == Severity::Error
        }));
        higher_order.settings.method = crate::AnalysisMethod::Wpls;
        higher_order.settings.case_weight_column = Some("case_wt".into());
        assert!(validate_recipe(&higher_order).iter().any(|item| {
            item.code == "wpls.higher_order_unsupported" && item.severity == Severity::Error
        }));

        let mut interaction = valid_recipe();
        interaction.settings.method = crate::AnalysisMethod::Plsc;
        interaction.model.constructs.extend([
            Construct {
                id: "m".into(),
                name: "Moderator".into(),
                short_name: "M".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["m1".into(), "m2".into()],
            },
            Construct {
                id: "xm".into(),
                name: "X x moderator".into(),
                short_name: "XM".into(),
                mode: MeasurementMode::Formative,
                indicators: Vec::new(),
            },
        ]);
        interaction.model.paths.push(StructuralPath {
            source: "xm".into(),
            target: "y".into(),
        });
        interaction.model.interactions.push(InteractionTerm {
            id: "x_by_m_to_y".into(),
            predictor: "x".into(),
            moderator: "m".into(),
            product_construct: "xm".into(),
            outcome: "y".into(),
            method: InteractionMethod::TwoStageProductScore,
        });
        assert!(validate_recipe(&interaction).iter().any(|item| {
            item.code == "plsc.interactions_unsupported" && item.severity == Severity::Error
        }));
        interaction.settings.method = crate::AnalysisMethod::Wpls;
        interaction.settings.case_weight_column = Some("case_wt".into());
        assert!(validate_recipe(&interaction).iter().any(|item| {
            item.code == "wpls.interactions_unsupported" && item.severity == Severity::Error
        }));
    }

    #[test]
    fn plsc_and_wpls_validated_scopes_reject_all_resampling_settings() {
        let mut wpls = valid_recipe();
        wpls.settings.method = crate::AnalysisMethod::Wpls;
        wpls.settings.case_weight_column = Some("case_wt".into());
        assert!(validate_recipe(&wpls).is_empty());

        for method in [crate::AnalysisMethod::Plsc, crate::AnalysisMethod::Wpls] {
            for (bootstrap, studentized, permutation) in [(99, 0, 0), (999, 99, 0), (0, 0, 99)] {
                let mut recipe = valid_recipe();
                recipe.settings.method = method;
                recipe.settings.case_weight_column =
                    (method == crate::AnalysisMethod::Wpls).then(|| "case_wt".into());
                recipe.settings.bootstrap_samples = bootstrap;
                recipe.settings.studentized_inner_samples = studentized;
                recipe.settings.permutation_samples = permutation;
                let expected_code = if method == crate::AnalysisMethod::Plsc {
                    "plsc.resampling_unsupported"
                } else {
                    "wpls.resampling_unsupported"
                };
                assert!(validate_recipe(&recipe).iter().any(|item| {
                    item.code == expected_code && item.severity == Severity::Error
                }));
            }
        }
    }

    #[test]
    fn plspredict_indicator_v2_is_validated_and_rejects_unsupported_shapes() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Predict;
        let issues = validate_recipe(&recipe);
        assert!(!issues.iter().any(|item| item.code == "method.experimental"));
        assert_eq!(method_status("predict"), MethodStatus::Validated);

        recipe.model.paths.clear();
        let issues = validate_recipe(&recipe);
        assert!(issues.iter().any(|item| {
            item.code == "predict.endogenous_required" && item.severity == Severity::Error
        }));

        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Predict;
        recipe.model.constructs[1].mode = crate::MeasurementMode::Formative;
        recipe.settings.bootstrap_samples = 99;
        recipe.settings.confidence_level = 0.90;
        let issues = validate_recipe(&recipe);
        for code in [
            "predict.reflective_endogenous_required",
            "predict.external_resampling_unsupported",
            "predict.confidence_fixed",
        ] {
            assert!(
                issues
                    .iter()
                    .any(|item| { item.code == code && item.severity == Severity::Error })
            );
        }
    }

    #[test]
    fn bounded_mga_requires_group_column_metadata() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Mga;
        let issues = validate_recipe(&recipe);
        assert!(
            issues
                .iter()
                .any(|item| item.code == "mga.group_column_required")
        );

        recipe
            .metadata
            .insert("mga_group_column".into(), "group".into());
        recipe.metadata.insert("mga_group_a".into(), "A".into());
        recipe.metadata.insert("mga_group_b".into(), "B".into());
        let issues = validate_recipe(&recipe);
        assert!(
            !issues
                .iter()
                .any(|item| item.code == "mga.group_column_required")
        );
        assert!(!issues.iter().any(|item| item.code == "method.experimental"));
        assert_eq!(method_status("mga"), MethodStatus::Validated);
    }

    #[test]
    fn micom_v2_requires_configural_confirmation_and_stable_permutation_count() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Mga;
        recipe
            .metadata
            .insert("mga_group_column".into(), "group".into());
        recipe.metadata.insert("mga_group_a".into(), "A".into());
        recipe.metadata.insert("mga_group_b".into(), "B".into());
        recipe
            .metadata
            .insert("group_methods".into(), "mga_permutation, MICOM".into());
        recipe
            .metadata
            .insert("group_permutation_samples".into(), "4999".into());

        let issues = validate_recipe(&recipe);
        assert!(issues.iter().any(|item| {
            item.code == "mga.permutation_samples" && item.severity == Severity::Error
        }));
        assert!(issues.iter().any(|item| {
            item.code == "micom.configural_confirmation_required"
                && item.severity == Severity::Error
        }));

        recipe
            .metadata
            .insert("group_permutation_samples".into(), "5000".into());
        recipe
            .metadata
            .insert("micom_configural_confirmed".into(), "true".into());
        assert!(validate_recipe(&recipe).is_empty());
    }

    #[test]
    fn bounded_mga_requires_explicit_distinct_groups_and_excludes_group_indicator() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Mga;
        recipe
            .metadata
            .insert("mga_group_column".into(), "x1".into());
        let issues = validate_recipe(&recipe);
        assert!(
            issues
                .iter()
                .any(|item| item.code == "mga.group_a_required")
        );
        assert!(
            issues
                .iter()
                .any(|item| item.code == "mga.group_b_required")
        );
        assert!(
            issues
                .iter()
                .any(|item| item.code == "mga.group_column_is_indicator")
        );

        recipe.metadata.insert("mga_group_a".into(), "same".into());
        recipe.metadata.insert("mga_group_b".into(), "same".into());
        assert!(
            validate_recipe(&recipe)
                .iter()
                .any(|item| item.code == "mga.groups_must_differ")
        );
    }

    #[test]
    fn bounded_mga_permutation_requires_supported_method_and_sample_count() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Mga;
        recipe
            .metadata
            .insert("mga_group_column".into(), "group".into());
        recipe.metadata.insert("mga_group_a".into(), "A".into());
        recipe.metadata.insert("mga_group_b".into(), "B".into());
        recipe
            .metadata
            .insert("group_methods".into(), "mga_permutation, mystery".into());
        recipe
            .metadata
            .insert("group_permutation_samples".into(), "98".into());
        let issues = validate_recipe(&recipe);
        assert!(
            issues
                .iter()
                .any(|item| item.code == "mga.group_method_unsupported")
        );
        assert!(
            issues
                .iter()
                .any(|item| item.code == "mga.permutation_samples")
        );
    }

    #[test]
    fn bounded_ipma_requires_an_endogenous_target() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Ipma;
        let issues = validate_recipe(&recipe);
        assert!(!issues.iter().any(|item| item.code == "method.experimental"));
        assert_eq!(method_status("ipma"), MethodStatus::Validated);
        assert_eq!(resolve_ipma_targets(&recipe).unwrap(), vec!["y"]);

        recipe.metadata.insert("ipma_targets".into(), "x".into());
        let issues = validate_recipe(&recipe);
        assert!(issues.iter().any(|item| {
            item.code == "ipma.target_must_be_endogenous"
                && item.severity == Severity::Error
                && item.subject.as_deref() == Some("x")
        }));

        recipe.metadata.insert("ipma_targets".into(), "y,y".into());
        assert!(
            validate_recipe(&recipe)
                .iter()
                .any(|item| item.code == "ipma.target_duplicate")
        );

        recipe.metadata.insert("ipma_targets".into(), "y".into());
        recipe.metadata.insert("ipma.targets".into(), "x".into());
        assert!(
            validate_recipe(&recipe)
                .iter()
                .any(|item| item.code == "ipma.target_metadata_conflict")
        );

        recipe.metadata.clear();
        recipe.model.paths.clear();
        let issues = validate_recipe(&recipe);
        assert!(issues.iter().any(|item| {
            item.code == "ipma.target_required" && item.severity == Severity::Error
        }));
    }

    #[test]
    fn bounded_ipma_locks_preprocessing_weighting_and_resampling_scope() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Ipma;
        recipe.settings.weighting_scheme = crate::WeightingScheme::Factor;
        recipe.settings.preprocessing = crate::Preprocessing::MeanCentered;
        recipe.settings.bootstrap_samples = 999;
        let issues = validate_recipe(&recipe);
        for code in [
            "ipma.path_weighting_required",
            "ipma.standardized_required",
            "ipma.resampling_unsupported",
        ] {
            assert!(
                issues.iter().any(|item| item.code == code),
                "missing {code}"
            );
        }
    }

    #[test]
    fn bounded_nca_v2_accepts_a_no_model_raw_numeric_recipe() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Nca;
        recipe.settings.preprocessing = crate::Preprocessing::Unstandardized;
        recipe.model.constructs.clear();
        recipe.model.paths.clear();
        recipe.metadata.extend([
            ("nca_x".into(), "x".into()),
            ("nca_y".into(), "y".into()),
            ("nca_ceiling".into(), "both".into()),
            ("nca_permutation_samples".into(), "999".into()),
        ]);

        let issues = validate_recipe(&recipe);
        assert!(
            issues.iter().all(|item| item.severity != Severity::Error),
            "valid no-model NCA v2 recipe was rejected: {issues:#?}"
        );
        assert_eq!(method_status("nca"), MethodStatus::Validated);
    }

    #[test]
    fn bounded_pca_v1_accepts_no_model_retention_rules_and_rejects_scope_drift() {
        for (rule, metadata) in [
            ("kaiser", Vec::new()),
            ("fixed", vec![("pca_components", "2")]),
            (
                "variance_threshold",
                vec![("pca_variance_threshold", "0.80")],
            ),
        ] {
            let mut recipe = valid_recipe();
            recipe.settings.method = crate::AnalysisMethod::Pca;
            recipe.model.constructs.clear();
            recipe.model.paths.clear();
            recipe
                .metadata
                .insert("pca_variables".into(), "x,y,z".into());
            recipe
                .metadata
                .insert("pca_component_rule".into(), rule.into());
            for (key, value) in metadata {
                recipe.metadata.insert(key.into(), value.into());
            }
            let issues = validate_recipe(&recipe);
            assert!(
                issues.iter().all(|item| item.severity != Severity::Error),
                "valid PCA {rule} recipe was rejected: {issues:#?}"
            );
        }

        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Pca;
        recipe.settings.weighting_scheme = crate::WeightingScheme::Factor;
        recipe.settings.preprocessing = crate::Preprocessing::MeanCentered;
        recipe.settings.case_weight_column = Some("weight".into());
        recipe.settings.bootstrap_samples = 100;
        recipe.metadata.insert("pca_variables".into(), "x,x".into());
        recipe
            .metadata
            .insert("pca_component_rule".into(), "fixed".into());
        recipe.metadata.insert("pca_components".into(), "3".into());
        let issues = validate_recipe(&recipe);
        for code in [
            "pca.variables_unique",
            "pca.components",
            "pca.weighting_sentinel",
            "pca.standardized_required",
            "pca.case_weights_unsupported",
            "pca.resampling_unsupported",
            "pca.empty_model_required",
        ] {
            assert!(
                issues
                    .iter()
                    .any(|item| item.code == code && item.severity == Severity::Error),
                "missing PCA validation issue {code}: {issues:#?}"
            );
        }
    }

    #[test]
    fn bounded_ols_v1_accepts_raw_listwise_no_model_scope_and_rejects_drift() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Regression;
        recipe.settings.preprocessing = crate::Preprocessing::Unstandardized;
        recipe.settings.confidence_level = 0.95;
        recipe.model.constructs.clear();
        recipe.model.paths.clear();
        recipe.model.controls.clear();
        recipe.model.interactions.clear();
        recipe.model.higher_order_constructs.clear();
        recipe.metadata = std::collections::BTreeMap::from([
            (
                "status".into(),
                "validated_regression_ols_v1_bounded_scope".into(),
            ),
            ("regression_type".into(), "ols".into()),
            ("regression_outcome".into(), "y".into()),
            ("regression_predictors".into(), "x,m".into()),
            ("regression_controls".into(), "z".into()),
            ("robust_se".into(), "hc3".into()),
        ]);
        assert!(validate_recipe(&recipe).is_empty());

        recipe
            .metadata
            .insert("regression_controls".into(), "m".into());
        recipe.metadata.remove("robust_se");
        recipe.settings.preprocessing = crate::Preprocessing::Standardized;
        recipe.settings.confidence_level = 0.90;
        recipe.settings.permutation_samples = 99;
        recipe
            .model
            .constructs
            .push(valid_recipe().model.constructs[0].clone());
        let issues = validate_recipe(&recipe);
        for code in [
            "regression.variables_distinct",
            "regression.hc3_required",
            "regression.raw_values_required",
            "regression.confidence_fixed",
            "regression.resampling_unsupported",
            "regression.empty_model_required",
        ] {
            assert!(
                issues.iter().any(|item| item.code == code),
                "missing {code}"
            );
        }
    }

    #[test]
    fn bounded_cbsem_and_cfa_accept_exact_ml_scope_and_reject_scope_drift() {
        let mut sem = valid_recipe();
        sem.settings.method = crate::AnalysisMethod::Cbsem;
        sem.settings.weighting_scheme = WeightingScheme::Path;
        sem.settings.preprocessing = Preprocessing::Standardized;
        sem.settings.missing_data = crate::MissingDataPolicy::ListwiseDeletion;
        sem.settings.workers = 1;
        sem.metadata.extend([
            ("cbsem_model_type".into(), "sem".into()),
            ("cbsem_input".into(), "raw".into()),
            ("cbsem_estimator".into(), "ml".into()),
            ("cbsem_mean_structure".into(), "false".into()),
        ]);
        let sem_issues = validate_recipe(&sem);
        assert!(
            sem_issues
                .iter()
                .all(|item| item.severity != Severity::Error),
            "valid bounded CB-SEM recipe was rejected: {sem_issues:#?}"
        );

        let mut cfa = sem.clone();
        cfa.model.paths.clear();
        cfa.metadata.insert("cbsem_model_type".into(), "cfa".into());
        let cfa_issues = validate_recipe(&cfa);
        assert!(
            cfa_issues
                .iter()
                .all(|item| item.severity != Severity::Error),
            "valid bounded CFA recipe was rejected: {cfa_issues:#?}"
        );

        let mut invalid = sem;
        invalid
            .metadata
            .insert("cbsem_model_type".into(), "cfa".into());
        invalid
            .metadata
            .insert("cbsem_input".into(), "covariance".into());
        invalid
            .metadata
            .insert("cbsem_estimator".into(), "robust_ml".into());
        invalid
            .metadata
            .insert("cbsem_mean_structure".into(), "true".into());
        invalid
            .metadata
            .insert("cbsem_group_column".into(), "group".into());
        invalid
            .metadata
            .insert("cbsem_bootstrap_samples".into(), "999".into());
        invalid.settings.weighting_scheme = WeightingScheme::Factor;
        invalid.settings.preprocessing = Preprocessing::MeanCentered;
        invalid.settings.workers = 2;
        invalid.settings.bootstrap_samples = 999;
        invalid.settings.case_weight_column = Some("weight".into());
        invalid.model.constructs[0].mode = MeasurementMode::Formative;
        invalid.model.constructs[1].indicators.truncate(1);
        let issues = validate_recipe(&invalid);
        for code in [
            "cbsem.cfa_paths_unsupported",
            "cbsem.path_initialization_required",
            "cbsem.standardized_input_required",
            "cbsem.workers_fixed",
            "cbsem.resampling_unsupported",
            "cbsem.raw_input_required",
            "cbsem.ml_required",
            "cbsem.mean_structure_unsupported",
            "cbsem.multigroup_unsupported",
            "cbsem.bootstrap_unsupported",
            "cbsem.case_weights_unsupported",
            "cbsem.reflective_only",
            "cbsem.indicators_per_factor",
        ] {
            assert!(
                issues
                    .iter()
                    .any(|item| item.code == code && item.severity == Severity::Error),
                "missing CB-SEM validation issue {code}: {issues:#?}"
            );
        }
    }

    #[test]
    fn bounded_nca_v2_rejects_ambiguous_variables_and_incompatible_settings() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Nca;
        recipe.settings.weighting_scheme = crate::WeightingScheme::Factor;
        recipe.settings.preprocessing = crate::Preprocessing::Standardized;
        recipe.settings.case_weight_column = Some("weight".into());
        recipe.settings.bootstrap_samples = 999;
        recipe.settings.studentized_inner_samples = 99;
        recipe.settings.permutation_samples = 99;
        recipe.metadata.extend([
            ("nca_x".into(), "x".into()),
            ("nca_y".into(), "x".into()),
            ("nca_ceiling".into(), "free_disposal_hull".into()),
            ("nca_permutation_samples".into(), "0".into()),
        ]);

        let issues = validate_recipe(&recipe);
        for code in [
            "nca.variables_distinct",
            "nca.ceiling",
            "nca.permutation_samples",
            "nca.raw_values_required",
            "nca.weighting_sentinel",
            "nca.case_weights_unsupported",
            "nca.external_resampling_unsupported",
        ] {
            assert!(
                issues
                    .iter()
                    .any(|item| item.code == code && item.severity == Severity::Error),
                "missing NCA v2 validation issue {code}: {issues:#?}"
            );
        }

        recipe.metadata.insert("nca_y".into(), "  ".into());
        assert!(validate_recipe(&recipe).iter().any(|item| {
            item.code == "nca.variables_required" && item.severity == Severity::Error
        }));
    }

    #[test]
    fn ipma_predecessors_include_direct_and_indirect_drivers_but_not_target_or_unrelated() {
        let mut recipe = valid_recipe();
        recipe.model.constructs.insert(
            1,
            Construct {
                id: "m".into(),
                name: "Mediator".into(),
                short_name: "M".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["m1".into()],
            },
        );
        recipe.model.constructs.push(Construct {
            id: "z".into(),
            name: "Unrelated".into(),
            short_name: "Z".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["z1".into()],
        });
        recipe.model.paths = vec![
            StructuralPath {
                source: "x".into(),
                target: "m".into(),
            },
            StructuralPath {
                source: "m".into(),
                target: "y".into(),
            },
        ];

        assert_eq!(ipma_predecessor_constructs(&recipe, "y"), vec!["x", "m"]);
    }

    #[test]
    fn duplicate_indicator_is_rejected() {
        let mut recipe = valid_recipe();
        recipe.model.constructs[1].indicators.push("x1".into());
        assert!(
            validate_recipe(&recipe)
                .iter()
                .any(|item| item.code == "indicator.duplicate" && item.severity == Severity::Error)
        );
    }

    #[test]
    fn bounded_bootstrap_request_is_validated_without_an_experimental_issue() {
        let mut recipe = valid_recipe();
        recipe.settings.bootstrap_samples = 500;
        let issues = validate_recipe(&recipe);
        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn permutation_count_is_bounded_and_validated_without_an_experimental_issue() {
        let mut recipe = valid_recipe();
        recipe.settings.permutation_samples = 98;
        assert!(validate_recipe(&recipe).iter().any(|item| {
            item.code == "settings.permutation_samples" && item.severity == Severity::Error
        }));
        recipe.settings.permutation_samples = 99;
        let issues = validate_recipe(&recipe);
        assert!(
            !issues
                .iter()
                .any(|item| item.code == "settings.permutation_samples")
        );
        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn studentized_plan_requires_qualified_outer_and_odd_inner_counts() {
        let mut recipe = valid_recipe();
        recipe.settings.bootstrap_samples = 998;
        recipe.settings.studentized_inner_samples = 99;
        assert!(validate_recipe(&recipe).iter().any(|item| {
            item.code == "settings.studentized_inner_samples" && item.severity == Severity::Error
        }));
        recipe.settings.bootstrap_samples = 999;
        recipe.settings.studentized_inner_samples = 100;
        assert!(
            validate_recipe(&recipe)
                .iter()
                .any(|item| item.code == "settings.studentized_inner_samples")
        );
        recipe.settings.studentized_inner_samples = 99;
        let issues = validate_recipe(&recipe);
        assert!(
            !issues
                .iter()
                .any(|item| item.code == "settings.studentized_inner_samples")
        );
        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn duplicate_structural_path_is_rejected() {
        let mut recipe = valid_recipe();
        recipe.model.paths.push(recipe.model.paths[0].clone());
        assert!(
            validate_recipe(&recipe)
                .iter()
                .any(|item| { item.code == "path.duplicate" && item.severity == Severity::Error })
        );
    }

    #[test]
    fn directed_structural_cycle_is_rejected() {
        let mut recipe = valid_recipe();
        recipe.model.paths.push(StructuralPath {
            source: "y".into(),
            target: "x".into(),
        });
        assert!(
            validate_recipe(&recipe)
                .iter()
                .any(|item| { item.code == "path.cycle" && item.severity == Severity::Error })
        );
    }

    #[test]
    fn controls_are_schema_validated_and_must_map_to_paths() {
        let mut recipe = valid_recipe();
        recipe.model.controls.push(ControlPath {
            source: "x".into(),
            target: "y".into(),
            label: Some("Age".into()),
        });
        let issues = validate_recipe(&recipe);
        assert!(!issues.iter().any(|item| item.severity == Severity::Error));
        assert!(issues.iter().any(|item| {
            item.code == "method.controls.experimental" && item.severity == Severity::Warning
        }));

        recipe.model.controls.push(ControlPath {
            source: "x".into(),
            target: "y".into(),
            label: None,
        });
        assert!(
            validate_recipe(&recipe)
                .iter()
                .any(|item| item.code == "control.duplicate" && item.severity == Severity::Error)
        );

        recipe.model.controls.pop();
        recipe.model.controls.push(ControlPath {
            source: "z".into(),
            target: "y".into(),
            label: None,
        });
        let issues = validate_recipe(&recipe);
        assert!(
            issues
                .iter()
                .any(|item| item.code == "control.unknown_construct")
        );
        assert!(
            issues
                .iter()
                .any(|item| item.code == "control.missing_structural_path")
        );

        recipe.model.controls.pop();
        recipe.model.controls.push(ControlPath {
            source: "y".into(),
            target: "y".into(),
            label: None,
        });
        assert!(
            validate_recipe(&recipe)
                .iter()
                .any(|item| item.code == "control.self" && item.severity == Severity::Error)
        );
    }

    #[test]
    fn higher_order_constructs_are_schema_validated_as_experimental() {
        let mut recipe = valid_recipe();
        recipe.model.constructs.push(Construct {
            id: "z".into(),
            name: "Second Component".into(),
            short_name: "Z".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["z1".into(), "z2".into()],
        });
        recipe
            .model
            .higher_order_constructs
            .push(HigherOrderConstruct {
                id: "y".into(),
                components: vec!["x".into(), "z".into()],
                method: HigherOrderMethod::RepeatedIndicators,
                stage_one_recipe: None,
            });
        let issues = validate_recipe(&recipe);
        assert!(!issues.iter().any(|item| item.severity == Severity::Error));
        assert!(issues.iter().any(|item| {
            item.code == "method.higher_order.experimental" && item.severity == Severity::Warning
        }));

        recipe.model.constructs[1].indicators.clear();
        let issues = validate_recipe(&recipe);
        assert!(!issues.iter().any(|item| {
            item.code == "construct.indicators" && item.severity == Severity::Error
        }));
        assert!(issues.iter().any(|item| {
            item.code == "higher_order.indicators.generated" && item.severity == Severity::Warning
        }));
        recipe.model.constructs[1].indicators = vec!["y1".into(), "y2".into()];

        recipe
            .model
            .higher_order_constructs
            .push(HigherOrderConstruct {
                id: "y".into(),
                components: vec!["y".into(), "missing".into(), "missing".into()],
                method: HigherOrderMethod::TwoStage,
                stage_one_recipe: Some("stage-1-run".into()),
            });
        let issues = validate_recipe(&recipe);
        for code in [
            "higher_order.duplicate_id",
            "higher_order.self_component",
            "higher_order.unknown_component",
            "higher_order.duplicate_component",
        ] {
            assert!(
                issues
                    .iter()
                    .any(|item| item.code == code && item.severity == Severity::Error),
                "expected {code}"
            );
        }

        recipe.model.higher_order_constructs.clear();
        recipe
            .model
            .higher_order_constructs
            .push(HigherOrderConstruct {
                id: "missing".into(),
                components: vec!["x".into()],
                method: HigherOrderMethod::Hybrid,
                stage_one_recipe: None,
            });
        let issues = validate_recipe(&recipe);
        for code in ["higher_order.unknown_construct", "higher_order.components"] {
            assert!(
                issues
                    .iter()
                    .any(|item| item.code == code && item.severity == Severity::Error),
                "expected {code}"
            );
        }

        recipe.model.higher_order_constructs.clear();
        recipe
            .model
            .higher_order_constructs
            .push(HigherOrderConstruct {
                id: "y".into(),
                components: vec!["x".into(), "z".into()],
                method: HigherOrderMethod::Hybrid,
                stage_one_recipe: None,
            });
        let issues = validate_recipe(&recipe);
        assert!(!issues.iter().any(|item| item.severity == Severity::Error));
        assert!(issues.iter().any(|item| {
            item.code == "method.higher_order.experimental" && item.severity == Severity::Warning
        }));

        recipe.model.constructs[0].indicators = vec!["x1".into()];
        let issues = validate_recipe(&recipe);
        assert!(issues.iter().any(|item| {
            item.code == "higher_order.hybrid_component_indicators"
                && item.severity == Severity::Error
        }));
    }

    #[test]
    fn two_stage_interactions_are_schema_validated_as_experimental() {
        let mut recipe = valid_recipe();
        recipe.model.constructs.push(Construct {
            id: "xm".into(),
            name: "X x moderator".into(),
            short_name: "XM".into(),
            mode: MeasurementMode::Formative,
            indicators: Vec::new(),
        });
        recipe.model.paths.push(StructuralPath {
            source: "xm".into(),
            target: "y".into(),
        });
        recipe.model.interactions.push(InteractionTerm {
            id: "x_by_m_to_y".into(),
            predictor: "x".into(),
            moderator: "m".into(),
            product_construct: "xm".into(),
            outcome: "y".into(),
            method: InteractionMethod::TwoStageProductScore,
        });
        let issues = validate_recipe(&recipe);
        assert!(
            issues
                .iter()
                .any(|item| item.code == "interaction.unknown_construct")
        );
        assert!(issues.iter().any(|item| {
            item.code == "method.moderation.experimental" && item.severity == Severity::Warning
        }));
        recipe.model.constructs.push(Construct {
            id: "m".into(),
            name: "Moderator".into(),
            short_name: "M".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["m1".into(), "m2".into()],
        });
        let issues = validate_recipe(&recipe);
        assert!(
            !issues
                .iter()
                .any(|item| item.code == "interaction.unknown_construct")
        );
        assert!(
            !issues
                .iter()
                .any(|item| item.code == "construct.indicators" && item.severity == Severity::Error)
        );
        assert!(issues.iter().any(|item| {
            item.code == "method.moderation.experimental" && item.severity == Severity::Warning
        }));
        assert!(issues.iter().any(|item| {
            item.code == "interaction.product_indicator.generated"
                && item.severity == Severity::Warning
        }));
    }
}
