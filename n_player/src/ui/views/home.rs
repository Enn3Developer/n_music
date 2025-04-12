use crate::ui::components::search::SearchBar;
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    rsx! {
        SearchBar {}
    }
}
