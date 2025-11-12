use dioxus::{CapturedError, prelude::*};
use gui::graph::{GraphEditor, load_workflow_graph};
use gui::{ApplicationState, use_app_state};
use gui::code::CodeViewer;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(ApplicationState::default()));
    let app_state = use_app_state();

    // when show_code is true, render code viewer
    if app_state().show_code {
        return rsx! { CodeViewer {} };
    }

    rsx! {
        document::Link { rel: "icon", href: asset!("/assets/favicon.ico") }
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
                    let (graph, cwl_code) = load_workflow_graph(path)?;

                    let mut binding = use_app_state();
                    let mut state = binding.write();

                    state.graph = graph;
                    state.cwl_code = Some(cwl_code);
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
                class: "flex justify-between items-center px-4 mb-2",
                button {
                    class: "rounded bg-gray-700 text-white px-3 py-1 hover:bg-gray-600 transition",
                    onclick: move |_| {
                        let mut binding = use_app_state();
                        if binding().cwl_code.is_some() {
                            binding.write().show_code = true;
                        }
                    },
                    "View CWL Code"
                }
            }

            GraphEditor {}
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
