mod meta;
mod provider_id;
mod template;

use crate::provider_id::ProviderID;
use clap::Parser;
use futures::StreamExt;
use k8s_openapi::api::core::v1::Node;
use kube::{
    api::{ObjectMeta, PartialObjectMetaExt, Patch, PatchParams},
    runtime::{
        controller::{Action, Config},
        watcher, Controller,
    },
    Api, Client, ResourceExt,
};
use meta::{Annotation, Label};
use provider_id::ProviderIDError;
use std::{collections::BTreeMap, process::ExitCode, sync::Arc, time::Duration};
use thiserror::Error;
use tracing::{debug, error, info, warn};

const MANAGER: &str = "node-provider-labeler";
const DEFAULT_LABEL_NAME: &str = "provider-id";
const DEFAULT_TEMPLATE: &str = "{:last}";

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
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The label to set
    #[arg(short, long)]
    label: Option<String>,
    /// The template to use for the label value
    #[arg(short, long, default_value = DEFAULT_TEMPLATE)]
    template: String,
    /// The annotation to set
    #[arg(long)]
    annotation: Option<String>,
    /// The template to use for the annotation value
    #[arg(long, default_value = DEFAULT_TEMPLATE)]
    annotation_template: String,
    /// Requeue reconciliation of a node after this duration in seconds
    #[arg(long, default_value_t = 300)]
    requeue_duration: u64,
}

struct Ctx {
    client: Client,
    label: Option<Label>,
    template: String,
    annotation: Option<Annotation>,
    annotation_template: String,
    requeue_duration: u64,
}

async fn reconcile(node: Arc<Node>, ctx: Arc<Ctx>) -> Result<Action, Error> {
    let node_name = node
        .metadata
        .name
        .as_ref()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.name"))?;

    debug!({ node = node_name }, "reconciling");

    let provider_id = node
        .spec
        .as_ref()
        .ok_or_else(|| Error::MissingObjectKey(".spec"))?
        .provider_id
        .as_ref();

    if let Some(provider_id) = provider_id {
        let provider_id = ProviderID::new(provider_id)?;
        info!({ node = node_name, provider_id = provider_id.to_string(), provider = provider_id.provider() }, "found provider id");

        // .spec.providerID is immutable except from "" to valid
        // spec.providerID: Forbidden: node updates may not change providerID except from "" to valid

        let mut labels = node.metadata.labels.clone();
        if let Some(label_name) = &ctx.label {
            let value = template::label(&ctx.template, &provider_id)?;
            labels
                .as_mut()
                .unwrap_or(&mut BTreeMap::new())
                .insert(label_name.to_string(), value.to_string());
        }

        let mut annotations = node.metadata.annotations.clone();
        if let Some(annotation_name) = &ctx.annotation {
            let value = template::annotation(&ctx.annotation_template, &provider_id)?;
            annotations
                .as_mut()
                .unwrap_or(&mut BTreeMap::new())
                .insert(annotation_name.to_string(), value.to_string());
        }

        let patch = ObjectMeta {
            labels,
            annotations,
            ..Default::default()
        }
        .into_request_partial::<Node>();

        let node_api: Api<Node> = Api::all(ctx.client.clone());
        node_api
            .patch_metadata(
                node_name,
                &PatchParams::apply(MANAGER),
                &Patch::Apply(&patch),
            )
            .await?;
    } else {
        warn!({ node = node_name }, "no provider id found");
    }

    Ok(Action::requeue(Duration::from_secs(ctx.requeue_duration)))
}

fn error_policy(object: Arc<Node>, error: &Error, _ctx: Arc<Ctx>) -> Action {
    let name = object.name_any();
    error!({ node = name }, "error processing node: {}", error);
    Action::requeue(Duration::from_secs(5))
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt::init();
    if let Err(e) = run_controller().await {
        error!({ error = e.to_string() }, "unable to run controller");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

async fn run_controller() -> color_eyre::Result<()> {
    info!("starting");
    let args = Args::parse();
    let client = Client::try_default().await?;
    let node: Api<Node> = Api::all(client.clone());
    let requeue_duration = args.requeue_duration;

    let mut label = args.label.map(|s| s.parse::<Label>()).transpose()?;
    let template = args.template;

    let annotation = args
        .annotation
        .map(|s| s.parse::<Annotation>())
        .transpose()?;
    let annotation_template = args.annotation_template;

    // if neither label or annotation is configured, use a default label
    if annotation.is_none() && label.is_none() {
        label = Some(DEFAULT_LABEL_NAME.parse::<Label>()?);
    }

    Controller::new(node, watcher::Config::default())
        .with_config(Config::default().concurrency(2))
        .shutdown_on_signal()
        .run(
            reconcile,
            error_policy,
            Arc::new(Ctx {
                client,
                label,
                template,
                annotation,
                annotation_template,
                requeue_duration,
            }),
        )
        .for_each(|res| async move {
            match res {
                Ok(o) => {
                    let node_name = o.0.clone().name;
                    info!({ node = node_name }, "reconciled");
                }
                Err(e) => info!("reconcile error: {:?}", e),
            }
        })
        .await;

    info!("stopping");

    Ok(())
}
