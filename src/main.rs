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
use meta::MetadataKey;
use provider_id::ProviderIDError;
use std::{collections::BTreeMap, process::ExitCode, str::FromStr, sync::Arc, time::Duration};
use template::{AnnotationTemplate, LabelTemplate, Template};
use thiserror::Error;
use tracing::{debug, error, info, warn};

const MANAGER: &str = "node-provider-labeler";
const DEFAULT_KEY_NAME: &str = "provider-id";
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
    #[error("MetadataKeyError: {0}")]
    MetadataKey(String),
}

// TODO: update help text
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The label key and optional template to use for the label value
    #[arg(
        short,
        long,
        long_help = "The label key and optional template to use for the label value.\nThe default is \"provider-id={:last}\" if no other labels or annotations are configured.\nExample: label-key={:last}"
    )]
    label: Option<String>,
    /// The annotation key and optional template to use for the annotation value
    #[arg(
        short,
        long,
        long_help = "The annotation key and optional template to use for the annotation value.\nIf not specified, the default template is \"{:last}\".\nExample: annotation-key={0}"
    )]
    annotation: Option<String>,
    /// Requeue reconciliation of a node after this duration in seconds
    #[arg(long, default_value_t = 300)]
    requeue_duration: u64,
}

#[derive(Debug)]
struct Renderer<T>
where
    T: std::fmt::Debug + std::default::Default + Template + std::str::FromStr,
    Error: std::convert::From<<T as std::str::FromStr>::Err>,
{
    key: MetadataKey,
    template: T,
}

impl<T> Default for Renderer<T>
where
    T: std::fmt::Debug + std::default::Default + Template + std::str::FromStr,
    Error: std::convert::From<<T as std::str::FromStr>::Err>,
{
    fn default() -> Self {
        Self {
            key: DEFAULT_KEY_NAME.parse::<MetadataKey>().unwrap(),
            template: T::from_str(DEFAULT_TEMPLATE).unwrap_or_default(),
        }
    }
}

impl<T> FromStr for Renderer<T>
where
    T: std::fmt::Debug + std::default::Default + Template + std::str::FromStr,
    Error: std::convert::From<<T as std::str::FromStr>::Err>,
{
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Self::default());
        }
        let parts = s.splitn(2, '=').collect::<Vec<&str>>();
        let key = parts[0]
            .parse::<MetadataKey>()
            .map_err(|e| Error::MetadataKey(e.to_string()))?;
        let template = if parts.len() > 1 {
            parts[1]
        } else {
            DEFAULT_TEMPLATE
        };
        let template = T::from_str(template)?;

        Ok(Self { key, template })
    }
}

struct Ctx {
    client: Client,
    label: Option<Renderer<LabelTemplate>>,
    annotation: Option<Renderer<AnnotationTemplate>>,
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
        if let Some(renderer) = &ctx.label {
            let value = renderer.template.render(&provider_id)?;
            labels
                .as_mut()
                .unwrap_or(&mut BTreeMap::new())
                .insert(renderer.key.to_string(), value.to_string());
        }

        let mut annotations = node.metadata.annotations.clone();
        if let Some(labeler) = &ctx.annotation {
            let value = labeler.template.render(&provider_id)?;
            annotations
                .as_mut()
                .unwrap_or(&mut BTreeMap::new())
                .insert(labeler.key.to_string(), value.to_string());
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

    let mut label = args
        .label
        .map(|s| s.parse::<Renderer<LabelTemplate>>())
        .transpose()?;

    let annotation = args
        .annotation
        .map(|s| s.parse::<Renderer<AnnotationTemplate>>())
        .transpose()?;

    // if neither label or annotation is configured, use a default label
    if annotation.is_none() && label.is_none() {
        label = Some(Renderer::default());
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
                annotation,
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
