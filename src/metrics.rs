use prometheus::{HistogramVec, IntCounterVec, Opts};

#[derive(Clone)]
pub(crate) struct Metrics {
    pub reconciliations: IntCounterVec,
    pub reconciliation_failures: IntCounterVec,
    pub controller_failures: IntCounterVec,
    pub object_not_found: IntCounterVec,
    pub reconcile_duration: HistogramVec,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            reconciliations: IntCounterVec::new(
                Opts::new("reconciliations", "Number of reconciliations"),
                &["name"],
            )
            .unwrap(),
            reconciliation_failures: IntCounterVec::new(
                Opts::new(
                    "reconciliation_failures",
                    "Number of reconciliation failures",
                ),
                &["name"],
            )
            .unwrap(),
            object_not_found: IntCounterVec::new(
                Opts::new(
                    "object_not_found_errors",
                    "Number of object not found errors",
                ),
                &["name"],
            )
            .unwrap(),
            reconcile_duration: HistogramVec::new(
                prometheus::HistogramOpts::new("reconcile_duration", "Reconciliation duration"),
                &["name"],
            )
            .unwrap(),
            controller_failures: IntCounterVec::new(
                Opts::new("controller_failures", "Number of controller failures"),
                &["type"],
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

    pub(crate) fn observe_controller_failure(&self, err_type: &str) {
        self.controller_failures
            .with_label_values(&[err_type])
            .inc();
    }

    pub(crate) fn observe_object_not_found_error(&self, name: &str) {
        self.object_not_found.with_label_values(&[name]).inc();
    }
}
