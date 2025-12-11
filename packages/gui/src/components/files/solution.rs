use crate::components::files::{Node, get_route};
use crate::components::{ICON_SIZE, SmallRoundActionButton};
use crate::files::{get_cwl_files, get_submodules_cwl_files};
use crate::layout::{RELOAD_TRIGGER, Route};
use crate::use_app_state;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::go_icons::{GoCloud, GoFileDirectory, GoTrash};
use repository::Repository;
use repository::submodule::remove_submodule;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[component]
pub fn SolutionView(project_path: ReadSignal<PathBuf>, dialog_signals: (Signal<bool>, Signal<bool>)) -> Element {
    let mut app_state = use_app_state();
    let files = use_memo(move || {
        RELOAD_TRIGGER(); //subscribe to changes
        get_cwl_files(project_path().join("workflows"))
    });
    let submodule_files = use_memo(move || {
        RELOAD_TRIGGER(); //subscribe to changes
        get_submodules_cwl_files(project_path())
    });

    let mut hover = use_signal(|| false);

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
                onmouseenter: move |_| hover.set(true),
                onmouseleave: move |_| hover.set(false),
                for item in files() {
                    li {
                        class: "select-none",
                        draggable: true,
                        ondragstart: move |e| {
                            e.data_transfer().set_effect_allowed("all");
                            e.data_transfer().set_drop_effect("move");
                            app_state.write().set_data_transfer(&item)?;
                            e.data_transfer()
                                .set_data("application/x-allow-dnd", "1")
                                .map_err(|e| anyhow::anyhow!("{e}"))?;
                            Ok(())
                        },
                        div { class: "flex",
                            Link {
                                draggable: "false",
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
                            if hover() {
                                SmallRoundActionButton {
                                    class: "ml-auto mr-3 hover:bg-fairagro-red-light",
                                    title: "Delete {item.name}",
                                    onclick: {
                                        //we need to double clone here ... ugly :/
                                        let item = item.clone();
                                        move |_| {
                                            let item = item.clone();
                                            async move {
                                                //0 open, 1 confirmed
                                                dialog_signals.0.set(true);
                                                loop {
                                                    if !dialog_signals.0() {
                                                        if dialog_signals.1() {
                                                            fs::remove_file(&item.path)?;
                                                            *RELOAD_TRIGGER.write() += 1;
                                                            let current_path = match use_route() {
                                                                Route::WorkflowView { path } => path.to_string(),
                                                                Route::ToolView { path } => path.to_string(),
                                                                _ => String::new(),
                                                            };
                                                            if current_path == item.path.to_string_lossy() {
                                                                router().push("/");
                                                            }
                                                        }
                                                        break;
                                                    }
                                                    tokio::time::sleep(Duration::from_millis(100)).await;
                                                }
                                                Ok(())
                                            }
                                        }
                                    },
                                    Icon {
                                        width: 10,
                                        height: 10,
                                        icon: GoTrash,
                                    }
                                }
                            }
                        }
                    }
                }
            }
            for (module , files) in submodule_files() {
                Submodule_View { module, files, dialog_signals }
            }
        }
    }
}

#[component]
pub fn Submodule_View(module: String, files: Vec<Node>, dialog_signals: (Signal<bool>, Signal<bool>)) -> Element {
    let mut app_state = use_app_state();
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
                        let module = module.clone();
                        async move {
                            //0 open, 1 confirmed
                            dialog_signals.0.set(true);
                            loop {
                                if !dialog_signals.0() {
                                    if dialog_signals.1() {
                                        let repo = Repository::open(
                                            //reset

                                            app_state().working_directory.unwrap(),
                                        )?;
                                        remove_submodule(&repo, &module)?;
                                        *RELOAD_TRIGGER.write() += 1;
                                        dialog_signals.1.set(false);
                                    }
                                    break;
                                }
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }
                            Ok(())
                        }
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
                        class: "select-none",
                        draggable: true,
                        ondragstart: move |e| {
                            e.data_transfer().set_effect_allowed("all");
                            e.data_transfer().set_drop_effect("move");
                            app_state.write().set_data_transfer(&item)?;
                            e.data_transfer()
                                .set_data("application/x-allow-dnd", "1")
                                .map_err(|e| anyhow::anyhow!("{e}"))?;
                            Ok(())
                        },
                        Link {
                            draggable: "false",
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
