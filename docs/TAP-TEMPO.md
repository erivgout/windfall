# Tap tempo

Open **Tap** in the transport, **Tap tempo** in the command palette, or the
tempo field's context menu. Tap the button with a pointer or press Space/Enter
while it has focus. Review the BPM, then choose **Apply tempo**. Reset and
Cancel do not change the project. Apply uses the ordinary settings command and
creates one undo entry; applying the current tempo creates none.

The estimator retains at most eight timestamps and uses their intervals.
Intervals shorter than the fastest supported tempo are ignored, as are
nonfinite or backwards timestamps. A gap longer than six seconds starts a new
sequence. With at least three intervals, intervals more than 35% from the
median are excluded; if none remain, all intervals are used. The result uses
the existing 10–522 BPM range and 0.001 BPM normalization. Reset when changing
rhythm intentionally. Keyboard repeats and synthesized duplicate key clicks
do not add taps.

New/Open resets the estimate. A pending Apply reply from another project
cannot close or update the replacement project's dialog. Existing tempo
automation retains its playback effect; tapping changes the stored base tempo.

The shared project store now rejects asynchronous edit/history/snapshot
replies from a replaced document, including replacements that reuse revision
numbers. Snapshot recovery also avoids rolling back a newer event patch and
refetches when patches overtake an in-flight snapshot. These guards reuse the
existing UI document-generation signal without changing persisted fields or
the IPC protocol.

Focused verification on Windows: 73 UI tests across transport, tempo flows
and project-store suites passed. This includes steady/uneven/missed beats,
tempo boundaries, bounded history, reset/cancel, keyboard input, Apply with
undo/redo/save/open, reused revisions, stale history replies and event/snapshot
ordering. ESLint and the TypeScript/Vite production build passed. Physical
timing, native packaged interaction and other operating systems were not
verified by these tests.
