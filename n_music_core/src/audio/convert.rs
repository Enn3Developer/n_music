//! Turns a track's decoded audio into the output device's format: channel remixing, then
//! resampling when the rates differ.

use super::output::OutputFormat;
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Indexing, Resampler};
use symphonia::core::audio::Channels;

/// Input frames per resampler call.
const CHUNK: usize = 1024;
const MINUS_3DB: f32 = std::f32::consts::FRAC_1_SQRT_2;

pub struct Converter {
    source_rate: u32,
    source_channels: Channels,
    output: OutputFormat,
    /// Weights `[output channel][source channel]`; `None` when the layouts already match.
    matrix: Option<Vec<f32>>,
    /// Applied while remixing; it never changes within a track.
    gain: f32,
    remixed: Vec<f32>,
    resampler: Option<Resampling>,
}

struct Resampling {
    resampler: Fft<f32>,
    /// Remixed frames waiting for a full chunk.
    pending: Vec<f32>,
    out: Vec<f32>,
    /// Startup delay frames still to drop.
    delay: usize,
    frames_in: u64,
    frames_out: u64,
}

impl Converter {
    pub fn new(
        source_rate: u32,
        source_channels: &Channels,
        output: OutputFormat,
        gain: f32,
    ) -> Result<Self, String> {
        let source_count = source_channels.count();
        if source_count == 0 || output.channels == 0 {
            return Err(String::from("No audio channels"));
        }
        let matrix = mix_matrix(source_channels, output.channels);
        let resampler = if source_rate == output.rate {
            None
        } else {
            let resampler = Fft::new(
                source_rate as usize,
                output.rate as usize,
                CHUNK,
                output.channels,
                FixedSync::Input,
            )
            .map_err(|error| error.to_string())?;
            Some(Resampling {
                delay: resampler.output_delay(),
                out: vec![0.0; resampler.output_frames_max() * output.channels],
                resampler,
                pending: Vec::with_capacity(CHUNK * output.channels),
                frames_in: 0,
                frames_out: 0,
            })
        };
        Ok(Self {
            source_rate,
            source_channels: source_channels.clone(),
            output,
            matrix,
            gain,
            remixed: vec![],
            resampler,
        })
    }

    /// Whether this converter handles audio of this layout for this device.
    pub fn fits(&self, source_rate: u32, source_channels: &Channels, output: OutputFormat) -> bool {
        self.source_rate == source_rate
            && &self.source_channels == source_channels
            && self.output == output
    }

    /// Converts interleaved source frames, appending the result to `out`. A resampler holds
    /// back up to one chunk until more input or [`Self::finish`].
    pub fn push(&mut self, input: &[f32], out: &mut Vec<f32>) {
        let target = if self.resampler.is_some() {
            &mut self.remixed
        } else {
            &mut *out
        };
        remix(
            input,
            self.source_channels.count(),
            self.output.channels,
            self.matrix.as_deref(),
            self.gain,
            target,
        );
        if let Some(resampling) = &mut self.resampler {
            resampling.pending.append(&mut self.remixed);
            resampling.run(self.output.channels, false, out);
        }
    }

    /// Flushes the resampler at the end of the track, so its output ends exactly where the
    /// input did.
    pub fn finish(&mut self, out: &mut Vec<f32>) {
        if let Some(resampling) = &mut self.resampler {
            resampling.run(self.output.channels, true, out);
        }
    }

    /// Drops buffered audio, e.g. after a seek.
    pub fn reset(&mut self) {
        if let Some(resampling) = &mut self.resampler {
            resampling.resampler.reset();
            resampling.pending.clear();
            resampling.delay = resampling.resampler.output_delay();
            resampling.frames_in = 0;
            resampling.frames_out = 0;
        }
    }
}

