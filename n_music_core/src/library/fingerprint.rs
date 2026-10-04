//! Identifies a track's audio apart from its location and tags, so references to it survive a
//! move, a rename or a retag (not a re-encode).
//!
//! Only the start of the encoded audio is hashed, together with the exact duration: seeking to
//! a second sample depends on the byte layout for some formats, which a retag changes.

use symphonia::core::formats::FormatReader;

/// Encoded audio hashed from the start of the track.
const SAMPLE_BYTES: usize = 64 * 1024;

/// The fingerprint of `track_id`, read from the reader's current position (the start). `None`
/// when the track has no audio packets.
pub fn fingerprint(format: &mut dyn FormatReader, track_id: u32) -> Option<u64> {
    let mut hash = Fnv::default();
    let track = format.tracks().iter().find(|track| track.id == track_id)?;
    hash.write(
        &track
            .duration
            .map_or(0, |duration| duration.get())
            .to_le_bytes(),
    );
    if let Some(params) = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
    {
        hash.write(&params.sample_rate.unwrap_or(0).to_le_bytes());
        let channels = params
            .channels
            .as_ref()
            .map_or(0, |channels| channels.count());
        hash.write(&(channels as u32).to_le_bytes());
    }
    let mut taken = 0;
    while taken < SAMPLE_BYTES {
        let Ok(Some(packet)) = format.next_packet() else {
            break;
        };
        if packet.track_id != track_id {
            continue;
        }
        let data = &packet.data[..packet.data.len().min(SAMPLE_BYTES - taken)];
        hash.write(data);
        taken += data.len();
    }
    (taken > 0).then_some(hash.0)
}

/// FNV-1a: stable across Rust versions and platforms, unlike `DefaultHasher`.
struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}
