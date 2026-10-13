use flow_like_ml_core::{Result, require};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct KsResult {
    pub statistic: f64,
    pub critical_value: f64,
    pub drift: bool,
    pub reference_samples: usize,
    pub current_samples: usize,
}
pub fn ks_test(reference: &[f64], current: &[f64], alpha: f64) -> Result<KsResult> {
    require(
        !reference.is_empty()
            && !current.is_empty()
            && reference.iter().chain(current).all(|v| v.is_finite()),
        "KS comparison needs nonempty finite samples",
    )?;
    require(
        alpha.is_finite() && alpha > 0.0 && alpha < 1.0,
        "KS alpha must be in (0,1)",
    )?;
    let mut a = reference.to_vec();
    let mut b = current.to_vec();
    a.sort_by(f64::total_cmp);
    b.sort_by(f64::total_cmp);
    let (mut i, mut j, mut statistic) = (0usize, 0usize, 0.0f64);
    while i < a.len() || j < b.len() {
        let next = if i == a.len() {
            b[j]
        } else if j == b.len() {
            a[i]
        } else {
            a[i].min(b[j])
        };
        while i < a.len() && a[i] <= next {
            i += 1;
        }
        while j < b.len() && b[j] <= next {
            j += 1;
        }
        statistic = statistic.max((i as f64 / a.len() as f64 - j as f64 / b.len() as f64).abs());
    }
    // Two-sample asymptotic bound; tied/discrete distributions make this conservative.
    let critical_value = (-0.5 * (alpha / 2.0).ln()).sqrt()
        * ((a.len() + b.len()) as f64 / (a.len() as f64 * b.len() as f64)).sqrt();
    Ok(KsResult {
        statistic,
        critical_value,
        drift: statistic > critical_value,
        reference_samples: a.len(),
        current_samples: b.len(),
    })
}

const PSI_MINIMUM_SAMPLES_PER_BIN: usize = 5;

/// Without drift, PSI x (1/reference + 1/current)^-1 is about chi-squared with bins - 1 degrees of
/// freedom; its mean plus this many standard deviations is the noise floor.
const PSI_NOISE_DEVIATIONS: f64 = 3.0;

