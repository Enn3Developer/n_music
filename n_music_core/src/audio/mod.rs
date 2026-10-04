//! Decoding and playback: probing, the extra formats and decoders, conversion to the device
//! format, and the device output.

mod convert;
mod dca;
mod opus;
mod output;
pub(crate) mod player;
mod raw;

use crate::library::track::ReplayGain;
use crate::source::{Locator, StreamProvider};
use dca::DcaReader;
use once_cell::sync::Lazy;
use opus::OpusDecoder;
use raw::RawReader;
use std::io;
use symphonia::core::codecs::registry::CodecRegistry;
use symphonia::core::formats::probe::Probe;
use symphonia::core::formats::{FormatOptions, FormatReader, Track};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::default::{register_enabled_codecs, register_enabled_formats};
use symphonia_core::meta::MetadataContainer;

/// Default Symphonia [`CodecRegistry`], including the (audiopus-backed) Opus codec.
pub(crate) static CODEC_REGISTRY: Lazy<CodecRegistry> = Lazy::new(|| {
    let mut registry = CodecRegistry::new();
    register_enabled_codecs(&mut registry);
    registry.register_audio_decoder::<OpusDecoder>();
    registry
});

static PROBE: Lazy<Probe> = Lazy::new(|| {
    let mut probe = Probe::default();
    probe.register_format::<DcaReader>();
    probe.register_format::<RawReader>();
    register_enabled_formats(&mut probe);
    probe
});

/// Opens `locator` and probes its format, positioned at the first byte.
pub(crate) fn open(
    provider: &dyn StreamProvider,
    locator: &Locator,
) -> io::Result<Box<dyn FormatReader>> {
    let stream = provider.open(locator)?;
    let hint = stream.hint();
    let media_stream = MediaSourceStream::new(stream.source, Default::default());
    PROBE
        .probe(
            &hint,
            media_stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|error| io::Error::other(format!("Could not probe {locator}: {error}")))
}

/// The track's length in seconds, when the container tells.
pub(crate) fn length(track: &Track) -> Option<f64> {
    track
        .time_base
        .zip(track.duration)
        .and_then(|(base, duration)| base.calc_duration(duration))
        .map(|time| time.as_secs_f64())
}

/// The ReplayGain tags of an opened track. Consumes the reader's pending metadata revisions.
pub(crate) fn replay_gain(format: &mut dyn FormatReader, track_id: u32) -> ReplayGain {
    let mut replay_gain = ReplayGain::default();
    for_each_container(format, track_id, |container| {
        for tag in &container.tags {
            if let Some(tag) = &tag.std {
                replay_gain.read(tag);
            }
        }
    });
    replay_gain
}

/// Visits the media-level and the track's own metadata of every revision, oldest first.
pub(crate) fn for_each_container(
    format: &mut dyn FormatReader,
    track_id: u32,
    mut visit: impl FnMut(&MetadataContainer),
) {
    let mut metadata_log = format.metadata();
    while let Some(metadata) = metadata_log.current() {
        visit(&metadata.media);
        metadata
            .per_track
            .iter()
            .filter(|metadata| metadata.track_id == u64::from(track_id))
            .for_each(|metadata| visit(&metadata.metadata));
        if metadata_log.pop().is_none() {
            break;
        }
    }
}
