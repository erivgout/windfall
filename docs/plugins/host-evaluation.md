# Plugin host evaluation

Written 2026-10-07, at the start of phase 4. It answers one question from `WINDFALL_PLAN.md`: which Rust libraries can Windfall build its CLAP and VST3 hosting on, and where does it have to bind the official SDKs itself.

Everything below comes from reading the published source of each crate (the crates.io tarballs named by version), from the crates.io and GitHub APIs on the day of writing, and from running the result against real plugins. Where something was not checked, it says so.

## What Windfall needs from a host library

- A license compatible with GPL-3.0-or-later.
- Hosting, not plugin authoring: load a file, list what is in it, create an instance, activate it, process audio with events, read and write parameters, save and restore state, open the editor.
- An audio path that Windfall can keep free of allocation, locks and blocking in its own code. The plugin may still do those things. The engine rule is about what Windfall adds.
- A threading model that matches the engine. Processors are built and prepared away from the audio thread, handed over ready, and dropped away from it again.
- Windows, macOS and Linux.
- A way to put the plugin's editor in a native window.

## CLAP

### clack (clack-host, clack-extensions, clack-common, clack-plugin) 0.2.0

| | |
|---|---|
| License | MIT OR Apache-2.0 |
| Source | github.com/prokopyl/clack, 241 stars, 4 open issues, last push 2026-09-12 |
| Releases | 0.1.0 on 2026-05-03, 0.1.1 on 2026-07-29, 0.2.0 on 2026-09-12 |
| Sits on | `clap-sys` 0.5.0 (CLAP 1.2.2), `libloading` 0.8, `bitflags` |
| Size | clack-host 5,752 lines, clack-extensions 14,251 lines |

Safe wrappers over the CLAP C API, split by the three thread classes CLAP defines. `PluginInstance` holds the main-thread half and is not `Send`. `activate` returns a `StoppedPluginAudioProcessor`, which is `Send` and not `Sync`, so it can travel to the audio thread and nothing can call it from two threads at once. `deactivate` takes the processor back. That is the engine's own rule, expressed in types.

What it covers for a host:

- Loading a `.clap` file and running its entry (`PluginEntry::load`), including macOS bundles through `NSBundle`. Entries are cached per path and kept alive by every instance made from them.
- The plugin factory and descriptors: id, name, vendor, version, features.
- `init`, `activate`, `start_processing`, `process`, `stop_processing`, `reset`, `deactivate`, `destroy`, `on_main_thread`.
- Every event type of CLAP 1.2: notes, note expressions, parameter values and modulation, gesture begin and end, transport, MIDI, MIDI 2.
- The host side of 27 extensions. Windfall uses audio-ports, note-ports, params, state, latency, tail, gui, timer, log and thread-check.
- Host callbacks run inside `catch_unwind`, so a panic in Windfall's handlers cannot unwind into the plugin.

What the process path costs. `StartedPluginAudioProcessor::process` (`clack-host/src/process.rs:462`) builds one `clap_process` struct on the stack and calls the plugin. It does not allocate or lock. The two places where a careless host would allocate are its own choices:

- Events. `EventBuffer` is a growing `Vec`. A host can instead implement `InputEventBuffer` and `OutputEventBuffer` on a fixed array (`clack-common/src/events/io/implementation.rs:15,31`), and Windfall does.
- Audio buffers. `AudioPorts::with_input_buffers` reuses a `Vec` sized at activation. Windfall skips it and builds the `clap_audio_buffer` array itself through `InputAudioBuffers::from_raw_buffers`, because it needs things the safe builder cannot express: the same memory for input and output when a plugin allows in-place processing, and silent or scratch buffers for the extra ports of a plugin with a sidechain.

How editors are embedded. `PluginGui` wraps `is_api_supported`, `create`, `set_scale`, `get_size`, `can_resize`, `get_resize_hints`, `adjust_size`, `set_size`, `set_parent`, `set_transient`, `suggest_title`, `show`, `hide` and `destroy`. `Window::from_win32_hwnd`, `from_cocoa_nsview` and `from_x11_handle` carry the parent. Clack creates no window and runs no event loop. The host owns both.

What is missing or weak:

- No scanning, no cache, no process isolation. Those are the host's job in any case.
- `HostWrapper::handle_main_thread` (`clack-host/src/extensions/wrapper.rs:126`) trusts the plugin to call main-thread callbacks from the main thread. Its safety comment is a TODO. A plugin that calls `latency.changed()` from its audio thread would reach the main-thread handler from the wrong thread. Windfall's handlers therefore check the calling thread and otherwise only set atomic flags.
- The audio buffer module is undocumented (`#[allow(missing_docs)] // TODO: doc this`).
- The crates are five months old on crates.io. The repository is older and is the base of at least one other published host (`plughost-formats`, by its lockfile).
- A non-UTF-8 path is converted lossily before it is handed to the plugin's entry.

