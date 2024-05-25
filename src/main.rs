mod controller;
mod meta;
mod provider_id;
mod template;

use clap::Parser;
use provider_id::ProviderIDError;
use std::process::ExitCode;
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
enum Error {
    #[error("kube error: {0}")]
    Kube(#[from] kube::Error),
    #[error("MissingObjectKey: {0}")]
    MissingObjectKey(&'static str),
    #[error("ProviderIDError: {0}")]
    ProviderID(#[from] ProviderIDError),
    #[error("ParseIntError: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("TemplateParseError: {0}")]
    TemplateParser(String),
    #[error("MetadataKeyError: {0}")]
    MetadataKey(String),
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The label key and optional template to use for the label value.
    /// The default is "provider-id={:last}" if no other labels or annotations are configured.
    /// Can be repeated to add multiple labels.
    ///
    /// Examples:
    /// * --label=label-key
    /// * --label=label-key={:last} --label=other-label-key={0}-{1}
    #[arg(short, long, verbatim_doc_comment)]
    label: Option<Vec<String>>,
    /// The annotation key and optional template to use for the annotation value
    /// Can be repeated to add multiple annotations.
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

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt::init();
    if let Err(e) = controller::run_controller().await {
        error!({ error = e.to_string() }, "unable to run controller");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
