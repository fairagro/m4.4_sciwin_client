use crate::use_app_state;
use commonwl::prelude::*;
use dioxus::prelude::*;
use petgraph::graph::EdgeIndex;

#[derive(Debug, Clone)]
pub struct Edge {
    pub source_port: String,
    pub target_port: String,
    pub data_type: CWLType,
}
#[derive(Props, Clone, Copy, PartialEq)]
pub struct EdgeProps {
    id: EdgeIndex,
}

#[component]
pub fn EdgeElement(props: EdgeProps) -> Element {
    let graph = use_app_state()().graph;
    let (from_node_id, to_node_id) = graph.edge_endpoints(props.id).unwrap(); //TODO!
    let from_node = &graph[from_node_id];
    let to_node = &graph[to_node_id];

    let edge = &graph[props.id];

    //get positions in array
    let fix = from_node.outputs.iter().position(|o| o.id == edge.source_port).unwrap();
    let tix = to_node.inputs.iter().position(|i| i.id == edge.target_port).unwrap();

    const HEADER_OFFSET: f32 = 24.0 + 4.0 + 4.0; //padding + height
    const ITEM_HEIGHT: f32 = 28.0;
    const NODE_WIDTH: f32 = 190.0;

    let y_source = HEADER_OFFSET + (fix as f32 * ITEM_HEIGHT) + (ITEM_HEIGHT / 2.0 + 5.0) + from_node.position.y;
    let x_source = NODE_WIDTH + from_node.position.x;

    let y_target =
        HEADER_OFFSET + (tix as f32 * ITEM_HEIGHT) + (ITEM_HEIGHT / 2.0 + 5.0) + to_node.position.y + (to_node.outputs.len() as f32 * ITEM_HEIGHT);
    let x_target = to_node.position.x;

    // Control points for a simple horizontal curve
    let cx1 = x_source + 50.0; // move 50px to the right from source
    let cy1 = y_source;

    let cx2 = x_target - 50.0; // move 50px to the left from target
    let cy2 = y_target;

    let path_data = format!(
        "M {} {} C {} {}, {} {}, {} {}",
        x_source, y_source, cx1, cy1, cx2, cy2, x_target, y_target
    );

    let slot_type = to_node.inputs.iter().find(|i| i.id == edge.target_port).unwrap().type_.clone();
    let stroke = match slot_type {
        CWLType::String => "stroke-red-400",
        CWLType::File => "stroke-green-400",
        CWLType::Directory => "stroke-blue-400",
        CWLType::Optional(_) => "stroke-red-700",
        CWLType::Array(_) => "stroke-green-700",
        _ => todo!(),
    };

    rsx! {
        div {
            class: "absolute w-0 h-0 z-[-1]",
            left: 0,
            top: 0,
            svg {
                class: "overflow-visible w-0 h-0",
                path {
                    d: "{path_data}",
                    class: "{stroke}",
                    fill: "transparent",
                    stroke_width: "3",
                }
            }
        }
    }
}
