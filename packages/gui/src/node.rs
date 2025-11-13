use crate::slot::{Slot, SlotElement, SlotType};
use crate::use_app_state;
use commonwl::prelude::*;
use dioxus::html::geometry::euclid::Point2D;
use dioxus::prelude::*;
use petgraph::graph::NodeIndex;

#[derive(Debug, Clone)]
pub struct VisualNode {
    pub instance: NodeInstance,
    pub position: Point2D<f32, f32>,
    pub inputs: Vec<Slot>,
    pub outputs: Vec<Slot>,
}

#[derive(Debug, Clone)]
pub enum NodeInstance {
    Step(CWLDocument),
    Input(CommandInputParameter), //WorkflowInputParameter
    Output(WorkflowOutputParameter),
}

impl NodeInstance {
    pub fn id(&self) -> String {
        match &self {
            Self::Step(doc) => doc.id.clone().unwrap().clone(),
            Self::Input(input) => input.id.clone(),
            Self::Output(output) => output.id.clone(),
        }
    }
}

#[derive(Props, Clone, Copy, PartialEq)]
pub struct NodeProps {
    id: NodeIndex,
}

#[component]
pub fn NodeElement(props: NodeProps) -> Element {
    let graph = use_app_state()().workflow.graph;
    let node = &graph[props.id];
    let pos_x = node.position.x;
    let pos_y = node.position.y;

    let top_color = match node.instance {
        NodeInstance::Step(_) => "bg-green-900",
        NodeInstance::Input(_) => "bg-blue-900",
        NodeInstance::Output(_) => "bg-red-900",
    };

    let mut drag_offset = use_app_state().write().drag_offset;

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
