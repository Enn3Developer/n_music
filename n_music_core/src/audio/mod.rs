//! Decoding and playback: probing, the extra formats and decoders, conversion to the device
//! format, and the device output.

mod convert;
mod dca;
mod opus;
mod output;
pub(crate) mod player;
mod raw;

pub(crate) use output::output_devices;

use crate::library::track::ReplayGain;
use crate::source::{Locator, OpenedStream, StreamProvider};
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
    probe(provider.open(locator)?, locator)
}

/// Probes the format of `stream`, opened from `locator`.
pub(crate) fn probe(stream: OpenedStream, locator: &Locator) -> io::Result<Box<dyn FormatReader>> {
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

/// Visits the media-level and the track's own metadata of every revision, newest first.
/// Tags read later are usually the better ones: an MP3's ID3v1 tag, a truncated Latin-1 relic,
/// lands in an older revision than its ID3v2 tag.
pub(crate) fn for_each_container(
    format: &mut dyn FormatReader,
    track_id: u32,
    mut visit: impl FnMut(&MetadataContainer),
) {
    let mut metadata_log = format.metadata();
    let mut older = Vec::new();
    while let Some(revision) = metadata_log.pop() {
        older.push(revision);
    }
    let newest = metadata_log.current();
    for metadata in newest.into_iter().chain(older.iter().rev()) {
        visit(&metadata.media);
        metadata
            .per_track
            .iter()
            .filter(|metadata| metadata.track_id == u64::from(track_id))
            .for_each(|metadata| visit(&metadata.metadata));
    }
}
