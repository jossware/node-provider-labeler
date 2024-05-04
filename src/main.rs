mod provider_id;

use crate::provider_id::ProviderID;
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

#[derive(Error, Debug)]
enum Error {
    #[error("kube error: {0}")]
    Kube(#[from] kube::Error),
    #[error("MissingObjectKey: {0}")]
    MissingObjectKey(&'static str),
    #[error("ProviderIDError: {0}")]
    ProviderID(#[from] ProviderIDError),
}

#[derive(Debug, Clone)]
enum ProviderIDPart {
    All,
    Last,
    First,
    Nth(usize),
}

struct Ctx {
    client: Client,
    label_name: String,
    provider_id_value: ProviderIDPart,
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
        info!({ node = node_name, provider_id = provider_id.to_string(), provider = provider_id.name() }, "found provider id");

        // .spec.providerID is immutable except from "" to valid
        // spec.providerID: Forbidden: node updates may not change providerID except from "" to valid

        let value = match &ctx.provider_id_value {
            ProviderIDPart::All => provider_id.node_id(),
            ProviderIDPart::Last => provider_id.last(),
            ProviderIDPart::First => provider_id.nth(0).unwrap_or_else(|| provider_id.node_id()),
            ProviderIDPart::Nth(i) => {
                if let Some(v) = provider_id.nth(*i) {
                    v
                } else {
                    warn!({ node = node_name, index = i }, "nth index out of bounds");
                    provider_id.node_id()
                }
            }
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
    let client = Client::try_default().await?;
    let node: Api<Node> = Api::all(client.clone());
    let label_name = "provider-id".to_string();

    Controller::new(node, watcher::Config::default())
        .with_config(Config::default().concurrency(2))
        .shutdown_on_signal()
        .run(
            reconcile,
            error_policy,
            Arc::new(Ctx {
                client,
                label_name,
                provider_id_value: ProviderIDPart::Last,
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
