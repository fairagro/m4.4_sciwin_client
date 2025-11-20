use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::{CapturedError, prelude::*};
use gui::code::CodeViewer;
use gui::components::footer::Footer;
use gui::components::main::Main;
use gui::components::sidebar::Sidebar;
use gui::components::tabs::{TabContent, TabList, TabTrigger, Tabs};
use gui::graph::GraphEditor;
use gui::workflow::VisualWorkflow;
use gui::{ApplicationState, use_app_state};

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

        div {
            class: "h-screen w-full flex flex-col",
            div {
                class: "h-full w-full flex flex-row flex-1",
                Sidebar {
                    h2 {
                        {use_app_state().read().working_directory.as_ref().map_or("No Project Loaded".to_string(), |p| format!("Project: {}", p.display()))}

                        //will be removed with proper project loading
                        form {
                            onsubmit: move |e| {
                                e.prevent_default();
                                let FormValue::Text(path) = e.get_first("path").unwrap()
                                    else { return Err(CapturedError::msg("Missing path")) };
                                let workflow = VisualWorkflow::from_file(path)?;
                                use_app_state().write().workflow = workflow;
                                Ok(())
                            },
                            input {
                                r#type: "text",
                                name: "path",
                                placeholder: "Path to CWL",
                                value: "/mnt/m4.4_sciwin_client_demo/workflows/demo/demo.cwl"
                            },
                            input {
                                r#type: "submit",
                                value: "Load CWL",
                                class: "rounded-lg bg-green-500 px-3 py-1 my-5 cursor-pointer"
                            }
                        }
                    }
                }
                Main {
                    Content_Area {  }
                }
            }
            Footer {
                {use_app_state().read().workflow.path.to_string_lossy().to_string()}
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
