use dioxus::prelude::*;

#[component]
pub fn Tracks(#[props(default)] class: String) -> Element {
    rsx! {
        ul {
            class: "list bg-base-100 shadow-md rounded-box {class}",

            li {
                class: "list-row",
                "Test 1"
            }

            li {
                class: "list-row",
                "Test 2"
            }

            li {
                class: "list-row",
                "Test 3"
            }

            li {
                class: "list-row",
                "Test 4"
            }
        }
    }
}
