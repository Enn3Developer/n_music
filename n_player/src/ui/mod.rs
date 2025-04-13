use crate::settings;
use dioxus::prelude::*;
use dioxus_material_icons::MaterialIconStylesheet;
use views::{Home, Settings};

pub mod components;
pub mod views;

const ICON: Asset = asset!("assets/icons/icon.ico");
const STYLE: Asset = asset!("assets/style/tailwind.css");

#[derive(Debug, Clone, Routable, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},

    #[route("/settings")]
    Settings {},
}

pub fn run() {
    launch(App);
}

#[component]
pub fn App() -> Element {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let platform = crate::platform::DesktopPlatform {};
    #[cfg(target_os = "linux")]
    let platform = crate::platform::LinuxPlatform::new();

    let settings = settings::Settings::read_saved(&platform);

    document::eval(&format!(
        "document.documentElement.setAttribute('data-theme', '{}');",
        String::from(settings.theme)
    ));

    use_context_provider(|| Signal::new(settings));

    document::eval(
        "window.onkeydown = function(evt) {\
                    if (evt.ctrlKey && (evt.key == 'f' || evt.key == 'F') && !evt.repeating) {\
                        document.getElementById('searchbar').focus();\
                    }\
               }",
    );

    rsx! {
        document::Link { rel: "icon", href: ICON }
        document::Link { rel: "stylesheet", href: STYLE }
        MaterialIconStylesheet {}

        Router::<Route> {}
    }
}
