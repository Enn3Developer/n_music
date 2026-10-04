//! Follows the system default output on hosts whose streams stay on a concrete device.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Watches for another default output device; the stream is then reopened on it.
pub(super) struct DefaultDeviceMonitor {
    changed: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
    thread: std::thread::Thread,
}

impl DefaultDeviceMonitor {
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd"
    ))]
    pub(super) fn new(device_id: cpal::DeviceId) -> super::Result<Self> {
        use super::DEVICE_CHECK_INTERVAL;
        use cpal::traits::{DeviceTrait, HostTrait};

        let changed = Arc::new(AtomicBool::new(false));
        let stopped = Arc::new(AtomicBool::new(false));
        let changed_thread = changed.clone();
        let stopped_thread = stopped.clone();
        let worker = std::thread::Builder::new()
            .name("audio-device-monitor".into())
            .spawn(move || {
                let Ok(host) = cpal::host_from_id(device_id.host()) else {
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
                    let changed = match host.default_output_device() {
                        Some(device) => device.id().is_ok_and(|id| id != device_id),
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

impl Drop for DefaultDeviceMonitor {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        self.thread.unpark();
        // Don't join a monitor that may still be waiting for an unresponsive server.
    }
}
