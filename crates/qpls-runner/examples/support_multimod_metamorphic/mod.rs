//! Validation-only fixture transformations for mapped MultiMod metamorphics.
//!
//! Production estimators never read these environment variables. The four
//! qualification examples opt into this helper solely while constructing
//! deterministic input authorities, then execute the ordinary public compiler
//! and raw runner unchanged.

use qpls_core::{SemGroupV4, SemModelV4, SemVariableV4};
use std::env;

pub const METAMORPHISM_ENV_V1: &str = "QPLS_MULTIMOD_METAMORPHISM_V1";
pub const SIGN_COLUMNS_ENV_V1: &str = "QPLS_MULTIMOD_SIGN_COLUMNS_V1";
pub const WORKERS_ENV_V1: &str = "QPLS_MULTIMOD_WORKERS_V1";

pub fn metamorphism_v1() -> String {
    env::var(METAMORPHISM_ENV_V1).unwrap_or_else(|_| "baseline".into())
}

pub fn row_reverse_v1() -> bool {
    metamorphism_v1() == "row_reverse"
}

pub fn mapped_source_row_v1(source_row: usize, row_count: usize) -> Result<usize, String> {
    if source_row >= row_count {
        return Err("metamorphic source row is outside the fixture".into());
    }
    Ok(if row_reverse_v1() {
        row_count - 1 - source_row
    } else {
        source_row
    })
}

pub fn transform_row_aligned_values_v1<T>(values: &mut [T]) {
    if row_reverse_v1() {
        values.reverse();
    }
}

pub fn transformed_columns_v1(
    headers: &[String],
    columns: &[Vec<Option<String>>],
) -> Result<(Vec<String>, Vec<Vec<Option<String>>>), String> {
    let mut headers = headers.to_vec();
    let mut columns = columns.to_vec();
    match metamorphism_v1().as_str() {
        "baseline" | "declaration_reverse" | "worker_parallel" | "seed_repeat" => {}
        "row_reverse" => columns.iter_mut().for_each(|column| column.reverse()),
        "input_column_reverse" => {
            headers.reverse();
            columns.reverse();
        }
        "sign_reverse" => {
            let selected = env::var(SIGN_COLUMNS_ENV_V1)
                .map_err(|_| "sign_reverse requires QPLS_MULTIMOD_SIGN_COLUMNS_V1".to_owned())?
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if selected.is_empty() {
                return Err("sign_reverse requires at least one exact column".into());
            }
            let mut matched = false;
            for selected_column in selected {
                let Some(index) = headers.iter().position(|header| header == &selected_column)
                else {
                    continue;
                };
                matched = true;
                for value in columns[index].iter_mut().flatten() {
                    let numeric = value.parse::<f64>().map_err(|_| {
                        format!("sign_reverse column is not numeric: {selected_column}")
                    })?;
                    *value = format!("{:.17}", -numeric);
                }
            }
            if !matched {
                return Err("sign_reverse matched no column in this fixture".into());
            }
        }
        other => return Err(format!("unknown MultiMod fixture metamorphism: {other}")),
    }
    Ok((headers, columns))
}

pub fn transform_model_declaration_order_v1(model: &mut SemModelV4) {
    if metamorphism_v1() != "declaration_reverse" {
        return;
    }
    model.variables.reverse();
    model.relations.reverse();
    model.parameters.reverse();
    model.constraints.reverse();
    model.derived_terms.reverse();
    model.annotations.reverse();
    if let SemGroupV4::ObservedGroups { levels, .. } = &mut model.group {
        levels.reverse();
    }
    for variable in &mut model.variables {
        if let SemVariableV4::Observed {
            categories,
            transformation_lineage,
            ..
        } = variable
        {
            categories.reverse();
            transformation_lineage.reverse();
        }
    }
}

pub fn configured_workers_v1(default: usize) -> Result<usize, String> {
    match env::var(WORKERS_ENV_V1) {
        Ok(value) => {
            let workers = value
                .parse::<usize>()
                .map_err(|_| "QPLS_MULTIMOD_WORKERS_V1 must be an integer".to_owned())?;
            if !(1..=64).contains(&workers) {
                return Err("QPLS_MULTIMOD_WORKERS_V1 must be in 1..=64".into());
            }
            Ok(workers)
        }
        Err(_) => Ok(default),
    }
}

pub fn compact_matrix_v1() -> bool {
    env::var("QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1").as_deref() == Ok("1")
}
