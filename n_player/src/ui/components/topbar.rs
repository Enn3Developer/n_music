use dioxus::prelude::*;

#[component]
pub fn TopBar(#[props(default)] class: String, children: Element) -> Element {
    rsx! {
        div {
            class: "navbar bg-base-100 shadow-sm {class}",

            {children}
        }
    }
}
