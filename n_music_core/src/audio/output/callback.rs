//! What the stream callback does, on the device's realtime thread: it must never block or
//! allocate. Backends call [`Renderer::render`] from their own callback, in the sample type their
//! stream uses.

use super::dsp::{dither, soft_clip};
use super::{OutputFormat, RAMP};
use dasp::sample::FromSample;
use dasp::Sample;
use rtrb::Consumer;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::Thread;
use std::time::{Duration as WallDuration, Instant};

/// State shared between the writer and the stream callback.
pub(super) struct SharedState {
    origin: Instant,
    /// Samples the callback took from the ring, played or skipped.
    consumed: AtomicU64,
    /// Samples before this are skipped instead of played.
    pub(super) discard_until: AtomicU64,
    /// When the device will have played what the callback handed it.
    device_until_ns: AtomicU64,
    /// The stream is paused with audio left in the device: [`Self::held_ns`] of it, which no
    /// clock runs down until it plays again.
    holding: AtomicBool,
    held_ns: AtomicU64,
    /// When the callback last ran.
    pub(super) heartbeat_ns: AtomicU64,
    pub(super) failed: AtomicBool,
    /// Target gain, as `f32` bits.
    pub(super) volume: AtomicU32,
    /// Fade to silence instead of `volume`; queued audio is kept, not played.
    pub(super) muted: AtomicBool,
    /// Set by the callback while muted and fully faded out.
    pub(super) silent: AtomicBool,
    /// The callback wakes the writer once fewer samples than this are queued; 0 when the
    /// writer is not waiting.
    pub(super) wake_below: AtomicUsize,
    pub(super) writer: Mutex<Option<Thread>>,
}

impl SharedState {
    pub(super) fn new(volume: f32) -> Self {
        Self {
            origin: Instant::now(),
            consumed: AtomicU64::new(0),
            discard_until: AtomicU64::new(0),
            device_until_ns: AtomicU64::new(0),
            holding: AtomicBool::new(false),
            held_ns: AtomicU64::new(0),
            heartbeat_ns: AtomicU64::new(0),
            failed: AtomicBool::new(false),
            volume: AtomicU32::new(volume.to_bits()),
            muted: AtomicBool::new(false),
            silent: AtomicBool::new(false),
            wake_below: AtomicUsize::new(0),
            writer: Mutex::new(None),
        }
    }

    pub(super) fn now_ns(&self) -> u64 {
        self.origin.elapsed().as_nanos() as u64
    }

    /// The stream broke for good: the writer opens another.
    pub(super) fn fail(&self) {
        self.failed.store(true, Ordering::Relaxed);
        self.wake();
    }

    /// The stream glitched and plays on, as after an underrun or once the system rerouted it.
    pub(super) fn glitch(&self) {
        self.wake();
    }

    /// Unparks the writer, unless it is being replaced right now (it then has a timeout).
    fn wake(&self) {
        if let Ok(writer) = self.writer.try_lock() {
            if let Some(writer) = writer.as_ref() {
                writer.unpark();
            }
        }
    }

    pub(super) fn pending_frames(&self, produced: u64, channels: usize, rate: u32) -> usize {
        // A concurrent callback can cause a brief overestimate, never an early EOF.
        let consumed = self.consumed.load(Ordering::Acquire);
        let discarded = self.discard_until.load(Ordering::Relaxed);
        let queued = produced.saturating_sub(consumed.max(discarded)) as usize;
        let device_ns = self.device_left().as_nanos() as f64;
        queued / channels + (device_ns * rate as f64 / 1_000_000_000.0).ceil() as usize
    }

    /// How long the device still plays what the callback handed it.
    pub(super) fn device_left(&self) -> WallDuration {
        if self.holding.load(Ordering::Acquire) {
            return WallDuration::from_nanos(self.held_ns.load(Ordering::Relaxed));
        }
        WallDuration::from_nanos(
            self.device_until_ns
                .load(Ordering::Relaxed)
                .saturating_sub(self.now_ns()),
        )
    }

