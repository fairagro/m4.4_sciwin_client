use crate::{
    components::{
        CodeViewer, NoProject,
        files::{FilesView, View},
        graph::GraphEditor,
        layout::{Footer, Main, Sidebar, TabContent, TabList, TabTrigger, Tabs},
    },
    use_app_state,
};
use dioxus::prelude::*;
use rfd::FileDialog;
use s4n_core::config::Config as ProjectConfig;

#[component]
pub fn Layout() -> Element {
    let mut app_state = use_app_state();
    let working_dir = use_memo(move || app_state.read().working_directory.clone());

    let mut view = use_signal(|| View::Solution);
    let route: Route = use_route();
    let current_path = use_memo(move || match &route {
        Route::Empty => "".to_string(),
        Route::WorkflowView { path } => path.to_string(),
        Route::ToolView { path } => path.to_string(),
    });

    rsx! {
        div { class: "h-screen w-screen grid grid-rows-[1fr_1.5rem]",
            div { class: "h-full w-full grid grid-cols-[auto_1fr] min-h-0",
                Sidebar {
                    form {
                        onsubmit: move |e| {
                            e.prevent_default();
                            let path = FileDialog::new().pick_folder().unwrap();
                            //get workflow.toml
                            let config_path = path.join("workflow.toml");
                            if !config_path.exists() {
                                //ask user to init a new project
                                return Ok(());
                            } else {
                                let toml = std::fs::read_to_string(config_path).unwrap();
                                let config: ProjectConfig = toml::from_str(&toml).unwrap();
                                app_state.write().project_name = Some(config.workflow.name);
                            }
                            app_state.write().working_directory = Some(path.clone());
                            Ok(())
                        },
                        input {
                            r#type: "submit",
                            value: "Load Project",
                            class: "rounded-lg bg-green-500 px-3 py-1 my-5 cursor-pointer",
                        }
                    }
                    h2 {
                        {
                            app_state
                                .read()
                                .project_name
                                .as_ref()
                                .map_or("".to_string(), |p| format!("Project: {p}"))
                        }
                    }
                    if let Some(working_dir) = working_dir() {
                        select {
                            onchange: move |e| view.set(e.value().parse().unwrap()),
                            class: "form-select appearance-none rounded-base bg-zinc-300 w-full px-2 py-1.5 font-bold bg-no-repeat",
                            option { value: "Solution", "Solution" }
                            option { value: "FileSystem", "Filesystem" }
                        }
                        FilesView { working_dir, view }
                    } else {
                        NoProject {}
                    }
                }
                Main { Outlet::<Route> {} }
            }
            Footer {{current_path}}
        }
    }
}

#[derive(Clone, Debug, PartialEq, Routable)]
pub enum Route {
    #[layout(Layout)]
    #[route("/")]
    Empty,

    #[route("/workflow?:path")]
    WorkflowView { path: String },

    #[route("/tool?:path")]
    ToolView { path: String },
}

#[component]
pub fn Empty() -> Element {
    rsx!(div {})
}

#[component]
pub fn WorkflowView(path: String) -> Element {
    rsx!(
        Tabs { class: "h-full min-h-0", default_value: "editor".to_string(),
            TabList {
                TabTrigger { index: 0usize, value: "editor".to_string(), "Nodes" }
                TabTrigger { index: 1usize, value: "code".to_string(), "Code" }
            }
            TabContent {
                index: 0usize,
                class: "h-full min-h-0",
                value: "editor".to_string(),
                GraphEditor { path: path.clone() }
            }
            TabContent {
                class: "h-full min-h-0",
                index: 1usize,
                value: "code".to_string(),
                CodeViewer { path: path.clone() }
            }

        }
    )
}

#[component]
pub fn ToolView(path: String) -> Element {
    rsx! {
        Tabs { class: "h-full min-h-0", default_value: "code".to_string(),
            TabList {
                TabTrigger { index: 0usize, value: "code".to_string(), "Code" }
            }
            TabContent { index: 0usize, value: "code".to_string(),
                CodeViewer { path }
            }
        }
    }
}
