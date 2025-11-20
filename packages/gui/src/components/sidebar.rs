use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SidebarProps {
    pub children: Element,
}

#[component]
pub fn Sidebar(props: SidebarProps) -> Element {
    rsx! {
        aside {
            class: "w-64 bg-zinc-200 border-r border-zinc-400 p-2 overflow-y-auto",
            h2 {
                class:"text-sm text-zinc-800 mb-2 text-center",
                "Project"
            }
            {props.children}
        }
    }
}
