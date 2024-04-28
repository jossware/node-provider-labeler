use thiserror::Error;

#[derive(Debug)]
pub struct ProviderID {
    provider_id: String,
    name: String,
    node_id: String,
}

impl ProviderID {
    pub fn new(provider_id: &str) -> Result<Self, ProviderIDError> {
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

        let pid = Self {
            provider_id: provider_id.into(),
            name,
            node_id,
        };

        Ok(pid)
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
        assert!(matches!(ProviderID::new(""), Err(ProviderIDError::Empty)));

        ["provider-id", "kind://", "://", "://node-id"]
            .iter()
            .for_each(|s| {
                assert!(matches!(ProviderID::new(s), Err(ProviderIDError::Invalid)));
            });
    }
}
