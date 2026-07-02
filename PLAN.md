# n_player event-bus rewrite: implementation findings

The rewrite described by the original plan is implemented on `experimentation/event-bus`. This file records what landed, where it deviates from the plan, what was verified, and the memory investigation results.

## What landed

- **`n_event_bus`** workspace crate: `Message`/`Tagged<P>`/`Envelope`, `Subscriber` marker trait, `Bus` (`TypeId`-indexed), `Registrar<S: Subscriber>`, `Handle<T>`/`Thunk`, `Outbox`, `Event`/`EventWriter`/`spawn_ticker`, `Job`/`JobControl`/`JobToken`/`JobHandle`/`RunningJob` + `job_emits!`, `Scene` (`on_mount`/`sync`), `App` (`register_subscriber`/`register_scene`/`dispatch_all`/`flush_ui`/`run_loop`), `UiPatch`/`UiThread`. No Slint dependency. 5 unit tests cover non-Scene dispatch, outbox chaining, tag staleness, scene mount/sync/flush, and superseded-job dropping.
- **`n_player`**: `messages.rs` (catalogue as planned, plus `Pause`/`Play` for MPRIS and `QueueReplaced`), `playback.rs` (`PlaybackEngine` + `LoadTrackJob`), `scenes/app_scene/{mod,playback_mirror,library}.rs`, `scenes/settings_scene.rs`, `bridges/mpris.rs`, `bridges/android.rs` (+ push-based `AndroidEventJob`), `jobs/scan.rs`, `services/image.rs`. Deleted: `runner.rs`, `bus_server/*`, old `run_app`/`setup_data`/`updater_task`/`loader`/`loader_task`. `Platform` trimmed to the 5 planned methods (plus cfg(android) `jni_handles()`); net −1415 lines of old code.

## Deviations from the plan

- `Scene::sync` takes `&mut self`, not `&self` — scenes drain change buffers/dirty flags during sync.
- `PlaybackEngine` keeps its own index/loop-status bookkeeping: `QueuePlayer`'s index is private and its async `play()` is bypassed by `LoadTrackJob`; `QueuePlayer` serves as queue storage + `Deref`-to-`Player`. `Player::play(format)` needs an explicit `deref_mut()` (QueuePlayer's own async `play()` shadows it).
- `Settings` persistence methods take `internal_dir: PathBuf` instead of `Deref<Target = impl Platform>` — that bound can't accept `Arc<dyn Platform>`.
- `AppScene` re-emits tag-validated `Tagged<TracksEnumerated>` as untagged `QueueReplaced` so `PlaybackEngine` updates its queue without owning the scan's `RunningJob`.
- Immediately-ready async calls in n_audio (flume sends) are invoked from sync handlers via `pollster::block_on`.
- `LoopStatusChanged` is not mirrored by `AppScene` (no `AppData` property for it); only the bridges consume it.
- `run_loop` batches: after receiving one event it drains everything already pending, then does one `dispatch_all` + one `flush_ui` per burst. Without this, a scan queues one UI patch (and one full track-list filter pass) per metadata message — the old `updater_task` batched per 250ms tick.
- **n_audio touched after all** (plan said untouched): `MusicTrack::get_format` streams from `File` instead of `fs::read`-ing the entire track into a `Cursor`. See memory findings. Revertible independently.

## Verified

- `cargo test -p n_event_bus` (5/5), `cargo build --workspace` clean.
- Live via `playerctl` (MPRIS): next → track loads/plays with metadata + cover, position advances, absolute seek, volume round-trip, pause. Exercises D-Bus adapter → bus → PlaybackEngine → LoadTrackJob → playback → Tick diff → `*Changed` → MprisBridge.
- Fresh (non-cached) scan of a 1035-track / 9.6GB library, including repeated rescans; cache save/restore across restarts works (`is cached: true` on restart).

## Not verified

- Search filtering + viewport save/restore, theme/locale/path-change UI, window-size persistence on exit (need hands on the UI).
- Android: the cfg'd code has never been compiled (no NDK target on this machine); expect a fixup round.

## Memory investigation (reported "leak on rescan")

Setup: 1035 tracks / 9.6GB library, RSS sampled across repeated full rescans (90s interval). Cached-start baseline ~450MB.

| variant | after scan 1 | scan 2 | scan 3+ |
|---|---|---|---|
| as first implemented | 1857MB | 2538MB | 2706 → 2807MB (decaying) |
| `MALLOC_ARENA_MAX=2` (env var) | 963MB | 1153MB | 1150MB plateau |
| streaming `get_format` | 1339MB | 1642MB | 1786MB |

Conclusions so far:

- **Not a per-scan leak in the bus code**: growth decays toward a plateau rather than climbing linearly; the `MALLOC_ARENA_MAX=2` run pins most of the initial retention on glibc arena behavior. The metadata pass fans ~64 concurrent blocking tasks (`num_cpus * 4`, same setting as the old loader) each allocating MB-sized buffers. Almost certainly predates the rewrite.
- Biggest single source was `get_format` reading whole audio files into RAM; streaming from `File` cut retention ~40% and also removes the whole-file-in-RAM cost during playback.
- `mallopt(M_ARENA_MAX, 2)` at startup was tried, worked (matches the env-var row; must run before tokio spawns workers), and was **reverted** — unsafe allocator tuning rejected as the fix.
- **Allocator swaps do not fix it**: both mimalloc and jemalloc (`tikv-jemallocator`, currently in the tree as the global allocator on non-Android) were tried and the user observed the same retention with each. Three allocators showing the same behavior shifts suspicion from allocator retention to **genuinely live memory** — something actually holding references after a scan (candidates: Slint model/texture side of the per-scan `VecModel` replacement + 1035 `set_row_data` cover images, cover extraction buffers, or an accumulating reference in the scan/settings-save path).

## Open items

1. **Heap-profile the remaining retention** — jemalloc is already the global allocator, so run with `MALLOC_CONF=prof:true,prof_gdump:true` (or `prof_final:true`) and inspect what's live after two rescans. This replaces further allocator roulette.
2. Decide whether jemalloc stays as the shipped allocator or gets removed once the real holder is found.
3. Decide whether the n_audio streaming change stays (recommended) or gets reverted.
4. Consider lowering metadata-pass concurrency from `num_cpus * 4` regardless.
5. Manual UI smoke test of the unverified paths above.
6. Android build + fixups.
