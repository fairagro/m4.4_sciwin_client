use crate::{
    ApplicationState,
    components::{
        CodeViewer, Dialog, ICON_SIZE, NoProject, close_dialog,
        files::{FilesView, View},
        graph::GraphEditor,
        layout::{Footer, Main, Sidebar, TabContent, TabList, TabTrigger, Tabs},
    },
    last_session_data, open_project, restore_last_session, use_app_state,
};
use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::go_icons::{GoRepo, GoX},
};
use rfd::FileDialog;
use std::{fs, path::PathBuf};

#[component]
pub fn Layout() -> Element {
    let mut app_state = use_app_state();
    let working_dir = use_memo(move || app_state.read().working_directory.clone());

    let mut view = use_signal(|| View::Solution);
    let route: Route = use_route();
    let mut route_rx = use_reactive(&route, |route| route);

    let mut dialog = use_context::<Signal<Option<Dialog>>>();
    let open_dialog = move |title: String, message: String| {
        dialog.set(Some(Dialog::new(&title, &message)));
    };
    let close_dialog = move || close_dialog(dialog);

    {
        use_effect(move || {
            app_state.write().current_file = match route_rx() {
                Route::Empty => None,
                Route::WorkflowView { path } => Some(PathBuf::from(path)),
                Route::ToolView { path } => Some(PathBuf::from(path)),
            };

            let serialized = serde_json::to_string(&app_state()).expect("Could not serialize app state");
            fs::write(last_session_data(), serialized).expect("Could not save app state");
        });
    }

    rsx! {
        div {
            class: "h-screen w-screen grid grid-rows-[1fr_1.5rem]",
            onmounted: move |_| async move{
                spawn(async move{if let Some(last_session) = restore_last_session(open_dialog, close_dialog).await.unwrap() {
                    app_state.set(last_session)
                }});
                Ok(())
            },
            div { class: "flex min-h-0 h-full w-full overflow-x-clip relative",
                Sidebar {
                    form {
                        onsubmit: move |e| {
                            e.prevent_default();
                            let path = FileDialog::new().pick_folder().unwrap();
                            spawn(
                                async move {if let Some(info) = open_project(path, open_dialog, close_dialog).await.unwrap() {
                                app_state.write().working_directory = Some(info.working_directory);
                                app_state.write().project_name = Some(info.project_name);
                            }});
                            Ok(())
                        },
                        input {
                            r#type: "submit",
                            value: "Load Project",
                            class: "rounded-lg bg-fairagro-light-500 px-3 py-1 my-5 cursor-pointer",
                        }
                    }
                    if let Some(project_name) = &app_state.read().project_name {
                        h2 { class: "text-fairagro-dark-500 mb-2 text-sm flex items-center gap-1.5",
                            Icon { icon: GoRepo, width: 16, height: 16 }
                            div { "{project_name}" }
                            button {
                                class: "p-1 hover:bg-fairagro-red-light/20 rounded-xl text-fairagro-red",
                                title: "Close Project",
                                onclick: move |_| {
                                    fs::remove_file(last_session_data())?;
                                    app_state.set(ApplicationState::default());
                                    router().push("/");
                                    Ok(())
                                },
                                Icon {
                                    icon: GoX,
                                    width: ICON_SIZE,
                                    height: ICON_SIZE,
                                }
                            }
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
            Footer {
                match &route {
                    Route::Empty => "".to_string(),
                    Route::WorkflowView { path } => path.to_string(),
                    Route::ToolView { path } => path.to_string(),
                }
            }
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
                index: 1usize,
                class: "h-full min-h-0",
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
            TabContent {
                index: 0usize,
                class: "h-full min-h-0",
                value: "code".to_string(),
                CodeViewer { path }
            }
        }
    }
}
