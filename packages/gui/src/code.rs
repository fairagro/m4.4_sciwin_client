use dioxus::prelude::*;
use crate::use_app_state;

#[component]
pub fn CodeViewer() -> Element {
    let mut app_state = use_app_state();

    // Get the CWL code from state
    let code = match &app_state().cwl_code {
        Some(c) => c.clone(),
        None => "No CWL code loaded.".to_string(),
    };

    rsx! {
        div {
            class: "flex flex-col h-full p-4 bg-gray-900 text-white overflow-auto",
            div {
                class: "flex justify-between mb-2",
                h2 { class: "text-lg font-bold", "CWL Code Viewer" },
                button {
                    class: "rounded bg-gray-700 px-2 py-1 hover:bg-gray-600 transition",
                    onclick: move |_| {
                        app_state.write().show_code = false;
                    },
                    "Back"
                }
            }
            pre {
                class: "whitespace-pre-wrap overflow-x-auto bg-gray-800 p-4 rounded-lg",
                "{code}"
            }
        }
    }
}