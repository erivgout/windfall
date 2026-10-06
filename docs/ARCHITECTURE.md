# Windfall architecture

This is the working contract between the parts of Windfall. `WINDFALL_PLAN.md` says what gets built and why; this file says where code lives and how the parts talk. If code and this file disagree, fix one of them in the same change.

## Layout

```
Cargo.toml                 Rust workspace
crates/
  windfall-core/           AudioBuffer, PPQ, gain and pan helpers. No dependencies.
  windfall-project/        Project model, Command, Document (undo history), .windfall file format.
  windfall-ipc/            Runtime types the engine, shell and UI exchange (transport, meters, devices, browser, export).
  windfall-codec/          Decode WAV, FLAC, MP3, OGG. Encode WAV. Waveform overviews.
  windfall-engine/         Realtime audio engine, offline renderer, soak-test CLI.
  windfall-factory/        Generates the CC0 factory drum samples.
content/factory/           Factory content shipped with the app (generated, checked in).
apps/desktop/              The app.
  src-tauri/               Tauri 2 shell. Owns the Document and the Engine. Crate name windfall-desktop.
  src/                     React 19 + Tailwind v4 + shadcn UI.
    bindings/              TypeScript types generated from Rust. Never edit by hand.
    components/ui/         shadcn components.
    components/audio/      Windfall's audio control kit (knob, fader, meter, step button...). Published as a shadcn registry.
    features/              One folder per window or panel.
    lib/ipc/               The Backend interface, its Tauri implementation and an in-browser mock.
    lib/store/             UI state.
docs/                      This file, the parity matrix, user docs.
scripts/                   msvc-env.sh, gen-bindings.sh, parity.mjs.
```

Dependency direction: `core` ← `project` ← `ipc` ← `engine`; `core` ← `codec`; the shell depends on all of them. The engine never depends on the codec: it is handed decoded audio.

## Building

On Windows, run `source scripts/msvc-env.sh` in Git Bash before any `cargo` command. It loads a Visual Studio environment that has the x64 C++ libraries. Without it, linking can fail.

- `cargo test --workspace` runs the Rust tests.
- `cargo clippy --workspace --all-targets -- -D warnings` must pass.
- `scripts/gen-bindings.sh` regenerates `apps/desktop/src/bindings` after any change to a type that derives `TS`.
- `pnpm --dir apps/desktop dev` runs the UI in a browser against the mock backend.
- `pnpm --dir apps/desktop tauri dev` runs the real app.

## Time and units

- Musical time is in ticks, 960 per quarter note (`PPQ`). A step is a sixteenth note, 240 ticks (`TICKS_PER_STEP`).
- Gain is linear. 1.0 is 0 dB. Faders stop at 2.0 (`MAX_GAIN`).
- Pan runs from -1 (left) to 1 (right) and uses the balance law in `windfall_core::pan_gains`.
- Colors are `0xRRGGBB` integers.
- JSON is camelCase everywhere.

## Project model and commands (`windfall-project`)

`model.rs` defines `Project` and everything in it. `command.rs` defines `Command`. Read those two files; their doc comments are the specification of each field and command.

- Ids are `u32` newtypes allocated from `Project::next_id`. They are never reused. Id 0 is the master mixer track.
- The step sequencer and the piano roll edit the same notes. A lit step is a note at `step * 240` ticks with key 60 and length 240.
- Adding a channel also adds a mixer track for it and routes the channel there. This is the plan's automatic routing.

`Document` owns the one copy of the `Project` plus the undo history.

