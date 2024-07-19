use thiserror::Error;

pub mod provider_id;
pub mod template;

#[derive(Error, Debug)]
pub enum Error {
    #[error("kube error: {0}")]
    Kube(#[from] kube::Error),
    #[error("MissingObjectKey: {0}")]
    MissingObjectKey(&'static str),
    #[error("ProviderIDError: {0}")]
    ProviderID(#[from] provider_id::ProviderIDError),
    #[error("ParseIntError: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("TemplateParseError: {0}")]
    TemplateParser(String),
    #[error("MetadataKeyError: {0}")]
    MetadataKey(String),
    #[error("JoinError: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("ServerError: {0}")]
    ServerError(#[from] std::io::Error),
}
