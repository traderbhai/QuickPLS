//! Validation-only product probe for pre-registry MGA v4 qualification.
//!
//! This example deliberately bypasses the customer capability registry so an
//! absent cell can be qualified before promotion. It is not installed by the
//! desktop application or the `qpls` CLI.

use qpls_core::AnalysisRecipe;
use qpls_data::{ImportOptions, import_delimited_bytes};
use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1).map(PathBuf::from);
    let recipe_path = args.next().ok_or("expected recipe path")?;
    let data_path = args.next().ok_or("expected CSV path")?;
    let output_path = args.next().ok_or("expected output path")?;
    if args.next().is_some() {
        return Err("expected exactly recipe, CSV, and output paths".into());
    }
    let recipe: AnalysisRecipe = serde_json::from_slice(&fs::read(&recipe_path)?)?;
    let dataset = import_delimited_bytes(
        &fs::read(&data_path)?,
        data_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("data.csv"),
        b',',
        &ImportOptions::default(),
    )?;
    if recipe.dataset_fingerprint != dataset.fingerprint.0 {
        return Err("recipe dataset fingerprint does not match imported CSV".into());
    }
    let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {})?;
    fs::write(output_path, serde_json::to_vec_pretty(&result)?)?;
    Ok(())
}
