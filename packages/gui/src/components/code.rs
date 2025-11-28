use crate::components::{ICON_SIZE, ToastItem};
use dioxus::{
    desktop::{HotKeyState, use_global_shortcut},
    prelude::*,
};
use dioxus_free_icons::{Icon, icons::go_icons::GoCheck};
use std::{fs, path::PathBuf};

#[component]
pub fn CodeViewer(path: String) -> Element {
    let mut path = use_reactive(&path, PathBuf::from);
    let mut editor_initialized = use_signal(|| false);
    let path_signal = use_signal(&mut path);

    let mut toast_items = use_context::<Signal<Vec<ToastItem>>>();

    {
        use_effect(move || {
            let contents = fs::read_to_string(path());
            let code = if let Ok(contents) = contents { contents } else { "".to_string() };

            spawn(async move {
                let escaped_value = code.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${");
                if !editor_initialized() {
                    document::eval(include_str!("../../assets/bundle.min.js")).await.unwrap();
                    editor_initialized.set(true);

                    document::eval(&format!("initMonaco(`{}`);", escaped_value)).await.unwrap();
                } else {
                    document::eval(&format!("updateMonaco(`{}`);", escaped_value)).await.unwrap();
                }
            });
        });
    }

    let save_code_file = move || async move {
        let value = document::eval("return getMonacoValue();");
        let value = value.await?;
        let code: String = serde_json::from_value(value)?;
        fs::write(path_signal(), code)?;

        toast_items.write().push(ToastItem::new(
            "Changes saved!".to_string(),
            format!("Saved changes for: {:?}", path_signal()),
            5,
        ));

        Ok(())
    };

    use_global_shortcut("Ctrl+S", move |s| {
        if s == HotKeyState::Pressed {
            spawn(async move {
                save_code_file().await.unwrap();
            });
        }
    })
    .unwrap();

    rsx! {
        div { class: "flex justify-end w-full py-1 px-3",
            button { title: "Save", onclick: move |_| save_code_file(),
                Icon { icon: GoCheck, width: ICON_SIZE, height: ICON_SIZE }
            }
        }
        div {
            id: "editor",
            class: "relative overflow-scroll w-full h-full min-h-0",
        }
    }
}
