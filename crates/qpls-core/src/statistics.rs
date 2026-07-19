use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum StatisticsError {
    #[error("at least two observations are required")]
    InsufficientObservations,
    #[error("columns must have equal lengths")]
    LengthMismatch,
    #[error("constant columns cannot be standardized")]
    ConstantColumn,
}

pub fn mean(values: &[f64]) -> Result<f64, StatisticsError> {
    if values.len() < 2 {
        return Err(StatisticsError::InsufficientObservations);
    }
    Ok(values.iter().sum::<f64>() / values.len() as f64)
}

pub fn sample_variance(values: &[f64]) -> Result<f64, StatisticsError> {
    let center = mean(values)?;
    Ok(values
        .iter()
        .map(|value| (value - center).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64)
}

pub fn standardize(values: &[f64]) -> Result<Vec<f64>, StatisticsError> {
    let center = mean(values)?;
    let deviation = sample_variance(values)?.sqrt();
    if deviation <= f64::EPSILON {
        return Err(StatisticsError::ConstantColumn);
    }
    Ok(values
        .iter()
        .map(|value| (value - center) / deviation)
        .collect())
}

pub fn pearson_correlation(left: &[f64], right: &[f64]) -> Result<f64, StatisticsError> {
    if left.len() != right.len() {
        return Err(StatisticsError::LengthMismatch);
    }
    let left = standardize(left)?;
    let right = standardize(right)?;
    Ok(left.iter().zip(right).map(|(a, b)| a * b).sum::<f64>() / (left.len() - 1) as f64)
}

pub fn cronbach_alpha(columns: &[Vec<f64>]) -> Result<f64, StatisticsError> {
    if columns.len() < 2 {
        return Err(StatisticsError::InsufficientObservations);
    }
    let rows = columns[0].len();
    if columns.iter().any(|column| column.len() != rows) {
        return Err(StatisticsError::LengthMismatch);
    }
    let item_variances = columns
        .iter()
        .map(|column| sample_variance(column))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .sum::<f64>();
    let totals: Vec<f64> = (0..rows)
        .map(|row| columns.iter().map(|column| column[row]).sum())
        .collect();
    let total_variance = sample_variance(&totals)?;
    let count = columns.len() as f64;
    Ok(count / (count - 1.0) * (1.0 - item_variances / total_variance))
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-12;

    #[test]
    fn standardized_values_have_zero_mean_and_unit_sample_variance() {
        let result = standardize(&[1.0, 2.0, 3.0, 4.0]).unwrap();
        assert!(mean(&result).unwrap().abs() < EPS);
        assert!((sample_variance(&result).unwrap() - 1.0).abs() < EPS);
    }

    #[test]
    fn correlation_is_invariant_to_positive_affine_scale() {
        let left = [1.0, 2.0, 4.0, 8.0, 16.0];
        let right = [2.0, 5.0, 3.0, 12.0, 20.0];
        let shifted: Vec<f64> = left.iter().map(|value| value * 7.0 + 13.0).collect();
        assert!(
            (pearson_correlation(&left, &right).unwrap()
                - pearson_correlation(&shifted, &right).unwrap())
            .abs()
                < EPS
        );
    }

    #[test]
    fn alpha_matches_hand_calculated_fixture() {
        let columns = vec![
            vec![1.0, 2.0, 3.0, 4.0],
            vec![1.0, 2.0, 4.0, 5.0],
            vec![2.0, 3.0, 4.0, 6.0],
        ];
        assert!((cronbach_alpha(&columns).unwrap() - 0.9818181818181818).abs() < EPS);
    }
}
