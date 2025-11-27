use crate::layout::Route;
use crate::use_app_state;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::go_icons::{GoChevronDown, GoChevronRight, GoFile, GoFileDirectory};
use serde_yaml::Value;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub enum FileType {
    Workflow,
    CommandLineTool,
    ExpressionTool,
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub path: PathBuf,
    pub children: Vec<Node>,
    pub is_dir: bool,
    pub type_: FileType,
}

#[component]
pub fn FileTree(node: ReadSignal<Node>, is_root: bool) -> Element {
    let padleft = if is_root { "" } else { "pl-2" };

    rsx! {
        div {
            class: "{padleft}",
            if node.read().is_dir {
                DirItem {  node: node, is_root: is_root }
            }
            else {
                FileItem {  node:node }
            }
        }
    }
}

const ICON_SIZE: Option<u32> = Some(14);

#[component]
pub fn FileItem(node: ReadSignal<Node>) -> Element {
    let route = match node.read().type_ {
        FileType::Workflow => Route::WorkflowView {
            path: format!("{}", node.read().path.to_string_lossy()),
        },
        FileType::CommandLineTool | FileType::ExpressionTool => Route::ToolView {
            path: format!("{}", node.read().path.to_string_lossy()),
        },
        _ => Route::Empty,
    };

    if let Route::Empty = route {
        return rsx! {
            div {
                class: "cursor-not-allowed select-none",
                div {
                    class: "flex gap-1 items-center",
                    div {
                        style: "width: {ICON_SIZE.unwrap()}px; height: {ICON_SIZE.unwrap()}px;",
                    }
                    Icon { width: ICON_SIZE, height: ICON_SIZE, icon: GoFile }

                    { node().name }
                }
            }
        };
    }

    rsx! {
        Link {
            active_class: "font-bold",
            to: route,
            class: "cursor-pointer select-none",
            div {
                class: "flex gap-1 items-center",
                div {
                    style: "width: {ICON_SIZE.unwrap()}px; height: {ICON_SIZE.unwrap()}px;",
                }
                div {
                    class: "flex",
                    style: "width: {ICON_SIZE.unwrap()}px; height: {ICON_SIZE.unwrap()}px;",
                    img { src: asset!("/assets/CWL.svg")}
                }

                { node().name }
            }
        }
    }
}

#[component]
pub fn DirItem(node: ReadSignal<Node>, is_root: bool) -> Element {
    let mut expanded = use_signal(|| false);

    if is_root {
        expanded.set(true);
    }
    rsx! {
    div {
        class: "cursor-pointer select-none",
        onclick: move |_| {
                //simply expand folder if directory
                if node.read().is_dir {
                    expanded.set(!expanded())
                }
        },
        div {
            class: "flex gap-1 items-center",
                if expanded() {
                    Icon { width: ICON_SIZE, height: ICON_SIZE, icon: GoChevronDown }
                } else {
                    Icon { width: ICON_SIZE, height: ICON_SIZE, icon: GoChevronRight }
                }
                Icon { width: ICON_SIZE, height: ICON_SIZE, icon: GoFileDirectory }

            { node().name }
            }
        },
        if expanded() {
            for child in node.read().children.clone() {
                FileTree { node: child , is_root: false}
            }
        }
    }
}

#[component]
pub fn FileSystemView(project_path: ReadSignal<PathBuf>) -> Element {
    let app_state = use_app_state();
    let root = use_memo(move || app_state.read().working_directory.as_ref().map(|path| load_project_tree(path)));

    rsx! {
        if let Some(root) = root(){
            FileTree { node: root , is_root: true }
        }
    }
}

fn load_project_tree(path: &Path) -> Node {
    let mut children = vec![];

    if let Ok(entries) = std::fs::read_dir(path) {
        let mut entries: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();

        entries.sort_by(|a, b| match (a.is_dir(), b.is_dir()) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().unwrap().to_string_lossy().cmp(&b.file_name().unwrap().to_string_lossy()),
        });

        for path in entries {
            let is_dir = path.is_dir();

            children.push(Node {
                name: path.file_name().unwrap().to_string_lossy().into(),
                path: path.clone(),
                is_dir,
                children: if is_dir { load_project_tree(&path).children } else { vec![] },
                type_: read_node_type(&path),
            });
        }
    }

    Node {
        name: path.file_name().unwrap().to_string_lossy().into(),
        path: path.to_path_buf(),
        is_dir: true,
        children,
        type_: read_node_type(path),
    }
}

fn read_node_type(path: impl AsRef<Path>) -> FileType {
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