    /// The stream pauses with what the device holds kept in it: its clock stops until
    /// [`Self::release_device`].
    pub(super) fn hold_device(&self) {
        if self.holding.load(Ordering::Acquire) {
            return;
        }
        self.held_ns
            .store(self.device_left().as_nanos() as u64, Ordering::Relaxed);
        self.holding.store(true, Ordering::Release);
    }

    /// The stream plays again: what the device held plays out from now.
    pub(super) fn release_device(&self) {
        if !self.holding.swap(false, Ordering::AcqRel) {
            return;
        }
        let held = self.held_ns.load(Ordering::Relaxed);
        self.device_until_ns
            .store(self.now_ns().saturating_add(held), Ordering::Relaxed);
    }

    /// The device dropped what it held.
    pub(super) fn clear_device(&self) {
        self.held_ns.store(0, Ordering::Relaxed);
        self.device_until_ns.store(self.now_ns(), Ordering::Relaxed);
    }
}

/// A sample type a stream can play.
pub(super) trait OutputSample: Sample + FromSample<f32> + Send + 'static {
    /// One least significant bit, the dither amplitude; 0 when the format needs no dither.
    const LSB: f32;
}

impl OutputSample for f32 {
    const LSB: f32 = 0.0;
}

impl OutputSample for i32 {
    const LSB: f32 = 0.0;
}

impl OutputSample for i16 {
    const LSB: f32 = 1.0 / 32768.0;
}

impl OutputSample for u16 {
    const LSB: f32 = 1.0 / 32768.0;
}

/// Turns what is queued into what the device plays: applies the volume (ramped, so changes,
/// pauses and seeks do not click), soft-clips and converts to the stream's sample type.
pub(super) struct Renderer {
    consumer: Consumer<f32>,
    consumed: u64,
    state: Arc<SharedState>,
    channels: usize,
    rate: u32,
    /// Sized for the largest request when the stream is built; grows only if a device asks
    /// for more, which would allocate on the audio thread.
    mix: Vec<f32>,
    gain: f32,
    /// Gain change per frame while ramping.
    step: f32,
    clip_mem: Vec<f32>,
    noise: u32,
}

impl Renderer {
    /// A renderer for `consumer`, with room to mix `frames` frames.
    pub(super) fn new(
        consumer: Consumer<f32>,
        state: Arc<SharedState>,
        format: OutputFormat,
        frames: usize,
    ) -> Self {
        Self {
            consumer,
            consumed: 0,
            state,
            channels: format.channels,
            rate: format.rate,
            mix: vec![0.0; frames * format.channels],
            gain: 0.0,
            step: 1.0 / (RAMP.as_secs_f32() * format.rate as f32).max(1.0),
            clip_mem: vec![0.0; format.channels],
            noise: 0x9E37_79B9,
        }
    }

