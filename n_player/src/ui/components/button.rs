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

                button {
                    class: "btn {class}",

                    {children}
                }
            }
        }
        else {
            button {
                class: "btn {class}",

                {children}
            }
        }
    }
}
