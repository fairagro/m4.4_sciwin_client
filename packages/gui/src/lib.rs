use crate::graph::WorkflowGraph;
use dioxus::{html::geometry::ClientPoint, prelude::*};
use petgraph::graph::{EdgeIndex, NodeIndex};

pub mod edge;
pub mod graph;
pub mod node;
pub mod slot;
pub mod code;
pub mod components;

#[derive(Default, Clone)]
pub struct ApplicationState {
    pub graph: WorkflowGraph,
    pub dragging: Option<NodeIndex>,
    pub drag_offset: Signal<ClientPoint>,
    pub selected_edge: Option<EdgeIndex>, 
    pub cwl_code: Option<String>,
}

pub fn use_app_state() -> Signal<ApplicationState> {
    use_context::<Signal<ApplicationState>>()
}
