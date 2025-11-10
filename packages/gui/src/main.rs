use dioxus::prelude::*;
use gui::graph::{WorkflowGraph, load_workflow_graph};
use petgraph::graph::NodeIndex;
use petgraph::visit::IntoNodeIdentifiers;

#[derive(Default, Clone)]
pub struct ApplicationState {
    graph: WorkflowGraph,
}

pub fn use_app_state() -> Signal<ApplicationState> {
    use_context::<Signal<ApplicationState>>()
}

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
        Load_Btn {  }
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
pub fn Load_Btn() -> Element {
    rsx! {
        button {
            class: "rounded-lg bg-green-500 px-3 py-1 my-5 cursor-pointer",
            onclick: move |_| {
                let graph = load_workflow_graph("testdata/hello_world/workflows/main/main.cwl")?;
                use_app_state().write().graph = graph;
                Ok(())
            },
            "Load CWL"
        }
    }
}

#[component]
pub fn Graph() -> Element {
    let graph = use_app_state()().graph;

    rsx! {
        div {
            div {
                class: "relative h-full w-full",
                for id in graph.node_identifiers() {
                    Node {id}
                },
            },
            div {
                "Debug: {graph:?}"
            }
        }
    }
}

#[derive(Props, Clone, Copy, PartialEq)]
pub struct NodeProps {
    id: NodeIndex,
}

#[component]
pub fn Node(props: NodeProps) -> Element {
    let graph = use_app_state()().graph;
    let node = &graph[props.id];
    let pos_x = props.id.index() * 100;

    rsx! {
        div {
            class: "absolute",
            left: "{pos_x}px",
            "{node.id()}"
        }
    }
}
