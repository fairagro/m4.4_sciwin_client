use crate::components::ICON_SIZE;
use crate::components::files::{Node, get_route, read_node_type};
use crate::use_app_state;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::go_icons::{GoCloud, GoFileDirectory};
use ignore::WalkBuilder;
use repository::Repository;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[component]
pub fn SolutionView(project_path: ReadSignal<PathBuf>, reload_trigger: ReadSignal<i32>) -> Element {
    let app_state = use_app_state();
    let files = use_memo(move || {
        reload_trigger(); //subscribe to changes
        get_cwl_files(project_path().join("workflows"))
    });
    let submodule_files = use_memo(move || {
        reload_trigger(); //subscribe to changes
        get_submodules_cwl_files(project_path())
    });

    rsx! {
        div { class: "flex flex-grow flex-col overflow-y-auto",
            h2 { class: "mt-2 font-bold flex gap-1 items-center",
                Icon {
                    width: ICON_SIZE,
                    height: ICON_SIZE,
                    icon: GoFileDirectory,
                }
                {app_state.read().project_name.as_ref().map_or("".to_string(), |p| p.to_string())}
            }
            ul {
                for item in files() {
                    li {
                        Link {
                            to: get_route(&item),
                            active_class: "font-bold",
                            class: "cursor-pointer select-none",
                            div { class: "flex gap-1 items-center",
                                div {
                                    class: "flex",
                                    style: "width: {ICON_SIZE.unwrap()}px; height: {ICON_SIZE.unwrap()}px;",
                                    img { src: asset!("/assets/CWL.svg") }
                                }

                                {item.name}
                            }
                        }
                    }
                }
            }
            for (module , files) in submodule_files() {
                h2 { class: "mt-2 font-bold flex gap-1 items-center",
                    Icon { width: ICON_SIZE, height: ICON_SIZE, icon: GoCloud }
                    {module}
                }
                ul {
                    for item in files {
                        li {
                            Link {
                                to: get_route(&item),
                                active_class: "font-bold",
                                class: "cursor-pointer select-none",
                                div { class: "flex gap-1 items-center",
                                    div {
                                        class: "flex",
                                        style: "width: {ICON_SIZE.unwrap()}px; height: {ICON_SIZE.unwrap()}px;",
                                        img { src: asset!("/assets/CWL.svg") }
                                    }

                                    {item.name}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn get_cwl_files(path: impl AsRef<Path>) -> Vec<Node> {
    let mut result = vec![];

    for entry in WalkBuilder::new(path).standard_filters(true).build().filter_map(Result::ok) {
        if entry.file_type().is_some_and(|t| t.is_file()) && entry.path().extension().is_some_and(|e| e.eq_ignore_ascii_case("cwl")) {
            let type_ = read_node_type(entry.path());

            result.push(Node {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: entry.path().to_path_buf(),
                children: vec![],
                is_dir: false,
                type_,
            });
        }
    }

    result
}

fn get_submodules_cwl_files(path: impl AsRef<Path>) -> HashMap<String, Vec<Node>> {
    let Ok(repo) = Repository::open(&path) else { return HashMap::new() };
    let mut map = HashMap::new();
    let Ok(submodules) = repo.submodules() else { return HashMap::new() };

    for module in submodules.iter() {
        let module_name = module.name().unwrap_or("unknown").to_string();
        map.insert(module_name, get_cwl_files(path.as_ref().join(module.path())));
    }

    map
}
