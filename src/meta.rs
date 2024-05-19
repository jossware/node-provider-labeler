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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_name_fromstr() {
        struct TestCase<'a> {
            input: &'a str,
            output: Option<Name>,
            error: Option<&'a str>,
        }

        let cases = vec![
            TestCase {
                input: "test",
                output: Some(Name("test".into())),
                error: None,
            },
            TestCase {
                input: "x--------------------------------------------------------------x",
                output: None,
                error: Some("invalid name (> 63 characters)"),
            },
            TestCase {
                input: "-test",
                output: None,
                error: Some("invalid name (must start and end with an alphanumeric character)"),
            },
            TestCase {
                input: "test-",
                output: None,
                error: Some("invalid name (must start and end with an alphanumeric character)"),
            },
            TestCase {
                input: "te~t",
                output: None,
                error: Some("invalid name (invalid character '~')"),
            },
        ];

        for case in cases {
            let result = case.input.parse::<Name>();

            if let Err(err) = result {
                assert_eq!(
                    case.error.expect("error occurred but expected None"),
                    err.to_string(),
                );
            } else {
                let expected = case.output;

                assert!(
                    expected.is_some(),
                    "success but output is None. input: {}",
                    case.input
                );

                let expected = expected.unwrap();
                let actual = result.unwrap();
                assert_eq!(expected, actual);
            }
        }
    }

    #[test]
    fn meta_prefix_fromstr() {
        struct TestCase<'a> {
            input: &'a str,
            output: Option<Prefix>,
            error: Option<&'a str>,
        }

        let cases = vec![
            TestCase {
                input: "test",
                output: Some(Prefix("test".into())),
                error: None,
            },
            TestCase {
                input: "x----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------x",
                output: None,
                error: Some("invalid prefix (> 253 characters)"),
            },
            TestCase {
                input: "test.xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.test",
                output: None,
                error: Some("invalid prefix (dns label > 63 characters)"),
            },
            TestCase {
                input: "",
                output: None,
                error: Some("invalid prefix (dns label < 1 character)"),
            },
            TestCase {
                input: "-test",
                output: None,
                error: Some(
                    "invalid prefix (must start and end with an alphanumeric character)",
                ),
            },
            TestCase {
                input: "test-",
                output: None,
                error: Some(
                    "invalid prefix (must start and end with an alphanumeric character)",
                ),
            },
            TestCase {
                input: "te~t",
                output: None,
                error: Some("invalid prefix (invalid character '~')"),
            },
            TestCase {
                input: "test.test",
                output: Some(Prefix("test.test".into())),
                error: None,
            },
            TestCase {
                input: "test.test.test",
                output: Some(Prefix("test.test.test".into())),
                error: None,
            },
        ];

        for case in cases {
            let result = case.input.parse::<Prefix>();

            if let Err(err) = result {
                assert_eq!(
                    case.error.expect("error occurred but expected None"),
                    err.to_string(),
                );
            } else {
                let expected = case.output;

                assert!(
                    expected.is_some(),
                    "success but output is None. input: {}",
                    case.input
                );

                let expected = expected.unwrap();
                let actual = result.unwrap();
                assert_eq!(expected, actual);
            }
        }
    }

    #[test]
    fn metadata_key_fromstr() {
        struct TestCase<'a> {
            input: &'a str,
            output: Option<MetadataKey>,
            error: Option<&'a str>,
        }

        let cases = vec![
            TestCase {
                input: "app",
                output: Some(MetadataKey {
                    key: "app".into(),
                    name: "app".parse().unwrap(),
                    prefix: None,
                }),
                error: None,
            },
            TestCase {
                input: "app=test=test2",
                output: None,
                error: Some("invalid name (invalid character '=')"),
            },
            TestCase {
                input: "domain.com/app/v1",
                output: None,
                error: Some("invalid key"),
            },
            TestCase {
                input: "domain.com/app",
                output: Some(MetadataKey {
                    key: "domain.com/app".into(),
                    name: "app".parse().unwrap(),
                    prefix: Some("domain.com".parse().unwrap()),
                }),
                error: None,
            },
            TestCase {
                input: "x.------------------------------------------------------------.x=test2",
                output: None,
                error: Some("invalid name (> 63 characters)"),
            },
            TestCase {
                input: "-app",
                output: None,
                error: Some(
                    "invalid name (must start and end with an alphanumeric character)",
                ),
            },
            TestCase {
                input: "app~1",
                output: None,
                error: Some("invalid name (invalid character '~')"),
            },
            TestCase {
                input: "domai~n.com/app=test2",
                output: None,
                error: Some("invalid prefix (invalid character '~')"),
            },
            TestCase {
                input: "domain.x--------------------------------------------------------------x.com/app",
                output: None,
                error: Some("invalid prefix (dns label > 63 characters)"),
            },
            TestCase {
                input: "domain..com/app",
                output: None,
                error: Some("invalid prefix (dns label < 1 character)"),
            },
            TestCase {
                input: "domain.-x.com/app",
                output: None,
                error: Some("invalid prefix (must start and end with an alphanumeric character)"),
            },
            TestCase {
                input: "domain.xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.com/app=test2",
                output: None,
                error: Some("invalid prefix (> 253 characters)"),
            },
            TestCase {
                input: "sub.domain.com/app",
                output: Some(MetadataKey{
                    prefix: Some("sub.domain.com".parse().unwrap()),
                    name: "app".parse().unwrap(),
                    key: "sub.domain.com/app".into(),
                }),
                error: None,
            },
        ];

        for case in cases {
            let result = case.input.parse::<MetadataKey>();

            if let Err(err) = result {
                assert_eq!(
                    case.error.expect("error occurred but expected None"),
                    err.to_string(),
                );
            } else {
                let expected = case.output;

                assert!(
                    expected.is_some(),
                    "success but output is None. input: {}",
                    case.input
                );

                let expected = expected.unwrap();
                let actual = result.unwrap();
                assert_eq!(expected.key, actual.key);
                assert_eq!(expected.prefix, actual.prefix);
                assert_eq!(expected.name, actual.name);
            }
        }
    }
}
