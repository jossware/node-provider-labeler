use futures::StreamExt;
use k8s_openapi::api::core::v1::Node;
use kube::{
    runtime::{
        controller::{Action, Config},
        watcher, Controller,
    },
    Api, Client,
};
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tracing::info;

type AppResult<T> = color_eyre::Result<T>;

#[derive(Error, Debug)]
enum Error {
    #[error("kube error: {0}")]
    Kube(#[from] kube::Error),
}

struct Data {
    client: Client,
}

async fn reconcile(node: Arc<Node>, _data: Arc<Data>) -> Result<Action, Error> {
    info!("reconciling {:?}", node.metadata.name);
    Ok(Action::requeue(Duration::from_secs(300)))
}

fn error_policy(_object: Arc<Node>, _error: &Error, _ctx: Arc<Data>) -> Action {
    Action::requeue(Duration::from_secs(1))
}

#[tokio::main]
async fn main() -> AppResult<()> {
    tracing_subscriber::fmt::init();

    info!("starting");
    let client = Client::try_default().await?;
    let node: Api<Node> = Api::all(client.clone());

    let config = Config::default().concurrency(2);
    let mut _c = Controller::new(node, watcher::Config::default())
        .with_config(config)
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
