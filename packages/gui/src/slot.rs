use crate::{DragState, use_app_state};
use commonwl::prelude::*;
use dioxus::prelude::*;
use petgraph::graph::NodeIndex;

#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    pub id: String,
    pub type_: CWLType,
}

#[derive(Clone, PartialEq)]
pub enum SlotType {
    Input,
    Output,
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct SlotProps {
    node_id: NodeIndex,
    slot: Slot,
    slot_type: SlotType,
}

#[component]
pub fn SlotElement(props: SlotProps) -> Element {
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

    rsx! {
        div {
            onmousedown: move |_| {
                use_app_state().write().dragging = Some(DragState::Connection { source_node: props.node_id, source_port: props.slot.id.clone() });
            },
            class: "{bg} w-3 h-3 m-2 {geometry} {margin} {border} z-2"
        }
    }
}
