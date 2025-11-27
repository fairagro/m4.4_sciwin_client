use crate::{
    DragState,
    types::{Slot, SlotType},
    use_app_state, use_drag,
};
use commonwl::prelude::*;
use dioxus::prelude::*;
use petgraph::graph::NodeIndex;

#[derive(Props, Clone, PartialEq)]
pub(crate) struct SlotProps {
    node_id: NodeIndex,
    slot: Slot,
    slot_type: SlotType,
}

#[component]
pub fn SlotElement(props: SlotProps) -> Element {
    let mut drag_state = use_drag();

    let margin = match props.slot_type {
        SlotType::Input => "ml-[-9px]",
        SlotType::Output => "mr-[-9px]",
    };

    //TODO: more styling
    let geometry = match props.slot.type_ {
        CWLType::File | CWLType::Directory | CWLType::Stdout | CWLType::Stderr => "rotate-45",
        CWLType::Optional(_) => "",
        CWLType::Array(_) => "",
        _ => "rounded-lg",
    };

    let bg = match props.slot.type_ {
        CWLType::File => "bg-green-400",
        CWLType::Directory => "bg-blue-400",
        CWLType::String => "bg-red-400",
        _ => "",
    };

    let border = match props.slot.type_ {
        CWLType::Array(_) => "border border-3 border-green-700",
        CWLType::Optional(_) => "border border-3 border-red-700",
        _ => "border border-1 border-black",
    };

    let node_id = props.node_id;
    let slot_id = props.slot.id.clone();

    rsx! {
        div {
            onmousedown: move |_| {
                drag_state.write().dragging = Some(DragState::Connection {
                    source_node: node_id,
                    source_port: slot_id.clone(),
                });
            },
            onmouseup: move |_| {
                //check whether we are in connection mode and node/port has changed
                let graph = &use_app_state()().workflow.graph;
                if let Some(DragState::Connection { source_node, source_port }) = drag_state()
                    //get source and target nodes

                    //check whether this edge already exists
                    //do not create edge twice

                    //check valid connection type
                    .dragging && (source_node, &source_port) != (node_id, &props.slot.id)
                {
                    let source = &graph[source_node];
                    let target = &graph[node_id];
                    if graph.contains_edge(source_node, node_id) {
                        let edges = graph.edges_connecting(source_node, node_id);
                        for edge in edges {
                            if edge.weight().source_port == source_port
                                && edge.weight().target_port == props.slot.id
                            {
                                return Ok(());
                            }
                        }
                    }
                    let cwl_type_source = source
                        .outputs
                        .iter()
                        .find(|i| i.id == source_port)
                        .unwrap()
                        .type_
                        .clone();
                    let cwl_type_target = target
                        .inputs
                        .iter()
                        .find(|i| i.id == props.slot.id)
                        .unwrap()
                        .type_
                        .clone();
                    if cwl_type_source == cwl_type_target {
                        use_app_state()
                            .write()
                            .workflow
                            .add_connection(source_node, &source_port, node_id, &props.slot.id)?;
                    }
                }
                Ok(())
            },
            class: "{bg} w-3 h-3 m-2 {geometry} {margin} {border} z-2",
        }
    }
}
