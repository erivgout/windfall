# Browser library

The browser searches Factory and every configured sample folder recursively, including folders that have never been opened in the tree. Adding and removing folders, normal folder browsing, keyboard selection, automatic audition, waveform preview, double-click/Enter import, context actions and dragging to the rack or playlist remain available while indexing.

## Searching

Type in **Filter the browser**. Results show the filename and its path relative to the configured folder; hover a result to see its full path. Clear the query, Starred and tag filters to return to the folder tree.

| Query | Meaning |
| --- | --- |
| `kick` | A filename or full path containing `kick` |
| `drums/k?cks *.wav` | Both path terms match; `?` consumes one character |
| `kick* AND NOT tight` | Contains a wildcard match and excludes `tight` |
| `kick OR snare` | Either term matches |
| `(kick OR snare) NOT tight` | Grouped alternatives, excluding `tight` |
| `"vocal chop"` | One term containing a space |

Terms match substrings, including the configured folder's path. `*` consumes zero or more characters, including directory separators. `/` and `\` are equivalent. Matching uses Unicode lowercase on every platform; it does not perform accent removal or locale-specific case folding. Operators are case-insensitive. `NOT` binds before `AND`, which binds before `OR`; adjacent terms mean `AND`. Quote words such as `"OR"` to search them as text. Wildcards still work inside quotes; literal wildcard escaping is not supported. Invalid syntax reports an actionable error instead of silently changing the query.

## Favorites and tags

Select a file, then use **Star** to add or remove its favorite mark. **Starred** switches the results to favorite files across all configured folders. It combines with the query and tag filter.

Enter comma-separated labels in **Tags**, then press **Save tags** or Enter. Star also saves the current labels. Labels are trimmed, lowercased, deduplicated and sorted. Each file allows 16 labels of up to 32 Unicode characters, without internal control characters. The tag selector filters by one label; the backend supports intersecting several labels. **All tags** clears that filter.

These annotations are local to the user. They do not modify project history, `.windfall` files or audio/MIDI settings. Native metadata is stored atomically in `browser-library.json` alongside the application's settings file. Empty annotations are removed. Missing metadata defaults to no stars or tags. A corrupt, oversized or newer-version file is preserved and reported with instructions to repair or rename it before saving. A failed write leaves the previous annotations intact.

Annotations use cleaned absolute file paths (case-insensitive keys on Windows, filesystem case on other platforms). They survive application restart and temporary root removal. Renaming or moving a file does not migrate its annotations, and files outside current roots do not appear in Starred or tag results. There is no project-level tag exchange or automatic metadata cleanup.

## Refresh, cancellation and unavailable folders

Indexing starts at application startup and whenever a folder is added or removed. Results arrive incrementally. **Cancel** keeps the batches already indexed; **Refresh** starts a new scan. There is no filesystem watcher: refresh after changing files on disk. Refresh invalidates previous results, so select the refreshed file again before importing it.

Tree selections retain their own root/selection epoch even before the first search reply. Refresh and root changes invalidate that epoch before their IPC request yields and again when the new roots/generation arrive. A late audition, facts or import token lookup cannot attach to an expired selection. Changing the query, stars or tag filters does not invalidate a file selection. Successful audition/facts lookup pins the selection's file token independently of search results, so selected files can still be dragged while search is pending.

With overlapping folders, a tree row can belong to a nested root while the backend's checked token belongs to the first containing root. Audition, facts and imports accept that canonical token when both configured roots contain the same file. The row retains its captured root and epoch; accepting the token cannot revive a selection invalidated by refresh or removal/re-addition of the nested root.

Rack, playlist and replacement actions retain the project generation from action entry through token lookup, playlist routing lookup and import replies. Replacing the project through New/Open prevents late lookups from starting an import in the replacement project and late replies from selecting/revealing its objects, even when channel/clip IDs are reused. Native document-generation and recording guards remain in force.

The rack's sample-drop handler also checks the project generation before applying its backend reply or selecting the created channel. A successful reply from before New/Open cannot select a channel with a reused ID in the replacement project.

Missing drives, unreadable folders and non-folder paths report errors with the affected path and instructions to check access, reconnect the drive, refresh or remove the root. Other roots and normal browsing/audition remain usable. An index-limit notice means the scan is partial; select smaller folders and refresh. A result-limit notice means there are more matches; narrow the query or use a tag.

Symbolic links and Windows junctions/reparse points are skipped rather than traversed. Add the original folder when its contents are needed. Dot-prefixed entries, Windows hidden/system entries and non-Unicode filenames are skipped. Cyclic links cannot expand the scan. Overlapping roots deduplicate file paths.

## Bounds and implementation

| Resource | Limit |
| --- | --- |
| Configured roots | 128, including Factory |
| Indexed files per scan | 50,000 |
| Examined directory entries per scan | 100,000 |
| Path length | 4,096 UTF-8 bytes |
| Retained path text per scan | 16 MiB (files and visited directories) |
| Nested directory depth | 64 |
| Scan time | 30 seconds, checked between filesystem operations |
| Reported filesystem issues | 32 per scan |
| Returned matches | 500 per query |
| Query | 512 UTF-8 bytes, 64 tokens, 16 nested parentheses/NOT levels |
| Search time | 2 seconds, checked every 128 candidates |
| Listed tag choices | 256; an active filter remains selectable |
| Annotated files / metadata file | 10,000 / 8 MiB |

