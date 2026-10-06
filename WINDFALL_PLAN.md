# Windfall plan

Windfall is a free, open-source alternative to FL Studio. It is a native desktop app with a UI built on shadcn. This document is the plan only. Nothing here has been built.

Written 2026-10-06.

## Decisions already made

| Decision | Choice |
|---|---|
| Name | Windfall |
| License | GPL-3.0 |
| Form | Native desktop app, not a web app |
| UI library | shadcn |

## What FL Studio is

FL Studio is a digital audio workstation, the program producers use to make a whole song on a computer. It has five main windows.

- **Channel rack.** A list of sounds, called channels. Each channel is a sample or an instrument. Each has a row of 16 buttons, and clicking them programs a drum pattern in seconds. This is what FL is famous for.
- **Piano roll.** A grid where you draw notes for one instrument. Up and down is pitch, left to right is time.
- **Playlist.** The song timeline. You make short patterns in the channel rack and piano roll, then lay them out here along with audio recordings and automation.
- **Mixer.** Every sound runs through a mixer track where you set volume and add effects such as reverb, EQ and compression. Each track has 10 effect slots.
- **Browser.** A side panel for finding samples, presets, plugins and projects.

On top of that, FL ships a large set of its own instruments and effects, and it loads third-party plugins in the VST, AU and CLAP formats. Most producers rely on third-party plugins, which is the main reason Windfall has to be native.

## What "100% parity" means here

A feature list this big needs a scoreboard. The plan uses a parity matrix, a table with one row per FL feature. Each row is marked done, in progress, or won't do with a reason. Windfall reaches 100% when no row is unaccounted for.

The size of the job, from FL's own edition comparison page for the All Plugins edition:

| Area | Count |
|---|---|
| Built-in instruments | 39 |
| Built-in effects | 71 |
| Visual and video plugins | 6 |
| Audio editors | 3 |
| Core features, such as audio recording, stem separation, automation, the AI assistant | about 23 |

The five main windows are the smaller job. The 110 instruments and effects are most of the work.

Three things can never be copied, whatever the engineering effort.

- **FL's sounds and presets.** Image-Line owns its bundled samples, loops and presets. Windfall ships its own, under CC0 or a similar license.
- **FL's plugin names.** Names such as Sytrus and Harmor are Image-Line's. Windfall plugins get their own names, and the docs carry an "FL equivalent" column.
- **Identical sound from any FL project file.** Windfall can read notes, clips, automation, routing and samples out of a `.flp` file. A project only sounds the same if every plugin in it has a Windfall equivalent or is a third-party plugin the user has installed.

## What native means when the UI is shadcn

shadcn is a web UI library. It builds interfaces out of React and Tailwind, which run in a browser engine. So a native app with a shadcn UI is built in two parts.

- **The engine is fully native.** Audio, plugins, files and MIDI run as compiled Rust code with direct access to the sound card.
- **The window is a native shell that draws the UI with a built-in browser view.** The user installs and runs it like any desktop program and never sees a browser.

The recommended shell is Tauri 2. It is written in Rust, so the engine links straight into it, and it uses the browser view the operating system already ships, so the installer stays small.

| Option | Good | Bad |
|---|---|---|
| Tauri 2, recommended | Rust engine links in directly. Small installer. | Linux uses WebKitGTK, which is slower and has known graphics-driver problems. |
| Electron | Same Chromium on every platform, so drawing is consistent. | Much larger installer. The Rust engine has to be attached as an add-on or a second process. |
| Fully native UI, no web view | Fastest drawing. This is what ArtCraft does. | Means dropping shadcn, which is a stated requirement. |

Going native removes most of the limits a browser version had.

| FL feature | Web app | Native app |
|---|---|---|
| Loading VST, AU and CLAP plugins | Impossible | Yes |
| Low-latency audio drivers such as ASIO | Impossible | Yes |
| Running Windfall as a plugin inside another DAW | Impossible | Possible later |
| MIDI hardware | Some browsers only | Yes |
| Direct access to the user's files and sample folders | Limited | Yes |

Two licensing facts make this workable under GPL-3.0. Steinberg released the VST3 SDK under the MIT license in October 2025, and it now offers the ASIO SDK under GPLv3.

## Architecture

### Audio engine, in Rust

- **Sound card access** through the `cpal` library. It covers WASAPI and ASIO on Windows, CoreAudio on macOS, and ALSA, JACK and PipeWire on Linux.
- **A dedicated audio thread** that never allocates memory, takes a lock, or waits on anything. The UI sends it edits over a lock-free queue and reads meters back through atomic values. This rule is what prevents clicks and dropouts.
- **An audio graph** that routes channels through mixer tracks, sends and effects, with plugin delay compensation and mixing spread across CPU cores.
- **Sample-accurate sequencing** for patterns, the playlist and automation.
- **Offline rendering** for export, using the same code path as playback so an export matches what was heard.

