use color_eyre::eyre;
use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Name(String);

impl FromStr for Name {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Err(e) = Self::validate(s) {
            return Err(eyre::eyre!("invalid name ({})", e.to_string()));
        }
        Ok(Self(s.to_string()))
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Name {
    fn validate(s: &str) -> eyre::Result<()> {
        // must be 63 characters or less
        if s.len() > 63 {
            return Err(eyre::eyre!("> 63 characters"));
        }

        // beginning and ending with an alphanumeric character ([a-z0-9A-Z])
        if !s
            .chars()
            .next()
            .map_or(false, |c| c.is_ascii_alphanumeric())
            || !s
                .chars()
                .last()
                .map_or(false, |c| c.is_ascii_alphanumeric())
        {
            return Err(eyre::eyre!(
                "must start and end with an alphanumeric character"
            ));
        }

        // with dashes (-), underscores (_), dots (.), and alphanumerics between.
        for c in s.chars() {
            if !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.' {
                return Err(eyre::eyre!("invalid character '{c}'"));
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Prefix(String);

impl FromStr for Prefix {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Err(e) = Self::validate(s) {
            return Err(eyre::eyre!("invalid prefix ({})", e.to_string()));
        }
        Ok(Self(s.to_string()))
    }
}

impl Prefix {
    fn validate(s: &str) -> eyre::Result<()> {
        // the prefix must be a DNS subdomain: a series of DNS labels separated
        // by dots (.), not longer than 253 characters in total.

        // check total length
        if s.len() > 253 {
            return Err(eyre::eyre!("> 253 characters"));
        }

        // split the prefix into dns labels
        let labels = s.split('.').collect::<Vec<_>>();

        // check each label
        for label in labels {
            // check label length
            let len = label.len();
            if len < 1 {
                return Err(eyre::eyre!("dns label < 1 character"));
            }
            if len > 63 {
                return Err(eyre::eyre!("dns label > 63 characters"));
            }

            // check label characters
            for c in label.chars() {
                if !c.is_ascii_alphanumeric() && c != '_' && c != '-' {
                    return Err(eyre::eyre!("invalid character '{c}'"));
                }
            }

            // Check label starts and ends with alphanumeric character
            if !label
                .chars()
                .next()
                .map_or(false, |c| c.is_ascii_alphanumeric())
                || !label
                    .chars()
                    .last()
                    .map_or(false, |c| c.is_ascii_alphanumeric())
            {
                return Err(eyre::eyre!(
                    "must start and end with an alphanumeric character"
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetadataKey {
    prefix: Option<Prefix>,
    name: Name,
    key: String,
}

impl FromStr for MetadataKey {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // extract and validate name and optional prefix
        let key = s.to_string();
        let parts = key.split('/').collect::<Vec<_>>();
        let (prefix, name) = match parts.len() {
            1 => (None, parts[0].parse()?),
            2 => (Some(parts[0].parse()?), parts[1].parse()?),
            _ => return Err(eyre::eyre!("invalid key")),
        };

        Ok(Self { key, prefix, name })
    }
}

impl std::fmt::Display for MetadataKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key)
    }
}

pub type Label = MetadataKey;
pub type Annotation = MetadataKey;
