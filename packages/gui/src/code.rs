use std::fs;

use crate::use_app_state;
use dioxus::prelude::*;
use syntect::{highlighting::ThemeSet, html::highlighted_html_for_string, parsing::SyntaxSet};

#[component]
pub fn CodeViewer() -> Element {
    let app_state = use_app_state();

    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_extension("yaml").unwrap();
    let theme = &ts.themes["InspiredGitHub"];
    // Get the CWL code from state
    let code = match fs::read_to_string(app_state().workflow.path) {
        Ok(c) => c.clone(),
        Err(_) => "No CWL code loaded.".to_string(),
    };

    let html_code = highlighted_html_for_string(&code, &ps, syntax, theme)?;

    rsx! {
        div {
            pre {
                class: "whitespace-pre-wrap overflow-x-auto bg-gray-800 p-4 rounded-lg",
                dangerous_inner_html: "{html_code}"
            }
        }
    }
}
