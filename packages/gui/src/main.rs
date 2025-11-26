use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;
use gui::ApplicationState;
use gui::layout::Route;

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(
            Config::default()
                .with_menu(None)
                .with_window(
                    WindowBuilder::new()
                        .with_inner_size(LogicalSize::new(1270, 720))
                        .with_title("SciWIn Studio"),
                )
                .with_icon(Icon::from_rgba(include_bytes!("../assets/icon.rgba").to_vec(), 192, 192).unwrap()),
        )
        .launch(App);
}

#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(ApplicationState::default()));

    rsx!(
        document::Link { rel: "icon", href: asset!("/assets/icon.png") }
        Stylesheet{ href: asset!("/assets/main.css") }
        Stylesheet{ href: asset!("/assets/bundle.min.css") }
        Stylesheet{ href: asset!("/assets/tailwind.css") }

        div {
            class: "h-screen w-full flex flex-col",
            Router::<Route> {}
        }
    )
}