/// PSI of two windows. Fails unless both windows hold at least 5 x bins values; that alone does
/// not keep sampling noise below common thresholds, which [`population_stability`] accounts for.
pub fn population_stability_index(
    reference: &[f64],
    current: &[f64],
    edges: &[f64],
) -> Result<f64> {
    let psi = population_stability(reference, current, edges)?;
    let bin_count = edges.len() + 1;
    let minimum = bin_count.saturating_mul(PSI_MINIMUM_SAMPLES_PER_BIN);
    require(
        reference.len() >= minimum && current.len() >= minimum,
        format!(
            "PSI over {bin_count} bins needs at least {minimum} values in each window \
             ({PSI_MINIMUM_SAMPLES_PER_BIN} x bins); got {} reference and {} current",
            reference.len(),
            current.len()
        ),
    )?;
    Ok(psi.value)
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PsiResult {
    pub value: f64,
    /// Mean PSI of two windows of these sizes drawn from one distribution:
    /// (bins - 1) * (1/reference + 1/current).
    pub expected_without_drift: f64,
    /// That mean plus three standard deviations; windows without drift exceed it in about 1% of
    /// comparisons.
    pub noise_floor: f64,
}

impl PsiResult {
    /// True when PSI exceeds both `threshold` and the noise floor, false when it stays within
    /// `threshold` while the noise floor is below it. None when the noise floor reaches `threshold`
    /// and PSI does not pass it: windows this small cannot tell such a drift from noise.
    pub fn drift(&self, threshold: f64) -> Option<bool> {
        if self.value > threshold.max(self.noise_floor) {
            Some(true)
        } else if self.noise_floor < threshold {
            Some(false)
        } else {
            None
        }
    }
}

/// PSI with the value sampling noise alone reaches at these window sizes; any nonempty windows.
pub fn population_stability(
    reference: &[f64],
    current: &[f64],
    edges: &[f64],
) -> Result<PsiResult> {
    require(
        !edges.is_empty(),
        "PSI needs at least one bin edge; a single bin always yields 0",
    )?;
    require(
        !reference.is_empty() && !current.is_empty(),
        "PSI needs nonempty reference and current windows",
    )?;
    require(
        reference
            .iter()
            .chain(current)
            .chain(edges)
            .all(|v| v.is_finite())
            && edges.windows(2).all(|w| w[0] < w[1]),
        "PSI needs finite samples and increasing bin edges",
    )?;
    let count = |values: &[f64]| {
        let mut bins = vec![0.5; edges.len() + 1];
        for value in values {
            bins[edges.partition_point(|edge| *value > *edge)] += 1.0;
        }
        let total = values.len() as f64 + 0.5 * bins.len() as f64;
        bins.into_iter().map(|b| b / total).collect::<Vec<_>>()
    };
    let a = count(reference);
    let b = count(current);
    let scale = 1.0 / reference.len() as f64 + 1.0 / current.len() as f64;
    let degrees_of_freedom = edges.len() as f64;
    Ok(PsiResult {
        value: a.iter().zip(b).map(|(a, b)| (b - a) * (b / a).ln()).sum(),
        expected_without_drift: scale * degrees_of_freedom,
        noise_floor: scale
            * (degrees_of_freedom + PSI_NOISE_DEVIATIONS * (2.0 * degrees_of_freedom).sqrt()),
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct DistributionDrift {
    pub ks: KsResult,
    /// Missing when no bin edges were given.
    pub population_stability_index: Option<f64>,
    /// Mean PSI of two windows of these sizes drawn from one distribution.
    pub psi_expected_without_drift: Option<f64>,
    /// PSI that windows of these sizes without drift exceed in about 1% of comparisons.
    pub psi_noise_floor: Option<f64>,
    /// Missing when PSI was not computed, or when the noise floor reaches the threshold and PSI
    /// does not pass it.
    pub psi_drift: Option<bool>,
    /// Why psi_drift is missing.
    pub psi_note: Option<String>,
}

/// KS always compares the windows. PSI needs bin edges, and its drift flag is withheld where
/// sampling noise alone could reach `psi_threshold`.
pub fn detect_distribution_drift(
    reference: &[f64],
    current: &[f64],
    alpha: f64,
    edges: &[f64],
    psi_threshold: f64,
) -> Result<DistributionDrift> {
    require(
        psi_threshold.is_finite() && psi_threshold > 0.0,
        "PSI threshold must be positive",
    )?;
    let ks = ks_test(reference, current, alpha)?;
    if edges.is_empty() {
        return Ok(DistributionDrift {
            ks,
            population_stability_index: None,
            psi_expected_without_drift: None,
            psi_noise_floor: None,
            psi_drift: None,
            psi_note: Some("No bin edges were given, so only KS compared the windows".into()),
        });
    }
    let psi = population_stability(reference, current, edges)?;
    let psi_drift = psi.drift(psi_threshold);
    let psi_note = psi_drift.is_none().then(|| {
        format!(
            "PSI {:.4} does not exceed its noise floor {:.4} for windows of {} reference and {} \
             current values, and that floor reaches the threshold {psi_threshold}; compare larger \
             windows or use fewer bins",
            psi.value,
            psi.noise_floor,
            reference.len(),
            current.len()
        )
    });
    Ok(DistributionDrift {
        ks,
        population_stability_index: Some(psi.value),
        psi_expected_without_drift: Some(psi.expected_without_drift),
        psi_noise_floor: Some(psi.noise_floor),
        psi_drift,
        psi_note,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PageHinkley {
    pub delta: f64,
    pub threshold: f64,
    pub minimum_samples: usize,
    count: usize,
    mean: f64,
    cumulative: f64,
    minimum: f64,
}
impl PageHinkley {
    pub fn new(delta: f64, threshold: f64, minimum_samples: usize) -> Result<Self> {
        require(
            delta.is_finite()
                && delta >= 0.0
                && threshold.is_finite()
                && threshold > 0.0
                && minimum_samples > 0,
            "Invalid Page-Hinkley parameters",
        )?;
        Ok(Self {
            delta,
            threshold,
            minimum_samples,
            count: 0,
            mean: 0.0,
            cumulative: 0.0,
            minimum: 0.0,
        })
    }
    /// Detects increases in a monitored loss or error rate; reset after accepting a new baseline.
    pub fn update(&mut self, value: f64) -> Result<bool> {
        require(value.is_finite(), "Drift observation must be finite")?;
        self.count += 1;
        self.mean += (value - self.mean) / self.count as f64;
        self.cumulative += value - self.mean - self.delta;
        self.minimum = self.minimum.min(self.cumulative);
        Ok(self.count >= self.minimum_samples && self.cumulative - self.minimum > self.threshold)
    }
    pub fn reset(&mut self) {
        self.count = 0;
        self.mean = 0.0;
        self.cumulative = 0.0;
        self.minimum = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drift_statistics_identify_shift_and_handle_ties() {
        let a = vec![0.0; 100];
        let b = vec![1.0; 100];
        assert_eq!(ks_test(&a, &a, 0.05).unwrap().statistic, 0.0);
        assert!(ks_test(&a, &b, 0.05).unwrap().drift);
        assert_eq!(population_stability_index(&a, &a, &[0.5]).unwrap(), 0.0);
        assert!(population_stability_index(&a, &b, &[0.5]).unwrap() > 1.0);
    }
    #[test]
    fn psi_rejects_a_single_bin_and_windows_too_small_for_their_bins() {
        let reference: Vec<f64> = (0..1000).map(|i| i as f64).collect();
        let deciles: Vec<f64> = (1..10).map(|i| i as f64 * 100.0 - 0.5).collect();
        assert!(population_stability_index(&reference, &reference, &[]).is_err());
        assert!(population_stability_index(&reference, &reference[..49], &deciles).is_err());
        assert!(population_stability_index(&reference[..49], &reference, &deciles).is_err());
        let current: Vec<f64> = (0..50).map(|i| i as f64 * 20.0).collect();
        assert!(population_stability_index(&reference, &current, &deciles).unwrap() < 0.1);
    }
    #[test]
    fn psi_drift_is_withheld_while_the_noise_floor_reaches_the_threshold() {
        let reference: Vec<f64> = (0..1000).map(|i| i as f64).collect();
        let deciles: Vec<f64> = (1..10).map(|i| i as f64 * 100.0 - 0.5).collect();
        let window = |scale: usize| -> Vec<f64> {
            [12usize, 0, 5, 5, 5, 5, 5, 5, 4, 4]
                .iter()
                .enumerate()
                .flat_map(|(decile, count)| {
                    (0..count * scale).map(move |k| (decile * 100 + k % 100) as f64)
                })
                .collect()
        };
        let small = population_stability(&reference, &window(1), &deciles).unwrap();
        assert!((small.expected_without_drift - 9.0 * (1.0 / 1000.0 + 1.0 / 50.0)).abs() < 1e-12);
        assert!(small.value > 0.25 && small.value < small.noise_floor);
        assert_eq!(small.drift(0.25), None);
        assert_eq!(small.drift(1.0), Some(false));
        let large = population_stability(&reference, &window(20), &deciles).unwrap();
        assert!(large.noise_floor < 0.25 && large.value > 0.25);
        assert_eq!(large.drift(0.25), Some(true));
        let same = population_stability(&reference, &reference, &deciles).unwrap();
        assert_eq!(same.drift(0.25), Some(false));
    }
    #[test]
    fn same_distribution_windows_rarely_report_psi_drift() {
        let mut rng = crate::Rng::new(7);
        let deciles: Vec<f64> = (1..10).map(|i| i as f64 / 10.0).collect();
        let (mut above_threshold, mut flagged) = (0, 0);
        for _ in 0..200 {
            let reference: Vec<f64> = (0..1000).map(|_| rng.uniform()).collect();
            let current: Vec<f64> = (0..50).map(|_| rng.uniform()).collect();
            let report =
                detect_distribution_drift(&reference, &current, 0.05, &deciles, 0.25).unwrap();
            above_threshold += usize::from(report.population_stability_index.unwrap() > 0.25);
            flagged += usize::from(report.psi_drift == Some(true));
        }
        assert!(above_threshold >= 20, "{above_threshold}");
        assert!(flagged <= 4, "{flagged}");
    }
    #[test]
    fn ks_still_compares_windows_that_psi_cannot_judge() {
        let reference: Vec<f64> = (0..1000).map(|i| i as f64).collect();
        let deciles: Vec<f64> = (1..10).map(|i| i as f64 * 100.0 - 0.5).collect();
        let moved: Vec<f64> = (0..20).map(|i| 2000.0 + i as f64).collect();
        let spread: Vec<f64> = (0..20).map(|i| i as f64 * 50.0).collect();
        let unbinned = detect_distribution_drift(&reference, &moved, 0.05, &[], 0.25).unwrap();
        assert!(unbinned.ks.drift);
        assert!(unbinned.population_stability_index.is_none() && unbinned.psi_drift.is_none());
        assert!(unbinned.psi_note.is_some());
        assert!(population_stability_index(&reference, &spread, &deciles).is_err());
        let small = detect_distribution_drift(&reference, &spread, 0.05, &deciles, 0.25).unwrap();
        assert!(!small.ks.drift);
        assert_eq!(small.psi_drift, None);
        assert!(small.psi_note.unwrap().contains("noise floor"));
        let shifted = detect_distribution_drift(&reference, &moved, 0.05, &deciles, 0.25).unwrap();
        assert!(shifted.ks.drift);
        assert_eq!(shifted.psi_drift, Some(true));
        assert!(shifted.psi_note.is_none());
        assert!(detect_distribution_drift(&reference, &moved, 0.05, &deciles, 0.0).is_err());
    }
    #[test]
    fn page_hinkley_detects_loss_increase_and_resets() {
        let mut detector = PageHinkley::new(0.01, 5.0, 20).unwrap();
        for _ in 0..100 {
            assert!(!detector.update(0.1).unwrap());
        }
        assert!((0..20).any(|_| detector.update(1.0).unwrap()));
        detector.reset();
        assert!(!detector.update(0.1).unwrap());
    }
}
