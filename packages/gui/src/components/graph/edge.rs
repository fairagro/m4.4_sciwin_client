use crate::{components::graph::Line, types::VisualNode, use_app_state};
use commonwl::CWLType;
use dioxus::prelude::*;
use petgraph::graph::EdgeIndex;

pub const HEADER_OFFSET: f32 = 18.0 + 4.0 + 6.0; //padding + height
pub const ITEM_HEIGHT: f32 = 28.0;
pub const NODE_WIDTH: f32 = 190.0;

pub(crate) fn calculate_source_position(source_node: &VisualNode, slot_id: &str) -> (f32, f32) {
    //get positions in array
    let fix = source_node.outputs.iter().position(|o| o.id == slot_id).unwrap_or_default();
    let y_source = HEADER_OFFSET + (fix as f32 * ITEM_HEIGHT) + (ITEM_HEIGHT / 2.0 + 5.0) + source_node.position.y;
    let x_source = NODE_WIDTH + source_node.position.x;
    (x_source, y_source)
}

fn calculate_target_position(target_node: &VisualNode, slot_id: &str) -> (f32, f32) {
    //get positions in array
    let tix = target_node.inputs.iter().position(|o| o.id == slot_id).unwrap_or_default();
    let y_target = HEADER_OFFSET
        + (tix as f32 * ITEM_HEIGHT)
        + (ITEM_HEIGHT / 2.0 + 5.0)
        + target_node.position.y
        + (target_node.outputs.len() as f32 * ITEM_HEIGHT);
    let x_target = target_node.position.x;
    (x_target, y_target)
}

pub(crate) fn get_stroke_from_cwl_type(type_: CWLType) -> &'static str {
    match type_ {
        CWLType::String => "stroke-red-400",
        CWLType::File => "stroke-green-400",
        CWLType::Directory => "stroke-blue-400",
        CWLType::Optional(_) => "stroke-red-700",
        CWLType::Array(_) => "stroke-green-700",
        _ => todo!(),
    }
}

#[derive(Props, Clone, Copy, PartialEq)]
pub struct EdgeProps {
    id: EdgeIndex,
}

#[component]
pub fn EdgeElement(props: EdgeProps) -> Element {
    let mut app_state = use_app_state();

    let graph = app_state().workflow.graph;
    let (from_node_id, to_node_id) = graph.edge_endpoints(props.id).unwrap(); //TODO!
    let from_node = &graph[from_node_id];
    let to_node = &graph[to_node_id];

    let edge = &graph[props.id];

    let (x_source, y_source) = calculate_source_position(from_node, &edge.source_port);
    let (x_target, y_target) = calculate_target_position(to_node, &edge.target_port);

    let slot_type = to_node.inputs.iter().find(|i| i.id == edge.target_port).unwrap().type_.clone();
    let stroke = get_stroke_from_cwl_type(slot_type);

    rsx! {
        Line {
            x_source,
            y_source,
            x_target,
            y_target,
            stroke,
            onclick: move |e: Event<MouseData>| {
                e.stop_propagation();
                if e.modifiers().shift() {
                    //disconnect on shift click
                    let mut state = app_state.write();
                    state.workflow.remove_connection(props.id)?;
                }
                Ok(())
            },
        }
    }
}
