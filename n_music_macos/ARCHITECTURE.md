# N Music for macOS: architecture

Where things are and what to call. The UX follows the Qt desktop app (`n_music_desktop/qml`):
same pages, flows and wording, with native controls.

## Build and run

- `xcodegen < /dev/null` in `n_music_macos/` after adding or removing files (the project is
  generated from `project.yml`; commit `NMusic.xcodeproj`).
- `xcodebuild -project NMusic.xcodeproj -scheme NMusic -configuration Debug -derivedDataPath ../target/<your-dir> build`.
  The pre-build phase runs `scripts/build-core.sh`: Debug builds the core with Cargo's `dev`
  profile (fast), Release with `[profile.macos]` (fat LTO, `-C target-cpu=apple-m1`, ~6 min).
  `N_MUSIC_RUST_PROFILE=macos` forces the optimized core in Debug.
- Bindings land in `Generated/NMusicCore.swift` (gitignored); read it for exact FFI names.
- Run it sandboxed (its own data folder, so your library is untouched) and in the background: `open -g -n --env
  CFFIXED_USER_HOME=$H --env HOME=$H --stdout $H/out.txt --stderr $H/err.txt "N Music.app"`.
  The main window opens without the app being activated (`Windows.openMain()` opens it when
  SwiftUI waits for an activation); it may sit behind other apps' windows, which
  `screencapture -l<id>` and the `snapshot` hook don't mind. The `window` hook orders it in
  front, still without activating. An inactive app has no key window: real key presses can't be
  checked this way.
- Debug builds listen for `DistributedNotificationCenter` notifications named
  `com.enn3developer.n-music.debug` (object = command). Start a command with `@<pid> ` so only
  that process runs it (unprefixed commands reach every running Debug build). Commands: `go
  <page id>`, `back`, `forward`, `size <w> <h>`, `window`, `mini`, `expand` (none activates the
  app), `play-all`, `play`, `pause`, `next`, `previous`, `volume <0…1>`, `toast`, `reject`,
  `new-playlist`, `close-dialog`, `drawer`, `close-drawer`, `snapshot <dir>`, `dump <dir>`,
  `state <dir>`, `menus <dir>`, `quit`. See `App/DebugHooks.swift`, which also installs the
  areas' hooks at launch (`TracksDebug` `.debug.tracks`, `QueueDebug` `.debug.player`,
  `SourcesDebug` `.debug.sources`, `SettingsDebug` `.debug.settings`, all taking `@<pid> `;
  `GroupsDebug` listens on `.debug.groups.<pid>`, without the prefix).
  A poster: `swift -e 'import Foundation; DistributedNotificationCenter.default().postNotificationName(.init("com.enn3developer.n-music.debug"), object: "@4242 go albums", deliverImmediately: true)'`.

## Files you own vs. foundation files

Replace only your placeholder(s); each has a doc comment saying what to build:

| Placeholder | File |
|---|---|
| Tracks page, filter drawer | `Views/Tracks/TracksPage.swift`, `Views/FilterDrawer/FilterDrawer.swift` |
| Albums/Artists/Genres grids | `Views/Groups/GroupsPage.swift` (`GroupsPage(kind: GroupKind)`) |
| Album/artist/genre/source page | `Views/Collection/CollectionPage.swift` (`CollectionPage(kind: CollectionKind, argument: String)`) |
| Playlist page | `Views/Playlist/PlaylistPage.swift` (`PlaylistPage(id: PlaylistId)`) |
| Queue page, mini player, Now Playing panel | `Views/Queue/QueuePage.swift`, `Views/MiniPlayer/MiniPlayerView.swift`, `Views/NowPlaying/NowPlayingPanel.swift` |
| Sources page, first run | `Views/Sources/SourcesPage.swift`, `Views/Import/ImportPage.swift` |
| Settings page | `Views/Settings/SettingsPage.swift` (`SettingsPage(section: String)`) |

The shell's `PageRouter` (`Views/Shell/MainWindow.swift`) creates the pages with these
initializers. A page's helpers live next to it in `Views/<Area>/`.

## Rules that bite

- Views hold singletons as `private let store = Store.shared` (also `Navigation.shared`,
  `UISettings.shared`, `Toasts.shared`). **Never** `@Environment(Store.self)`: Table cells are
  hosted separately and don't get the environment (the last attempt crashed on it).
- Swift 6: a closure you hand to AppKit/MediaPlayer/Core Audio/NotificationCenter(nil queue)/
  NSEvent monitors/KVO that may run off the main thread must be created in a `nonisolated`
  function (or be `@Sendable`) and hop with `Task { @MainActor in … }`. See
  `Platform/NowPlaying.swift`.