    /// Fills `data` with what is queued, silence where nothing is. `playback_delay` is how long
    /// until the device plays the first frame of `data`.
    pub(super) fn render<T: OutputSample>(&mut self, data: &mut [T], playback_delay: WallDuration) {
        let state = &*self.state;
        let now = state.now_ns();
        state.heartbeat_ns.store(now, Ordering::Relaxed);
        // Only the consumer advances the read index, including on seek while paused.
        let discard = state.discard_until.load(Ordering::Acquire);
        let skip = discard.saturating_sub(self.consumed) as usize;
        if skip > 0 {
            if let Ok(chunk) = self.consumer.read_chunk(skip) {
                chunk.commit_all();
                self.consumed += skip as u64;
                self.clip_mem.fill(0.0);
            }
        }

        let requested = data.len() / self.channels * self.channels;
        if self.mix.len() < requested {
            self.mix.resize(requested, 0.0);
        }
        let mix = &mut self.mix[..requested];
        let muted = state.muted.load(Ordering::Relaxed);
        let target = if muted {
            0.0
        } else {
            f32::from_bits(state.volume.load(Ordering::Relaxed))
        };
        // Faded out: keep the queue for when the fade-in starts.
        let written = if muted && self.gain == 0.0 {
            0
        } else {
            self.consumer.pop_partial_slice(mix).0.len()
        };
        mix[written..].fill(0.0);
        self.consumed += written as u64;
        if written > 0 {
            let duration = WallDuration::from_secs_f64(
                written as f64 / self.channels as f64 / self.rate as f64,
            );
            state.device_until_ns.store(
                now.saturating_add(playback_delay.as_nanos() as u64)
                    .saturating_add(duration.as_nanos() as u64),
                Ordering::Relaxed,
            );
        }
        // Publish the device deadline before removing these samples from pending_frames.
        state.consumed.store(self.consumed, Ordering::Release);

        for frame in mix.chunks_exact_mut(self.channels) {
            if self.gain != target {
                self.gain = if self.gain < target {
                    (self.gain + self.step).min(target)
                } else {
                    (self.gain - self.step).max(target)
                };
            }
            for sample in frame {
                *sample *= self.gain;
            }
        }
        state
            .silent
            .store(muted && self.gain == 0.0, Ordering::Release);
        soft_clip(mix, self.channels, &mut self.clip_mem);
        for (out, &sample) in data.iter_mut().zip(mix.iter()) {
            *out = T::from_sample(dither(sample, T::LSB, &mut self.noise));
        }
        data[requested..].fill(T::EQUILIBRIUM);

        let wake_below = state.wake_below.load(Ordering::Acquire);
        if wake_below > 0
            && self.consumer.slots() < wake_below
            && state.wake_below.swap(0, Ordering::AcqRel) != 0
        {
            state.wake();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtrb::RingBuffer;

    const FORMAT: OutputFormat = OutputFormat {
        rate: 1000,
        channels: 2,
    };

    /// A renderer over a queue holding `frames` frames of full scale.
    fn queued(frames: usize) -> (Renderer, Arc<SharedState>, u64) {
        let (mut producer, consumer) = RingBuffer::new(1000);
        let samples = vec![1.0; frames * FORMAT.channels];
        let (pushed, _) = producer.push_partial_slice(&samples);
        assert_eq!(pushed.len(), samples.len());
        let state = Arc::new(SharedState::new(1.0));
        let renderer = Renderer::new(consumer, state.clone(), FORMAT, 500);
        (renderer, state, samples.len() as u64)
    }

    #[test]
    fn what_the_device_holds_counts_as_pending() {
        let (mut renderer, state, produced) = queued(20);
        let mut data = [0.0f32; 20];
        renderer.render(&mut data, WallDuration::from_millis(100));
        // 10 frames are left queued, and the device plays 100 ms, then the 10 frames it got.
        let pending = state.pending_frames(produced, FORMAT.channels, FORMAT.rate);
        assert!((115..=120).contains(&pending), "{pending}");
    }

    #[test]
    fn a_held_device_keeps_its_audio_until_released() {
        let (mut renderer, state, _) = queued(20);
        let mut data = [0.0f32; 20];
        renderer.render(&mut data, WallDuration::from_millis(100));
        state.hold_device();
        let held = state.device_left();
        std::thread::sleep(WallDuration::from_millis(30));
        assert_eq!(state.device_left(), held);
        state.release_device();
        let left = state.device_left();
        assert!(
            left <= held && left + WallDuration::from_millis(20) > held,
            "{left:?} of {held:?}"
        );
        state.clear_device();
        assert_eq!(state.device_left(), WallDuration::ZERO);
    }

    #[test]
    fn clearing_a_held_device_leaves_nothing_to_play() {
        let (mut renderer, state, _) = queued(20);
        let mut data = [0.0f32; 20];
        renderer.render(&mut data, WallDuration::from_millis(100));
        state.hold_device();
        state.clear_device();
        assert_eq!(state.device_left(), WallDuration::ZERO);
        state.release_device();
        assert_eq!(state.device_left(), WallDuration::ZERO);
    }

    #[test]
    fn renders_into_integer_streams_with_silence_past_the_queue() {
        let (mut renderer, _, _) = queued(5);
        let mut data = [0i16; 20];
        renderer.render(&mut data, WallDuration::ZERO);
        // The volume ramps up from silence, then the queue runs out.
        assert!(data[..10].iter().any(|&sample| sample > 0), "{data:?}");
        assert!(data[10..].iter().all(|&sample| sample == 0), "{data:?}");
    }
}
