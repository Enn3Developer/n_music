//! The output through cpal, on every platform without a backend of its own.

mod device_monitor;

use super::{Backend, BackendError, ErrorKind, Opened, Request, Shape, Stream};
use crate::audio::output::callback::{OutputSample, Renderer};
use crate::audio::output::{OutputFormat, STALL_TIMEOUT};
use crate::settings::OutputDevice;
use ::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use device_monitor::DeviceMonitor;
use std::time::Duration as WallDuration;

/// The device's period: what pause and seek fades wait for, as queued audio is not.
const PERIOD: WallDuration = WallDuration::from_millis(10);

pub(in crate::audio::output) struct Cpal;

pub(in crate::audio::output) struct CpalStream {
    stream: ::cpal::Stream,
    monitor: Option<DeviceMonitor>,
}

impl From<::cpal::Error> for BackendError {
    fn from(error: ::cpal::Error) -> Self {
        use ::cpal::ErrorKind as Cpal;
        let kind = match error.kind() {
            Cpal::DeviceNotAvailable | Cpal::HostUnavailable => ErrorKind::Unavailable,
            Cpal::DeviceBusy => ErrorKind::Busy,
            Cpal::StreamInvalidated => ErrorKind::Invalidated,
            Cpal::BackendError => ErrorKind::Backend,
            Cpal::UnsupportedConfig | Cpal::UnsupportedOperation => ErrorKind::Unsupported,
            _ => ErrorKind::Other,
        };
        BackendError::new(kind, error.to_string())
    }
}

/// The device to play on: `chosen` while it is there, else the default.
fn device(host: &::cpal::Host, chosen: Option<&::cpal::DeviceId>) -> Option<::cpal::Device> {
    chosen
        .and_then(|id| host.device_by_id(id))
        .or_else(|| host.default_output_device())
}

impl Backend for Cpal {
    type Stream = CpalStream;

    fn devices() -> Vec<OutputDevice> {
        let devices = match ::cpal::default_host().output_devices() {
            Ok(devices) => devices,
            Err(error) => {
                log::warn!("Could not list the output devices: {error}");
                return vec![];
            }
        };
        devices
            .filter_map(|device| {
                let id = device.id().ok()?;
                let name = device
                    .description()
                    .map_or_else(|_| id.id().to_string(), |about| about.name().to_string());
                Some(OutputDevice {
                    id: id.to_string(),
                    name,
                })
            })
            .collect()
    }

    fn open(
        request: &Request,
        renderer: impl FnOnce(&Shape) -> Renderer,
    ) -> Result<Opened<CpalStream>, BackendError> {
        let host = ::cpal::default_host();
        let chosen = request.device.and_then(|id| {
            id.parse::<::cpal::DeviceId>()
                .inspect_err(|error| log::warn!("Ignoring the output device {id:?}: {error}"))
                .ok()
        });
        let device = self::device(&host, chosen.as_ref())
            .ok_or_else(|| BackendError::new(ErrorKind::Unavailable, "No output device"))?;
        if let Some(chosen) = &chosen {
            if device.id().ok().as_ref() != Some(chosen) {
                log::info!("Output device {chosen} is not there, playing on the default");
            }
        }
        let config = device.default_output_config()?;
        let watch = match host.id() {
            // CPAL's PulseAudio backend targets a concrete sink and doesn't watch defaults.
            // WASAPI/CoreAudio supply native notifications.
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd"
            ))]
            ::cpal::HostId::PulseAudio => true,
            // A chosen device is watched everywhere, to get back to it.
            _ => chosen.is_some(),
        };
        let monitor = if watch {
            Some(DeviceMonitor::new(device.id()?, chosen)?)
        } else {
            None
        };

        match config.sample_format() {
            ::cpal::SampleFormat::F32 => build::<f32>(&device, &config, monitor, request, renderer),
            ::cpal::SampleFormat::I32 => build::<i32>(&device, &config, monitor, request, renderer),
            ::cpal::SampleFormat::I16 => build::<i16>(&device, &config, monitor, request, renderer),
            ::cpal::SampleFormat::U16 => build::<u16>(&device, &config, monitor, request, renderer),
            format => Err(BackendError::new(
                ErrorKind::Unsupported,
                format!("Unsupported output sample format: {format}"),
            )),
        }
    }
}

fn build<T: OutputSample + ::cpal::SizedSample>(
    device: &::cpal::Device,
    config: &::cpal::SupportedStreamConfig,
    monitor: Option<DeviceMonitor>,
    request: &Request,
    renderer: impl FnOnce(&Shape) -> Renderer,
) -> Result<Opened<CpalStream>, BackendError> {
    let format = OutputFormat {
        rate: config.sample_rate(),
        channels: usize::from(config.channels()),
    };
    let period = ((format.rate as f64 * PERIOD.as_secs_f64()) as u32).max(1);
    let (buffer_size, device_frames) = match config.buffer_size() {
        ::cpal::SupportedBufferSize::Range { min, max } => {
            let frames = period.clamp(*min, *max);
            (::cpal::BufferSize::Fixed(frames), frames as usize)
        }
        ::cpal::SupportedBufferSize::Unknown => (::cpal::BufferSize::Default, 0),
    };
    let stream_config = ::cpal::StreamConfig {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        buffer_size,
    };
    let shape = Shape {
        format,
        deep: false,
        device_frames,
    };
    let mut renderer = renderer(&shape);
    let state = request.state.clone();
    let stream = device.build_output_stream(
        stream_config,
        move |data: &mut [T], info: &::cpal::OutputCallbackInfo| {
            let timestamp = info.timestamp();
            renderer.render(data, timestamp.playback.duration_since(timestamp.callback));
        },
        move |err| match err.kind() {
            // CPAL already rerouted the stream, or only failed to boost its priority. An xrun
            // is a glitch the device recovers from by itself.
            ::cpal::ErrorKind::DeviceChanged
            | ::cpal::ErrorKind::RealtimeDenied
            | ::cpal::ErrorKind::Xrun => state.glitch(),
            _ => state.fail(),
        },
        Some(STALL_TIMEOUT),
    )?;
    log::debug!(
        "Opened cpal output: {} Hz, {} channels, {:?}",
        format.rate,
        format.channels,
        config.sample_format()
    );
    Ok(Opened {
        stream: CpalStream { stream, monitor },
        shape,
    })
}

impl Stream for CpalStream {
    fn play(&mut self) -> Result<(), BackendError> {
        Ok(self.stream.play()?)
    }

    fn pause(&mut self) -> Result<(), BackendError> {
        Ok(self.stream.pause()?)
    }

    fn moved(&self) -> bool {
        self.monitor.as_ref().is_some_and(DeviceMonitor::changed)
    }
}
