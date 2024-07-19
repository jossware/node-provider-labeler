mod controller;
mod diagnostics;
mod meta;
mod metrics;

use axum::{extract, http::StatusCode, routing::get, Router};
use clap::Parser;
use diagnostics::Diagnostics;
use futures::TryFutureExt;
use node_provider_labeler::Error;
use prometheus::{Encoder, TextEncoder};
use std::{future::IntoFuture, process::ExitCode, sync::Arc};
use tokio::{net::TcpListener, sync::RwLock, task::JoinHandle};
use tracing::{error, warn};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The label key and optional template to use for the label value.
    /// The default is "provider-id={:last}" if there are no other labels or annotations configured.
    /// Repeat to add multiple labels.
    ///
    /// Examples:
    /// * --label=label-key
    /// * --label=label-key={:last} --label=other-label-key={0}-{1}
    #[arg(short, long, verbatim_doc_comment)]
    label: Option<Vec<String>>,
    /// The annotation key and optional template to use for the annotation value
    /// Repeat to add multiple annotations.
    ///
    /// Examples:
    /// * --annotation=annotation-key
    /// * --annotation=annotation-key={:last} --annotation=other-annotation-key={0}-{1}
    #[arg(short, long, verbatim_doc_comment)]
    annotation: Option<Vec<String>>,
    /// Requeue reconciliation of a node after this duration in seconds
    #[arg(long, default_value_t = 3600)]
    requeue_duration: u64,
}

#[derive(Clone, Debug, Default)]
struct State {
    diagnostics: Arc<RwLock<Diagnostics>>,
    /// Metrics registry
    registry: prometheus::Registry,
}

impl State {
    fn metrics(&self) -> Vec<prometheus::proto::MetricFamily> {
        self.registry.gather()
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let state = State::default();

    tracing::info!("initializing kubernetes client");
    let client = match kube::Client::try_default().await {
        Ok(client) => client,
        Err(e) => {
            error!({ error = e.to_string() }, "unable to create kube client");
            return ExitCode::FAILURE;
        }
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .with_state(state.clone());
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.unwrap();
        })
        .into_future()
        .map_err(Error::from);
    let controller = controller::run(
        client,
        state,
        args.label,
        args.annotation,
        args.requeue_duration,
    );

    tracing::info!("starting controller");
    tracing::info!("starting server");

    let controller = tokio::spawn(controller);
    let server = tokio::spawn(server);

    match tokio::try_join!(run(server, "server"), run(controller, "controller")) {
        Ok(_) => {}
        Err(_) => {
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

async fn run(handle: JoinHandle<Result<(), Error>>, name: &str) -> Result<(), Error> {
    match handle.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            error!("{name}: {e}");
            Err(e)
        }
        Err(e) => {
            error!("{name}: {e}");
            Err(e.into())
        }
    }
}

async fn metrics(extract::State(state): extract::State<State>) -> (StatusCode, Vec<u8>) {
    let m = state.metrics();
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    if let Err(e) = encoder.encode(&m, &mut buffer) {
        warn!({ error = e.to_string() }, "error encoding metrics");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal server error".into(),
        );
    }

    (StatusCode::OK, buffer)
}

async fn health(extract::State(state): extract::State<State>) -> (StatusCode, &'static str) {
    let err_count = state.diagnostics.write().await.error_count.refresh();
    if err_count > 0 {
        (StatusCode::INTERNAL_SERVER_ERROR, "Unhealthy")
    } else {
        (StatusCode::OK, "OK")
    }
}
