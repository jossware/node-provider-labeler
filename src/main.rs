mod provider_id;

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
use std::{str::FromStr, sync::Arc, time::Duration};
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
}

#[derive(Debug, Clone, PartialEq)]
enum ProviderIDPart {
    All,
    Last,
    First,
    Nth(usize),
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The label to set
    #[arg(short, long)]
    label: Option<String>,
    /// The provider id part to use
    #[arg(short, long)]
    provider_part: Option<String>,
}

impl FromStr for ProviderIDPart {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "all" => Ok(Self::All),
            "last" => Ok(Self::Last),
            "first" => Ok(Self::First),
            _ => {
                let idx = s.parse::<usize>().map_err(Error::ParseInt)?;
                Ok(Self::Nth(idx))
            }
        }
    }
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
        info!({ node = node_name, provider_id = provider_id.to_string(), provider = provider_id.provider() }, "found provider id");

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
        }
        .replace('/', "_");

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
    let provider_id_value: ProviderIDPart = args
        .provider_part
        .unwrap_or_else(|| "last".to_string())
        .parse()?;

    Controller::new(node, watcher::Config::default())
        .with_config(Config::default().concurrency(2))
        .shutdown_on_signal()
        .run(
            reconcile,
            error_policy,
            Arc::new(Ctx {
                client,
                label_name,
                provider_id_value,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id_part() {
        let valid = [
            ("all", ProviderIDPart::All),
            ("ALL", ProviderIDPart::All),
            ("ALl", ProviderIDPart::All),
            ("last", ProviderIDPart::Last),
            ("LAST", ProviderIDPart::Last),
            ("Last", ProviderIDPart::Last),
            ("first", ProviderIDPart::First),
            ("FIRST", ProviderIDPart::First),
            ("First", ProviderIDPart::First),
            ("0", ProviderIDPart::Nth(0)),
            ("1", ProviderIDPart::Nth(1)),
            ("2", ProviderIDPart::Nth(2)),
            ("3", ProviderIDPart::Nth(3)),
            ("4", ProviderIDPart::Nth(4)),
            ("5", ProviderIDPart::Nth(5)),
            ("6", ProviderIDPart::Nth(6)),
            ("7", ProviderIDPart::Nth(7)),
            ("8", ProviderIDPart::Nth(8)),
            ("9", ProviderIDPart::Nth(9)),
        ];

        for test in valid {
            let (input, expected) = test;
            let p = ProviderIDPart::from_str(input).unwrap();
            assert_eq!(p, expected);
        }

        let invalid = ["huh", "", " ", "-", "fsdfds", "akljsf dajdk  sjdf"];
        for test in invalid {
            let p = ProviderIDPart::from_str(test);
            assert!(p.is_err());
            let e = p.unwrap_err();
            assert!(matches!(Some(e), Some(Error::ParseInt(_))));
        }
    }
}
