use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SidebarProps {
    pub children: Element,
}

#[component]
pub fn Sidebar(props: SidebarProps) -> Element {
    rsx! {
        aside { class: "select-none w-64 bg-zinc-200 border-r border-zinc-400 p-2 overflow-y-auto",
            Logo {}
            div { class: "mt-4" }
            {props.children}
        }
    }
}

#[component]
pub fn Logo() -> Element {
    rsx! {
        div {
            img { src: asset!("/assets/logo.png"), width: 150 }
        }
    }
}
