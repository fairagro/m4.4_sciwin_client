use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ButtonProps {
    pub title: Option<String>,
    pub class: Option<String>,
    pub children: Element,
    pub onclick: Option<EventHandler<MouseEvent>>,
}

#[component]
pub fn RoundActionButton(props: ButtonProps) -> Element {
    let title = props.title.unwrap_or_default();
    let class = props.class.unwrap_or_default();
    rsx! {
        button {
            onclick: move |e| {
                if let Some(handler) = props.onclick {
                    handler.call(e);
                }
            },
            class: "cursor-pointer rounded-full justify-center items-center p-3 bg-fairagro-mid-500 select-none hover:bg-fairagro-dark-500 hover:text-white hover:rotate-45 transition-[rotate] duration-500 {class}",
            title,
            {props.children}
        }
    }
}

#[component]
pub fn SmallRoundActionButton(props: ButtonProps) -> Element {
    let title = props.title.unwrap_or_default();
    let class = props.class.unwrap_or_default();
    rsx! {
        button {
            class: "cursor-pointer p-1 rounded-full hover:rotate-20 transition-[rotate] duration-200 {class}",
            onclick: move |e| {
                if let Some(handler) = props.onclick {
                    handler.call(e);
                }
            },
            title,
            {props.children}
        }
    }
}
