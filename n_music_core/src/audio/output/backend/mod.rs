//! The platform side of the audio output: listing the devices to play on, and opening a stream
//! on one that renders through the [`Renderer`] the shared side makes once the stream's format is
//! known.
//!
//! Each target compiles in one backend, picked below: AAudio on Android, cpal everywhere else. A
//! platform with a backend of its own gets a module and a `cfg` line here.

#[cfg(target_os = "android")]
mod aaudio;
#[cfg(not(target_os = "android"))]
mod cpal;

#[cfg(not(target_os = "android"))]
pub(super) use self::cpal::Cpal as Platform;
#[cfg(target_os = "android")]
pub(super) use aaudio::AAudio as Platform;
#[cfg(target_os = "android")]
pub use aaudio::{set_devices, AndroidOutput};

use super::callback::{Renderer, SharedState};
use super::OutputFormat;
use crate::settings::OutputDevice;
use std::fmt;
use std::sync::Arc;

/// What [`super::Output`] asks of a platform.
pub(super) trait Backend {
    type Stream: Stream;

    /// The devices there are to play on.
    fn devices() -> Vec<OutputDevice>;

    /// Opens a stream as `request` asks, paused. `renderer` makes what the stream's callback
    /// renders with once the stream's [`Shape`] is known; it is called once, for the stream
    /// returned.
    fn open(
        request: &Request,
        renderer: impl FnOnce(&Shape) -> Renderer,
    ) -> Result<Opened<Self::Stream>, BackendError>;
}

/// An open stream. Dropping it closes it.
pub(super) trait Stream: Send {
    fn play(&mut self) -> Result<(), BackendError>;

    fn pause(&mut self) -> Result<(), BackendError>;

    /// Drops what the device was handed and has not played. Only a [`Shape::deep`] stream needs
    /// it: the others play out the little they hold.
    fn flush(&mut self) -> Result<(), BackendError> {
        Ok(())
    }

    /// The device to play on is not the one the stream opened on any more: the stream should be
    /// opened again.
    fn moved(&self) -> bool {
        false
    }
}

/// What to open.
pub(super) struct Request<'a> {
    /// The device asked for, an [`OutputDevice::id`]; the default plays while it is not there.
    pub(super) device: Option<&'a str>,
    /// Offloaded playback may be used where the platform and the device have it.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(super) offload: bool,
    /// Where the stream's callbacks report glitches and failures.
    pub(super) state: Arc<SharedState>,
}

/// What an open stream plays and how it holds audio.
#[derive(Clone, Copy, Debug)]
pub(super) struct Shape {
    pub(super) format: OutputFormat,
    /// The device takes seconds of audio at a time, as offloaded playback does. What it was
    /// handed is committed: pausing and seeking act on the device, and a fade applied in the
    /// stream callback is heard that much later.
    pub(super) deep: bool,
    /// The most frames the device holds or asks for at once, as far as the backend can tell.
    pub(super) device_frames: usize,
}

pub(super) struct Opened<S> {
    pub(super) stream: S,
    pub(super) shape: Shape,
}

/// Why a backend could not do what it was asked.
#[derive(Debug)]
pub struct BackendError {
    kind: ErrorKind,
    message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// The device or the sound server is not there, for now.
    Unavailable,
    /// Another client holds the device.
    Busy,
    /// The stream stopped working, as when its device went away.
    Invalidated,
    /// The platform's own error, which may pass.
    Backend,
    /// The device can't play what was asked.
    Unsupported,
    /// Anything else, which opening again won't fix.
    Other,
}

impl BackendError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Opening the stream again may work.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self.kind,
            ErrorKind::Unavailable | ErrorKind::Busy | ErrorKind::Invalidated | ErrorKind::Backend
        )
    }
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for BackendError {}