The native `library` module has one background worker and one pending scan request. A new request cancels the previous generation and replaces pending work. Batches publish every 128 examined entries. Search takes a snapshot, then matches/sorts off index, document and audio locks. Filesystem operations are outside State/audio locks; metadata has its own owner and file, separate from device settings. Time limits and cancellation cannot interrupt an individual blocked OS filesystem call. Published batches and late search replies are generation-checked; removing a root clears old results immediately. Refresh reads roots and invalidates them in one critical section, preventing a concurrent removal from being undone by an old refresh snapshot.

Native results carry a root generation and file fingerprint. Audition, waveform reads and the three audio import paths check root membership, existence, file type, ancestors within the configured root for reparse links, OS file identity, size and timestamps through the existing native loader before and after loading. Manual tree selection/drag carries the root generation and pins file identity at operation start. The decode cache also keys on file identity. A removed root or changed/deleted/replaced file refuses the operation with refresh/reselection guidance, including audio previously cached for preview. This is a filesystem identity check, not a content hash: an in-place rewrite that preserves identity, size and timestamps may remain indistinguishable. The final in-memory import acquires recording exclusion, then the library guard, then State; existing document-generation checks, sample loading, undo and recording rules remain in force. No filesystem work was added to the audio callback.

The query parser and label normalization live in `windfall-ipc` and are exported by `windfall-sim`, so the browser mock uses the same Rust behavior. The mock recursively searches fictional fixture folders and persists its roots and annotations in separate browser-storage keys. It displays its filesystem limitation directly in the panel. Fixture scans finish synchronously, so native incremental traversal, permissions, cancellation, junctions, disconnected drives and file identity changes require the desktop app. No real user paths or proprietary samples are part of these fixtures.

Checked imports also compare the decoded file version with any audio already loaded for that project sample. An unchanged file reuses its sample, including after decoded-cache eviction. A changed or unverified loaded version is refused before creating channels, clips, mixer tracks or history; the old audible source stays intact. To intentionally use the changed sound, import it under a new path or reopen the project to reload its linked sources. Refresh/reselection alone updates library identity, not audio already held by the project. Project files reference their external sources; reopening can therefore load a newer on-disk version. Source-version records weakly reference live decoded allocations and survive cache eviction without retaining additional audio. This comparison performs no filesystem/decode work under State or audio locks.

All file import routes, including ordinary file picking and checked browser rack/playlist/replacement, build a private candidate with the exact decoded or already-held source handles. Sampler banks and the plan are prepared before musical/source commit; a budget, capability or preparation failure leaves document, dirty state, history/redo, source pool and audible plan intact. Unchanged source reuse preserves its original audio allocation and stored asset path, including when saving makes an External path eligible for Project-relative storage. Ordinary file imports now use the same loaded-version and pending-load refusal rules, preventing new clip geometry from being paired with an older held source.

After preparation, browser imports repeat file/root identity checks and ordinary imports repeat the cache's file-version check off State/audio locks. Final recording exclusion, library generation, document generation/edit/replacement, project-folder, original source-map and pending-load checks precede one atomic prepared publication. A successful source replacement is one undo edit and playback/export use the same prepared source. An identical replacement preserves history/redo and does not attach a replacement buffer. Recording finalization retains its existing recording exclusion through clip attachment; its private route does not reacquire that lock.

Reuse is also refused while that project sample has an outstanding load, including missing-source reload and undo/redo after cache eviction. The pending load may already hold an older decoded version, so a checked import cannot install newer audio that the loader would overwrite later. The refusal leaves history, clip geometry and audible sources unchanged. Wait for loading to finish and retry; a subsequently detected version mismatch still requires a new path or explicit reopen. Decoding stays outside State/audio locks and the recording → library → State lock order is unchanged.

## Verification and parity

Native tests use temporary trees for nested/case/wildcard/Boolean search, bounded scans and results, cancelled/superseded generations, missing/non-folder roots, a Windows junction cycle, metadata roundtrip and corrupt/write-failure preservation. Session tests cover checked audition/facts, rack/playlist/replacement import, undo, removal during decode, deleted cached files and project replacement. The cache has a same-size/same-timestamp replacement regression. Shared Rust and simulator ABI tests cover parsing and normalization; mock/UI tests cover persistence, tag/favorite intersection, late replies, stale roots and selection/audition/import regressions. Platform-specific test code is not a claim that other platforms or physical audio hardware were verified.

Real-file sampler import regressions cover reduced-budget transactional refusal, unchanged source/history/redo/audio on failure, one-edit successful replacement, bit-identical playback/export, no-op preservation and undo/redo/save/reopen. An off-lock preparation barrier checks all ordinary/checked destinations against concurrent file, project, source/loading, root, folder and recording changes. The original real pending reload and cache-evicted redo regressions remain covered. Exact commands, dependency commits and scoped results are in `BROWSER-REPAIRS.md`; parent artifact/parity integration remains separate.

Repair regressions additionally hold native reload/redo at `samples:decoded` across a 480-to-960-frame file rewrite at every import destination. UI regressions render the actual rack drop handler across New/Open with reused channel IDs and select nested-root files while search is pending, including refresh/removal invalidation. Exact RED/GREEN commands and scoped results are recorded in [BROWSER-REPAIRS.md](BROWSER-REPAIRS.md).

This workflow fulfills the behavioral scope of `win-browser-search`, `win-browser-starred` (the Starred filtered view) and `win-browser-tags`. Root parity status updates and generated TypeScript/WASM integration belong to the parent task.
