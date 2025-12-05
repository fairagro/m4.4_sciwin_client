use crate::components::files::{Node, get_route};
use crate::components::{ICON_SIZE, SmallRoundActionButton};
use crate::files::{get_cwl_files, get_submodules_cwl_files};
use crate::use_app_state;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::go_icons::{GoCloud, GoFileDirectory, GoTrash};
use repository::Repository;
use repository::submodule::remove_submodule;
use std::path::PathBuf;

#[component]
pub fn SolutionView(project_path: ReadSignal<PathBuf>, reload_trigger: Signal<i32>) -> Element {
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
                if let Some(config) = &app_state.read().config {
                    "{config.workflow.name}"
                }
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
                                "{item.name}"
                            }
                        }
                    }
                }
            }
            for (module , files) in submodule_files() {
                Submodule_View { module, files, reload_trigger }
            }
        }
    }
}

#[component]
pub fn Submodule_View(module: String, files: Vec<Node>, reload_trigger: Signal<i32>) -> Element {
    let app_state = use_app_state();
    let mut hover = use_signal(|| false);

    rsx! {
        div {
            onmouseenter: move |_| hover.set(true),
            onmouseleave: move |_| hover.set(false),
            h2 { class: "mt-2 font-bold flex gap-1 items-center h-4",
                Icon { width: ICON_SIZE, height: ICON_SIZE, icon: GoCloud }
                "{module}"
                SmallRoundActionButton {
                    class: "ml-auto mr-3 hover:bg-fairagro-red-light",
                    title: "Uninstall {module}",
                    onclick: move |_| {
                        let repo = Repository::open(app_state().working_directory.unwrap())?;
                        remove_submodule(&repo, &module)?;
                        reload_trigger += 1;
                        Ok(())
                    },
                    if hover() {
                        Icon {
                            width: ICON_SIZE,
                            height: ICON_SIZE,
                            icon: GoTrash,
                        }
                    }
                }
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

                                "{item.name}"
                            }
                        }
                    }
                }
            }
        }
    }
}
