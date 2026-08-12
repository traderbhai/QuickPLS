use thiserror::Error;

pub const DIJKSTRA_HENSELER_RHO_A_METHOD_VERSION: &str = "dijkstra_henseler_rho_a_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RhoABoundaryWarning {
    ImproperBelowZero,
    ImproperAboveOne,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RhoAEquationResult {
    pub value: f64,
    pub weight_norm_squared: f64,
    pub off_diagonal_numerator: f64,
    pub off_diagonal_denominator: f64,
    pub boundary_warning: Option<RhoABoundaryWarning>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum RhoAEquationError {
    #[error("rho_A requires a square indicator-correlation matrix matching the weight vector")]
    DimensionMismatch,
    #[error("rho_A inputs contain a nonfinite value")]
    NonfiniteInput,
    #[error("rho_A weights are not normalized to unit score variance")]
    InvalidScoreVariance,
    #[error("rho_A off-diagonal denominator is zero")]
    OffDiagonalDenominatorZero,
    #[error("rho_A result is nonfinite")]
    NonfiniteResult,
}

/// Evaluates Dijkstra and Henseler's rho_A equation for Mode A weights that
/// have already been normalized so `w' R w = 1`.
///
/// The implementation follows Equation 3 of Dijkstra and Henseler (2015):
/// `(w'w)^2 * w'(R - diag(R))w / w'(ww' - diag(ww'))w`.
/// Improper finite-sample values are preserved and identified; only excursions
/// within floating-point boundary tolerance are canonicalized to zero or one.
pub fn dijkstra_henseler_rho_a_from_normalized(
    correlations: &[Vec<f64>],
    weights: &[f64],
) -> Result<RhoAEquationResult, RhoAEquationError> {
    if weights.len() < 2
        || correlations.len() != weights.len()
        || correlations.iter().any(|row| row.len() != weights.len())
    {
        return Err(RhoAEquationError::DimensionMismatch);
    }
    if weights.iter().any(|value| !value.is_finite())
        || correlations
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
    {
        return Err(RhoAEquationError::NonfiniteInput);
    }

    let score_variance = quadratic_form(weights, correlations);
    if !score_variance.is_finite() || (score_variance - 1.0).abs() > 1e-10 {
        return Err(RhoAEquationError::InvalidScoreVariance);
    }

    let weight_norm_squared = weights.iter().map(|value| value * value).sum::<f64>();
    let fourth_sum = weights.iter().map(|value| value.powi(4)).sum::<f64>();
    let off_diagonal_denominator = weight_norm_squared.powi(2) - fourth_sum;
    let off_diagonal_numerator = (0..weights.len())
        .flat_map(|row| (0..weights.len()).map(move |column| (row, column)))
        .filter(|(row, column)| row != column)
        .map(|(row, column)| weights[row] * weights[column] * correlations[row][column])
        .sum::<f64>();
    let tolerance = 64.0 * f64::EPSILON * weight_norm_squared.powi(2).max(fourth_sum).max(1.0);
    if !weight_norm_squared.is_finite()
        || !off_diagonal_numerator.is_finite()
        || !off_diagonal_denominator.is_finite()
    {
        return Err(RhoAEquationError::NonfiniteResult);
    }
    if off_diagonal_denominator <= tolerance {
        return Err(RhoAEquationError::OffDiagonalDenominatorZero);
    }

    let mut value = weight_norm_squared.powi(2) * off_diagonal_numerator / off_diagonal_denominator;
    if !value.is_finite() {
        return Err(RhoAEquationError::NonfiniteResult);
    }
    let boundary_tolerance = 64.0 * f64::EPSILON * value.abs().max(1.0);
    let boundary_warning = if value < 0.0 {
        if value >= -boundary_tolerance {
            value = 0.0;
            None
        } else {
            Some(RhoABoundaryWarning::ImproperBelowZero)
        }
    } else if value > 1.0 {
        if value <= 1.0 + boundary_tolerance {
            value = 1.0;
            None
        } else {
            Some(RhoABoundaryWarning::ImproperAboveOne)
        }
    } else {
        None
    };

    Ok(RhoAEquationResult {
        value,
        weight_norm_squared,
        off_diagonal_numerator,
        off_diagonal_denominator,
        boundary_warning,
    })
}

fn quadratic_form(weights: &[f64], matrix: &[Vec<f64>]) -> f64 {
    let mut total = 0.0;
    for row in 0..weights.len() {
        for column in 0..weights.len() {
            total += weights[row] * matrix[row][column] * weights[column];
        }
    }
    total
}

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

    #[test]
    fn rho_a_equation_matches_primary_hand_fixtures() {
        let three = vec![
            vec![1.0, 0.5, 0.5],
            vec![0.5, 1.0, 0.5],
            vec![0.5, 0.5, 1.0],
        ];
        let equal_three = vec![1.0 / 6.0_f64.sqrt(); 3];
        let result = dijkstra_henseler_rho_a_from_normalized(&three, &equal_three).unwrap();
        assert!((result.value - 0.75).abs() < EPS);
        assert!((result.off_diagonal_numerator - 0.5).abs() < EPS);
        assert!((result.off_diagonal_denominator - (1.0 / 6.0)).abs() < EPS);

        let two = vec![vec![1.0, 0.6], vec![0.6, 1.0]];
        let equal_two = vec![1.0 / 3.2_f64.sqrt(); 2];
        let result = dijkstra_henseler_rho_a_from_normalized(&two, &equal_two).unwrap();
        assert!((result.value - 0.75).abs() < EPS);
    }
}
