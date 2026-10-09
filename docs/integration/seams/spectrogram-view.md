# Spectrogram view

Partial coverage for `vis-fruity-spectroman`. Windfall names the panel
**Spectrogram** and its command-palette action **Show spectrogram**
(`view.spectrogram`), following `view.spectrum`. This displays a short history,
not a calibrated measurement or a new analyzer.

## Row contract

`RealtimeFrame.spectrogram` is a flat Rust `Vec<f32>` and optional desktop
`number[]`. Missing input defaults to an empty vector; empty vectors are omitted
on serialization. Empty means no valid history slice is available. Valid silence
contains zero power rather than empty input.

The vector is row-major, oldest valid row first, with at most **16 rows** and
**64 columns**. The existing analyzer currently retains **eight** chronological
slices, so this publication currently has at most eight rows. Invalid slices are
omitted, not replaced by silent rows; time gaps are not represented. A frame read
copies that history and does not append a new row or retain another history.

Each row follows the unchanged `spectrum` band contract. For `M` source bins,
`B = min(M, 64)`. Band `i` sums bins in
`[floor(i*M/B), floor((i+1)*M/B))`, including DC and Nyquist exactly once, then
averages over valid stereo channels for that slice. The shared summary helper
reads the existing `SpectrumBin.mean_square` values (already copied into analyzer
slices). Published values remain finite, nonnegative **mean-square power**, not
dB. A slice with neither channel valid, a negative/nonfinite source power, or
power not representable as finite f32 is omitted in full.

Columns match the nonempty companion `spectrum`. With the current default
1024-point analyzer, every row has **64 columns**, even when the newest spectrum
is invalid but older valid slices remain. The desktop uses that default width
when `spectrum` is absent/empty; a smaller nonempty companion spectrum supplies
its width. No dimensions or sample-rate field is added to the frame.

## Publication and view

`Controller::frame()` borrows the same snapshot once and copies both spectrum
and history off the audio callback. No analyzer, FFT, audio tap, or audio-side
allocation changes are needed. History inherits the spectrum reader's selection,
plan, stream, seek, cancellation, and overflow invalidation. Project replacement
also clears history in the desktop's shared realtime store.

The view uses `useRealtime` and a fixed pool of 16×64 SVG rectangles. It updates
cells directly without React state or a React render per frame. Frequency runs
from DC on the left to Nyquist on the right; older rows are at the top and newer
rows at the bottom. Power maps through `10*log10(power)`, clamped to **−90…0 dB**,
to the brand color's opacity. Zero power uses the floor. Empty/missing input
clears the grid and says **No spectrogram available**. Malformed incomplete rows
are cleared, invalid cell values are hidden, and oversized history draws only
the newest 16 complete rows.

Closing/unmounting the panel drops its subscription. The shared feed remains
active for other subscribers and stops when the last subscriber leaves. Minimal
shell glue mounts the panel and registers its one command-palette action.

## Validation

From Git Bash at the repository root:

```sh
source scripts/msvc-env.sh
cargo test -p windfall-engine --test spectrogram_view -- --test-threads=1
cargo test -p windfall-engine --test spectrum_view -- --test-threads=1
```

From `apps/desktop`:

```sh
pnpm test -- src/features/spectrogram/view.test.tsx src/features/spectrogram/panel.test.tsx src/features/spectrum/view.test.tsx src/features/spectrum/panel.test.tsx
```

Native coverage checks empty history, oldest-first packing through history wrap,
identical row/current-spectrum band summaries, finite nonnegative power, no
audio-side allocator calls, repeated frame reads, and seek/project/stream/
overflow invalidation. Desktop coverage checks empty/omitted frames and stale
cell clearing, the expected 2×4 cell positions and power mapping, no React render
per frame, bounded and invalid input, valid silence/older history without a
current spectrum, project replacement, action opening, and subscription cleanup.
