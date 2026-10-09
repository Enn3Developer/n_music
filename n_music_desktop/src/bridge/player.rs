//! The current track and playback: what the player bar shows and controls.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// A track is current.
        #[qproperty(bool, loaded)]
        #[qproperty(QString, title)]
        #[qproperty(QString, artist)]
        #[qproperty(QString, album)]
        /// Empty when unknown.
        #[qproperty(QString, year)]
        /// The cover thumbnail's file; empty without one.
        #[qproperty(QString, cover)]
        /// Short codec name in capitals, like `FLAC`; empty when unknown.
        #[qproperty(QString, codec)]
        /// In Hz; 0 when unknown.
        #[qproperty(i32, sample_rate)]
        /// 0 when unknown or not fixed (lossy codecs).
        #[qproperty(i32, bits)]
        /// ReplayGain in dB; NaN when untagged.
        #[qproperty(f64, track_gain)]
        #[qproperty(f64, album_gain)]
        /// How many times the current track was played.
        #[qproperty(i32, plays)]
        #[qproperty(bool, playing)]
        /// Seconds into the current track.
        #[qproperty(f64, position)]
        /// The current track's length in seconds; 0 while unknown.
        #[qproperty(f64, length)]
        /// From 0 to 1.
        #[qproperty(f64, volume)]
        #[qproperty(bool, shuffle)]
        /// 0: plays once, 1: the context starts over, 2: the track repeats.
        #[qproperty(i32, repeat)]
        type Player = super::PlayerRust;

        /// Another item of the session is about to be current; `back` when it plays before the
        /// one that was. The properties still tell the old track, and change right after.
        #[qsignal]
        fn track_changing(self: Pin<&mut Player>, back: bool);

        /// Pauses, or plays: the current track, else the library.
        #[qinvokable]
        fn toggle(self: &Player);
        #[qinvokable]
        fn next(self: &Player);
        /// The previous track, or the start of this one when past its first seconds.
        #[qinvokable]
        fn previous(self: &Player);
        #[qinvokable]
        fn seek(self: Pin<&mut Player>, position: f64);
        #[qinvokable]
        fn change_volume(self: &Player, volume: f64);
        /// Silences playback, or brings back the volume it had.
        #[qinvokable]
        fn toggle_mute(self: Pin<&mut Player>);
        #[qinvokable]
        fn toggle_shuffle(self: &Player);
        /// Off, then repeat the context, then repeat the track.
        #[qinvokable]
        fn cycle_repeat(self: &Player);
        /// See `repeat`.
        #[qinvokable]
        fn change_repeat(self: &Player, repeat: i32);
        #[qinvokable]
        fn change_shuffle(self: &Player, shuffle: bool);
    }

    impl cxx_qt::Threading for Player {}
    impl cxx_qt::Initialize for Player {}
}

use crate::bus;
use crate::hub::{hub, Changed};
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use n_music_core::messages::{
    PlayNext, PlayPrevious, Seek, SetLoopStatus, SetShuffle, SetVolume, TogglePause, ToggleShuffle,
};
use n_music_core::queue::{ItemId, LoopStatus};

#[derive(Default)]
pub struct PlayerRust {
    loaded: bool,
    title: QString,
    artist: QString,
    album: QString,
    year: QString,
    cover: QString,
    codec: QString,
    sample_rate: i32,
    bits: i32,
    track_gain: f64,
    album_gain: f64,
    plays: i32,
    playing: bool,
    position: f64,
    length: f64,
    volume: f64,
    shuffle: bool,
    repeat: i32,
    /// The last tracked seek sent: positions reported before it applied are stale.
    seek_sent: u64,
    /// The volume before muting.
    unmuted: f64,
    /// The current item of the session.
    item: Option<ItemId>,
}

impl cxx_qt::Initialize for qobject::Player {
    fn initialize(mut self: Pin<&mut Self>) {
        hub().watch(
            self.qt_thread(),
            Changed::CURRENT
                | Changed::PLAYBACK
                | Changed::POSITION
                | Changed::VOLUME
                | Changed::MODES
                | Changed::STATS,
            Self::changed,
        );
        self.as_mut().changed(Changed::all());
    }
}

