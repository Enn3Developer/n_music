use dioxus::prelude::*;
use views::{Home, Settings};

pub mod components;
pub mod views;

const ICON: Asset = asset!("/assets/icons/icon.ico");

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

        Router::<Route> {}
    }
}
