<img src="assets/icon.svg" width="64" height="64" alt="">

# Windfall

Windfall is a free, open-source digital audio workstation in the style of FL Studio. It is a native desktop app: the audio engine is Rust, the window is Tauri 2, and the interface is React with shadcn.

It is early. `WINDFALL_PLAN.md` describes the whole plan, and `docs/parity/PARITY.md` tracks every FL Studio feature and whether Windfall has it yet.

Windfall is not affiliated with Image-Line. It contains no FL Studio code, samples, presets or artwork.

Audio export supports WAV, FLAC, OGG and MP3, with mixer stems and cancellation. Desktop workflows include MIDI import/export, reviewed FL Studio project conversion with retained unsupported sound data, and independent playlist time-stretch and pitch controls.

Channel samplers support forward and ping-pong loops with editable points, note release, undo and saved project settings. Playback and export share the same loop processing. Independent sampler time-stretch remains unfinished. See `docs/SAMPLER-LOOPS.md`.

The Windows app hosts CLAP instruments and effects with a plugin manager, parameter automation, native editors and saved state. Its VST3 backend processes audio and supports Windows editors, while desktop VST3 addition remains gated on active state capture. Native hosting on macOS/Linux and audio plugin crash containment remain unfinished. See `docs/plugins/desktop-integration.md`.

Microphone/line recording writes a take to an ordinary audio clip with undo and project persistence. Input must match the output sample rate; monitoring, automatic latency alignment and hardware microphone verification remain unfinished. See `docs/RECORDING.md`.

## Run it

Windows users can download the installer from [GitHub Releases](https://github.com/erivgout/windfall/releases). The first release is `v0.1.0-alpha.1`; this private repository and its downloads require repository access.

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
