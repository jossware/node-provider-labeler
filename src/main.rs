mod provider_id;

use crate::provider_id::ProviderID;
use futures::StreamExt;
use k8s_openapi::api::core::v1::Node;
use kube::{
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

#[derive(Error, Debug)]
enum Error {
    #[error("kube error: {0}")]
    Kube(#[from] kube::Error),
    #[error("MissingObjectKey: {0}")]
    MissingObjectKey(&'static str),
    #[error("ProviderIDError: {0}")]
    ProviderID(#[from] ProviderIDError),
}

struct Data {
    client: Client,
}

async fn reconcile(node: Arc<Node>, _data: Arc<Data>) -> Result<Action, Error> {
    let node_name = node
        .metadata
        .name
        .as_ref()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.name"))?;

    debug!("reconciling {node_name}");

    let provider_id = node
        .spec
        .as_ref()
        .ok_or_else(|| Error::MissingObjectKey(".spec"))?
        .provider_id
        .as_ref();

    if let Some(provider_id) = provider_id {
        let provider_id = ProviderID::new(provider_id)?;
        info!("provider id: {}", provider_id);
    } else {
        warn!("no provider id found for node: {node_name}");
    }

    Ok(Action::requeue(Duration::from_secs(300)))
}

fn error_policy(object: Arc<Node>, error: &Error, _ctx: Arc<Data>) -> Action {
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

    Controller::new(node, watcher::Config::default())
        .with_config(Config::default().concurrency(2))
        .shutdown_on_signal()
        .run(reconcile, error_policy, Arc::new(Data { client }))
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("reconciled {:?}", o),
                Err(e) => info!("reconcile error: {:?}", e),
            }
        })
        .await;

    info!("stopping");

    Ok(())
}
