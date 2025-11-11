use commonwl::CWLType;
use dioxus::html::geometry::ClientPoint;
use dioxus::html::geometry::euclid::Point2D;
use dioxus::{CapturedError, prelude::*};
use gui::graph::{NodeInstance, WorkflowGraph, load_workflow_graph};
use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::visit::IntoNodeIdentifiers;

#[derive(Default, Clone)]
pub struct ApplicationState {
    graph: WorkflowGraph,
    dragging: Option<NodeIndex>,
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
                class: "relative h-full w-full select-none",
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

#[derive(Props, Clone, Copy, PartialEq)]
pub struct NodeProps {
    id: NodeIndex,
}

#[derive(Clone, PartialEq)]
enum SlotType {
    Input,
    Output,
}

#[derive(Props, Clone, PartialEq)]
struct SlotProps {
    type_: CWLType,
    slot_type: SlotType,
}

#[component]
pub fn SlotElement(props: SlotProps) -> Element {
    let margin = match props.slot_type {
        SlotType::Input => "ml-[-9px]",
        SlotType::Output => "mr-[-9px]",
    };

    //TODO: more styling
    let geometry = match props.type_ {
        CWLType::File | CWLType::Directory | CWLType::Stdout | CWLType::Stderr => "rotate-45",
        CWLType::Optional(_) => "",
        CWLType::Array(_) => "",
        _ => "rounded-lg",
    };

    let bg = match props.type_ {
        CWLType::File => "bg-green-400",
        CWLType::Directory => "bg-blue-400",
        CWLType::String => "bg-red-400",
        _ => "",
    };

    let border = match props.type_ {
        CWLType::Array(_) => "border border-3 border-green-700",
        CWLType::Optional(_) => "border border-3 border-red-700",
        _ => "border border-1 border-black",
    };

    rsx! {
        div {
            class: "{bg} w-2 h-2 m-2 {geometry} {margin} {border}"
        }
    }
}

#[component]
pub fn Node(props: NodeProps) -> Element {
    let graph = use_app_state()().graph;
    let node = &graph[props.id];
    let pos_x = node.position.x;
    let pos_y = node.position.y;

    let top_color = match node.instance {
        NodeInstance::Step(_) => "bg-green-900",
        NodeInstance::Input(_) => "bg-blue-900",
        NodeInstance::Output(_) => "bg-red-900",
    };

    let mut drag_offset = use_signal(ClientPoint::zero);

    rsx! {
        div {
            class: "absolute border bg-gray-800 rounded-lg cursor-pointer w-48",
            left: "{pos_x}px",
            top: "{pos_y}px",
            div {
                onmousedown: move |e| {
                    drag_offset.write().x = e.data.client_coordinates().x;
                    drag_offset.write().y = e.data.client_coordinates().y;

                    use_app_state().write().dragging = Some(props.id);
                },
                onmousemove: move |e| {
                    if let Some(current) = use_app_state()().dragging{
                        //we are dragging
                        let last_pos = drag_offset();
                        let deltaX = e.data.client_coordinates().x - last_pos.x;
                        let deltaY = e.data.client_coordinates().y - last_pos.y;

                        let pos = use_app_state()().graph[current].position;
                        use_app_state().write().graph[current].position = Point2D::new(pos.x+deltaX as f32, pos.y+deltaY as f32);
                    }
                },
                onmouseup: move |_| {
                    use_app_state().write().dragging = None;
                },
                onmouseleave: move  |_| {
                    use_app_state().write().dragging = None;
                },
                class: "{top_color} rounded-t-lg p-1 overflow-hidden",
                "{node.instance.id()}",

            },
            div { // slot wrapper
                class: "p-1",

                div{
                    for slot in node.outputs.iter() {
                        div {
                            class: "flex justify-end items-center",
                            "{slot.id}",
                            SlotElement {type_: slot.type_.clone(), slot_type: SlotType::Output}
                        }
                    }
                }

                div {
                    for slot in node.inputs.iter() {
                        div {
                            class: "flex justify-start items-center",
                            SlotElement {type_: slot.type_.clone(), slot_type: SlotType::Input},
                            "{slot.id}"
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, Copy, PartialEq)]
pub struct EdgeProps {
    id: EdgeIndex,
}

#[component]
pub fn Edge(props: EdgeProps) -> Element {
    let graph = use_app_state()().graph;
    let (from_node_id, to_node_id) = graph.edge_endpoints(props.id).unwrap(); //TODO!
    let from_node = &graph[from_node_id];
    let to_node = &graph[to_node_id];

    let edge = &graph[props.id];

    //get positions in array
    let fix = from_node.outputs.iter().position(|o| o.id == edge.source_port).unwrap();
    let tix = to_node.inputs.iter().position(|i| i.id == edge.target_port).unwrap();

    const HEADER_OFFSET: f32 = 24.0 + 4.0 + 4.0; //padding + height
    const ITEM_HEIGHT: f32 = 24.0;
    const NODE_WIDTH: f32 = 190.0;

    let y_source = HEADER_OFFSET + (fix as f32 * ITEM_HEIGHT) + 12.0 + from_node.position.y;
    let x_source = NODE_WIDTH + from_node.position.x;

    let y_target = HEADER_OFFSET + (tix as f32 * ITEM_HEIGHT) + 12.0 + to_node.position.y + (to_node.outputs.len() as f32 * ITEM_HEIGHT);
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

    rsx! {
        div {
            class: "absolute w-0 h-0 z-[-1]",
            left: 0,
            top: 0,
            svg {
                class: "overflow-visible w-0 h-0",
                path {
                    d: "{path_data}",
                    stroke: "green",
                    fill: "transparent",
                    stroke_width: "2",
                }
            }
        }
    }
}
