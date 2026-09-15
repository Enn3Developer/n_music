use crate::state::{Backend, Change, Emit, MediaEvent, State};
use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::NSImage;
use objc2_foundation::{NSMutableDictionary, NSNumber, NSSize, NSString};
use objc2_media_player::{
    MPChangePlaybackPositionCommandEvent, MPMediaItemArtwork, MPMediaItemPropertyArtist,
    MPMediaItemPropertyArtwork, MPMediaItemPropertyPlaybackDuration, MPMediaItemPropertyTitle,
    MPNowPlayingInfoCenter, MPNowPlayingInfoPropertyElapsedPlaybackTime,
    MPNowPlayingInfoPropertyPlaybackRate, MPNowPlayingPlaybackState, MPRemoteCommand,
    MPRemoteCommandCenter, MPRemoteCommandEvent, MPRemoteCommandHandlerStatus,
};
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, RwLock};

pub(crate) fn new_controls(emit: Emit, state: Arc<RwLock<State>>) -> Option<Box<dyn Backend>> {
    Some(Box::new(NowPlaying::new(emit, state)?))
}

/// Raw macOS Now Playing backend (`MPNowPlayingInfoCenter` + `MPRemoteCommandCenter`).
///
/// MediaPlayer is main-thread only, so everything is dispatched to the main
/// queue.
pub(crate) struct NowPlaying {
    targets: Arc<Targets>,
}

impl NowPlaying {
    pub(crate) fn new(emit: Emit, _state: Arc<RwLock<State>>) -> Option<Self> {
        let targets = if MainThreadMarker::new().is_some() {
            // Synchronously dispatching to the main queue from the main thread
            // deadlocks (or traps) whenever the queue is not idle.
            Targets::new(unsafe { attach(&emit) })
        } else {
            let (tx, rx) = std::sync::mpsc::channel();
            DispatchQueue::main().exec_sync(move || {
                let _ = tx.send(Targets::new(unsafe { attach(&emit) }));
            });
            rx.recv().ok()?
        };

        Some(Self {
            targets: Arc::new(targets),
        })
    }
}

impl Backend for NowPlaying {
    fn update(&self, state: &State, change: Change) {
        let update = match change {
            Change::Playback => Update::Playback,
            Change::Metadata => Update::Metadata,
            Change::Position {
                discontinuity: true,
            } => Update::Position,
            // MediaPlayer extrapolates the position itself and has no volume,
            // loop or shuffle controls, so the 250 ms ticks are dropped here.
            Change::Position {
                discontinuity: false,
            }
            | Change::Volume
            | Change::Loop => return,
        };

        let state = Snapshot::from(state);
        DispatchQueue::main().exec_async(move || unsafe { apply(&state, update) });
    }
}

impl Drop for NowPlaying {
    fn drop(&mut self) {
        let targets = self.targets.clone();
        DispatchQueue::main().exec_async(move || unsafe { detach(&targets) });
    }
}

#[derive(Clone)]
struct Snapshot {
    playing: bool,
    position: f64,
    length: f64,
    title: String,
    artist: String,
    cover: Option<PathBuf>,
}

impl From<&State> for Snapshot {
    fn from(state: &State) -> Self {
        Self {
            playing: state.playing,
            position: state.position,
            length: state.length,
            title: state.title.clone(),
            artist: state.artist.clone(),
            cover: state.cover.clone(),
        }
    }
}

#[derive(Clone, Copy)]
enum Update {
    Playback,
    Metadata,
    Position,
}

#[derive(Clone, Copy)]
enum RemoteCommand {
    TogglePlayPause,
    Play,
    Pause,
    Next,
    Previous,
    ChangePlaybackPosition,
}

/// The opaque targets returned by `addTargetWithHandler:`, needed to unregister
/// the command handlers again.
struct Targets {
    entries: Mutex<Vec<(RemoteCommand, Retained<AnyObject>)>>,
}

// SAFETY: the retained targets are only ever dereferenced on the main queue
// (both registration and removal run there); elsewhere they are only moved.
unsafe impl Send for Targets {}
unsafe impl Sync for Targets {}

impl Targets {
    fn new(entries: Vec<(RemoteCommand, Retained<AnyObject>)>) -> Self {
        Self {
            entries: Mutex::new(entries),
        }
    }
}

