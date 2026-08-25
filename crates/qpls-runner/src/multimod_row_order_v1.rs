//! Representation-independent row ordering for deterministic MultiMod refits.
//!
//! Dataset row numbers are physical addresses, not scientific identities. A
//! bootstrap seeded against their current order changes its realised sample
//! when the same observations are imported in another order. This helper
//! derives an internal order from the typed values of the columns actually
//! used by an analysis. Physical row numbers remain unchanged in public
//! preparation and exclusion receipts.

use qpls_core::sha256_hex;
use qpls_data::{ColumnType, Dataset};
use std::collections::{BTreeMap, BTreeSet};

const ROW_ORDER_DOMAIN_V1: &[u8] = b"qpls.multimod.scientific-row-order.v1\0";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ScientificRowKeyV1 {
    digest: String,
    canonical: Vec<u8>,
}

fn append_bytes(target: &mut Vec<u8>, value: &[u8]) {
    target.extend_from_slice(&(value.len() as u64).to_le_bytes());
    target.extend_from_slice(value);
}

fn canonical_numeric_bits(value: f64, absolute: bool) -> u64 {
    let value = if absolute { value.abs() } else { value };
    if value == 0.0 {
        0.0f64.to_bits()
    } else {
        value.to_bits()
    }
}

fn scientific_row_key_v1(
    row: &BTreeMap<String, Option<String>>,
    column_types: &BTreeMap<String, ColumnType>,
    ordered_columns: &[String],
    sign_invariant_numeric_columns: &BTreeSet<String>,
) -> Result<ScientificRowKeyV1, String> {
    let mut canonical = Vec::new();
    canonical.extend_from_slice(ROW_ORDER_DOMAIN_V1);
    for column in ordered_columns {
        append_bytes(&mut canonical, column.as_bytes());
        let column_type = column_types
            .get(column)
            .ok_or_else(|| format!("scientific row-order column {column} lacks metadata"))?;
        canonical.push(match column_type {
            ColumnType::Numeric => 1,
            ColumnType::Text => 2,
            ColumnType::Boolean => 3,
        });
        let value = row
            .get(column)
            .ok_or_else(|| format!("scientific row-order column {column} is absent"))?;
        let Some(value) = value.as_deref() else {
            canonical.push(0);
            continue;
        };
        canonical.push(1);
        match column_type {
            ColumnType::Numeric => {
                let parsed = value.parse::<f64>().map_err(|_| {
                    format!("scientific row-order numeric column {column} is not numeric")
                })?;
                if !parsed.is_finite() {
                    return Err(format!(
                        "scientific row-order numeric column {column} is nonfinite"
                    ));
                }
                canonical.extend_from_slice(
                    &canonical_numeric_bits(
                        parsed,
                        sign_invariant_numeric_columns.contains(column),
                    )
                    .to_le_bytes(),
                );
            }
            ColumnType::Text | ColumnType::Boolean => {
                append_bytes(&mut canonical, value.as_bytes())
            }
        }
    }
    let digest = sha256_hex(&canonical);
    Ok(ScientificRowKeyV1 { digest, canonical })
}

