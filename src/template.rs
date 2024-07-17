use crate::{provider_id::ProviderID, Error};
use pest::Parser;
use pest_derive::Parser;
use std::str::FromStr;

#[derive(Parser)]
#[grammar = "template.pest"]
struct TemplateParser;

pub trait Template {
    fn render(&self, provider_id: &ProviderID) -> Result<String, Error>;
}

#[derive(Default, Debug)]
pub struct LabelTemplate(String);

impl FromStr for LabelTemplate {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate_template(s, Rule::label)?;
        Ok(Self(s.to_string()))
    }
}

impl Template for LabelTemplate {
    fn render(&self, provider_id: &ProviderID) -> Result<String, Error> {
        do_render(&self.0, provider_id, Rule::label).map(|s| {
            let mut s = s.replace("://", "_").replace('/', "_");
            s.truncate(63);
            s
        })
    }
}

#[derive(Default, Debug)]
pub struct AnnotationTemplate(String);

impl FromStr for AnnotationTemplate {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate_template(s, Rule::annotation)?;
        Ok(Self(s.to_string()))
    }
}

impl Template for AnnotationTemplate {
    fn render(&self, provider_id: &ProviderID) -> Result<String, Error> {
        do_render(&self.0, provider_id, Rule::annotation)
    }
}

fn validate_template(template: &str, rule: Rule) -> Result<(), Error> {
    TemplateParser::parse(rule, template)
        .map(|_| ())
        .map_err(|e| Error::TemplateParser(e.to_string()))
}

fn do_render(template: &str, provider_id: &ProviderID, rule: Rule) -> Result<String, Error> {
    let mut pairs =
        TemplateParser::parse(rule, template).map_err(|e| Error::TemplateParser(e.to_string()))?;
    let pair = pairs.next().unwrap();
    let mut output = String::new();

    for token in pair.into_inner() {
        match token.as_rule() {
            Rule::last => output.push_str(&provider_id.last()),
            Rule::first => output.push_str(&provider_id.nth(0).unwrap()),
            Rule::all => output.push_str(&provider_id.node_id()),
            Rule::provider => output.push_str(&provider_id.provider()),
            Rule::url => {
                output.push_str(&provider_id.to_string());
            }
            Rule::nth => {
                let nth = token.into_inner().next().unwrap().as_str();
                let idx = nth.parse::<usize>()?;
                output.push_str(&provider_id.nth(idx).unwrap());
            }
            Rule::label_char => output.push_str(token.as_str()),
            Rule::char => output.push_str(token.as_str()),
            Rule::EOI => (),
            _ => {
                return Err(Error::TemplateParser(format!(
                    "unable to parse template '{}'",
                    template
                )))
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_id::ProviderID;

    #[test]
    fn test_label_template_from_str() {
        let t = |template: &str| LabelTemplate::from_str(template).expect(template);

        let _ = t("aws-{:last}");
        let _ = t("{:last}");
        let _ = t("{:first}");
        let _ = t("{:all}");
        let _ = t("{:provider}");
        let _ = t("{:url}");
        let _ = t("{0}");
        let _ = t("{1}");
        let _ = t("{:last}-{:first}_{:all}.{:last}");

        assert!(LabelTemplate::from_str("{:incorrect}").is_err());
        assert!(LabelTemplate::from_str("n0tall/ow#D").is_err());
    }

    #[test]
    fn test_label_template_render() {
        let t = |template: &str, id: &ProviderID| {
            LabelTemplate::from_str(template)
                .unwrap()
                .render(id)
                .unwrap()
        };

        let id = ProviderID::new("aws://us-east-2/i-1234567890abcdef0").unwrap();

        let output = t("aws-{:last}", &id);
        assert_eq!(output, "aws-i-1234567890abcdef0");

        let output = t("{:last}", &id);
        assert_eq!(output, "i-1234567890abcdef0");

        let output = t("{:first}", &id);
        assert_eq!(output, "us-east-2");

        let output = t("{:all}", &id);
        assert_eq!(output, "us-east-2_i-1234567890abcdef0");

        let output = t("{:provider}", &id);
        assert_eq!(output, "aws");

        let output = t("{:url}", &id);
        assert_eq!(output, "aws_us-east-2_i-1234567890abcdef0");

        let output = t("{0}", &id);
        assert_eq!(output, "us-east-2");

        let output = t("{1}", &id);
        assert_eq!(output, "i-1234567890abcdef0");

        let output = t("{:last}-{:first}_{:all}.{:last}", &id);
        assert_eq!(
            output,
            "i-1234567890abcdef0-us-east-2_us-east-2_i-1234567890abcdef0.i-1",
        );
    }

    #[test]
    fn test_annotation_template_parser() {
        let a = |template: &str, id: &ProviderID| {
            AnnotationTemplate::from_str(template)
                .unwrap()
                .render(id)
                .unwrap()
        };

        let id = ProviderID::new("aws://us-east-2/i-1234567890abcdef0").unwrap();

        let output = a("{:last}", &id);
        assert_eq!(output, "i-1234567890abcdef0");

        let output = a("{:first}", &id);
        assert_eq!(output, "us-east-2");

        let output = a("{:all}", &id);
        assert_eq!(output, "us-east-2/i-1234567890abcdef0");

        let output = a("{:provider}", &id);
        assert_eq!(output, "aws");

        let output = a("{:url}", &id);
        assert_eq!(output, "aws://us-east-2/i-1234567890abcdef0");

        let output = a("{0}", &id);
        assert_eq!(output, "us-east-2");

        let output = a("{1}", &id);
        assert_eq!(output, "i-1234567890abcdef0");

        let output = a("{:last}-{:first}_{:all}.{:last}", &id);
        assert_eq!(
            output,
            "i-1234567890abcdef0-us-east-2_us-east-2/i-1234567890abcdef0.i-1234567890abcdef0"
        );

        let output = a("{:last}-{:first} {:all}/{:last}", &id);
        assert_eq!(
            output,
            "i-1234567890abcdef0-us-east-2 us-east-2/i-1234567890abcdef0/i-1234567890abcdef0"
        );
    }
}