Platforms. Loading is `libloading` on all three, with bundle resolution on macOS. The GUI types name win32, cocoa, x11 and wayland.

### clap-sys 0.5.0

| | |
|---|---|
| License | MIT OR Apache-2.0 |
| Source | github.com/micahrj/clap-sys, 225 stars, last push 2026-07-20 |
| Releases | 0.1.0 in June 2022, 0.5.0 on 2025-01-03 |

A hand-maintained, line-for-line translation of the CLAP headers at version 1.2.2. No build script, no dependencies. Complete for the stable API. It is what clack is built on, so choosing clack does not rule it out: Windfall names `clap-sys` types where it builds raw buffers.

Using it alone means writing what clack already has: the host struct and its extension lookup, the lifetime rules between entry, instance and processor, panic guards on every callback, and the stream and event wrappers. That is about 6,000 lines of unsafe code to own for no gain in control, since clack lets the host replace the two parts that matter for realtime work.

Windfall's test plugins are written against `clap-sys` directly, on purpose. A host built on clack and tested only against plugins built on clack would share one reading of the ABI. The test plugins are a second, independent one.

### Newer crates

Read by source on 2026-10-07. None can carry a realtime host.

| Crate | License | Built on | Finding |
|---|---|---|---|
| truce-rack-clap 1.1.5 | MIT OR Apache-2.0 | `clap-sys` 0.5 | Passes a null `clap_host` to the plugin (`src/lib.rs:503`, marked TODO), which breaks the CLAP contract. Stereo hard-coded. Three `Vec` allocations per block. |
| maolan-plugin-host 0.0.8 | BSD-2-Clause | its own `#[repr(C)]` structs | A helper binary for one DAW's protocol. One heap allocation per event. Its buffer lock hands out `&mut T` from `&self` without locking (`src/util.rs:4-22`). No macOS. |
| plughost 0.1.0 | MIT OR Apache-2.0 | clack, in a sub-crate not read | Out of process only, and offline by its own README: each block is a blocking socket round trip. One week old. Worth reading for how it supervises scans. |
| plugin_host 0.1.0 | MIT | nothing | Every method is a stub. `process` returns `true` without calling a plugin. |

## VST3

### vst3 0.3.0 (coupler-rs)

| | |
|---|---|
| License | MIT OR Apache-2.0 |
| Source | github.com/coupler-rs/vst3-rs, 78 stars, 1 open issue, last push 2026-03-28 |
| Releases | 0.1.0 in August 2023, 0.2.0 on 2025-11-01, 0.3.0 on 2025-12-07 |
| Sits on | `com-scrape-types` 0.1.1, nothing else |

Bindings generated from the VST3 SDK headers. Since 0.3.0 the generated file (15,436 lines) ships in the crate, identical on all three platforms, so building needs no SDK checkout, no libclang and no C++ compiler. The SDK the bindings come from is MIT since version 3.8 (October 2025), which is what lets a crate publish them.

It gives every interface a vtable struct and a trait, plus `ComPtr`, `ComRef` and `ComWrapper` for holding COM objects and for implementing host-side interfaces in Rust. Everything a host needs is present as a binding: `IPluginFactory` 1 to 3, `IComponent`, `IAudioProcessor`, `IEditController`, `IConnectionPoint`, `IComponentHandler`, `IHostApplication`, `IBStream`, `IParameterChanges`, `IParamValueQueue`, `IEventList`, `IPlugView`, `IPlugFrame`, `IRunLoop`.

It is bindings and nothing more, and says so. No module loading, no bundle layout, no host objects, no safe process call. A host writes all of that. The thread rules are the SDK's: the controller and the view belong to the UI thread, `process` to the audio thread, and the host implements the parameter and event queues that cross between them. `ComWrapper::new` allocates, so a host must build its queues once at setup and reuse them. Nothing in the crate prevents that.

### vst3-sys (RustAudio)

Not on crates.io. The repository's `Cargo.toml` says GPL-3.0, written by hand from the SDK when the SDK was GPL or proprietary. Last push 2023-06-19. GPL-3.0 is compatible with Windfall, but the crate is unmaintained, unpublished and older than SDK 3.8. Not considered further.

### Newer crates

