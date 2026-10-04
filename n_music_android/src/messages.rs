//! Messages of this interface only.

use crate::settings::WindowSize;
use n_event_bus::Message;

macro_rules! messages {
    ($($msg:ty),+ $(,)?) => {
        $( impl Message for $msg {} )+
    };
}

pub struct OpenLink(pub String);
pub struct SearchChanged(pub String);
pub struct WindowSizeCaptured(pub WindowSize);
pub struct ThemeChangeRequested(pub i32);
pub struct ToggleSaveWindowSize(pub bool);
pub struct LocaleChangeRequested(pub String);
pub struct PathChangeRequested;

messages!(
    OpenLink,
    SearchChanged,
    WindowSizeCaptured,
    ThemeChangeRequested,
    ToggleSaveWindowSize,
    LocaleChangeRequested,
    PathChangeRequested,
);
