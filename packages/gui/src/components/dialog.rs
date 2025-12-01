use dioxus::prelude::*;

#[derive(Clone, Debug)]
pub struct Dialog {
    title: String,
    message: String,
    result: Option<MessageResult>,
}

impl Dialog {
    pub fn new(title: &str, message: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            result: None,
        }
    }
}

pub fn close_dialog(ctx: Signal<Option<Dialog>>) -> Option<MessageResult> {
    let mut item = ctx;
    if let Some(dialog) = item()
        && let Some(result) = dialog.result
    {
        item.set(None);
        return Some(result);
    }
    None
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum MessageResult {
    Ok,
    #[default]
    Cancel,
}

#[component]
pub fn DialogProvider() -> Element {
    let mut item = use_context::<Signal<Option<Dialog>>>();

    rsx! {
        if let Some(dialog_data) = item.cloned() {
            div { class: "absolute h-screen w-screen left-0 top-0 overflow-hidden bg-zinc-500/60 z-50",
                div {
                    role: "dialog",
                    class: "select-none absolute justify-center bg-white top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 rounded-sm min-w-64 shadow-xl border-1 border-fairagro-dark-500",
                    h2 { class: "py-1 px-4 bg-fairagro-mid-500 rounded-t-sm font-bold center border-b-1 border-fairagro-dark-500",
                        "{dialog_data.title}"
                    }
                    p { class: "py-2 px-4", "{dialog_data.message}" }
                    div { class: "flex justify-center py-2 gap-2",
                        button {
                            class: "cursor-pointer border-1 border-fairagro-mid-500 rounded-sm px-4 py-1 hover:bg-fairagro-mid-500 hover:text-white",
                            onclick: move |_| {
                                if let Some(item) = item.write().as_mut() {
                                    item.result = Some(MessageResult::Ok);
                                }
                            },
                            "Ok"
                        }
                        button {
                            class: "cursor-pointer border-1 border-fairagro-red-light rounded-sm px-4 py-1 hover:bg-fairagro-red-light hover:text-white",
                            onclick: move |_| {
                                if let Some(item) = item.write().as_mut() {
                                    item.result = Some(MessageResult::Cancel);
                                }
                            },
                            "Cancel"
                        }
                    }
                }
            }
        }
    }
}