### Project model, in Rust

- One copy of the project is the source of truth. The UI and the audio thread both follow it.
- Every edit is a command. That gives linear undo and redo with a visible history, and it gives the command palette, scripting and the assistant one shared way to change a project.
- Windfall has its own open project format, a folder or zip of readable JSON plus audio files, with a version number for upgrades. Autosave and timestamped backups are built in.

### Plugins

- Windfall hosts CLAP and VST3 plugins on all platforms, and AU on macOS.
- Plugin scanning runs in a separate process, so one broken plugin cannot crash the app at startup.
- Plugin windows open as floating native windows, the same as in FL.
- Windfall's own instruments and effects use the same plugin interface internally. Built-in and third-party plugins then behave the same in the mixer and the channel rack.
- Rust host libraries exist for CLAP and VST3, but their maturity varies. Phase 4 starts by testing them against real plugins. The fallback is binding the official C and C++ SDKs directly.

### UI

- Tauri 2, React 19, Vite, Tailwind v4 and shadcn.
- shadcn components cover menus, dialogs, panels, the browser, settings, the command palette and the assistant.
- The piano roll, playlist, waveforms, automation curves and meters draw on a GPU canvas. Ordinary page elements cannot draw ten thousand notes smoothly. The canvas reads the same theme colors as the shadcn components, so the two match.
- The engine pushes playhead and meter values to the UI 60 times a second.
- Any panel can detach into its own native window for multi-monitor setups.

### Other parts

- **MIDI** in and out, controller mapping, and later scripting for hardware controllers.
- **File formats.** Read WAV, FLAC, MP3, OGG and MIDI. Export WAV, FLAC, MP3, OGG, MIDI and separate stems.
- **Machine-learning features** run natively. These are stem separation, denoising, pitch detection for vocal tuning, and audio-to-MIDI.

### Open-source code that GPL-3.0 lets Windfall build on

| Need | Candidate | License |
|---|---|---|
| FL project file parsing | PyFLP | GPL-3.0 |
| Synth engines for the instrument list | Surge XT, Vital, Dexed | GPL-3.0 |
| Time-stretch and pitch-shift | Signalsmith Stretch | MIT |
| Stem separation | Demucs | MIT |
| Denoising | RNNoise | BSD |
| Audio-to-MIDI | Basic Pitch | Apache-2.0 |

## shadcn setup

This reflects shadcn's changelog and repository as of 2026-10-06.

| Choice | Reason |
|---|---|
| CLI `shadcn@4.21.3` or later | Released 2026-10-06. Fixes a `pnpm dlx` crash on Windows. 4.21.2 fixed config files that PowerShell saved with a byte order mark. |
| Base UI as the component library | The default since July 2026. Some new components ship only for it. Radix and React Aria remain supported. |
| Mira style, with Rhea as the alternative | These are the two dense styles out of eight. A DAW needs density. `shadcn apply --preset` switches style later without redoing components. |
| The `cn` package | Since September 2026 it replaces `clsx` plus `tailwind-merge`, and its repo claims 30 times the speed. Windfall will have thousands of mixer and channel elements on screen. |
| Chat components, Questionnaire, `@shadcn/helpers` | Shipped June to August 2026. They cover the assistant panel, and the helpers let the chat UI be developed without an API key. |
| `shadcn/skills` and `shadcn docs <component>` | These make a coding agent write against current component APIs. |

The scaffold command is `pnpm dlx shadcn@latest init -t vite -b base -p mira --pointer`.

shadcn has no knob, fader, level meter, piano keyboard, step grid, envelope editor or timeline. Windfall builds these as its own components and publishes them as a shadcn registry in the GitHub repo, so other audio projects can install them with `npx shadcn add`. GitHub repositories have worked as registries since June 2026.

## Where the UI beats FL

- **Docked panels.** Resizable panels with saved layouts replace FL's floating, overlapping windows. Panels can still detach.
- **Command palette.** Every action is a named command with a visible shortcut, searchable from one box.
- **Automatic routing.** A new channel gets its own mixer track, and the link between them is visible. In FL this is manual, and beginners often get it wrong.
- **Consistent right-click.** A context menu opens everywhere and lists what can be done to the thing clicked.
- **Normal undo.** Undo is linear, with a history list.
- **One control kit.** Every built-in plugin uses the same knobs, faders and envelopes, so learning one teaches the rest.
- **Helpful empty states.** An empty project shows what to do first.
- **FL keymap.** An optional preset keeps FL shortcuts for people switching over.
- **Light and dark themes** from the start.

## Phases

Each phase ends with a check that can be run, so progress is measured and not guessed.

