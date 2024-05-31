use prometheus::{HistogramVec, IntCounter, IntCounterVec};

#[derive(Clone)]
pub(crate) struct Metrics {
    pub reconciliations: IntCounterVec,
    pub reconciliation_failures: IntCounterVec,
    pub controller_failures: IntCounter,
    pub reconcile_duration: HistogramVec,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            reconciliations: IntCounterVec::new(
                prometheus::Opts::new("reconciliations", "Number of reconciliations"),
                &["name"],
            )
            .unwrap(),
            reconciliation_failures: IntCounterVec::new(
                prometheus::Opts::new(
                    "reconciliation_failures",
                    "Number of reconciliation failures",
                ),
                &["name"],
            )
            .unwrap(),
            reconcile_duration: HistogramVec::new(
                prometheus::HistogramOpts::new("reconcile_duration", "Reconciliation duration"),
                &["name"],
            )
            .unwrap(),
            controller_failures: IntCounter::new(
                "controller_failures",
                "Number of controller failures",
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

    pub(crate) fn observe_reconciliation(&self, name: &str) {
        self.reconciliations.with_label_values(&[name]).inc();
    }

    pub(crate) fn observe_reconciliation_failure(&self, name: &str) {
        self.reconciliation_failures
            .with_label_values(&[name])
            .inc();
    }

    pub(crate) fn observe_controller_failure(&self) {
        self.controller_failures.inc();
    }
}
