# n_player event-bus rewrite: implementation findings

## Memory investigation (reported "leak on rescan")

Setup: 1035 tracks / 9.6GB library, RSS sampled across repeated full rescans (90s interval). Cached-start baseline ~
450MB.

| variant                        | after scan 1 | scan 2 | scan 3+                  |
|--------------------------------|--------------|--------|--------------------------|
| as first implemented           | 1857MB       | 2538MB | 2706 → 2807MB (decaying) |
| `MALLOC_ARENA_MAX=2` (env var) | 963MB        | 1153MB | 1150MB plateau           |
| streaming `get_format`         | 1339MB       | 1642MB | 1786MB                   |

Conclusions so far:

- **Not a per-scan leak in the bus code**: growth decays toward a plateau rather than climbing linearly; the
  `MALLOC_ARENA_MAX=2` run pins most of the initial retention on glibc arena behavior. The metadata pass fans ~64
  concurrent blocking tasks (`num_cpus * 4`, same setting as the old loader) each allocating MB-sized buffers. Almost
  certainly predates the rewrite.
- Biggest single source was `get_format` reading whole audio files into RAM; streaming from `File` cut retention ~40%
  and also removes the whole-file-in-RAM cost during playback.
- `mallopt(M_ARENA_MAX, 2)` at startup was tried, worked (matches the env-var row; must run before tokio spawns
  workers), and was **reverted** — unsafe allocator tuning rejected as the fix.
- **Allocator swaps do not fix it**: both mimalloc and jemalloc (`tikv-jemallocator`, currently in the tree as the
  global allocator on non-Android) were tried and the user observed the same retention with each. Three allocators
  showing the same behavior shifts suspicion from allocator retention to **genuinely live memory** — something actually
  holding references after a scan (candidates: Slint model/texture side of the per-scan `VecModel` replacement + 1035
  `set_row_data` cover images, cover extraction buffers, or an accumulating reference in the scan/settings-save path).

## Open items

1. **Heap-profile the remaining retention** — jemalloc is already the global allocator, so run with
   `MALLOC_CONF=prof:true,prof_gdump:true` (or `prof_final:true`) and inspect what's live after two rescans. This
   replaces further allocator roulette.
2. Decide whether jemalloc stays as the shipped allocator or gets removed once the real holder is found.
3. Decide whether the n_audio streaming change stays (recommended) or gets reverted.
4. Consider lowering metadata-pass concurrency from `num_cpus * 4` regardless.
5. Manual UI smoke test of the unverified paths above.
6. Android build + fixups.
