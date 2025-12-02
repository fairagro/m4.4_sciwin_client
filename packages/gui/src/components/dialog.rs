use std::path::{Path, PathBuf};

use dioxus::prelude::*;
use dioxus_primitives::alert_dialog::*;
use s4n_core::{io::get_workflows_folder, workflow::create_workflow};

use crate::layout::Route;

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

#[component]
pub fn NewDialog(title: String, children: Element, open: Signal<bool>, on_confirm: Option<EventHandler<MouseEvent>>) -> Element {
    rsx! {
        AlertDialogRoot {
            class: "absolute h-screen w-screen left-0 top-0 overflow-hidden bg-zinc-500/60 z-900",
            open: open(),
            on_open_change: move |v| open.set(v),
            AlertDialogContent { class: "select-none absolute justify-center bg-white top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 rounded-sm min-w-64 shadow-xl border-1 border-fairagro-dark-500",
                AlertDialogTitle { class: "py-1 px-4 bg-fairagro-mid-500 rounded-t-sm font-bold center border-b-1 border-fairagro-dark-500",
                    "{title}"
                }
                AlertDialogDescription { class: "py-2 px-4", {children} }
                AlertDialogActions {
                    class: "flex justify-center py-2 gap-2",
                    AlertDialogAction {
                        class: "cursor-pointer border-1 border-fairagro-mid-500 rounded-sm px-4 py-1 hover:bg-fairagro-mid-500 hover:text-white",
                        on_click: on_confirm,
                        "Ok"
                    }
                    AlertDialogCancel { class: "cursor-pointer border-1 border-fairagro-red-light rounded-sm px-4 py-1 hover:bg-fairagro-red-light hover:text-white",
                        "Cancel"
                    }
                }
            }
        }
    }
}

#[component]
pub fn WorkflowAddDialog(
    open: Signal<bool>,
    working_dir: ReadSignal<PathBuf>,
    show_add_actions: Signal<bool>,
    reload_trigger: Signal<i32>,
) -> Element {
    let mut workflow_name = use_signal(|| "".to_string());

    rsx! {
        NewDialog {
            open,
            title: "Create new Workflow",
            on_confirm: move |_| {
                create_workflow_impl(working_dir(), workflow_name())?;

                workflow_name.set("".to_string());
                show_add_actions.set(false);
                reload_trigger+=1;
                open.set(false);

                Ok(())
            },
            div { class: "flex flex-col",
                label { class:"text-fairagro-dark-500 font-bold", "Enter Workflow Name" }
                input {
                    class: "mt-2 shadow appearance-none border rounded w-full py-2 px-3 text-zinc-700 leading-tight focus:outline-none focus:shadow-outline",
                    value: "{workflow_name}",
                    r#type: "text",
                    placeholder: "workflow name ",
                    oninput: move |e| workflow_name.set(e.value()),
                }
            }
        }
    }
}

fn create_workflow_impl(project_root: impl AsRef<Path>, name: String) -> anyhow::Result<()> {
    let path = project_root.as_ref().join(get_workflows_folder()).join(&name).join(format!("{name}.cwl"));
    create_workflow(&path, false)?;

    navigator().push(Route::WorkflowView {
        path: path.to_string_lossy().to_string(),
    });
    Ok(())
}
