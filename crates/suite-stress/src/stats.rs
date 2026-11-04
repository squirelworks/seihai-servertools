//! Small numeric helpers: percentiles and a least-squares slope.

/// The `p`th percentile (0..=100) of `v`, nearest rank. `None` when empty.
pub fn percentile(v: &[f64], p: f64) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    let rank = ((p / 100.0) * s.len() as f64).ceil() as usize;
    Some(s[rank.clamp(1, s.len()) - 1])
}

/// p50, p95, p99 and max, in the order a report prints them.
pub fn summary(v: &[f64]) -> Option<[f64; 4]> {
    Some([
        percentile(v, 50.0)?,
        percentile(v, 95.0)?,
        percentile(v, 99.0)?,
        percentile(v, 100.0)?,
    ])
}

/// Slope of `y` over `x` by least squares. `None` with fewer than two
/// points or no spread in `x`.
pub fn slope(points: &[(f64, f64)]) -> Option<f64> {
    if points.len() < 2 {
        return None;
    }
    let n = points.len() as f64;
    let mx = points.iter().map(|p| p.0).sum::<f64>() / n;
    let my = points.iter().map(|p| p.1).sum::<f64>() / n;
    let sxx: f64 = points.iter().map(|p| (p.0 - mx).powi(2)).sum();
    if sxx == 0.0 {
        return None;
    }
    let sxy: f64 = points.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    Some(sxy / sxx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank() {
        let v: Vec<f64> = (1..=100).map(f64::from).collect();
        assert_eq!(percentile(&v, 50.0), Some(50.0));
        assert_eq!(percentile(&v, 95.0), Some(95.0));
        assert_eq!(percentile(&v, 100.0), Some(100.0));
        assert_eq!(percentile(&[7.0], 99.0), Some(7.0));
        assert_eq!(percentile(&[], 50.0), None);
    }

    #[test]
    fn line_slope() {
        let pts: Vec<(f64, f64)> = (0..10).map(|i| (i as f64, 3.0 * i as f64 + 1.0)).collect();
        assert!((slope(&pts).unwrap() - 3.0).abs() < 1e-9);
        assert_eq!(slope(&[(1.0, 1.0)]), None);
        assert_eq!(slope(&[(1.0, 1.0), (1.0, 2.0)]), None);
    }
}
