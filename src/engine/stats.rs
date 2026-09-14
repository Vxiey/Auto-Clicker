#[derive(Debug, Clone, Copy, Default)]
pub struct SampleStats {
    pub mean_us: f64,
    pub p50_us: f64,
    pub p95_us: f64,
    pub p99_us: f64,
    pub worst_us: f64,
}

#[derive(Debug, Clone, Default)]
pub struct BenchmarkStats {
    pub target_cps: f64,
    pub actual_cps: f64,
    pub elapsed_seconds: f64,
    pub clicks: u64,
    pub missed_deadlines: u64,
    pub interval: SampleStats,
    pub jitter: SampleStats,
}

impl BenchmarkStats {
    pub fn from_samples(
        target_cps: f64,
        elapsed_seconds: f64,
        click_timestamps_us: &[f64],
        deadline_errors_us: &[f64],
        missed_deadlines: u64,
    ) -> Self {
        let actual_cps = if elapsed_seconds > 0.0 {
            click_timestamps_us.len() as f64 / elapsed_seconds
        } else {
            0.0
        };

        let intervals: Vec<f64> = click_timestamps_us
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .collect();

        Self {
            target_cps,
            actual_cps,
            elapsed_seconds,
            clicks: click_timestamps_us.len() as u64,
            missed_deadlines,
            interval: summarize(&intervals),
            jitter: summarize(deadline_errors_us),
        }
    }
}

fn summarize(values: &[f64]) -> SampleStats {
    if values.is_empty() {
        return SampleStats::default();
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;

    SampleStats {
        mean_us: mean,
        p50_us: percentile(&sorted, 0.50),
        p95_us: percentile(&sorted, 0.95),
        p99_us: percentile(&sorted, 0.99),
        worst_us: sorted.iter().copied().map(f64::abs).fold(0.0, f64::max),
    }
}

fn percentile(sorted: &[f64], percentile: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
    sorted[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_stats_compute_expected_cps() {
        let timestamps = vec![0.0, 1_000.0, 2_000.0, 3_000.0];
        let jitter = vec![0.0, 2.0, -1.0, 1.0];
        let stats = BenchmarkStats::from_samples(1_000.0, 0.004, &timestamps, &jitter, 0);
        assert_eq!(stats.clicks, 4);
        assert!((stats.actual_cps - 1_000.0).abs() < f64::EPSILON);
        assert!((stats.interval.mean_us - 1_000.0).abs() < f64::EPSILON);
    }
}
