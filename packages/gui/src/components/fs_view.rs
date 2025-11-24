use crate::use_app_state;
use crate::workflow::VisualWorkflow;
use commonwl::load_doc;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::go_icons::{GoChevronDown, GoChevronRight, GoFile, GoFileDirectory};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub path: PathBuf,
    pub children: Vec<Node>,
    pub is_dir: bool,
}

#[component]
pub fn FileTree(node: ReadSignal<Node>, is_root: bool) -> Element {
    let mut expanded = use_signal(|| false);
    let mut app_state = use_app_state();

    if is_root {
        expanded.set(true);
    }

    const ICON_SIZE: Option<u32> = Some(14);

    let cursor_class = if node.read().is_dir | node.read().name.ends_with(".cwl") {
        "cursor-pointer"
    } else {
        "cursor-not-allowed"
    };

    let padleft = if is_root { "" } else { "pl-2" };

    rsx! {
        div {
            class: "{padleft}",
            div {
                class: "{cursor_class} select-none",
                onclick: move |_| {
                    //simply expand folder if directory
                    if node.read().is_dir {
                        expanded.set(!expanded())
                    }
                    else if node.read().name.ends_with(".cwl") {
                        let data = load_doc(&node().path).unwrap();
                        if let commonwl::CWLDocument::Workflow(_) = data {
                            let workflow = VisualWorkflow::from_file(&node.read().path).unwrap();
                            app_state.write().workflow = workflow;
                        }
                    }
                },
                div {
                    class: "flex gap-1 items-center",
                    if node().is_dir {
                        if expanded() {
                            Icon { width: ICON_SIZE, height: ICON_SIZE, icon: GoChevronDown }
                        } else {
                            Icon { width: ICON_SIZE, height: ICON_SIZE, icon: GoChevronRight }
                        }
                        Icon { width: ICON_SIZE, height: ICON_SIZE, icon: GoFileDirectory }
                    } else {
                        div {
                            style: "width: {ICON_SIZE.unwrap()}px; height: {ICON_SIZE.unwrap()}px;",
                        }
                        Icon { width: ICON_SIZE, height: ICON_SIZE, icon: GoFile }
                    },
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
            });
        }
    }

    Node {
        name: path.file_name().unwrap().to_string_lossy().into(),
        path: path.to_path_buf(),
        is_dir: true,
        children,
    }
}
