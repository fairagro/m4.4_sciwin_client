use dioxus::prelude::*;
use std::fs;

#[component]
pub fn CodeViewer(path: String) -> Element {
    let mut code = use_signal(|| "No CWL code loaded.".to_string());

    // Get the CWL code from state
    let contents = fs::read_to_string(path);
    if let Ok(contents) = contents {
        code.set(contents);
    }

    let value = code();
    rsx! {
        div {
            onmounted: move |_| {
            let value = value.clone();
            async move{
                document::eval(include_str!("../../assets/bundle.min.js")).await.unwrap();
                let escaped_value = value
                    .replace('\\', "\\\\")
                    .replace('`', "\\`")
                    .replace("${", "\\${");

                document::eval(&format!("initMonaco(`{}`);", escaped_value)).await.unwrap();
            }
            },
            id: "editor",
            width: "100%",
            height: "100%",
            class: "relative p-4",
        }
    }
}
