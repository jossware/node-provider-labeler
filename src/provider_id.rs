use thiserror::Error;

#[derive(Debug)]
pub struct ProviderID {
    provider_id: String,
    name: String,
    node_id: String,
    id_parts: Vec<String>,
}

impl ProviderID {
    pub fn new(provider_id: &str) -> Result<Self, ProviderIDError> {
        // <ProviderName>://<ProviderSpecificNodeID>

        if provider_id.is_empty() {
            return Err(ProviderIDError::Empty);
        }

        let parts = provider_id.split("://").collect::<Vec<&str>>();
        if parts.len() != 2 {
            return Err(ProviderIDError::Invalid);
        }

        let name = parts[0].to_string();
        let node_id = parts[1].to_string();

        if name.is_empty() || node_id.is_empty() {
            return Err(ProviderIDError::Invalid);
        }

        let id_parts = node_id.split('/').map(String::from).collect::<Vec<_>>();

        let pid = Self {
            provider_id: provider_id.into(),
            name,
            node_id,
            id_parts,
        };

        Ok(pid)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn last(&self) -> &str {
        self.id_parts
            .last()
            .map(String::as_str)
            .unwrap_or(self.node_id())
    }

    pub fn nth(&self, n: usize) -> Option<&str> {
        self.id_parts.get(n).map(String::as_str)
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
        // <ProviderName>://<ProviderSpecificNodeID>
        let provider_id =
            ProviderID::new("kind://podman/kind-cluster/kind-cluster-control-plane").unwrap();
        assert_eq!("kind", provider_id.name);
        assert_eq!(
            provider_id.to_string(),
            "kind://podman/kind-cluster/kind-cluster-control-plane"
        );

        // errors

        // empty
        assert!(matches!(ProviderID::new(""), Err(ProviderIDError::Empty)));

        // invalid
        ["provider-id", "kind://", "://", "://node-id", " "]
            .iter()
            .for_each(|s| {
                assert!(matches!(ProviderID::new(s), Err(ProviderIDError::Invalid)));
            });
    }

    #[test]
    fn test_provider_id_name() {
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
            let provider_id = ProviderID::new(i.0).unwrap();
            assert_eq!(provider_id.name(), i.1);
        });
    }

    #[test]
    fn test_provider_id_node_id() {
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
            let provider_id = ProviderID::new(i.0).unwrap();
            assert_eq!(provider_id.node_id(), i.1);
        });
    }

    #[test]
    fn test_provider_id_last() {
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
            let provider_id = ProviderID::new(i.0).unwrap();
            assert_eq!(provider_id.last(), i.1);
        });
    }

    #[test]
    fn test_provider_id_nth() {
        let provider_id = "kind://podman/kind-cluster/kind-cluster-control-plane";
        let provider_id = ProviderID::new(provider_id).unwrap();

        assert_eq!(provider_id.nth(0), Some("podman"));
        assert_eq!(provider_id.nth(1), Some("kind-cluster"));
        assert_eq!(provider_id.nth(2), Some("kind-cluster-control-plane"));
        assert_eq!(provider_id.nth(3), None);
    }
}