unsafe fn attach(emit: &Emit) -> Vec<(RemoteCommand, Retained<AnyObject>)> {
    let center = MPRemoteCommandCenter::sharedCommandCenter();
    let mut targets = Vec::with_capacity(6);

    targets.push((
        RemoteCommand::TogglePlayPause,
        register(
            &center.togglePlayPauseCommand(),
            emit,
            MediaEvent::TogglePause,
        ),
    ));
    targets.push((
        RemoteCommand::Play,
        register(&center.playCommand(), emit, MediaEvent::Play),
    ));
    targets.push((
        RemoteCommand::Pause,
        register(&center.pauseCommand(), emit, MediaEvent::Pause),
    ));
    targets.push((
        RemoteCommand::Next,
        register(&center.nextTrackCommand(), emit, MediaEvent::Next),
    ));
    targets.push((
        RemoteCommand::Previous,
        register(&center.previousTrackCommand(), emit, MediaEvent::Previous),
    ));

    // The progress bar scrubber only exists next to the position command.
    let seek_emit = emit.clone();
    let seek_handler: RcBlock<
        dyn Fn(NonNull<MPRemoteCommandEvent>) -> MPRemoteCommandHandlerStatus,
    > = RcBlock::new(move |event: NonNull<MPRemoteCommandEvent>| {
        let event = unsafe { event.as_ref() };
        if let Some(event) = event.downcast_ref::<MPChangePlaybackPositionCommandEvent>() {
            seek_emit(MediaEvent::SeekAbsolute(unsafe { event.positionTime() }));
        }
        MPRemoteCommandHandlerStatus::Success
    });
    let seek_command = center.changePlaybackPositionCommand();
    seek_command.setEnabled(true);
    targets.push((
        RemoteCommand::ChangePlaybackPosition,
        seek_command.addTargetWithHandler(&seek_handler),
    ));

    targets
}

unsafe fn register(
    command: &MPRemoteCommand,
    emit: &Emit,
    event: MediaEvent,
) -> Retained<AnyObject> {
    command.setEnabled(true);
    let emit = emit.clone();
    let handler: RcBlock<dyn Fn(NonNull<MPRemoteCommandEvent>) -> MPRemoteCommandHandlerStatus> =
        RcBlock::new(move |_event: NonNull<MPRemoteCommandEvent>| {
            emit(event);
            MPRemoteCommandHandlerStatus::Success
        });
    command.addTargetWithHandler(&handler)
}

unsafe fn detach(targets: &Targets) {
    let center = MPRemoteCommandCenter::sharedCommandCenter();
    for (kind, target) in std::mem::take(&mut *targets.entries.lock().unwrap()) {
        let target = Some(&*target);
        match kind {
            RemoteCommand::TogglePlayPause => center.togglePlayPauseCommand().removeTarget(target),
            RemoteCommand::Play => center.playCommand().removeTarget(target),
            RemoteCommand::Pause => center.pauseCommand().removeTarget(target),
            RemoteCommand::Next => center.nextTrackCommand().removeTarget(target),
            RemoteCommand::Previous => center.previousTrackCommand().removeTarget(target),
            RemoteCommand::ChangePlaybackPosition => {
                center.changePlaybackPositionCommand().removeTarget(target)
            }
        }
    }
}

unsafe fn apply(state: &Snapshot, update: Update) {
    let center = MPNowPlayingInfoCenter::defaultCenter();
    if let Update::Playback = update {
        let playback_state = if state.playing {
            MPNowPlayingPlaybackState::Playing
        } else {
            MPNowPlayingPlaybackState::Paused
        };
        center.setPlaybackState(playback_state);
    }
    update_info(&center, state, matches!(update, Update::Metadata));
}

unsafe fn update_info(center: &MPNowPlayingInfoCenter, state: &Snapshot, rebuild: bool) {
    let info = if rebuild {
        let info = NSMutableDictionary::<NSString, AnyObject>::new();
        if !state.title.is_empty() {
            info.insert(MPMediaItemPropertyTitle, &NSString::from_str(&state.title));
        }
        if !state.artist.is_empty() {
            info.insert(
                MPMediaItemPropertyArtist,
                &NSString::from_str(&state.artist),
            );
        }
        if let Some(cover) = &state.cover {
            if let Some(artwork) = artwork(cover) {
                info.insert(MPMediaItemPropertyArtwork, &artwork);
            }
        }
        info
    } else {
        // Reuse the existing entry so the artwork is not decoded again.
        match center.nowPlayingInfo() {
            Some(info) => {
                NSMutableDictionary::initWithDictionary(NSMutableDictionary::alloc(), &info)
            }
            None => NSMutableDictionary::new(),
        }
    };

    if state.length > 0.0 {
        info.insert(
            MPMediaItemPropertyPlaybackDuration,
            &NSNumber::new_f64(state.length),
        );
    }
    info.insert(
        MPNowPlayingInfoPropertyElapsedPlaybackTime,
        &NSNumber::new_f64(state.position.max(0.0)),
    );
    info.insert(
        MPNowPlayingInfoPropertyPlaybackRate,
        &NSNumber::new_f64(if state.playing { 1.0 } else { 0.0 }),
    );
    center.setNowPlayingInfo(Some(&info));
}

unsafe fn artwork(path: &Path) -> Option<Retained<MPMediaItemArtwork>> {
    let path = NSString::from_str(path.to_string_lossy().as_ref());
    let image = NSImage::initWithContentsOfFile(NSImage::alloc(), &path)?;
    let size = image.size();

    let handler: RcBlock<dyn Fn(NSSize) -> NonNull<NSImage>> =
        RcBlock::new(move |_size: NSSize| -> NonNull<NSImage> {
            unsafe { NonNull::new_unchecked(Retained::autorelease_return(image.clone())) }
        });

    Some(MPMediaItemArtwork::initWithBoundsSize_requestHandler(
        MPMediaItemArtwork::alloc(),
        size,
        &handler,
    ))
}
