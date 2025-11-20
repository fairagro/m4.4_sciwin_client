use dioxus::{CapturedError, prelude::*};
use gui::code::CodeViewer;
use gui::components::main::Main;
use gui::components::sidebar::Sidebar;
use gui::components::tabs::{TabContent, TabList, TabTrigger, Tabs};
use gui::graph::GraphEditor;
use gui::workflow::VisualWorkflow;
use gui::{ApplicationState, use_app_state};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(ApplicationState::default()));

    rsx! {
        document::Link { rel: "icon", href: asset!("/assets/favicon.ico") }
        document::Stylesheet { href: asset!("/assets/main.css") }
        document::Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "h-screen w-full flex",
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

/*
        div {
            class: "flex flex-col h-dvh overflow-hidden select-none p-1",
            Logo {}

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
            Content_Area {  }
        }
    }
}

#[component]
pub fn Logo() -> Element {
    rsx! {
        div {
            img { src: asset!("/assets/logo.svg"), width: 150 }
        }
    }
}


 */
