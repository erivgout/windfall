<img src="assets/icon.svg" width="64" height="64" alt="">

# Windfall

Windfall is a free, open-source digital audio workstation in the style of FL Studio. It is a native desktop app: the audio engine is Rust, the window is Tauri 2, and the interface is React with shadcn.

It is early. `WINDFALL_PLAN.md` describes the whole plan, and `docs/parity/PARITY.md` tracks every FL Studio feature and whether Windfall has it yet.

Windfall is not affiliated with Image-Line. It contains no FL Studio code, samples, presets or artwork.

Audio export supports WAV, FLAC, OGG and MP3, with mixer stems and cancellation. Desktop workflows include MIDI import/export, reviewed FL Studio project conversion with retained unsupported sound data, and independent playlist time-stretch and pitch controls.

The Windows app supports live MIDI input with sustain, an explicit audition channel and optional live-note output. Note recording, controller mapping and sequenced hardware output remain unfinished. See `docs/MIDI-HARDWARE.md`.

Selected audio clips can be trimmed, extracted, normalized, reversed, faded, silenced or cut in the native audio editor. Edits create a new WAV and one undo step. Reviewed grid/transient slicing creates clips linked to the original source; playable slice mapping remains unfinished. See `docs/AUDIO-EDITOR.md` and `docs/SLICER.md`.

The piano roll includes selected-note quantize, legato/staccato, grid chop, compatible glue, strum, time/pitch flip, pitch-range and velocity tools with atomic undo. Opt-in scale guidance and pitch snap accompany one-click chord/scale stamps; stamp cancellation repairs remain in progress. The transport includes reviewed tap tempo. Portamento, custom chopping patterns and further piano tools remain unfinished. See `docs/PIANO-TOOLS.md`, `docs/PIANO-SCALES.md` and `docs/TAP-TEMPO.md`.

The sample browser indexes bounded folder trees, searches paths with Boolean and wildcard queries, and stores local favorites and tags. Refreshed file imports use checked source tokens. Delayed-import and changed-source repair work is still in progress. See `docs/BROWSER-LIBRARY.md`.

Channel samplers support forward and ping-pong loops with editable points, note release, undo and saved project settings. Playback and export share the same loop processing. Independent sampler time-stretch remains unfinished. See `docs/SAMPLER-LOOPS.md`.

Mixer chains include balance, DC removal, channel mute, polarity, stereo matrix with channel delays, soft clipping and oversampled distortion. Matrix transition and latency-display repairs remain in progress. See `docs/UTILITY-EFFECTS.md`.

The Windows app hosts CLAP and VST3 instruments and effects with a plugin manager, parameter automation and saved state. Native state capture defers during recording, preserves current note/parameter intent during ownership exchange, and reports native lifecycle refusal. Repairs remain in progress for save-before-drain parameter capture, bundle binary validation and multi-channel panic expansion. Native VST3 editor windows and installed external plugins remain unverified. Native hosting on macOS/Linux and audio plugin crash containment remain unfinished. See `docs/plugins/desktop-integration.md` and `docs/plugins/vst3-desktop.md`.

Microphone/line recording writes a take to an ordinary audio clip with undo and project persistence. Input must match the output sample rate; monitoring, automatic latency alignment and hardware microphone verification remain unfinished. See `docs/RECORDING.md`.

Native file actions include portable ZIP projects with project, plugin-state and audio assets, plus collision-safe numbered saves. Archive parser validation and one numbered-save sample-root case are undergoing repair. See `docs/PORTABLE-PROJECTS.md`.

## Run it

Windows users can download the installer from [GitHub Releases](https://github.com/erivgout/windfall/releases). The first release is `v0.1.0-alpha.1`; this private repository and its downloads require repository access.

That installer remains fixed at its release tag. Subsequent development features described above require a source build.

You need Rust, Node 24 or newer, and pnpm. On Windows you also need the Visual Studio C++ build tools, and on Linux the WebKitGTK and ALSA development packages that the CI workflow installs.

```bash
# Windows, in Git Bash: load a Visual Studio environment that can link
source scripts/msvc-env.sh

cd apps/desktop
pnpm install
pnpm tauri dev
```

To work on the interface without building the engine, `pnpm dev` runs it in a browser against a simulated backend at http://localhost:1420.

## Layout

| Path | What it is |
|---|---|
| `crates/windfall-core` | Audio buffer and unit helpers shared by everything |
| `crates/windfall-project` | Project model, edit commands, undo history, `.windfall` file format |
| `crates/windfall-archive` | Portable ZIP schema, bounded validation and staged audio extraction |
| `crates/windfall-engine` | Realtime audio engine and offline renderer |
| `crates/windfall-flp` | Reads FL Studio projects and converts musical structure with an import report |
| `crates/windfall-codec` | Reads WAV, AIFF, FLAC, MP3 and OGG; writes WAV, FLAC, OGG and MP3 |
| `crates/windfall-midi` | Reads and writes MIDI files; project import plans and playlist export |
| `crates/windfall-dsp` | Effects and synth DSP |
| `crates/windfall-plugin-host` | CLAP/VST3 audio hosting, isolated scanning and Windows native editors |
| `crates/windfall-stretch` | Pure Rust streaming/offline time-stretch, pitch-shift and loop tempo helpers |
| `crates/windfall-ipc` | Types the engine, shell and interface exchange |
| `crates/windfall-factory` | Generates the factory sounds |
| `crates/windfall-sim` | The project document compiled to WebAssembly, which the simulated backend runs |
| `content/factory` | Factory sounds, CC0 |
| `apps/desktop` | The app: `src-tauri` is the shell, `src` is the interface |
| `apps/desktop/src/components/audio` | Knob, fader, meter, step grid and other audio controls, MIT licensed and installable as a shadcn registry |
| `docs` | Architecture, parity matrix, performance measurements |

`docs/ARCHITECTURE.md` explains how the parts talk to each other.

## Checks

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
pnpm --dir apps/desktop typecheck
pnpm --dir apps/desktop lint
pnpm --dir apps/desktop test
node scripts/parity.mjs --check
node scripts/check-sim.mjs
```

After changing a Rust type that the interface uses, run `scripts/gen-bindings.sh`.

The simulated backend edits projects with the real Rust code, built to WebAssembly and checked in. After changing `windfall-project`, `windfall-core` or `windfall-sim`, run `scripts/build-sim.sh` and commit the two files it writes. It needs `rustup target add wasm32-unknown-unknown` once.

## License

Windfall is licensed under the GNU General Public License, version 3 or later. See `LICENSE`.

Two parts carry their own, more permissive terms so other projects can reuse them: the audio control kit in `apps/desktop/src/components/audio` is MIT, and the factory sounds in `content/factory` are CC0.