| Phase | What gets built | Done when |
|---|---|---|
| 0. Spike | Rust engine playing sound through the sound card. Tauri window with a shadcn UI controlling it. The parity matrix. Builds running on Windows, macOS and Linux. | A drum pattern plays for 10 minutes with zero dropouts at a small buffer size, and the UI shows live meters. |
| 1. Make a beat | Channel rack, step sequencer, sampler, sample browser with preview, transport, basic mixer, save and load, WAV export. | Someone new can build a drum loop, save it, reopen it and export it. |
| 2. Write a song | Piano roll. Playlist with pattern, audio and automation clips. Mixer routing, sends and effect slots. Core effects, which are EQ, compressor, limiter, reverb and delay. One subtractive synth. Undo history. Command palette. | A full song with melody, drums, automation and effects can be made and exported. |
| 3. Record and edit audio | Recording, time-stretch and pitch-shift, an audio editor, a slicer, MIDI in and out, controller mapping, export to MP3, FLAC, OGG, MIDI and stems. | A vocal can be recorded over a beat, edited, and exported as stems. |
| 4. Plugins and files | CLAP and VST3 hosting with crash protection, a modular plugin rack, FL project import, MIDI import. | A set of popular free plugins loads and plays, and a real FL project opens with its notes and arrangement intact. |
| 5. The long tail | The remaining built-in instruments and effects, ordered by how often producers use them. Stem separation, denoising, pitch correction, audio-to-MIDI. | The plugin rows in the parity matrix are filled in. |
| 6. Extras | Assistant, performance mode, scripting, visualizer, video player, phone remote, loop starter with its own content, optional cloud backup. | The remaining rows in the parity matrix are filled in. |
| 7. Release | Signed installers, auto-update, crash reporting, documentation, polish on macOS and Linux. | A stranger can download, install and update it on all three platforms. |

Phases 1 to 4 produce a usable DAW that can also load third-party plugins. Phases 5 and 6 hold most of the total work. Because the app is native, hosting third-party plugins in phase 4 takes pressure off phase 5, since users can fill gaps with free VSTs in the meantime.

## Risks

| Risk | Effect | Response |
|---|---|---|
| The web view cannot draw the editors fast enough | A laggy piano roll and playlist | Test with 10,000 notes in phase 0. Move drawing to a GPU canvas early. Fall back to Electron if the system web view is the limit. |
| Linux web view quality | Slow or broken drawing on some graphics drivers | Ship Windows and macOS first. Treat Linux as best effort until phase 7, or use Electron there. |
| Third-party plugins crash or misbehave | Lost work | Scan out of process. Offer running plugins in a separate process. Autosave often. |
| Scope | The plugin list is over 100 items | The parity matrix sets the order. Reuse GPL synth engines where they fit. |
| FL project format changes | Import breaks on new FL versions | Build on PyFLP's knowledge of the format. Keep a folder of test projects from several FL versions. |
| Legal exposure | Takedown risk | Follow the rules in the next section. |

## Legal rules

- No decompiling or disassembling FL Studio or its plugins. Behavior is learned from the public manual, from using the product, and from open-source code with a compatible license.
- No Image-Line samples, presets, artwork or plugin names in the repo or the installers.
- FL names appear only as plain references, such as "equivalent to X" in the docs.
- Every dependency's license is checked against GPL-3.0 before it is added.

## Development machine

| Tool | Status on this machine |
|---|---|
| Rust | Installed, version 1.99.0 |
| Node and pnpm | Installed |
| WebView2 | Installed |
| C++ build tools | Needs one fix, described below |

Rust picks the newest Visual Studio install, which is Visual Studio 2026 Community, and that install lacks the 64-bit C++ libraries. Linking fails as a result. There are two ways to fix it. One is to add the "Desktop development with C++" workload to Visual Studio 2026. The other is to build from the Visual Studio 2022 Build Tools command prompt, which has the full set.

## Decisions still open

1. **Shell.** Tauri 2 is recommended. Electron is the alternative if consistent drawing on Linux matters more than installer size.
2. **License for the UI component registry.** MIT is recommended, so other projects can reuse the knobs and faders. The app itself stays GPL-3.0.
3. **Platform order.** Windows first is recommended, then macOS, then Linux.
4. **Where the project lives.** A GitHub organization and repo name. `github.com/windfall` returned 404 on 2026-10-06, which means it may be available.

## Sources

- [shadcn/ui changelog](https://ui.shadcn.com/docs/changelog)
- [shadcn-ui/ui releases](https://github.com/shadcn-ui/ui/releases)
- [FL Studio edition comparison](https://www.image-line.com/fl-studio/compare-editions)
- [FL Studio 2026 release notes](https://forum.image-line.com/viewtopic.php?p=2065297)
- [Steinberg relicenses VST3 and ASIO](https://librearts.org/2025/11/steinberg-relicenses-vst3-and-asio/)
- [Tauri webview versions](https://v2.tauri.app/reference/webview-versions/)
- [ArtCraft apps](https://getartcraft.com/apps)
