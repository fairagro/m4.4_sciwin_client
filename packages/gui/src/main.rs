use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;
use dioxus_free_icons::Icon as DioxusIcon;
use dioxus_free_icons::icons::go_icons::GoRocket;
use gui::{
    ApplicationState,
    code::CodeViewer,
    components::{
        footer::Footer,
        fs_view::FileSystemView,
        main::Main,
        sidebar::Sidebar,
        tabs::{TabContent, TabList, TabTrigger, Tabs},
    },
    graph::GraphEditor,
    use_app_state,
};
use rfd::FileDialog;
use s4n_core::config::Config as ProjectConfig;
use std::path::PathBuf;

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(
            Config::default()
                .with_menu(None)
                .with_window(
                    WindowBuilder::new()
                        .with_inner_size(LogicalSize::new(1270, 720))
                        .with_title("SciWIn Studio"),
                )
                .with_icon(Icon::from_rgba(include_bytes!("../assets/icon.rgba").to_vec(), 192, 192).unwrap()),
        )
        .launch(App);
}

#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(ApplicationState::default()));

    rsx! {
        document::Link { rel: "icon", href: asset!("/assets/icon.png") }
        document::Stylesheet { href: asset!("/assets/main.css") }
        document::Stylesheet { href: asset!("/assets/tailwind.css") }
        document::Stylesheet { href: asset!("/assets/bundle.min.css") }

        div {
            class: "h-screen w-full flex flex-col",
            div {
                class: "h-full w-full flex flex-row flex-1",
                Sidebar {
                    form {
                        onsubmit: move |e| {
                            e.prevent_default();
                            let path =  FileDialog::new().pick_folder().unwrap();

                            //get workflow.toml
                            let config_path = path.join("workflow.toml");
                            if !config_path.exists() {
                                //ask user to init a new project
                                return Ok(());
                            } else {
                                let toml = std::fs::read_to_string(config_path).unwrap();
                                let config: ProjectConfig = toml::from_str(&toml).unwrap();
                                use_app_state().write().project_name = Some(config.workflow.name);
                            }

                            use_app_state().write().working_directory = Some(path.clone());
                            Ok(())
                        },
                        input {
                            r#type: "submit",
                            value: "Load Project",
                            class: "rounded-lg bg-green-500 px-3 py-1 my-5 cursor-pointer"
                        },
                    }
                    h2 {
                        {use_app_state().read().project_name.as_ref().map_or("".to_string(), |p| format!("Project: {p}" ))}
                    }
                    if use_app_state().read().working_directory.is_some() {
                        FileSystemView{
                            project_path: use_app_state().read().working_directory.clone().unwrap_or(PathBuf::from("."))
                        }
                    }
                    else {
                        div {
                            class: "flex flex-col items-center mt-10 gap-4 text-lg text-center text-zinc-400",
                            DioxusIcon { width: Some(64), height: Some(64), icon: GoRocket }
                            div { "Start by loading up a project" }
                        }
                    }

                }
                Main {
                    if use_app_state().read().workflow.path.is_some() {
                        Content_Area {  }
                    }
                }
            }
            Footer {
                if let Some(path) = &use_app_state().read().workflow.path {
                    {path.to_string_lossy().to_string()}
                }
            }
        }

    }
}

#[component]
pub fn Content_Area() -> Element {
    rsx!(
        Tabs{
            class: "h-full",
            default_value: "editor".to_string(),
            TabList {
                TabTrigger { index: 0usize, value: "editor".to_string(), "Nodes"}
                TabTrigger { index: 1usize, value: "code".to_string(), "Code"}
            }
            TabContent{
                index: 0usize,
                value: "editor".to_string(),
                GraphEditor {}
            }
            TabContent{
                index: 1usize,
                value: "code".to_string(),
                CodeViewer {}
            }
        }
    )
}
