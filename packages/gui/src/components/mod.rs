mod code;
mod fs_view;
pub mod graph;
pub mod layout;

pub use code::*;
pub use fs_view::*;

use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::go_icons::GoRocket};

#[component]
pub fn NoProject() -> Element {
    rsx! {
        div {
            class: "flex flex-col items-center mt-10 gap-4 text-lg text-center text-zinc-400",
            Icon { width: Some(64), height: Some(64), icon: GoRocket }
            div { "Start by loading up a project" }
        }
    }
}