| Crate | License | Built on | Finding |
|---|---|---|---|
| vst3-host 0.9.0 | MIT | `vst3` 0.3 | The widest feature set of any crate here: scanning with a crash-isolated probe, parameters, gestures, state, editor on three platforms. Its audio path locks mutexes that the plugin's editor thread also takes (`src/internal/plugin_impl.rs:2120`, `com_implementations.rs:1770`), copies every buffer in and out, and logs. Its own docs say the path is not realtime-safe. The host owns the transport, with no way to set the song position per block. Good reference for COM thread affinity. |
| truce-rack-vst3 1.1.5 | MIT OR Apache-2.0 | `vst3` 0.3 | `initialize` with a null host context. No component handler. Parameter changes never reach the processor. Two COM allocations per block. |
| rack 0.4.8 | MIT OR Apache-2.0 | a C++ shim | The build script clones the Steinberg SDK from the network, unpinned, and compiles it with CMake. No component handler, no transport, no editor for VST3. |
| maolan-plugin-host 0.0.8 | BSD-2-Clause | `vst3` 0.3 | VST3 on Linux and FreeBSD only. Up to three COM allocations and five `Vec`s per block. |

### Binding the official SDK directly

The SDK is MIT, so Windfall could vendor `pluginterfaces` and compile Steinberg's C++ hosting helpers (`public.sdk/source/vst/hosting`) behind a C shim. That brings a C++ toolchain into every build of the workspace on three platforms, plus a shim to write and keep in step. The hosting helpers are convenience classes (module loading, a parameter-change container, an event list). They are a few hundred lines each, and the interfaces they implement are in the `vst3` crate already. Writing them in Rust is less work than binding them, and keeps the build pure Rust.

## Decision

CLAP is hosted on clack-host and clack-extensions 0.2.0, with `clap-sys` 0.5.0 named directly where raw buffers are built.

VST3 is hosted on the `vst3` 0.3.0 bindings, with the host objects written in Windfall. No other VST3 crate is used, and the C++ SDK is not built.

The reasons:

1. Clack's types state the engine's threading rule. The main-thread half cannot leave its thread, the audio half can be sent but not shared, and deactivation needs the audio half back. Windfall would have to write the same split over `clap-sys`.
2. Clack does not force an allocation on the audio path. The two growing containers it offers are optional, and Windfall replaces both with fixed ones. A test counts allocator calls around `process` and requires zero.
3. Every crate that wraps more than clack does has a blocking or allocating audio path, or leaves out part of the plugin contract. Each would have to be rewritten where it matters.
4. For VST3 there is no safe host crate that keeps the audio path clean, so the choice is between raw bindings in Rust and raw bindings in C++. The Rust ones need no C++ toolchain and are generated from the same headers.
5. All of it is MIT or Apache-2.0, which GPL-3.0-or-later can include.

Windfall keeps the binding out of its public API. `windfall-plugin-host` exposes its own types (`PluginInstance`, `PluginProcessor`, `PluginEffect`, `PluginInstrument`), so replacing clack, or moving a plugin into another process, changes one module.

### Risks of the decision

- Clack is young and has one maintainer. Windfall uses a narrow part of it and pins the version. If it stalls, the fallback is the same code over `clap-sys`, which is the layer below and changes only when CLAP does.
- `vst3` 0.3.0 has had no commit since March 2026. It is generated code from a stable ABI, so it ages slowly, but new SDK interfaces arrive only when someone regenerates it. `vst3-bindgen` 0.3.0 is published for that.

## Dependencies added

Every crate `windfall-plugin-host` adds to the workspace lock file, with the license from its manifest:

| Crate | Version | License | Used for |
|---|---|---|---|
| clack-host | 0.2.0 | MIT OR Apache-2.0 | CLAP hosting |
| clack-extensions | 0.2.0 | MIT OR Apache-2.0 | CLAP extensions |
| clack-common | 0.2.0 | MIT OR Apache-2.0 | shared by the two above |
| clap-sys | 0.5.0 | MIT OR Apache-2.0 | CLAP C types |
| libloading | 0.8.x | ISC | loading plugin files |
| bitflags | 2.x | MIT OR Apache-2.0 | used by clack |
| vst3 | 0.3.0 | MIT OR Apache-2.0 | VST3 interface bindings |
| com-scrape-types | 0.1.1 | MIT OR Apache-2.0 | COM pointers for `vst3` |
| rtrb | 0.4.x | MIT OR Apache-2.0 | lock-free queues between threads (already used by the engine) |
| windows-sys, windows-link | 0.61.x, 0.2.x | MIT OR Apache-2.0 | Win32 windows, Windows only |
| serde, serde_json | 1.x | MIT OR Apache-2.0 | scanner output and cache (already in the workspace) |

The test plugins add nothing beyond `clap-sys`, `vst3` and `windows-sys`.

The sections on real plugins and on what VST3 does today are at the end of this file. They are filled in from the runs made after the code was written.
