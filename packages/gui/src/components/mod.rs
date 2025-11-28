mod code;
pub mod files;
pub mod graph;
pub mod layout;
mod toast;

pub use code::*;
pub use toast::*;

use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::go_icons::GoRocket};

pub const ICON_SIZE: Option<u32> = Some(14);

#[component]
pub fn NoProject() -> Element {
    rsx! {
        div { class: "flex flex-col items-center mt-10 gap-4 text-lg text-center text-zinc-400",
            Icon { width: Some(64), height: Some(64), icon: GoRocket }
            div { "Start by loading up a project" }
        }
    }
}
