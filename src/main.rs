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
    labels: Option<Vec<Renderer<LabelTemplate>>>,
    annotations: Option<Vec<Renderer<AnnotationTemplate>>>,
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

        let mut payload = ObjectMeta::default();

        if let Some(renderers) = &ctx.labels {
            payload.labels = Some(render(renderers, &provider_id)?);
        }

        if let Some(renderers) = &ctx.annotations {
            payload.annotations = Some(render(renderers, &provider_id)?);
        }

        debug!({ node = node_name }, "patching {:?}", payload);
        let patch = payload.into_request_partial::<Node>();
        let node_api: Api<Node> = Api::all(ctx.client.clone());
        node_api
            .patch_metadata(
                node_name,
                &PatchParams::apply(MANAGER).force(),
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
    let args = Args::parse();
    let client = Client::try_default().await?;
    let node: Api<Node> = Api::all(client.clone());
    let requeue_duration = args.requeue_duration;

    let mut labels = parse_renderers(args.label);
    let annotations = parse_renderers(args.annotation);

    // if neither labels or annotations are configured, use a default label and
    // template
    if annotations.is_none() && labels.is_none() {
        labels = Some(vec![Renderer::default()]);
    }

    info!("starting");
    debug!({ labels = ?labels, annotation = ?annotations }, "config");
    Controller::new(node, watcher::Config::default())
        .with_config(Config::default().concurrency(2))
        .shutdown_on_signal()
        .run(
            reconcile,
            error_policy,
            Arc::new(Ctx {
                client,
                labels,
                annotations,
                requeue_duration,
            }),
        )
        .for_each(|res| async move {
            match res {
                Ok(o) => {
                    let node_name = o.0.clone().name;
                    debug!({ node = node_name }, "reconciled");
                }
                Err(e) => error!("reconcile error: {:?}", e),
            }
        })
        .await;

    info!("stopping");

    Ok(())
}

fn render<T>(
    renderers: &[Renderer<T>],
    provider_id: &ProviderID,
) -> Result<BTreeMap<String, String>, Error>
where
    T: std::fmt::Debug + std::default::Default + Template + std::str::FromStr,
    Error: std::convert::From<<T as std::str::FromStr>::Err>,
{
    let mut fields = BTreeMap::new();
    for renderer in renderers.iter() {
        let key = renderer.key.to_string();
        let value = renderer.template.render(provider_id)?;
        fields.insert(key, value);
    }
    Ok(fields)
}

fn parse_renderers<T>(args: Option<Vec<String>>) -> Option<Vec<Renderer<T>>>
where
    T: std::fmt::Debug + std::default::Default + Template + std::str::FromStr,
    Error: std::convert::From<<T as std::str::FromStr>::Err>,
{
    if let Some(inner) = args {
        let x = inner
            .iter()
            .map(|s| s.parse::<Renderer<T>>())
            .collect::<Result<Vec<_>, _>>();
        x.ok()
    } else {
        None
    }
}