/// Returns positions into `source_rows` in a representation-independent
/// scientific order. Exact duplicate rows are exchangeable; their physical
/// address is used only as the final in-run tie-break and never enters the
/// scientific key.
pub(crate) fn canonical_multimod_row_permutation_v1(
    dataset: &Dataset,
    source_rows: &[u32],
    columns: &[String],
    sign_invariant_numeric_columns: &BTreeSet<String>,
) -> Result<Vec<usize>, String> {
    if source_rows.is_empty() || columns.is_empty() {
        return Err("scientific row ordering requires rows and columns".into());
    }
    let mut ordered_columns = columns.to_vec();
    ordered_columns.sort();
    ordered_columns.dedup();
    let column_types = dataset
        .schema
        .columns
        .iter()
        .map(|column| (column.name.clone(), column.column_type))
        .collect::<BTreeMap<_, _>>();
    for column in &ordered_columns {
        if !column_types.contains_key(column) {
            return Err(format!(
                "scientific row-order column {column} is absent from the dataset schema"
            ));
        }
    }
    if !sign_invariant_numeric_columns.iter().all(|column| {
        ordered_columns.binary_search(column).is_ok()
            && column_types.get(column) == Some(&ColumnType::Numeric)
    }) {
        return Err(
            "sign-invariant scientific row-order columns must be bound numeric columns".into(),
        );
    }
    let rows = qpls_data::preview_page(dataset, 0, dataset.batch.num_rows());
    let mut keyed = source_rows
        .iter()
        .copied()
        .enumerate()
        .map(|(position, source_row)| {
            let row = rows
                .get(source_row as usize)
                .ok_or_else(|| format!("scientific source row {source_row} is out of range"))?;
            Ok((
                scientific_row_key_v1(
                    row,
                    &column_types,
                    &ordered_columns,
                    sign_invariant_numeric_columns,
                )?,
                source_row,
                position,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    keyed.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then(left.1.cmp(&right.1))
            .then(left.2.cmp(&right.2))
    });
    Ok(keyed.into_iter().map(|(_, _, position)| position).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_data::{ImportOptions, import_delimited_bytes, preview_page};

    fn dataset(csv: &str) -> Dataset {
        import_delimited_bytes(csv.as_bytes(), "rows.csv", b',', &ImportOptions::default())
            .expect("fixture dataset")
    }

    fn ordered_values(
        dataset: &Dataset,
        columns: &[String],
        signed: &BTreeSet<String>,
    ) -> Vec<String> {
        let source_rows = (0..dataset.batch.num_rows() as u32).collect::<Vec<_>>();
        let permutation =
            canonical_multimod_row_permutation_v1(dataset, &source_rows, columns, signed)
                .expect("canonical order");
        let rows = preview_page(dataset, 0, dataset.batch.num_rows());
        permutation
            .into_iter()
            .map(|position| {
                let source_row = source_rows[position] as usize;
                format!(
                    "{}|{}",
                    rows[source_row]["x"].as_deref().unwrap(),
                    rows[source_row]["c"].as_deref().unwrap()
                )
            })
            .collect()
    }

    #[test]
    fn numeric_sign_invariance_is_explicit_and_negative_zero_is_canonical() {
        assert_eq!(
            canonical_numeric_bits(-0.0, false),
            canonical_numeric_bits(0.0, false)
        );
        assert_eq!(
            canonical_numeric_bits(-2.5, true),
            canonical_numeric_bits(2.5, true)
        );
        assert_ne!(
            canonical_numeric_bits(-2.5, false),
            canonical_numeric_bits(2.5, false)
        );
    }

    #[test]
    fn row_and_column_representation_do_not_change_scientific_order() {
        let baseline = dataset("x,c\n1,3\n4,-2\n2,5\n");
        let reversed = dataset("c,x\n5,2\n-2,4\n3,1\n");
        let columns = vec!["x".into(), "c".into()];
        let signed = BTreeSet::new();
        assert_eq!(
            ordered_values(&baseline, &columns, &signed),
            ordered_values(&reversed, &columns, &signed)
        );
    }

    #[test]
    fn declared_sign_invariant_column_preserves_sampling_order() {
        let baseline = dataset("x,c\n1,3\n4,-2\n2,5\n");
        let sign_reversed = dataset("x,c\n1,-3\n4,2\n2,-5\n");
        let columns = vec!["x".into(), "c".into()];
        let signed = BTreeSet::from(["c".to_owned()]);
        let baseline_order = ordered_values(&baseline, &columns, &signed)
            .into_iter()
            .map(|value| value.split('|').next().unwrap().to_owned())
            .collect::<Vec<_>>();
        let reversed_order = ordered_values(&sign_reversed, &columns, &signed)
            .into_iter()
            .map(|value| value.split('|').next().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(baseline_order, reversed_order);
    }
}
