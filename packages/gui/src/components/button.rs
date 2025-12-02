use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ButtonProps {
    pub title: Option<String>,
    pub children: Element,
    pub onclick: Option<EventHandler<MouseEvent>>,
}

#[component]
pub fn RoundActionButton(props: ButtonProps) -> Element {
    let title = props.title.unwrap_or_default();
    rsx! {
        button {
            onclick: move |e| {
                if let Some(handler) = props.onclick {
                    handler.call(e);
                }
            },
            class: "rounded-full justify-center items-center p-5 bg-fairagro-mid-500 right-30 select-none hover:bg-fairagro-dark-500 hover:text-white mr-3",
            title,
            {props.children}
        }
    }
}
