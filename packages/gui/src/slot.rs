use commonwl::prelude::*;
use dioxus::prelude::*;

#[derive(Debug, Clone)]
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
            class: "{bg} w-3 h-3 m-2 {geometry} {margin} {border}"
        }
    }
}