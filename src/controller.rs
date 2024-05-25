use crate::{
    meta::MetadataKey,
    provider_id::ProviderID,
    template::{AnnotationTemplate, LabelTemplate, Template},
    Error,
};
use futures::StreamExt;
use k8s_openapi::api::core::v1::Node;
use kube::{
    api::{ObjectMeta, PartialObjectMetaExt, Patch, PatchParams},
    runtime::{controller::Action, watcher, Config, Controller},
    Api, Client, ResourceExt,
};
use std::{str::FromStr, sync::Arc, time::Duration};
use tracing::{debug, error, info, warn};

const MANAGER: &str = "node-provider-labeler";
const DEFAULT_KEY_NAME: &str = "provider-id";
const DEFAULT_TEMPLATE: &str = "{:last}";

type MetadataPairs = std::collections::BTreeMap<String, String>;

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
        debug!({ node = node_name, provider_id = provider_id.to_string(), provider = provider_id.provider() }, "found provider id");

        let (new_labels, old_labels) =
            calculate_metadata_pairs(node.metadata.labels.clone(), &ctx.labels, &provider_id)?;

        let (new_annotations, old_annotations) = calculate_metadata_pairs(
            node.metadata.annotations.clone(),
            &ctx.annotations,
            &provider_id,
        )?;

        if new_labels == old_labels && new_annotations == old_annotations {
            debug!({ node = node_name }, "no changes to apply");
            return Ok(Action::requeue(Duration::from_secs(ctx.requeue_duration)));
        }

        let payload = ObjectMeta {
            labels: Some(new_labels),
            annotations: Some(new_annotations),
            ..Default::default()
        };
        info!({ node = node_name }, "patching");
        debug!({ node = node_name }, "payload {:?}", payload);
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

pub(crate) async fn run(
    label_templates: Option<Vec<String>>,
    annotation_templates: Option<Vec<String>>,
    requeue_duration: u64,
) -> Result<(), Error> {
    let client = Client::try_default().await?;
    let node: Api<Node> = Api::all(client.clone());

    let mut labels = parse_renderers(label_templates)?;
    let annotations = parse_renderers(annotation_templates)?;

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

fn calculate_metadata_pairs<T>(
    current: Option<MetadataPairs>,
    renderers: &Option<Vec<Renderer<T>>>,
    provider_id: &ProviderID,
) -> Result<(MetadataPairs, MetadataPairs), Error>
where
    T: std::fmt::Debug + std::default::Default + Template + std::str::FromStr,
    Error: std::convert::From<<T as std::str::FromStr>::Err>,
{
    let current = current.unwrap_or_default();
    let mut old = MetadataPairs::new();
    let mut new = MetadataPairs::new();

    if let Some(renderers) = renderers {
        for r in renderers {
            let key = r.key.to_string();
            let value = r.template.render(provider_id)?;
            if let Some(v) = current.get(&key).cloned() {
                old.insert(key.clone(), v);
            }
            new.insert(key, value);
        }
    }

    Ok((new, old))
}

fn parse_renderers<T>(args: Option<Vec<String>>) -> Result<Option<Vec<Renderer<T>>>, Error>
where
    T: std::fmt::Debug + std::default::Default + Template + std::str::FromStr,
    Error: std::convert::From<<T as std::str::FromStr>::Err>,
{
    args.map(|list| {
        list.iter()
            .map(|s| s.parse::<Renderer<T>>())
            .collect::<Result<Vec<_>, _>>()
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_metadata_pairs() {
        let provider_id = ProviderID::new("fake://region/instance").unwrap();

        {
            // no renderers
            let renderers: Option<Vec<Renderer<LabelTemplate>>> = None;
            let current = Some(MetadataPairs::new());
            let (old, new) = calculate_metadata_pairs(current, &renderers, &provider_id).unwrap();
            assert_eq!(old, new);
            assert!(new.is_empty());
        }

        {
            // new node with single default renderer
            let renderer: Renderer<LabelTemplate> = Renderer::default();
            let renderers = Some(vec![renderer]);
            let current = Some(MetadataPairs::new());
            let (new, old) = calculate_metadata_pairs(current, &renderers, &provider_id).unwrap();
            assert_ne!(new, old);
            assert!(!new.is_empty());
            assert_eq!("instance", new.get("provider-id").unwrap());
        }

        {
            // already reconciled node
            let renderers: Option<Vec<Renderer<AnnotationTemplate>>> = Some(vec![
                Renderer::from_str("some={:last}").unwrap(),
                Renderer::from_str("other={:first}").unwrap(),
            ]);
            let mut current = MetadataPairs::new();
            current.insert("some".to_string(), "instance".to_string());
            current.insert("other".to_string(), "region".to_string());
            let (new, old) =
                calculate_metadata_pairs(Some(current), &renderers, &provider_id).unwrap();
            assert_eq!(new, old);
            assert!(!new.is_empty());
            assert_eq!("instance", new.get("some").unwrap());
            assert_eq!("region", new.get("other").unwrap());
        }

        {
            // node with one key missing
            let renderers: Option<Vec<Renderer<AnnotationTemplate>>> = Some(vec![
                Renderer::from_str("some={:last}").unwrap(),
                Renderer::from_str("other={:first}").unwrap(),
            ]);
            let mut current = MetadataPairs::new();
            current.insert("some".to_string(), "instance".to_string());
            let (new, old) =
                calculate_metadata_pairs(Some(current), &renderers, &provider_id).unwrap();
            assert_ne!(new, old);
            assert!(!new.is_empty());
            assert_eq!("instance", new.get("some").unwrap());
            assert_eq!("region", new.get("other").unwrap());
        }

        {
            // node with one different value
            let renderers: Option<Vec<Renderer<AnnotationTemplate>>> = Some(vec![
                Renderer::from_str("some={:last}").unwrap(),
                Renderer::from_str("other={:first}").unwrap(),
            ]);
            let mut current = MetadataPairs::new();
            current.insert("some".to_string(), "instance".to_string());
            current.insert("other".to_string(), "notregion".to_string());
            let (new, old) =
                calculate_metadata_pairs(Some(current), &renderers, &provider_id).unwrap();
            assert_ne!(new, old);
            assert!(!new.is_empty());
            assert_eq!("instance", new.get("some").unwrap());
            assert_eq!("region", new.get("other").unwrap());
        }
    }
}
