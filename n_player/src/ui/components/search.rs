use dioxus::prelude::*;

#[component]
pub fn SearchBar(#[props(default)] class: String) -> Element {
    rsx! {
        label {
            class: "input md:w-auto {class}",

            svg {
                class: "h-[1em] opacity-50",
                xmlns: "http://www.w3.org/2000/svg",
                view_box: "0 0 24 24",

                g {
                    stroke_linejoin: "round",
                    stroke_linecap: "round",
                    stroke_width: 2.5,
                    fill: "none",
                    stroke: "currentColor",

                    circle {
                        cx: 11,
                        cy: 11,
                        r: 8,
                    }

                    path {
                        d: "m21 21-4.3-4.3"
                    }
                }
            }

            input {
                r#type: "search",
                class: "grow",
                placeholder: "Search..."
            }

            kbd {
                class: "kbd kbd-sm",
                "CTRL"
            }

            kbd {
                class: "kbd kbd-sm",
                "F"
            }
        }
    }
}
