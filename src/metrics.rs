use prometheus::{HistogramVec, IntCounter, IntCounterVec, Opts};
use tokio::time::Instant;

#[derive(Clone)]
pub(crate) struct Metrics {
    pub reconciliations: IntCounter,
    pub reconciliation_failures: IntCounter,
    pub controller_failures: IntCounterVec,
    pub object_not_found: IntCounter,
    pub reconcile_duration: HistogramVec,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            reconciliations: IntCounter::new("reconciliations", "Number of reconciliations")
                .unwrap(),
            reconciliation_failures: IntCounter::new(
                "reconciliation_failures",
                "Number of reconciliation failures",
            )
            .unwrap(),
            controller_failures: IntCounterVec::new(
                Opts::new("controller_failures", "Number of controller failures"),
                &["type"],
            )
            .unwrap(),
            object_not_found: IntCounter::new(
                "object_not_found_errors",
                "Number of object not found errors",
            )
            .unwrap(),
            reconcile_duration: HistogramVec::new(
                prometheus::HistogramOpts::new("reconcile_duration", "Reconciliation duration")
                    .buckets(vec![0.01, 0.1, 0.25, 0.5, 1., 5., 15., 60.]),
                &[],
            )
            .unwrap(),
        }
    }
}

impl Metrics {
    pub(crate) fn register(
        self,
        registry: &prometheus::Registry,
    ) -> Result<Self, prometheus::Error> {
        registry.register(Box::new(self.reconciliations.clone()))?;
        registry.register(Box::new(self.reconciliation_failures.clone()))?;
        registry.register(Box::new(self.reconcile_duration.clone()))?;
        registry.register(Box::new(self.controller_failures.clone()))?;
        Ok(self)
    }

    pub(crate) fn observe_reconciliation(&self) -> ReconciliationTimer {
        self.reconciliations.inc();
        ReconciliationTimer {
            start: Instant::now(),
            metric: self.reconcile_duration.clone(),
        }
    }

    pub(crate) fn observe_reconciliation_failure(&self) {
        self.reconciliation_failures.inc();
    }

    pub(crate) fn observe_controller_failure(&self, err_type: &str) {
        self.controller_failures
            .with_label_values(&[err_type])
            .inc();
    }

    pub(crate) fn observe_object_not_found_error(&self) {
        self.object_not_found.inc();
    }
}

pub struct ReconciliationTimer {
    start: Instant,
    metric: HistogramVec,
}

impl Drop for ReconciliationTimer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed().as_millis() as f64 / 1000.0;
        self.metric.with_label_values(&[]).observe(elapsed);
    }
}
