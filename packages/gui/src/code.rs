use crate::use_app_state;
use dioxus::prelude::*;
use std::fs;

#[component]
pub fn CodeViewer() -> Element {
    let app_state = use_app_state();

    // Get the CWL code from state
    let code = match fs::read_to_string(app_state().workflow.path.unwrap()) {
        Ok(c) => c.clone(),
        Err(_) => "No CWL code loaded.".to_string(),
    };

    //let html_code = highlighted_html_for_string(&code, &ps, syntax, theme)?;
    let value = code.clone();
    rsx! {
        div {
            onmounted: move |_| {
            let value = value.clone();
            async move{
                document::eval(include_str!("../assets/bundle.min.js")).await.unwrap();
                let escaped_value = serde_json::to_string(&value).unwrap();
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
