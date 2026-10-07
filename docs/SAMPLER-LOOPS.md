# Sampler loops

The channel sampler supports persisted Off, Forward and Ping-pong loop modes,
with editable start/end points in the channel inspector. Playback and offline
export use the existing sampler voice path; no second player is introduced.

## Range and playback

`SamplerSettings.loopMode` is `off`, `forward` or `pingPong`. `loopStart` and
`loopEnd` are fractions of the **trimmed playback range**, in original source
order, with `0 <= loopStart < loopEnd <= 1`. The inspector shows percentages
and keeps each point before/after the other. Turning the mode off preserves
the selected points. Trim edits keep the loop's relative placement.

The control-side plan rounds these fractions to source frames, clamps them to
the trimmed region and guarantees at least one frame for a nonempty sample.
The end is exclusive. Reverse mirrors the prepared bounds into playback order,
so the same physical part of the trimmed source loops in either direction.
Any lead-in from the playback start to the loop plays once.

- Forward repeats the selected frames in the chosen playback direction.
- Ping-pong reflects at the first and last included frames. Endpoints are not
  duplicated: a three-frame loop plays `A B C B A B C B ...`.
- A one-frame loop holds that frame. Subframe ranges round to at least one frame.

Fractional pitch/sample-rate steps retain their overshoot at every wrap or
turn. Traversal uses bounded modulo arithmetic, including when one output frame
crosses several loop periods. Four-point Hermite interpolation wraps its taps
at forward seams and reflects them at ping-pong turns, rather than reading
silence or unrelated source frames. During the initial lead-in it reads actual
lead-in frames and extends the first playback frame at the leading edge.
Whole-frame reads remain exact. Loop seams are not crossfaded; a discontinuous
selection can still produce a click, and interpolation can overshoot as usual.

## Notes, edits and ownership

A looping note always honors its sequenced end tick or live note-off. Without
an explicit volume envelope it holds at unity and releases over 4 ms. With an
envelope, attack/decay/sustain/release remain unchanged, including the existing
1 ms minimum for an explicitly zero release. The loop continues throughout
release and ends when the envelope reaches silence; it does not jump to a
post-loop outro. Off mode preserves the original drum one-shot behavior:
without an envelope it ignores note length and plays the full trimmed sample.

The existing polyphonic voice pool, deterministic stealing, cut-self, cut groups,
note velocity/pan, sampler gain, root key, fine tuning and mixer routes still
apply. Missing or empty decoded sources are silent. Browser previews remain
one-shots. Playlist clips keep their separate stretch/pitch processing.

Loop bounds and the implicit release envelope are prepared on the control side.
Voices store only fixed-size loop state and share immutable sample buffers;
the callback performs no allocation, locking or file I/O. Existing sample and
plan retirement stays on the controller side. Loop/source edits affect new
notes; already sounding notes finish with their original sample and loop
settings. Reloading a source does not mutate audio held by existing notes.
No recording/session locking, plugin ownership or VST3 availability changes
are part of this feature.

## Persistence, validation and undo

New fields have serde defaults. `off`, start `0` and end `1` are omitted when
serialized, preserving the legacy project shape and format version 1. The
generated TypeScript fields are optional and the inspector supplies the same
defaults. Nondefault points persist even when looping is disabled.

`UpdateSampler` patches mode/start/end atomically. Finite values are clamped
to 0..1; nonfinite values and crossed/equal points are rejected without changing
the project or history. File loading validates these rules even in Off mode.
Existing sampler transactions provide undo/redo, no-op handling and gesture
coalescing. Mode edits are labeled "Change sample loop mode" and point edits
"Change sample loop range". Existing duplication/save/load retain the fields.

## Verification

Checks run on Windows with Git Bash and `scripts/msvc-env.sh`, setting
`TS_RS_EXPORT_DIR` to a task-specific temporary directory:

- `cargo test -p windfall-project -p windfall-engine -p windfall-flp --lib --tests`
  passed all 693 tests. Coverage includes legacy serialization, on-disk loop
  round-trips, invalid persisted/command points, undo/redo and randomized edits;
  forward versus ping-pong, reverse/trim, fractional and large rates, stereo,
  one-frame and short loops, implicit/explicit release, polyphony/cuts, missing
  and empty sources, immutable source reload, and playback/export bit parity
  across block sizes. Existing playlist spectral tests also passed.
- The allocator guard exercises both modes through 320 simultaneous notes,
  stealing, note-offs, plan edits, source replacement and retirement. It observes
  zero allocations, reallocations or frees inside `Processor::process`.
- `pnpm test src/features/channel-rack/inspector.test.tsx
  src/features/channel-rack/rack.test.tsx
  src/features/channel-rack/rack-render.test.tsx --maxWorkers=4` passed 82 tests.
  The inspector tests exercise mode changes, point gestures, history labels,
  undo, crossing prevention and channel selection with the locally rebuilt Rust
  WASM backend. Typechecking uses locally generated TS, never handwritten bindings.
- `cargo fmt --all --check`, `cargo clippy -p windfall-project -p windfall-engine
  -p windfall-flp --all-targets -- -D warnings`, `pnpm typecheck` and `pnpm lint`
  passed. The final test-only clippy repair was followed by the seven matching
  engine loop integration tests, including export and allocator checks.
- T3 collaborative preview at 1280x800: opened Kick's inspector, selected Forward
  then Ping-pong, nudged the loop start with the keyboard, and verified Undo/Redo
  restored its value. The inspector remained readable in its 300 px pane.

This is synthetic processor/export and browser UI verification. No audio device,
hardware listening, installed native shell, or non-Windows OS verification is
claimed. Final combined bindings/WASM regeneration belongs to integration;
branch-local generated artifacts are excluded from this feature commit.

## Remaining sampler stretch gap

Sampler pitch remains tape-style resampling: transposing a note changes its
playback speed. Independent sampler time-stretch is not added here. Playlist
spectral preparation remains independent and unchanged.

The existing bounded control-side prepared-audio cache is a possible foundation,
but a sampler must account for note-key-dependent pitch variants, release/loop
boundaries and cache eviction while old voices retain their sources. Preparing
on note-on would violate the callback contract. A follow-up needs a bounded
prepared variant policy and playback/export tests before enabling independent
sampler stretch; reusing a single playlist variant would not establish that
contract.
