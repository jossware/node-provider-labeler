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
use provider_id::ProviderIDError;
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tracing::{debug, error, info, warn};

const MANAGER: &str = "node-provider-labeler";
const DEFAULT_LABEL_NAME: &str = "provider-id";

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
    #[arg(short, long)]
    template: Option<String>,
    #[arg(long)]
    annotation: bool,
}

struct Ctx {
    client: Client,
    label_name: String,
    template: String,
    annotation: bool,
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

        let value = if ctx.annotation {
            template::annotation(&ctx.template, &provider_id)?
        } else {
            template::label(&ctx.template, &provider_id)?.replace('/', "_")
        };

        let mut labels = node.metadata.labels.clone().unwrap_or_default();
        labels.insert(ctx.label_name.clone(), value.to_string());

        let patch = ObjectMeta {
            labels: Some(labels),
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

    Ok(Action::requeue(Duration::from_secs(300)))
}

fn error_policy(object: Arc<Node>, error: &Error, _ctx: Arc<Ctx>) -> Action {
    let name = object.name_any();
    error!({ node = name }, "error processing node: {}", error);
    Action::requeue(Duration::from_secs(5))
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    tracing_subscriber::fmt::init();

    info!("starting");
    let args = Args::parse();
    let client = Client::try_default().await?;
    let node: Api<Node> = Api::all(client.clone());
    let label_name = args.label.unwrap_or_else(|| DEFAULT_LABEL_NAME.to_string());
    let template = args.template.unwrap_or_else(|| "{:last}".to_string());
    let annotation = args.annotation;

    Controller::new(node, watcher::Config::default())
        .with_config(Config::default().concurrency(2))
        .shutdown_on_signal()
        .run(
            reconcile,
            error_policy,
            Arc::new(Ctx {
                client,
                label_name,
                template,
                annotation,
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
