mod filesystem;
mod solution;
pub use filesystem::*;
pub use solution::*;

use crate::layout::Route;
use dioxus::prelude::*;
use serde_yaml::Value;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    str::FromStr,
};

pub enum View {
    Solution,
    FileSystem,
}

impl FromStr for View {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "FileSystem" => View::FileSystem,
            _ => View::Solution,
        })
    }
}

#[component]
pub fn FilesView(working_dir: ReadSignal<PathBuf>, view: ReadSignal<View>) -> Element {
    match *view.read() {
        View::Solution => rsx! {
            SolutionView { project_path: working_dir }
        },
        View::FileSystem => rsx! {
            FileSystemView { project_path: working_dir }
        },
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub path: PathBuf,
    pub children: Vec<Node>,
    pub is_dir: bool,
    pub type_: FileType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FileType {
    Workflow,
    CommandLineTool,
    ExpressionTool,
    Other,
}

pub fn get_route(node: &Node) -> Route {
    match node.type_ {
        FileType::Workflow => Route::WorkflowView {
            path: format!("{}", node.path.to_string_lossy()),
        },
        FileType::CommandLineTool | FileType::ExpressionTool => Route::ToolView {
            path: format!("{}", node.path.to_string_lossy()),
        },
        _ => Route::Empty,
    }
}

pub fn read_node_type(path: impl AsRef<Path>) -> FileType {
    if path.as_ref().is_dir() || path.as_ref().extension() != Some(OsStr::new("cwl")) {
        return FileType::Other;
    }
    let content = std::fs::read_to_string(path).expect("Can not read file!");
    let yaml: Value = serde_yaml::from_str(&content).unwrap_or(Value::Null);

    match yaml.get("class").and_then(|v| v.as_str()) {
        Some("CommandLineTool") => FileType::CommandLineTool,
        Some("Workflow") => FileType::Workflow,
        Some("ExpressionTool") => FileType::ExpressionTool,
        _ => FileType::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_node_type() {
        let path = "../../testdata/hello_world/workflows/main/main.cwl";
        assert_eq!(read_node_type(path), FileType::Workflow);

        let path = "../../testdata/hello_world/workflows/calculation/calculation.cwl";
        assert_eq!(read_node_type(path), FileType::CommandLineTool);
    }
}
