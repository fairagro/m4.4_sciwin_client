use crate::{
    components::{ICON_SIZE, files::Node},
    files::{get_cwl_files, get_submodules_cwl_files},
    use_app_state,
};
use commonwl::load_doc;
use dioxus::{html::geometry::ClientPoint, prelude::*};
use dioxus_free_icons::{Icon, icons::go_icons::GoChevronRight};
use std::path::PathBuf;

#[component]
pub fn NodeAddForm(open: Signal<bool>, pos: Signal<ClientPoint>, project_path: ReadSignal<PathBuf>) -> Element {
    let app_state = use_app_state();
    let files = use_memo(move || {
        open();
        get_cwl_files(project_path().join("workflows"))
    });
    let submodule_files = use_memo(move || {
        open();
        get_submodules_cwl_files(project_path())
    });

    rsx! {
        if open() {
            div {
                class: "absolute z-15",
                style: "left: {pos().x}px; top: {pos().y}px;",
                onclick: move |_| open.set(false),
                ul {
                    li {
                        NodeAddItem {
                            name: app_state.read().project_name.as_ref().map_or("".to_string(), |p| p.to_string()),
                            files: files(),
                        }
                    }
                    for (module , files) in submodule_files() {
                        li {
                            NodeAddItem { name: module, files }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn NodeAddItem(name: String, files: Vec<Node>) -> Element {
    let mut app_state = use_app_state();
    let mut open = use_signal(|| false);

    rsx! {
        div {
            class: "flex",
            onmouseenter: move |_| open.set(true),
            onmouseleave: move |_| open.set(false),
            div { class: "flex w-48 bg-fairagro-light-200/80 hover:bg-fairagro-light-400 px-2 py-1 items-center justify-end",
                "{name}"
                div { class: "ml-auto",
                    Icon {
                        width: ICON_SIZE,
                        height: ICON_SIZE,
                        icon: GoChevronRight,
                    }
                }
            }
            if open() {
                div { class: "ml-auto absolute left-48",
                    ul {
                        for file in files {
                            li { class: "px-2 py-1 items-center bg-fairagro-light-200/80 hover:bg-fairagro-light-400",
                                button {
                                    onclick: move |_| {
                                        let mut cwl = load_doc(&file.path).map_err(|e| anyhow::anyhow!("{e}"))?;
                                        if let Some(path_relative_to_root) = pathdiff::diff_paths(
                                            &file.path,
                                            app_state().working_directory.unwrap(),
                                        ) {
                                            let name = file.name.strip_suffix(".cwl").unwrap_or(&file.name);
                                            app_state.write()
                                                .workflow
                                                .add_new_step_if_not_exists(
                                                    name,
                                                    path_relative_to_root.to_string_lossy().as_ref(),
                                                    &mut cwl,
                                                )?;
                                        }
                                        Ok(())
                                    },
                                    "{file.name}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
