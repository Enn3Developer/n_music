use dioxus::prelude::*;

#[component]
pub fn SearchBar() -> Element {
    rsx! {
        input {
            r#type: "text"
        }
    }
}