impl Resampling {
    fn run(&mut self, channels: usize, finish: bool, out: &mut Vec<f32>) {
        let expected = |frames_in: u64, resampler: &Fft<f32>| {
            (frames_in as f64 * resampler.resample_ratio()).ceil() as u64
        };
        loop {
            let needed = self.resampler.input_frames_next();
            let available = self.pending.len() / channels;
            let flushing = finish && available < needed;
            if !flushing && available < needed {
                return;
            }
            if flushing && self.frames_out >= expected(self.frames_in, &self.resampler) {
                self.pending.clear();
                return;
            }
            let used = available.min(needed);
            let indexing = Indexing {
                partial_len: flushing.then_some(used),
                ..Indexing::new()
            };
            let input = InterleavedSlice::new(&self.pending[..], channels, available)
                .expect("pending holds whole frames");
            let frames = self.out.len() / channels;
            let mut output = InterleavedSlice::new_mut(&mut self.out[..], channels, frames)
                .expect("output holds whole frames");
            let Ok((_, produced)) =
                self.resampler
                    .process_into_buffer(&input, &mut output, Some(&indexing))
            else {
                log::error!("Resampling failed; dropping {used} frames");
                self.pending.drain(..used * channels);
                continue;
            };
            self.pending.drain(..used * channels);
            self.frames_in += used as u64;
            let skip = self.delay.min(produced);
            self.delay -= skip;
            let mut keep = produced - skip;
            if flushing {
                let missing = expected(self.frames_in, &self.resampler) - self.frames_out;
                keep = keep.min(missing as usize);
            }
            self.frames_out += keep as u64;
            out.extend_from_slice(&self.out[skip * channels..(skip + keep) * channels]);
        }
    }
}

fn remix(
    input: &[f32],
    from: usize,
    to: usize,
    matrix: Option<&[f32]>,
    gain: f32,
    out: &mut Vec<f32>,
) {
    let Some(matrix) = matrix else {
        if gain == 1.0 {
            out.extend_from_slice(input);
        } else {
            out.extend(input.iter().map(|sample| sample * gain));
        }
        return;
    };
    out.reserve(input.len() / from * to);
    for frame in input.chunks_exact(from) {
        for weights in matrix.chunks_exact(from) {
            let sample: f32 = frame.iter().zip(weights).map(|(s, w)| s * w).sum();
            out.push(sample * gain);
        }
    }
}

enum Side {
    Left,
    Right,
    Center,
    Lfe,
}

/// Downmix and upmix weights, or `None` when the source is already in the output layout.
/// Multichannel devices get the front pair only: CPAL does not say where their other
/// speakers are.
fn mix_matrix(source: &Channels, outputs: usize) -> Option<Vec<f32>> {
    let inputs = source.count();
    let sides: Vec<Side> = match source {
        Channels::Positioned(positions) => positions
            .iter_names()
            .map(|(name, _)| {
                if name.starts_with("LFE") {
                    Side::Lfe
                } else if name.contains("LEFT") {
                    Side::Left
                } else if name.contains("RIGHT") {
                    Side::Right
                } else {
                    Side::Center
                }
            })
            .collect(),
        _ => vec![],
    };
    if inputs == outputs {
        return None;
    }
    let mut matrix = vec![0.0; inputs * outputs];
    let mut set =
        |output: usize, input: usize, weight: f32| matrix[output * inputs + input] = weight;
    if inputs == 1 {
        for output in 0..outputs.min(2) {
            set(output, 0, 1.0);
        }
    } else if outputs == 1 {
        let audible = sides
            .iter()
            .filter(|side| !matches!(side, Side::Lfe))
            .count()
            .max(if sides.is_empty() { inputs } else { 0 })
            .max(1);
        for input in 0..inputs {
            if !matches!(sides.get(input), Some(Side::Lfe)) {
                set(0, input, 1.0 / audible as f32);
            }
        }
    } else if sides.len() == inputs {
        // Fronts at full level, everything else 3 dB down; LFE is dropped.
        let mut seen_left = false;
        let mut seen_right = false;
        for (input, side) in sides.iter().enumerate() {
            match side {
                Side::Left => {
                    set(0, input, if seen_left { MINUS_3DB } else { 1.0 });
                    seen_left = true;
                }
                Side::Right => {
                    set(1, input, if seen_right { MINUS_3DB } else { 1.0 });
                    seen_right = true;
                }
                Side::Center => {
                    set(0, input, MINUS_3DB);
                    set(1, input, MINUS_3DB);
                }
                Side::Lfe => {}
            }
        }
    } else {
        // Unknown layout: channel by channel, wrapping around the outputs.
        for input in 0..inputs {
            set(input % outputs, input, 1.0);
        }
    }
    Some(matrix)
}
