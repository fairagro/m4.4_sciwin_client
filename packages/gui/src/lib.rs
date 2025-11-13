use crate::workflow::VisualWorkflow;
use dioxus::{html::geometry::ClientPoint, prelude::*};
use petgraph::graph::NodeIndex;

pub mod code;
pub mod components;
pub mod edge;
pub mod graph;
pub mod node;
pub mod slot;
pub mod workflow;

#[derive(Default, Clone, Debug)]
pub struct ApplicationState {
    pub workflow: VisualWorkflow,
    pub dragging: Option<DragState>,
    pub drag_offset: Signal<ClientPoint>,
}

#[derive(Default, Debug, Clone)]
pub enum DragState {
    #[default]
    None, // not used maybe
    Node(NodeIndex), //used when drag starts on Node Header
    Connection {
        //used when drag starts from slot
        source_node: NodeIndex,
        source_port: String,
    },
}

pub fn use_app_state() -> Signal<ApplicationState> {
    use_context::<Signal<ApplicationState>>()
}
