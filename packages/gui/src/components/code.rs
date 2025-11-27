use dioxus::prelude::*;
use std::{fs, path::PathBuf};

#[component]
pub fn CodeViewer(path: String) -> Element {
    let mut path = use_reactive(&path, PathBuf::from);
    let mut editor_initialized = use_signal(|| false);
    let path_signal = use_signal(&mut path);

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

    rsx! {
         button {
            onclick: move |_| async move {
                let value = document::eval("return getMonacoValue();");
                let value = value.await?;
                let code: String = serde_json::from_value(value)?;
                fs::write(path_signal(), code)?;
                Ok(())
            },
            "Save"
        }
        div { id: "editor", class: "h-full p-4 w-full min-h-0" }
    }
}
