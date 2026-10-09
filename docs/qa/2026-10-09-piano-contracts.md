# Randomizer and pattern timeline contract audit

> Final execution reconciliation: the combined tree passed the complete native workspace, strict checks, fresh artifact checks and final affected frontend checks. See [completion acceptance](2026-10-09-completion.md) for exact results and promoted IDs. Earlier pending statements below describe intermediate states; explicit remaining feature gates still apply.

The current implementation contains the complete native operations described
by `win-piano-randomize`, `win-piano-time-markers` and
`core-time-signature-changes`. The earlier source-pass documents' blanket
“QA/artifacts deferred” statements are stale after this QA campaign. This
worker added only new native integration test files, without frontend changes
during the source freeze and without running Cargo. Native execution belongs
to `native_resume`; passing evidence is pending until that worker reports it.

## Added public acceptance targets

- `cargo test -p windfall-project --test randomizer_qa`: three tests exercise
  serialized `TransformNotes` commands through the public `Document`. Fixed
  seeds reproduce exact projects; distinct seeds change results. Scalar note
  expression, pan and color/articulation survive. Humanization preserves IDs,
  unrelated notes and bounded changes. Generation exercises a partial last
  cell, chord/key/velocity bounds, fresh unique IDs, inherited expression,
  zero-density rests, one undo step, exact redo IDs, persistence and refusal
  without history/ID mutation for stale, invalid, empty-key-map and oversized
  generation requests.
- `cargo test -p windfall-midi --test pattern_timeline_qa`: one document-to-SMF
  test applies a captured pattern signature, unaligned local meter and named
  marker, alongside a different song map. It verifies exact distinct song and
  pattern metadata through actual write/read bytes; unchanged complete note
  tracks and durations; unchanged stored notes/clips; one-step meter undo and
  exact redo; file round-trip; stale captured editor refusal; and removal that
  leaves the song map untouched.
- `cargo test -p windfall-engine --test pattern_timeline_qa`: one public
  Processor/offline-render test uses real impulse samples before and after
  adding independent pattern/song maps. At 48 kHz/120 bpm, native pattern and
  song onsets remain exactly 25 frames per absolute tick. Small and large
  callback blocks, complete offline PCM, total render durations, notes and
  clips must agree with the meter-free reference.

All three files pass standalone rustfmt. No execution success is claimed yet.

## Existing evidence and exact remaining acceptance

Existing song-meter commands/conversion/persistence are covered by
`crates/windfall-project/tests/timeline/{commands,meter,identities}.rs`.
`crates/windfall-midi/tests/timeline.rs` already writes/reads/imports a song
4/4 → 7/8 → 3/4 map. Native hosted-transport metadata already has
`timeline_hosted_transport_uses_song_meters_only_in_song_mode` in
`crates/windfall-engine/tests/engine/timeline.rs` (the engine integration target).
The new targets qualify independent pattern metadata and unchanged audible
onsets, rather than inferring those behaviors from provisional bindings.

Frontend registration/source inspection confirms randomize/generate tools in
`piano-roll/note-tools.ts`, palette registration via `actions.ts`, menu exposure
in `menu.ts`, and controls in `note-tools-dialog.tsx`. Captured pattern edits
are exposed through `pianoRoll.patternTimeline`, `pattern-timeline.tsx` and its
dialog. Generic captured selected-note review/stale/cancel behavior passes
the existing 15-test `note-tools.test.tsx` target; musical/custom-grid invalid
fields were verified in this campaign. Actual ruler/note-layer rendering and
end-marker input pass the eight-case `layer-scaling.test.tsx` target.

