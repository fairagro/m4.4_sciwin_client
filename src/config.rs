use semver::Version;
use serde::{Deserialize, Deserializer, Serialize};
use smart_default::SmartDefault;

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct Config {
    pub workflow: WorkflowConfig,
}

#[derive(Serialize, Deserialize, Debug, SmartDefault, PartialEq)]
pub struct WorkflowConfig {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[default(default_version())]
    pub version: Version,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_authors")]
    pub authors: Option<Vec<Author>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct Author {
    pub name: String,
    pub email: Option<String>,
    pub orcid: Option<String>,
}

impl From<String> for Author {
    fn from(value: String) -> Self {
        Self {
            name: value,
            ..Default::default()
        }
    }
}

fn deserialize_authors<'de, D>(deserializer: D) -> Result<Option<Vec<Author>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum AuthorOrString {
        Full(Author),
        Name(String),
    }

    let raw: Option<Vec<AuthorOrString>> = Option::deserialize(deserializer)?;
    Ok(raw.map(|list| {
        list.into_iter()
            .map(|entry| match entry {
                AuthorOrString::Full(author) => author,
                AuthorOrString::Name(name) => Author::from(name),
            })
            .collect()
    }))
}

fn default_version() -> Version {
    Version::new(0, 1, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_serialize_config() {
        let config = Config {
            workflow: WorkflowConfig {
                name: "my-workflow".to_string(),
                version: Version::new(1, 0, 0),
                authors: Some(vec![Author {
                    name: "Günther".to_string(),
                    orcid: Some("no-orcid".to_string()),
                    ..Default::default()
                }]),
                ..Default::default()
            },
        };

        let str = toml::to_string(&config).unwrap();
        //convert back to toml and check if fits input
        let toml = toml::from_str(&str).unwrap();
        assert_eq!(config, toml);
    }

    #[test]
    fn test_deserialize_config() {
        let workflow_toml = r#"
        [workflow]
        name = "my-workflow"
        description = "a workflow that does ... things!"
        version = "0.1.0"
        authors = ["Derp Derpson", "Dudette Derpson"]
        license = "MIT"
        keywords = ["workflow"]        
        "#;

        let parsed: Config = toml::from_str(workflow_toml).expect("Failed to parse toml");
        assert_eq!(parsed.workflow.name, "my-workflow");
        assert_eq!(parsed.workflow.description, Some("a workflow that does ... things!".to_string()));
        assert_eq!(parsed.workflow.version, Version::parse("0.1.0").unwrap());
        assert_eq!(
            parsed.workflow.authors,
            Some(vec![
                Author {
                    name: "Derp Derpson".to_string(),
                    ..Default::default()
                },
                Author {
                    name: "Dudette Derpson".to_string(),
                    ..Default::default()
                }
            ])
        );
        assert_eq!(parsed.workflow.license, Some("MIT".to_string()));
        assert_eq!(parsed.workflow.keywords, Some(vec!["workflow".to_string()]));
    }
}
