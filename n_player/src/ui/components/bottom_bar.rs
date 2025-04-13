use dioxus::prelude::*;

#[component]
pub fn BottomBar(#[props(default)] class: String, children: Element) -> Element {
    rsx! {
        footer {
            class: "footer footer-center {class}",

            {children}
        }
    }
}
