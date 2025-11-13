use dioxus::{CapturedError, prelude::*};
use gui::code::CodeViewer;
use gui::components::tabs::*;
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
        document::Stylesheet { href: asset!("/assets/dx-components-theme.css") }
        document::Stylesheet { href: asset!("/assets/main.css") }
        document::Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "flex flex-col h-dvh overflow-hidden select-none",
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
            div {
                class: "flex-1 min-h-0",
                Content_Area {  }
            }
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

#[component]
pub fn Content_Area() -> Element {
    rsx!(
        Tabs{
            default_value: "editor".to_string(),
            TabList {
                TabTrigger { index: 0usize, value: "editor".to_string(), "Nodes"}
                TabTrigger { index: 1usize, value: "code".to_string(), "Code"}
            }
            TabContent{
                class: "h-dvh",
                index: 0usize,
                value: "editor".to_string(),
                GraphEditor {}
            }
            TabContent{ index: 1usize, value: "code".to_string(),
                CodeViewer {}
            }
        }
    )
}
