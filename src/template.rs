#[allow(unused_imports)]
use crate::{provider_id::ProviderID, Error};
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "template.pest"]
struct TemplateParser;

pub fn label(template: &str, provider_id: &ProviderID) -> Result<String, Error> {
    do_render(template, provider_id, Rule::label).map(|s| {
        let mut s = s.replace('/', "_");
        s.truncate(63);
        s
    })
}

pub fn annotation(template: &str, provider_id: &ProviderID) -> Result<String, Error> {
    do_render(template, provider_id, Rule::annotation)
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
    fn test_label_template_parser() {
        let id = ProviderID::new("aws://us-east-2/i-1234567890abcdef0").unwrap();

        let output = label("{:last}", &id).unwrap();
        assert_eq!(output, "i-1234567890abcdef0");

        let output = label("{:first}", &id).unwrap();
        assert_eq!(output, "us-east-2");

        let output = label("{:all}", &id).unwrap();
        assert_eq!(output, "us-east-2_i-1234567890abcdef0");

        let output = label("{0}", &id).unwrap();
        assert_eq!(output, "us-east-2");

        let output = label("{1}", &id).unwrap();
        assert_eq!(output, "i-1234567890abcdef0");

        let output = label("{:last}-{:first}_{:all}.{:last}", &id).unwrap();
        assert_eq!(
            output,
            "i-1234567890abcdef0-us-east-2_us-east-2_i-1234567890abcdef0.i-1",
        );
    }

    #[test]
    fn test_annotation_template_parser() {
        let id = ProviderID::new("aws://us-east-2/i-1234567890abcdef0").unwrap();

        let output = annotation("{:last}", &id).unwrap();
        assert_eq!(output, "i-1234567890abcdef0");

        let output = annotation("{:first}", &id).unwrap();
        assert_eq!(output, "us-east-2");

        let output = annotation("{:all}", &id).unwrap();
        assert_eq!(output, "us-east-2/i-1234567890abcdef0");

        let output = annotation("{0}", &id).unwrap();
        assert_eq!(output, "us-east-2");

        let output = annotation("{1}", &id).unwrap();
        assert_eq!(output, "i-1234567890abcdef0");

        let output = annotation("{:last}-{:first}_{:all}.{:last}", &id).unwrap();
        assert_eq!(
            output,
            "i-1234567890abcdef0-us-east-2_us-east-2/i-1234567890abcdef0.i-1234567890abcdef0"
        );

        let output = annotation("{:last}-{:first} {:all}/{:last}", &id).unwrap();
        assert_eq!(
            output,
            "i-1234567890abcdef0-us-east-2 us-east-2/i-1234567890abcdef0/i-1234567890abcdef0"
        );
    }
}
