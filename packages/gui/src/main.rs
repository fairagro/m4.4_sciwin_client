use dioxus::{CapturedError, prelude::*};
use gui::graph::{GraphEditor, load_workflow_graph};
use gui::{ApplicationState, use_app_state};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(ApplicationState::default()));
    rsx! {
            document::Link { rel: "icon", href:  asset!("/assets/favicon.ico") }
            document::Stylesheet {  href: asset!("/assets/main.css") }
            document::Stylesheet { href: asset!("/assets/tailwind.css") }
            div {
                class: "flex flex-col h-dvh overflow-hidden select-none",
                Logo { }
                form {
                    onsubmit: move |e| {
                        e.prevent_default();
                        let FormValue::Text(path) = e.get_first("path").unwrap() else {return Err(CapturedError::msg("Mist"))};
                        let graph = load_workflow_graph(path)?;
                        use_app_state().write().graph = graph;
                        Ok(())
                    },
                    input { r#type: "text", name: "path", placeholder: "Path to CWL", value:"/mnt/m4.4_sciwin_client_demo/workflows/demo/demo.cwl" },
                    input { r#type: "submit", value: "Load CWL", class: "rounded-lg bg-green-500 px-3 py-1 my-5 cursor-pointer"}
                }
                GraphEditor {  }
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