- Never start a nested event loop (`NSMenu.popUp`, `runModal`, a panel's `ok`) inside a `Task`
  or any other main-actor job: the main queue stops being served until it ends, so the core's
  events and every other task stall. Hand it to `MainRunLoop.perform { … }` (`App/`), as
  `SourceMenus.popUp` and the track table's menus do.
- Never query the core on the main thread: use the async `Store` queries. Re-query when
  `store.libraryVersion` (tracks, names, plays; at most once a second during scans) or
  `store.playlistsVersion` changes, e.g. `.task(id: store.libraryVersion) { … }`. Debounce search
  ~150 ms. Show nothing until the first result (spec §9.2).
- No timers while idle. For the playback position use `PlayClock { position in … }` (or
  `store.position()`), never a stored value; it only redraws while playing and visible.
- All text through `tr("key", args…)` with the Qt keys (`%1 %2` placeholders). New strings the Qt
  app never had: `tr("key", fallback: "English")`.
- Each section's page stays alive (hidden) once visited: keep page state in `@State`; it
  survives navigation. Your page is disabled and hidden while another section shows.

## Core API (`Core/`)

### `Store.shared` (`Core/Store.swift`, `Core/Store+Desktop.swift`)

State (observable, main actor): `playing`, `current: CurrentTrack?` (`.item`, `.track: TrackRow`),
`details: TrackDetails?` (codec, sample rate, bits, genres, plays, last played of the current
track), `trackChange` / `trackChangeBack` (for TrackSwap direction), `length`, `position(at:)`,
`volume`, `muted`, `loopStatus`, `shuffle`, `queue: [QueueRow]`, `options: PlaybackOptions`,
`outputDevices`, `outputDevice`, `windowVisible`, `libraryVersion`, `playlistsVersion`,
`roots: [Locator]?`, `sources: [SourceRow]`, `playlists: [PlaylistSummary]` (core order),
`summary` (whole library), `facets`, `scan: ScanState?` (`found`, `read`, `progress`),
`lastCompleteScan: Date?`, `streamCache: StreamCacheState?`, `ready`, `firstRun`,
`telegram: TelegramState?` (nil = no Telegram in this build), `telegramChats`, `dataDirectory`,
`cacheDirectory`, `logsDirectory`. `observers` hear every `CoreEvent` after it applied.

Commands: `play(_ query, from:)`, `shufflePlay(_:)`, `togglePause()`, `play()`, `pause()`,
`next()`, `previous()`, `skipBack()`, `play(item:at:)`, `seek(to:)`, `seek(by:)`,
`enqueue(_:next:title:)` (title → queued toast + queue ping when out of sight),
`removeQueued`, `clearQueued`, `moveUpcoming(_:before:)`, `moveCurrent(_:before:)`,
`setVolume`, `toggleMute`, `setShuffle`, `toggleShuffle`, `setLoopStatus`, `cycleRepeat`,
`toggleRepeat`, `setOutputDevice`, `listOutputDevices`, `setReplayGain`, `setResume`,
`setCrossfade`, `setOffload`, `setSleepFade(endsIn:fade:)`, `createPlaylist(name:rule:sort:tracks:)`
(opens the new playlist when it arrives), `renamePlaylist`, `deletePlaylist`, `setPlaylistRule`,
`setPlaylistSort`, `addToPlaylist`, `removeFromPlaylist`, `scan(_:checkCache:)`, `setRoots`
(ends first run), `addRoots`, `removeRoot`, `renameSource(_:to:)`, `setStreamCache`,
`telegramSignIn/Code/Password/SignOut`, `findTelegramChats`, `send(_ command)` (raw).

Queries (async, off main): `tracks`, `count`, `summary(_ filter)`, `track`, `trackDetails`,
`coverArt`, `albums/artists/genres(_:search:sort:)`, `loadFacets`, `playlistRows`, `playlist(_:)`,
`playlistHolds`, `loadSources`, `missingTracks`, `playbackOptions`, `trackEntries`, `trackEntry`,
`listSummary`, `index(of:in:)`, `playlists(with:)`, `fieldValues`, `libraryRoots`, `sourceList`,
`sourceName`, `addSource`, `removeSource`, `setSources`, `scanStatus`, `outputChoices`,
`chosenOutputDevice`, `chooseOutputDevice`, `telegramSignInState`, `streamCacheState`; anything
else: `await store.query { core in … }`. Settings sections: `settingsSection(_:)`,
`setSettingsSection(_:_:)`, `settingField`, `setSettingField`.
Pure FFI functions are global: `tracksQuery`, `parseSort`, `sortString`, `parseFilter`,
`filterSpec`, `collectionFilter`, `queueLayout`, `queueMove`, `sourceFromLocation`,
`sourceLocation`, `normalizeWebAddress`, `webSource`, `defaultSourceName`, `telegramErrorText`,
`telegramChatKindKey`, `coreVersion`, `libraryQuery`.

Model helpers (`Core/Models.swift`): `Locator.key/location/fileURL/isRemote/defaultName`,
`TrackRow.artistAndAlbum`, `AlbumRow/ArtistRow/GenreRow .filter/.page`, `PlaylistRow.isSmart`,
`SourceRow.displayName/.page`, `Filter.everything`, `filter.searching(text)`, `Query.library`,
`LoopStatus.cycled/.symbol/.labelKey/.repeatIndex`.

### Navigation (`Core/Navigation.swift`, `Core/History.swift`, `Core/PageID.swift`)

`Navigation.shared`: `go(_ page)`, `back()`, `forward()`, `toggleQueue()`, `page`, `section`,
`argument`, `canGoBack/Forward`, `subpages`, `visited`, `isWide` / `nowPlayingPanelShown`, requests:
`requestNewPlaylist(with:)` (the shell shows the New playlist dialog), `addSourceRequest`
(counter; Sources page opens its kinds menu when it changes), `focusSearchRequest` (⌘F; Tracks),
`filterDrawer: FilterDrawerContext?` / `openFilterDrawer(_:)` (the shell lays the drawer over the
page area's right edge, min(520, page) wide; the page draws its own scrim), `queued(_:next:count:)`,
`upNextShows` (replaceable closure deciding whether the queued toast shows).
History is exactly Qt's (cap 100, pruned when playlists are deleted). Page ids only through
`PageID`: `.tracks/.albums/.artists/.genres/.sources/.queue/.settings`, `album(name:artist:)`,
`artist(_:)`, `genre(_:)`, `source(_ root)`, `playlist(_ id)`, `settings(_ section)`; decoding:
`albumKey(_:)`, `name(_:)`, `sourceRoot(_:among:)`, `playlistID(_:)`.

### Text and formats

- `Core/Tr.swift`: `tr("key", args…)`, `tr("key", fallback:, args…)`; `Tr.shared.languages`,
  `Tr.shared.systemChoiceLabel`, `Tr.shared.language`. Change the language with
  `UISettings.shared.locale = code` (nil = system); it switches live.
- `Core/Format.swift` (spec §14 exactly): `Format.number`, `count(n, oneKey, manyKey)`,
  `clock` (floored, player), `length` (rounded, track lists; "" when unknown), `size`,
  `daysSince`, `ago`, `audio(codec:sampleRate:bits:)`, `gain(album:track:)`, `duration`.

### Look

- `Core/Theme.swift`: `Theme.bg/side/panel/surface/field/input/raised/hover/selected/menu/
  menuHover/line/line2/border/track/shadow/text/textSoft/text2/text3/danger/dangerBg/dangerLine/
  dangerTitle/dangerText/success` (dynamic light/dark, spec §12.1), `Theme.accent/accentText/
  accentInk`, `Accent.presets` / `Accent.current`; `Motion.shared.reduced` (app OR system),
  `Motion.shared.travel`, `Motion.exit/fade/move/enter/quick`, `Motion.standard(_)/leaving(_)/
  landing(_)`. The app tint is the accent; the theme sets `NSApp.appearance`.
- `UISettings.shared` (`desktop.ui`, shared with the Qt app): `theme`, `locale`, `accent`,
  `compactRows`, `reduceMotion`, `hiddenColumns` (+ `isColumnVisible`, `setColumn`; names
  `number album genre year added plays lastPlayed format time`), `saveWindowSize`, `windowSize`,
  `checkUpdates`, `miniOnTop`, `showHiddenFolders`. Assigning saves.
- Components (`Views/Components/`): `CoverView(path:size:radius:placeholder:)` (ImageIO
  thumbnails, cached), `MosaicView`, `LogoView`, `IconButton(symbol:label:size:iconSize:tint:)`
  (tooltip = label), `PillButtonStyle(kind: .primary/.plain/.danger, small:)`, `PillOutline`
(a pill shape whose `strokeBorder` has no ticks), `MonoLogo(size:)`, `BarSlider`,
  `PlayPauseButton`, `PlayClock`, `EmptyStateView`, `SectionHeader`, `PressScaleStyle`.
  `Core/Artwork.swift`: `Artwork.shared.thumbnail(path:pixels:)`, `full(_:pixels:core:)`.
- Dialogs: `PromptDialog(title:message:text:placeholder:confirmTitle:danger:requiresName:extra:onConfirm:)`
  (`Views/Shell/PromptDialog.swift`) as a `.sheet`; `onConfirm` returns a problem string or nil.
- Toasts: `Toasts.shared.showQueued(text)`, `showRejected(message)`, `showFailure(title:message:)`.

## Windows and app (`App/`)

`Windows.shared.showMiniPlayer(activate:)`, `expand(activate:)`, `showMain(activate:)`,
`openMain()`, `miniShown`, `main`. The menu bar
is `App/Commands.swift`; Space / mouse buttons 4-5 / Dock click in `App/AppDelegate.swift`.
`Platform/`: `NowPlaying` (MPNowPlayingInfoCenter, remote commands), `Power`
(AppVisibilityChanged, ProcessInfo activity while playing), `OutputDevices` (device list
follows Core Audio).
