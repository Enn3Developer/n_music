//! Follows the device the stream should be on, where streams stay on the device they opened.

use cpal::traits::DeviceTrait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Watches for the device to play on to change: another default, or the chosen device leaving
/// or coming back. The stream is then reopened on it.
pub(super) struct DeviceMonitor {
    changed: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
    thread: std::thread::Thread,
}

impl DeviceMonitor {
    /// Watches a stream on `current`, `chosen` being the device asked for.
    pub(super) fn new(
        current: cpal::DeviceId,
        chosen: Option<cpal::DeviceId>,
    ) -> super::Result<Self> {
        use super::DEVICE_CHECK_INTERVAL;

        let changed = Arc::new(AtomicBool::new(false));
        let stopped = Arc::new(AtomicBool::new(false));
        let changed_thread = changed.clone();
        let stopped_thread = stopped.clone();
        let worker = std::thread::Builder::new()
            .name("audio-device-monitor".into())
            .spawn(move || {
                let Ok(host) = cpal::host_from_id(current.host()) else {
                    changed_thread.store(true, Ordering::Relaxed);
                    return;
                };
                loop {
                    std::thread::park_timeout(DEVICE_CHECK_INTERVAL);
                    if stopped_thread.load(Ordering::Relaxed) {
                        break;
                    }
                    // PulseAudio enumeration waits for the server. Never do it on the
                    // playback worker, which must remain able to handle pause/seek/stop.
                    let changed = match super::device(&host, chosen.as_ref()) {
                        Some(device) => device.id().is_ok_and(|id| id != current),
                        None => true,
                    };
                    if changed {
                        changed_thread.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            })
            .map_err(|error| {
                cpal::Error::with_message(cpal::ErrorKind::ResourceExhausted, error.to_string())
            })?;
        Ok(Self {
            changed,
            stopped,
            thread: worker.thread().clone(),
        })
    }

    pub(super) fn changed(&self) -> bool {
        self.changed.load(Ordering::Relaxed)
    }
}

impl Drop for DeviceMonitor {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        self.thread.unpark();
        // Don't join a monitor that may still be waiting for an unresponsive server.
    }
}
