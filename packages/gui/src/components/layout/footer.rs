use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct FooterProps {
    pub children: Element,
}

#[component]
pub fn Footer(props: FooterProps) -> Element {
    rsx! {
        footer { class: "select-none bg-lime-800 h-6 flex items-center border-t border-zinc-400 text-xs p-2 z-10",
            {props.children}
        }
    }
}
