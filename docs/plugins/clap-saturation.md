# CLAP saturation and retirement

## Immediate releases

The ordinary processor event limit remains 1024. Timed calls through
`PluginProcessor::push_event`, `note_on`, `note_off`, `all_notes_off`, and
`set_param` return `false` on invalid input or full admission; callers must
honor that result.

The instrument adapter has a separate guaranteed path for its frame-zero,
channel-zero releases. Its pending vector is allocated at activation for
1024 ordinary events plus 128 key releases and one global panic. Once the
ordinary limit is reached, no additional note-ons can enter. Repeated key
releases or panics coalesce only when an earlier frame-zero release already
covers them and no matching note-on intervenes. Events retain time order and
insertion order at the same frame. A note-on queued before panic is followed
by its release before the plugin renders that frame.

`PluginInstrument` clears held-note ownership after this guaranteed admission.
The engine can consequently clear hardware ownership after requesting panic:
hardware-only notes receive global panic, while independently held UI or
sequence keys receive no hardware release. This requires no engine trait,
rack ownership, or public host signature change.

CLAP native event storage also accounts for the reserved releases. CLAP note
dialect uses at most 1153 slots. MIDI dialect preallocates 32928 slots: each
of the 1024 ordinary events could expand into 32 panic controller messages,
followed by 128 reserved note-offs and a further 32-message panic. Translation
therefore cannot drop an admitted release, including after a panic-heavy
ordinary queue. These allocations happen during activation, not processing.

## Accepted parameter edits

The owner-to-audio queue remains bounded at 4096. Audio admission takes only
as many edits as the ordinary pending list can hold and leaves the rest in
that queue. This preserves edits already accepted by `PluginInstance`, also
across zero-frame callbacks and saturation by notes or reserved releases.
Each callback admits at most 1024 main-thread edits; a full burst takes several
callbacks rather than silently dropping its final value.

Retirement collects pending and queued parameters on the owner thread,
preserving their existing time order and stable order at equal times. CLAP
flushes that list in ordered chunks of at most 1024 into its fixed parameter
event list. An empty flush still runs once for plugin-requested notifications.
Notes from the abandoned audio timeline are omitted as before. Saving state
after deactivation includes the final accepted edit, and reactivation restores
that value. Collection may allocate on the owner thread; callback admission,
release handling, translation, and processing do not allocate or free memory.

## Regression evidence

The tests use the repository's compiled native CLAP gain, sine, and MIDI-only
sine fixtures, with no installed third-party plugin or audio device.

Before the fixes:

- The desktop zero-frame boundary test queued 1024 hardware note-ons, then
  requested panic. Audio failed with `queued hardware notes sounded after panic`.
- The desktop mixed-owner test filled the queue while a UI key and hardware
  keys were held. After hardware panic and eventual UI release, audio failed
  with `hardware key survived panic after UI release`.
- Native adapter panic and individual release tests failed with queued notes
  sounding after panic and a released key still sounding.
- Retirement accepted 4095 gain edits at 0.25 and a final edit at 0.75, plus
  pending audio-side edits. The saved parameter was `Some(0.25)` instead of
  `Some(0.75)`.
- A 4096-edit main-thread burst followed by a zero-frame callback and four
  audio blocks still rendered gain 0.25 instead of the accepted final 0.75.

After the fixes, regression coverage includes both note dialects, hardware
panic without other owners, preservation of a UI key, individual releases,
repeated release storms across all 128 keys, ordered retirement of 5120 edits,
save/load/reactivate state, and bounded callback admission of a full edit
burst. Native realtime tests count allocations, reallocations, and frees and
require zero calls through every saturation/release callback.

Run from this worktree in Git Bash with an isolated build and binding folder:

```bash
source scripts/msvc-env.sh
export CARGO_TARGET_DIR="$PWD/target"
export TS_RS_EXPORT_DIR="$PWD/target/ts-rs-clap-saturation"
export CARGO_BUILD_JOBS=1
cargo test -p windfall-plugin-host --test processing --test params_state --test realtime
cargo test -p windfall-engine
cargo test -p windfall-desktop --lib midi_hardware -- --nocapture
cargo test -p windfall-desktop --lib session::tests::plugins -- --nocapture
cargo fmt --all -- --check
cargo clippy -p windfall-plugin-host -p windfall-engine -p windfall-desktop --all-targets -- -D warnings
```

Windows verification on 2026-10-07 passed: 43 host processing/state/realtime
tests, the full engine unit/integration suite, eight desktop MIDI tests, and
two desktop native plugin save/reopen/export/ownership tests. Formatting and
strict clippy for all targets of the three affected packages passed. The build
directory was created from scratch in this isolated worktree; final native
regression, clippy, and desktop plugin checks ran serially with one Cargo job.

The only shared integration changes are processor pending storage/admission
and the adapter's release calls. The retirement flush change stays inside the
CLAP backend. VST3 enablement, ownership exchange, dirty-state handling,
deactivation results, and native lifetime changes remain with their worker.
The tiny CLAP processor constructor change supplies its note dialect to size
the fixed event list. No generated bindings, WASM, root parity documentation,
release files, or fixture plugin behavior change is needed.