The root subsequently authorized the missing QA-only mounted acceptance file,
`piano-roll/piano-contracts.qa.test.tsx`. It mounts the actual dialogs over the
current shared Rust WASM document harness. Six tests verify selected seed and
variation/chord-map controls dispatch exact transforms and update real notes;
unchanged unrelated IDs, complete expression, inherited pan, new generated IDs,
partial final cells, one-entry history, undo/redo and project save/open. Both
tools reject invalid seeds and stale reviews and cancel without mutation.
Pattern metadata tests apply base signature/meter/marker CRUD, inspect complete
captured command state, preserve notes and the song map, verify history/save,
and inject a real concurrent native edit so the old captured map refuses without
overwriting it. Invalid fields, Close and project replacement are also covered.

`bun run test src/features/piano-roll/piano-contracts.qa.test.tsx --maxWorkers=1`:
**6 tests passed**, 6.77 seconds, default timeout. Evidence:
`2026-10-09-piano-contracts-ui-final.log`. ESLint on the new file exits 0.
`bunx tsc -p tsconfig.app.json --noEmit` also exits 0 after the final test edits.
The mounted-dialog gap is closed; the full frontend owner tracks whether these
six cases are included in the final complete run or supplemental to its count.

Physical plugin/device acceptance, latency preparation, musical recording gates,
and other broader source-pass follow-ups are separate from the three parity
summaries. None is needed to demonstrate that random notes/property edits,
pattern labels/meters, or chosen song/pattern meter changes exist. These tests
also do not claim continuous hosted expression, MPE/MIDI hardware pitch support,
FLP conversion acceptance, or visual metronome behavior.

Recommendation: accept all three rows' software contracts after the native owner
reports all three targets passing and final native/frontend gates pass. The
root owns parity status decisions and evidence consolidation.

The root also assigned three strict-Clippy diagnostics in native `piano_tools.rs`.
The cleanup adds explicit multiplication parentheses before the existing RNG
shift and a named alias for the existing identity tuple. It preserves algorithm,
types and results; the native owner owns Clippy and final artifact regeneration.

## Native MIDI preview regression

The native inventory's MIDI preview failure was an old expectation that a
supported marker emits an import adjustment. The native owner had already
updated it to require no false adjustments. This worker preserved that repair
and added direct song-timeline marker identity/content assertions, exact tempo
and named-marker checks through actual exported song/pattern SMF bytes, and a
positive warning regression with an unsupported key signature. That regression
requires the genuine key-signature warning while forbidding a false marker-loss
warning, verifies preview is read-only, and checks imported markers and one-step
history. Existing undo/redo/document-path assertions remain.

File: `apps/desktop/src-tauri/src/session/tests/midi.rs`. Rustfmt passes;
the native owner runs the `session::tests::midi` desktop library target. Runtime
results are pending until its report; no Cargo invocation was duplicated here.

## Native conversion inventory repairs

The broad native inventory exposed supported-marker expectations in FLP/MIDI
tests that predated timeline import. FLP conversion now asserts the actual
pattern-local 5/4 meter and named marker, retains the unsupported kind-3 warning,
and counts the genuine unknown-event loss independently of the supported marker.
Author metadata remains a reported project loss. MIDI import still requires the
exact pitch-bend, aftertouch and key-signature losses; song and pattern markers
are asserted directly after real Document dispatch, including undo/redo checks
from the shared import harness.

The full-mixer MIDI failure was a product defect: import's batch ID planner
counted the optional Current utility strip toward the signal-strip capacity.
It now mirrors AddChannel's non-Current count and MAX_MIXER_SIGNAL_TRACKS bound.
The regression fills the signal mixer with exactly one free strip and imports
three parts, both with and without Current. It requires one new signal strip,
two Master fallbacks, correct pattern/channel/note IDs, and single-entry history.

The pattern-timeline export fixture now explicitly sets its intended 32-step
length before inserting notes. Previously its 4001-tick meter lay beyond the
default 3840-tick export duration. Exact local/song meter, marker, note, history
and persistence assertions remain unchanged.

Targets handed to the native owner: `windfall-flp --test conversion`,
`windfall-midi --test import`, and `windfall-midi --test pattern_timeline_qa`.
Rustfmt passes on all changed files. Runtime results remain pending; this
worker did not duplicate Cargo execution.