```rust
impl Document {
    pub fn new(project: Project) -> Self;
    pub fn project(&self) -> &Project;
    pub fn revision(&self) -> u64;
    /// Applies a command and records it for undo. Consecutive dispatches that
    /// carry the same non-None gesture id collapse into one undo step (a fader drag).
    pub fn dispatch(&mut self, command: Command, gesture: Option<u64>) -> Result<Applied, CommandError>;
    pub fn undo(&mut self) -> Option<Touched>;
    pub fn redo(&mut self) -> Option<Touched>;
    /// Undoes or redoes until `cursor` entries are applied.
    pub fn jump(&mut self, cursor: u32) -> Touched;
    pub fn history(&self) -> HistoryView;
    pub fn is_dirty(&self) -> bool;
    pub fn mark_saved(&mut self);
    /// Builds the patch for the UI and bumps the revision.
    pub fn patch(&mut self, touched: &Touched) -> ProjectPatch;
    pub fn snapshot(&self, path: Option<String>) -> DocumentSnapshot;
}
pub struct Applied { pub created: Vec<u32>, pub touched: Touched, pub label: String }
```

A failed command leaves the project exactly as it was. Undo restores every entity exactly, including its id, and redo restores it exactly again. The one thing undo does not roll back is `next_id`: an id never comes to mean something else for the life of a document, so caches and selections keyed by id stay valid.

Rules the document enforces beyond the type shapes: lanes are sorted by channel id, the master track has no sends, and a mixer that is already full routes a new channel to the master.

### File format

A project is one readable JSON file with the extension `.windfall`, pretty-printed, holding the `Project`. It lives in a project folder. Samples copied into the project sit beside it and are stored as `SamplePath::Project` paths relative to that folder. Backups are written to `Backup/<name> <timestamp>.windfall` in the same folder. Writes go to a temporary file first and are then renamed, so a crash cannot leave half a file.

## Engine (`windfall-engine`)

The audio thread never allocates, locks, or blocks. Everything it needs arrives ready-made.

- The control side compiles the `Project` plus a `SamplePool` (decoded `AudioBuffer`s by `SampleId`) into an immutable plan and sends it to the audio thread over a lock-free queue. The audio thread swaps plans at a buffer boundary and sends the old one back to be freed off-thread.
- Voice state, transport position and meters live on the audio thread. Meters and the playhead are published through atomics.
- `Processor` is the whole audio path with no device attached: `process(&mut [f32] interleaved stereo)`. The device callback calls it, the offline renderer calls it in a loop, and tests call it directly. Export therefore matches playback.
- The sampler resamples by ratio, so a sample at any rate plays in tune at any device rate.

```rust
pub struct SamplePool;                    // insert(SampleId, AudioBuffer), remove, get; cheap to clone
pub struct Engine;                        // Send + Sync; the cpal stream lives on a thread it owns
impl Engine {
    pub fn start(settings: &AudioSettings) -> Engine;   // never fails: with no device, status().running is false and error says why
    pub fn reconfigure(&self, settings: &AudioSettings);
    pub fn status(&self) -> EngineStatus;
    pub fn devices() -> Vec<AudioHost>;
    pub fn controller(&self) -> Controller;
}
#[derive(Clone)] pub struct Controller;   // Send + Sync
impl Controller {
    pub fn set_project(&self, project: &Project, pool: &SamplePool);
    pub fn play(&self); pub fn stop(&self); pub fn seek(&self, tick: f64);
    pub fn set_transport(&self, patch: TransportPatch);
    pub fn transport(&self) -> TransportState;
    pub fn note_on(&self, channel: ChannelId, key: u8, velocity: f32);
    pub fn note_off(&self, channel: ChannelId, key: u8);
    pub fn preview(&self, sample: AudioBuffer); pub fn stop_preview(&self);
    pub fn frame(&self) -> RealtimeFrame;  // resets the meter peaks it reads
}
pub fn render(project: &Project, pool: &SamplePool, options: &RenderOptions,
              progress: &mut dyn FnMut(f32) -> bool) -> AudioBuffer;
```

## IPC between the shell and the UI

The UI never touches Tauri directly. It calls the `Backend` interface in `src/lib/ipc`, which has a Tauri implementation and an in-browser mock. Types come from `src/bindings`.

Tauri commands. Arguments are camelCase. A failed call rejects with a plain string message.

