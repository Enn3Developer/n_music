use crate::ui::components::button::Button;
use crate::ui::components::topbar::TopBar;
use crate::ui::Route;
use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use std::ops::Deref;
use web_sys::window;

#[component]
pub fn SettingData(text: String, children: Element) -> Element {
    rsx! {
        div {
            class: "flex w-full flex-row",

            p {
                class: "flex-1 pl-2",

                {text}
            }

            div {
                class: "flex-none pr-2 gap-1",

                {children}
            }
        }
    }
}

#[component]
pub fn ThemeController() -> Element {
    let theme = use_signal(|| "default");

    rsx! {
        div {
            class: "dropdown mb-72",

            div {
                tabindex: 0,
                role: "button",
                class: "btn m-1",

                "Theme"
                svg {
                    width: "12px",
                    height: "12px",
                    class: "inline-block h-2 w-2 fill-current opacity-60",
                    xmlns: "http://www.w3.org/2000/svg",
                    view_box: "0 0 2048 2048",

                    path {
                        d: "M1799 349l242 241-1017 1017L7 590l242-241 775 775 775-775z"
                    }
                }
            }

            ul {
                tabindex: 0,
                class: "dropdown-content bg-base-300 rounded-box z-1 w-26 p-2 shadow-2xl",

                li {
                    input {
                        r#type: "radio",
                        name: "theme-dropdown",
                        class: "theme-controller w-full btn btn-sm btn-block btn-ghost justify-start",
                        aria_label: "Default",
                        value: "default",
                        onselect: move |_| {
                        }
                    }

                    input {
                        r#type: "radio",
                        name: "theme-dropdown",
                        class: "theme-controller w-full btn btn-sm btn-block btn-ghost justify-start",
                        aria_label: "Retro",
                        value: "retro",
                    }

                    input {
                        r#type: "radio",
                        name: "theme-dropdown",
                        class: "theme-controller w-full btn btn-sm btn-block btn-ghost justify-start",
                        aria_label: "Cyberpunk",
                        value: "cyberpunk",
                    }

                    input {
                        r#type: "radio",
                        name: "theme-dropdown",
                        class: "theme-controller w-full btn btn-sm btn-block btn-ghost justify-start",
                        aria_label: "Light",
                        value: "light",
                    }
                }
            }
        }
    }
}

#[component]
pub fn Settings() -> Element {
    rsx! {
        div {
            class: "flex w-full flex-col min-h-screen",

            TopBar {
                class: "sticky top-0 z-1 gap-1",

                // empty element to align back arrow to the right
                div {
                    class: "flex-1 text-3xl",
                    "Settings"
                }
                Button {
                    class: "btn-soft flex-none",
                    Link {
                        to: Route::Home {},
                        MaterialIcon { name: "arrow_back" }
                    }
                }
            }

            div {
                SettingData {
                    text: "Theme",

                    ThemeController {}
                }
            }
        }
    }
}
