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
        /// The cover thumbnail's file; empty without one.
        #[qproperty(QString, cover)]
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
    PlayNext, PlayPrevious, Seek, SetLoopStatus, SetVolume, TogglePause, ToggleShuffle,
};
use n_music_core::queue::LoopStatus;

#[derive(Default)]
pub struct PlayerRust {
    loaded: bool,
    title: QString,
    artist: QString,
    album: QString,
    cover: QString,
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
}

impl cxx_qt::Initialize for qobject::Player {
    fn initialize(mut self: Pin<&mut Self>) {
        hub().watch(
            self.qt_thread(),
            Changed::CURRENT
                | Changed::PLAYBACK
                | Changed::POSITION
                | Changed::VOLUME
                | Changed::MODES,
            Self::changed,
        );
        self.as_mut().changed(Changed::all());
    }
}

impl qobject::Player {
    fn changed(mut self: Pin<&mut Self>, changed: Changed) {
        // A copy: setting properties runs QML bindings, which must not find the hub locked.
        let (track, time, seek, playing, volume, shuffle, loop_status) = {
            let state = hub().state();
            let track = state.current.clone();
            let modes = (state.shuffle, state.loop_status.clone());
            (
                track,
                state.time,
                state.seek,
                state.playing,
                state.volume,
                modes.0,
                modes.1,
            )
        };
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
        bus::emit(SetLoopStatus(match self.repeat {
            0 => LoopStatus::Playlist,
            1 => LoopStatus::File,
            _ => LoopStatus::Off,
        }));
    }
}
