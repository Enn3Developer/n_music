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
    rsx! {
        document::Link { rel: "icon", href: ICON }
        document::Link { rel: "stylesheet", href: STYLE }
        MaterialIconStylesheet {}

        Router::<Route> {}
    }
}
