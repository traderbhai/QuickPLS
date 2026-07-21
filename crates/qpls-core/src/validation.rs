use crate::{
    AnalysisRecipe, HigherOrderMethod, InteractionMethod, MeasurementMode, MethodStatus,
    PROJECT_SCHEMA_VERSION, method_status,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
    if recipe.settings.bootstrap_samples > 0 {
        issues.push(issue(
            "method.bootstrap.experimental",
            Severity::Warning,
            "Bootstrap inference is validated only inside the documented QuickPLS v1.0.0 supported scope",
            Some(recipe.settings.bootstrap_samples.to_string()),
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
    if recipe.settings.studentized_inner_samples > 0 {
        issues.push(issue(
            "method.studentized.experimental",
            Severity::Warning,
            format!(
                "Studentized bootstrap is experimental and may execute {} inner PLS fits",
                recipe.settings.bootstrap_samples as u64
                    * recipe.settings.studentized_inner_samples as u64
            ),
            Some(recipe.settings.studentized_inner_samples.to_string()),
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
    if recipe.settings.permutation_samples > 0 {
        issues.push(issue(
            "method.permutation.experimental",
            Severity::Warning,
            "Permutation inference is validated only inside the documented QuickPLS v1.0.0 supported scope",
            Some(recipe.settings.permutation_samples.to_string()),
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
    }
    if recipe.settings.method == crate::AnalysisMethod::Cca {
        if recipe.settings.weighting_scheme == crate::WeightingScheme::Pca {
            issues.push(issue(
                "cca.pca_unsupported",
                Severity::Error,
                "CCA requires path or factor weighting in this experimental release",
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
                "CCA is limited to reflective composites in this experimental release",
                None,
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
                "PLSpredict holdout v1 does not yet support case weights",
                None,
            ));
        }
        if !recipe.model.interactions.is_empty() {
            issues.push(issue(
                "predict.interactions_unsupported",
                Severity::Error,
                "PLSpredict holdout v1 does not yet support generated interaction constructs",
                None,
            ));
        }
        if !recipe.model.higher_order_constructs.is_empty() {
            issues.push(issue(
                "predict.higher_order_unsupported",
                Severity::Error,
                "PLSpredict holdout v1 does not yet support higher-order construct expansion",
                None,
            ));
        }
        if recipe.model.paths.is_empty() {
            issues.push(issue(
                "predict.endogenous_required",
                Severity::Error,
                "PLSpredict holdout v1 requires at least one endogenous construct",
                None,
            ));
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::Mga {
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
        match recipe
            .metadata
            .get("mga_group_column")
            .or_else(|| recipe.metadata.get("mga.group_column"))
        {
            Some(column) if !column.trim().is_empty() => {}
            _ => issues.push(issue(
                "mga.group_column_required",
                Severity::Error,
                "Bounded MGA v1 requires metadata.mga_group_column naming a two-group column",
                None,
            )),
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::Ipma {
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
        if recipe.model.paths.is_empty() {
            issues.push(issue(
                "ipma.target_required",
                Severity::Error,
                "IPMA v1 requires at least one endogenous target construct",
                None,
            ));
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::Cbsem {
        issues.push(issue(
            "cbsem.experimental",
            Severity::Warning,
            "CB-SEM/CFA ML v1 is validated only for the documented QuickPLS v1.0.0 bounded reflective ML scope",
            None,
        ));
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
        if variables.len() < 2
            && recipe
                .model
                .constructs
                .iter()
                .flat_map(|c| &c.indicators)
                .count()
                < 2
        {
            issues.push(issue(
                "pca.variables_required",
                Severity::Error,
                "Standalone PCA requires metadata.pca_variables with at least two numeric columns or at least two model indicators",
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
        if regression_type != "ols" {
            issues.push(issue(
                "regression.experimental",
                Severity::Warning,
                "Only OLS regression is validated for the documented QuickPLS v1.2 scope; logistic regression and PROCESS-style workflows remain experimental",
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
    }
    if recipe.settings.method == crate::AnalysisMethod::Nca {
        if !recipe.metadata.contains_key("nca_x") || !recipe.metadata.contains_key("nca_y") {
            issues.push(issue(
                "nca.variables_required",
                Severity::Error,
                "NCA requires metadata.nca_x and metadata.nca_y",
                None,
            ));
        }
    }
    if recipe.settings.method == crate::AnalysisMethod::Gsca {
        issues.push(issue(
            "gsca.experimental",
            Severity::Warning,
            "GSCA v1 is an experimental bounded component-model preview",
            None,
        ));
        if recipe.settings.case_weight_column.is_some() {
            issues.push(issue(
                "gsca.case_weights_unsupported",
                Severity::Error,
                "GSCA v1 does not yet support case weights",
                None,
            ));
        }
        if !recipe.model.interactions.is_empty() {
            issues.push(issue(
                "gsca.interactions_unsupported",
                Severity::Error,
                "GSCA v1 does not yet support generated interaction constructs",
                None,
            ));
        }
        if !recipe.model.higher_order_constructs.is_empty() {
            issues.push(issue(
                "gsca.higher_order_unsupported",
                Severity::Error,
                "GSCA v1 does not yet support higher-order construct expansion",
                None,
            ));
        }
        if recipe.model.constructs.is_empty() {
            issues.push(issue(
                "gsca.constructs_required",
                Severity::Error,
                "GSCA v1 requires at least one construct",
                None,
            ));
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
    fn plspredict_holdout_is_validated_and_rejects_unsupported_shapes() {
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
        let issues = validate_recipe(&recipe);
        assert!(
            !issues
                .iter()
                .any(|item| item.code == "mga.group_column_required")
        );
        assert!(issues.iter().any(|item| {
            item.code == "method.experimental" && item.severity == Severity::Warning
        }));
    }

    #[test]
    fn bounded_ipma_requires_an_endogenous_target() {
        let mut recipe = valid_recipe();
        recipe.settings.method = crate::AnalysisMethod::Ipma;
        let issues = validate_recipe(&recipe);
        assert!(!issues.iter().any(|item| item.code == "method.experimental"));
        assert_eq!(method_status("ipma"), MethodStatus::Validated);
        recipe.model.paths.clear();
        let issues = validate_recipe(&recipe);
        assert!(issues.iter().any(|item| {
            item.code == "ipma.target_required" && item.severity == Severity::Error
        }));
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
    fn bootstrap_request_is_marked_experimental() {
        let mut recipe = valid_recipe();
        recipe.settings.bootstrap_samples = 500;
        let issues = validate_recipe(&recipe);
        assert!(issues.iter().any(|item| {
            item.code == "method.bootstrap.experimental" && item.severity == Severity::Warning
        }));
    }

    #[test]
    fn permutation_count_is_bounded_and_marked_experimental() {
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
        assert!(issues.iter().any(|item| {
            item.code == "method.permutation.experimental" && item.severity == Severity::Warning
        }));
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
        assert!(
            issues
                .iter()
                .any(|item| item.code == "method.studentized.experimental")
        );
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
