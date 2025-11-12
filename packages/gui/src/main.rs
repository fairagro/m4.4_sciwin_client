use dioxus::html::geometry::euclid::Point2D;
use dioxus::{CapturedError, prelude::*};
use gui::edge::Edge;
use gui::graph::load_workflow_graph;
use gui::node::Node;
use gui::{ApplicationState, use_app_state};
use petgraph::visit::IntoNodeIdentifiers;

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
        Graph {  }
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
pub fn Graph() -> Element {
    let graph = use_app_state()().graph;

    rsx! {
        div {
            div {
                class: "relative w-100 inset-0 select-none",
                onmousemove: move |e| {
                    if let Some(current) = use_app_state()().dragging{
                        //we are dragging
                        let current_pos = e.data.client_coordinates();
                        let last_pos = (use_app_state()().drag_offset)();

                        let deltaX = current_pos.x - last_pos.x;
                        let deltaY = current_pos.y - last_pos.y;
                        let pos = use_app_state()().graph[current].position;
                        use_app_state().write().graph[current].position = Point2D::new(pos.x + deltaX as f32, pos.y + deltaY as f32);

                        use_app_state().write().drag_offset.set(current_pos);
                    }
                },
                onmouseup: move |_| {
                    use_app_state().write().dragging = None;
                },
                for id in graph.node_identifiers() {
                    Node {id}
                },
                for id in graph.edge_indices() {
                    Edge {id}
                }
            },
            div {
                //"Debug: {graph:?}"
            }
        }
    }
}
