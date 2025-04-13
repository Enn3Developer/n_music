use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum TooltipPosition {
    Top,
    Bottom,
    Left,
    Right,
}

impl From<TooltipPosition> for &str {
    fn from(value: TooltipPosition) -> Self {
        match value {
            TooltipPosition::Top => "tooltip-top",
            TooltipPosition::Bottom => "tooltip-bottom",
            TooltipPosition::Left => "tooltip-left",
            TooltipPosition::Right => "tooltip-right",
        }
    }
}

#[component]
pub fn Button(
    #[props(default)] class: String,
    tooltip_position: Option<TooltipPosition>,
    tooltip: Option<String>,
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let tooltip_class = format!(
        "tooltip {}",
        match tooltip_position {
            None => "",
            Some(tooltip_position) => tooltip_position.into(),
        }
    );

    rsx! {
        if tooltip.is_some() {
            div {
                class: tooltip_class,
                "data-tip": tooltip.unwrap(),

                BaseButton {
                    class,
                    children,
                    onclick
                }
            }
        }
        else {
            BaseButton {
                class,
                children,
                onclick
            }
        }
    }
}

#[component]
pub fn BaseButton(
    #[props(default)] class: String,
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    rsx! {
        if let Some(onclick) = onclick {
            button {
                class: "btn {class}",
                onclick,

                {children}
            }
        } else {
            button {
                class: "btn {class}",

                {children}
            }
        }
    }
}
