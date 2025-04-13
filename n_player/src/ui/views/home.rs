use crate::ui::components::bottom_bar::BottomBar;
use crate::ui::components::button::{Button, TooltipPosition};
use crate::ui::components::search::SearchBar;
use crate::ui::components::topbar::TopBar;
use crate::ui::components::tracks::Tracks;
use crate::ui::Route;
use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn Home() -> Element {
    rsx! {
        div {
            class: "flex w-full flex-col min-h-screen",

            TopBar {
                class: "sticky top-0 z-1 gap-1",
                SearchBar {
                    class: "flex-1"
                }
                Button {
                    class: "btn-soft flex-none",
                    MaterialIcon { name: "arrow_downward" }
                }
                Button {
                    class: "btn-soft flex-none",
                    Link {
                        to: Route::Settings {},
                        MaterialIcon { name: "settings" }
                    }
                }
            }

            Tracks {
                class: "grow"
            }

            BottomBar {
                class: "fixed bottom-0 bg-base-100 z-2 flex flex-col",

                "Test"
            }
        }
    }
}
