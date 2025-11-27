use crate::workflow::VisualWorkflow;
use dioxus::{html::geometry::ClientPoint, prelude::*};
use petgraph::graph::NodeIndex;
use std::path::PathBuf;

pub mod components;
pub mod types;
pub mod graph;
pub mod layout;
pub mod workflow;

#[derive(Default, Clone, Debug)]
pub struct ApplicationState {
    pub working_directory: Option<PathBuf>,
    pub project_name: Option<String>,
    pub workflow: VisualWorkflow,
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

#[derive(Default, Clone, Debug)]
pub struct DragContext {
    pub dragging: Option<DragState>,
    pub drag_offset: Signal<ClientPoint>,
}

pub fn use_app_state() -> Signal<ApplicationState> {
    use_context::<Signal<ApplicationState>>()
}

pub fn use_drag() -> Signal<DragContext> {
    use_context::<Signal<DragContext>>()
}
