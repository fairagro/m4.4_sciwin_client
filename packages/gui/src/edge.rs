use crate::{node::VisualNode, use_app_state};
use commonwl::prelude::*;
use dioxus::prelude::*;
use petgraph::graph::EdgeIndex;

#[derive(Debug, Clone)]
pub struct VisualEdge {
    pub source_port: String,
    pub target_port: String,
    pub data_type: CWLType,
}

#[derive(Props, Clone, Copy, PartialEq)]
pub struct EdgeProps {
    id: EdgeIndex,
}

pub const HEADER_OFFSET: f32 = 18.0 + 4.0 + 6.0; //padding + height
pub const ITEM_HEIGHT: f32 = 28.0;
pub const NODE_WIDTH: f32 = 190.0;

pub fn calculate_source_position(source_node: &VisualNode, slot_id: &str) -> (f32, f32) {
    //get positions in array
    let fix = source_node.outputs.iter().position(|o| o.id == slot_id).unwrap_or_default();
    let y_source = HEADER_OFFSET + (fix as f32 * ITEM_HEIGHT) + (ITEM_HEIGHT / 2.0 + 5.0) + source_node.position.y;
    let x_source = NODE_WIDTH + source_node.position.x;
    (x_source, y_source)
}
pub fn calculate_target_position(target_node: &VisualNode, slot_id: &str) -> (f32, f32) {
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

pub fn get_stroke_from_cwl_type(type_: CWLType) -> &'static str {
    match type_ {
        CWLType::String => "stroke-red-400",
        CWLType::File => "stroke-green-400",
        CWLType::Directory => "stroke-blue-400",
        CWLType::Optional(_) => "stroke-red-700",
        CWLType::Array(_) => "stroke-green-700",
        _ => todo!(),
    }
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

#[derive(Props, Clone, PartialEq, Default)]
pub struct LineProps {
    pub x_source: f32,
    pub y_source: f32,
    pub x_target: f32,
    pub y_target: f32,
    pub stroke: String,
    pub onclick: Option<EventHandler<MouseEvent>>,
}
#[component]
pub fn Line(props: LineProps) -> Element {
    let cx1 = props.x_source + 25.0; // move 50px to the right from source
    let cy1 = props.y_source;

    let cx2 = props.x_target - 25.0; // move 50px to the left from target
    let cy2 = props.y_target;

    let path_data = format!(
        "M {} {} C {} {}, {} {}, {} {}",
        props.x_source, props.y_source, cx1, cy1, cx2, cy2, props.x_target, props.y_target
    );

    let stroke_width = 3;

    rsx! {
        div {
            class: "absolute z-[1]",
            left: 0,
            top: 0,
            svg {
                view_box: "0 0 1920 1080",
                class: "overflow-visible block",
                onclick: move |e| {
                    if let Some(handler) = props.onclick {
                        handler.call(e);
                    }
                },
                path {
                    class: "{props.stroke}",
                    d: "{path_data}",
                    stroke_width: "{stroke_width}",
                    fill: "transparent",
                    style: "cursor: pointer;",
                }
            }
        }
    }
}
