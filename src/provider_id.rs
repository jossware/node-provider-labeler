use thiserror::Error;

#[derive(Debug)]
pub struct ProviderID {
    provider_id: String,
    provider: String,
    node_id: String,
    id_parts: Vec<String>,
    node_name: String,
}

impl ProviderID {
    pub fn new(name: &str, provider_id: &str) -> Result<Self, ProviderIDError> {
        // <ProviderName>://<ProviderSpecificNodeID>

        if provider_id.is_empty() {
            return Err(ProviderIDError::Empty);
        }

        let parts = provider_id.split("://").collect::<Vec<&str>>();
        if parts.len() != 2 {
            return Err(ProviderIDError::Invalid);
        }

        let provider = parts[0].to_string();
        let node_id = parts[1].to_string();

        if provider.is_empty() || node_id.is_empty() {
            return Err(ProviderIDError::Invalid);
        }

        let id_parts = node_id.split('/').map(String::from).collect::<Vec<_>>();

        let pid = Self {
            node_name: name.into(),
            provider_id: provider_id.into(),
            provider,
            node_id,
            id_parts,
        };

        Ok(pid)
    }

    pub fn provider(&self) -> String {
        self.provider.to_string()
    }

    pub fn node_name(&self) -> String {
        self.node_name.to_string()
    }

    pub fn node_id(&self) -> String {
        self.node_id.to_string()
    }

    pub fn last(&self) -> String {
        self.id_parts
            .last()
            .map(String::as_str)
            .unwrap_or(&self.node_id())
            .to_string()
    }

    pub fn nth(&self, n: usize) -> Option<String> {
        self.id_parts.get(n).map(String::to_string)
    }
}

impl std::fmt::Display for ProviderID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.provider_id)
    }
}

#[derive(Error, Debug)]
pub enum ProviderIDError {
    Empty,
    Invalid,
}

impl std::fmt::Display for ProviderIDError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderIDError::Empty => write!(f, "provider id is empty"),
            ProviderIDError::Invalid => write!(
                f,
                "provider id must be in the format: <ProviderName>://<ProviderSpecificNodeID>"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id_new() {
        let node_name = "my-node-name";

        // <ProviderName>://<ProviderSpecificNodeID>
        let provider_id = ProviderID::new(
            node_name,
            "kind://podman/kind-cluster/kind-cluster-control-plane",
        )
        .unwrap();
        assert_eq!("kind", provider_id.provider);
        assert_eq!(
            provider_id.to_string(),
            "kind://podman/kind-cluster/kind-cluster-control-plane"
        );

        // errors

        // empty
        assert!(matches!(
            ProviderID::new(node_name, ""),
            Err(ProviderIDError::Empty)
        ));

        // invalid
        ["provider-id", "kind://", "://", "://node-id", " "]
            .iter()
            .for_each(|s| {
                assert!(matches!(
                    ProviderID::new(node_name, s),
                    Err(ProviderIDError::Invalid)
                ));
            });
    }

    #[test]
    fn test_provider_id_name() {
        let node_name = "my-node-name";
        [
            (
                "kind://podman/kind-cluster/kind-cluster-control-plane",
                "kind",
            ),
            ("aws:///us-west-2a/i-0a1b2c3d4e5f6g7h8", "aws"),
            ("gce://my-project/us-central1-a/my-instance", "gce"),
        ]
        .iter()
        .for_each(|i| {
            let provider_id = ProviderID::new(node_name, i.0).unwrap();
            assert_eq!(provider_id.provider(), i.1);
        });
    }

    #[test]
    fn test_provider_id_node_id() {
        let node_name = "my-node-name";
        [
            (
                "kind://podman/kind-cluster/kind-cluster-control-plane",
                "podman/kind-cluster/kind-cluster-control-plane",
            ),
            (
                "aws://us-west-2a/i-0a1b2c3d4e5f6g7h8",
                "us-west-2a/i-0a1b2c3d4e5f6g7h8",
            ),
            (
                "gce://my-project/us-central1-a/my-instance",
                "my-project/us-central1-a/my-instance",
            ),
        ]
        .iter()
        .for_each(|i| {
            let provider_id = ProviderID::new(node_name, i.0).unwrap();
            assert_eq!(provider_id.node_id(), i.1);
        });
    }

    #[test]
    fn test_provider_id_last() {
        let node_name = "my-node-name";
        [
            (
                "kind://podman/kind-cluster/kind-cluster-control-plane",
                "kind-cluster-control-plane",
            ),
            (
                "aws://us-west-2a/i-0a1b2c3d4e5f6g7h8",
                "i-0a1b2c3d4e5f6g7h8",
            ),
            ("gce://my-project/us-central1-a/my-instance", "my-instance"),
        ]
        .iter()
        .for_each(|i| {
            let provider_id = ProviderID::new(node_name, i.0).unwrap();
            assert_eq!(provider_id.last(), i.1);
        });
    }

    #[test]
    fn test_provider_id_nth() {
        let node_name = "my-node-name";
        let provider_id = "kind://podman/kind-cluster/kind-cluster-control-plane";
        let provider_id = ProviderID::new(node_name, provider_id).unwrap();

        assert_eq!(provider_id.nth(0), Some("podman".to_string()));
        assert_eq!(provider_id.nth(1), Some("kind-cluster".to_string()));
        assert_eq!(
            provider_id.nth(2),
            Some("kind-cluster-control-plane".to_string())
        );
        assert_eq!(provider_id.nth(3), None);
    }

    #[test]
    fn test_provider_id_node_name() {
        let node_name = "my-node-name";
        let provider_id = "kind://podman/kind-cluster/kind-cluster-control-plane";
        let provider_id = ProviderID::new(node_name, provider_id).unwrap();

        assert_eq!(provider_id.node_name(), node_name);
    }
}
