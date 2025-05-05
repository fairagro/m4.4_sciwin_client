use semver::Version;
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub workflow: WorkflowConfig,
}

#[derive(Serialize, Deserialize, Debug, SmartDefault)]
pub struct WorkflowConfig {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[default(default_version())]
    pub version: Version,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

fn default_version() -> Version {
    Version::new(0, 1, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
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
            Some(vec!["Derp Derpson".to_string(), "Dudette Derpson".to_string()])
        );
        assert_eq!(parsed.workflow.license, Some("MIT".to_string()));
        assert_eq!(parsed.workflow.keywords, Some(vec!["workflow".to_string()]));
    }
}