impl qobject::Player {
    fn changed(mut self: Pin<&mut Self>, changed: Changed) {
        // A copy: setting properties runs QML bindings, which must not find the hub locked.
        let (track, item, queue, time, seek, playing, volume, shuffle, loop_status) = {
            let state = hub().state();
            let track = state.current.clone();
            let modes = (state.shuffle, state.loop_status.clone());
            (
                track,
                state.current_item,
                state.queue.clone(),
                state.time,
                state.seek,
                state.playing,
                state.volume,
                modes.0,
                modes.1,
            )
        };
        if changed.contains(Changed::CURRENT) && item != self.item {
            let place = |item: Option<ItemId>| {
                item.and_then(|item| queue.iter().position(|entry| entry.item == item))
            };
            let back =
                matches!((place(self.item), place(item)), (Some(was), Some(now)) if now < was);
            self.as_mut().rust_mut().item = item;
            self.as_mut().track_changing(back);
        }
        if changed.contains(Changed::CURRENT) {
            let text = |text: Option<&str>| QString::from(text.unwrap_or_default());
            self.as_mut().set_loaded(track.is_some());
            self.as_mut()
                .set_title(text(track.as_ref().map(|track| track.title.as_str())));
            self.as_mut().set_artist(QString::from(
                &track
                    .as_ref()
                    .map(|track| track.artist())
                    .unwrap_or_default(),
            ));
            self.as_mut().set_album(text(
                track.as_ref().and_then(|track| track.album.as_deref()),
            ));
            let cover = track
                .as_ref()
                .and_then(|track| track.cover.as_deref())
                .map(|path| path.to_string_lossy().into_owned());
            self.as_mut().set_cover(text(cover.as_deref()));
            self.as_mut().set_year(QString::from(
                &track
                    .as_ref()
                    .and_then(|track| track.year)
                    .map(|year| year.to_string())
                    .unwrap_or_default(),
            ));
            self.as_mut().set_codec(QString::from(
                &track
                    .as_ref()
                    .and_then(|track| track.codec.as_deref())
                    .unwrap_or_default()
                    .to_uppercase(),
            ));
            let number = |value: Option<u32>| value.map_or(0, |value| value as i32);
            self.as_mut()
                .set_sample_rate(number(track.as_ref().and_then(|track| track.sample_rate)));
            self.as_mut().set_bits(number(
                track.as_ref().and_then(|track| track.bits_per_sample),
            ));
            let gain = |gain: Option<f32>| gain.map_or(f64::NAN, f64::from);
            self.as_mut().set_track_gain(gain(
                track
                    .as_ref()
                    .and_then(|track| track.replay_gain.track_gain),
            ));
            self.as_mut().set_album_gain(gain(
                track
                    .as_ref()
                    .and_then(|track| track.replay_gain.album_gain),
            ));
        }
        if changed.intersects(Changed::CURRENT | Changed::STATS) {
            let plays = track.as_ref().map_or(0, |track| {
                hub()
                    .library()
                    .read()
                    .stats(&track.locator)
                    .map_or(0, |stats| stats.plays)
            });
            self.as_mut()
                .set_plays(i32::try_from(plays).unwrap_or(i32::MAX));
        }
        if changed.intersects(Changed::CURRENT | Changed::POSITION) {
            // The decoder knows the length best; the tags tell it until it does.
            let length = match time.length {
                length if length > 0.0 => length,
                _ => track.as_ref().map_or(0.0, |track| track.length),
            };
            self.as_mut().set_length(length);
            if seek >= self.seek_sent {
                self.as_mut().set_position(time.position);
            }
        }
        if changed.contains(Changed::PLAYBACK) {
            self.as_mut().set_playing(playing);
        }
        if changed.contains(Changed::VOLUME) {
            self.as_mut().set_volume(volume);
        }
        if changed.contains(Changed::MODES) {
            self.as_mut().set_shuffle(shuffle);
            self.as_mut().set_repeat(match loop_status {
                LoopStatus::Off => 0,
                LoopStatus::Playlist => 1,
                LoopStatus::File => 2,
            });
        }
    }

    fn toggle(&self) {
        bus::emit(TogglePause);
    }

    fn next(&self) {
        bus::emit(PlayNext);
    }

    fn previous(&self) {
        bus::emit(PlayPrevious);
    }

    fn seek(mut self: Pin<&mut Self>, position: f64) {
        let request = self.seek_sent + 1;
        self.as_mut().rust_mut().seek_sent = request;
        // Shows the new position at once; reports from before the seek are ignored.
        self.as_mut().set_position(position);
        bus::emit(Seek::Tracked { position, request });
    }

    fn change_volume(&self, volume: f64) {
        bus::emit(SetVolume(volume));
    }

    fn toggle_mute(mut self: Pin<&mut Self>) {
        if self.volume > 0.0 {
            let volume = self.volume;
            self.as_mut().rust_mut().unmuted = volume;
            bus::emit(SetVolume(0.0));
        } else {
            let unmuted = if self.unmuted > 0.0 {
                self.unmuted
            } else {
                0.5
            };
            bus::emit(SetVolume(unmuted));
        }
    }

    fn toggle_shuffle(&self) {
        bus::emit(ToggleShuffle);
    }

    fn cycle_repeat(&self) {
        self.change_repeat((self.repeat + 1) % 3);
    }

    fn change_repeat(&self, repeat: i32) {
        bus::emit(SetLoopStatus(match repeat {
            1 => LoopStatus::Playlist,
            2 => LoopStatus::File,
            _ => LoopStatus::Off,
        }));
    }

    fn change_shuffle(&self, shuffle: bool) {
        bus::emit(SetShuffle(shuffle));
    }
}