| Command | Arguments | Returns |
|---|---|---|
| `document_snapshot` | | `DocumentSnapshot` |
| `dispatch` | `command: Command`, `gesture?: number` | `DispatchResult` |
| `undo`, `redo` | | `ProjectPatch \| null` |
| `history_jump` | `cursor: number` | `ProjectPatch` |
| `project_new` | | `DocumentSnapshot` |
| `project_open` | `path: string` | `DocumentSnapshot` |
| `project_save` | `path?: string` | `string`, the saved path. Rejects when there is no path yet. |
| `recent_projects` | | `string[]` |
| `transport_play`, `transport_stop`, `transport_toggle` | | `TransportState` |
| `transport_seek` | `tick: number` | |
| `transport_set` | `patch: TransportPatch` | `TransportState` |
| `transport_state` | | `TransportState` |
| `realtime_subscribe` | `channel: Channel<RealtimeFrame>` | Frames arrive about 60 times a second. |
| `engine_status` | | `EngineStatus` |
| `engine_devices` | | `AudioHost[]` |
| `engine_configure` | `settings: AudioSettings` | `EngineStatus` |
| `audition_note_on` | `channel: ChannelId`, `key: number`, `velocity: number` | |
| `audition_note_off` | `channel: ChannelId`, `key: number` | |
| `preview_play` | `path: string` | |
| `preview_stop` | | |
| `browser_roots` | | `BrowserRoot[]` |
| `browser_add_root`, `browser_remove_root` | `path: string` | `BrowserRoot[]` |
| `browser_list` | `path: string` | `BrowserEntry[]`, folders first, then by name |
| `sample_info` | `path: string` | `SampleInfo` |
| `sample_info_by_id` | `sample: SampleId` | `SampleInfo` of a sample in the project's pool |
| `add_channel_from_file` | `path: string`, `index?: number` | `DispatchResult`. One undo step: adds the sample to the pool and a channel that plays it. |
| `set_channel_sample_from_file` | `channel: ChannelId`, `path: string` | `DispatchResult` |
| `export_audio` | `options: ExportOptions` | Returns at once. Progress arrives as events. |

Events the shell emits to every window:

| Event | Payload | When |
|---|---|---|
| `project:patch` | `ProjectPatch` | After every edit, undo and redo, from any window. |
| `project:loaded` | `DocumentSnapshot` | After new and open. |
| `transport:state` | `TransportState` | When the transport changes. |
| `engine:status` | `EngineStatus` | When the audio device changes or fails. |
| `export:progress` | `ExportProgress` | While exporting. |
| `project:warnings` | `string[]` | After a project loads with problems, such as a missing sample file. |

The UI store applies a patch only when `patch.revision` is exactly one more than the revision it holds. A patch it has already applied is ignored. On a gap it calls `document_snapshot` and replaces its copy.

## UI rules

- Every user action is a named entry in the action registry: id, title, default shortcut, run function. Menus, the command palette, context menus and the keymap all read the registry. Nothing binds a key or builds a menu item any other way.
- Project state comes from the store, which mirrors the `Document`. Components never keep their own copy of project data. A control being dragged shows its local value and dispatches with a gesture id.
- Realtime values (playhead, meters) do not go through React state. Components subscribe to the realtime feed and draw to a canvas or set a style inside `requestAnimationFrame`.
- Canvas drawing reads colors from the same CSS variables the shadcn theme defines, so both themes work.
- Audio controls come from `components/audio`. Features do not build their own knobs.

## Legal rules that bind every change

- No decompiling or disassembling FL Studio or its plugins.
- No Image-Line samples, presets, artwork or plugin names in the repo.
- FL names appear only as plain references, such as "FL equivalent: X" in docs.
- Check every new dependency's license against GPL-3.0 before adding it. MIT, BSD, Apache-2.0, ISC, Zlib, MPL-2.0 and GPL-3.0-compatible licenses are fine.
