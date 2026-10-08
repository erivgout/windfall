# Advanced step fill

Implementation is present; QA is deferred under the user's feature-first
instruction. This is not a tested end-to-end acceptance claim.

The channel context menu and command palette expose **Advanced step fill**.
Its dialog reviews a selected channel/pattern without changing the document.
Regular spacing, Euclidean distribution and reproducible seeded selection
produce a repeating cycle; right rotation and a first/last step range control
placement. Partial final cycles stop at the selected last step. MIDI key,
velocity and note-length controls author ordinary pattern notes.

**Replace range** replaces notes whose starts fall in the selected range,
including off-grid piano notes and chords. **Add to range** preserves every
existing note and skips generated hits at exactly occupied onsets. A preview
shows occupied steps and reports replacements or skipped onsets. Zero hits
can clear a reviewed range; the full 1024-step pattern bound is supported.

The native `FillStepRange` command compares the captured complete lane and
pattern length before editing. It refuses changed notes, added/removed notes,
changed length, invalid ranges, duplicate/non-grid onsets and lengths outside
1..=240 ticks. The ordinary document transaction handles atomic undo and
rollback, including id exhaustion. Notes outside the range retain their ids;
identical replacement notes retain theirs, and an identical fill is a no-op.
Authored notes use the existing persistence and sequencing paths.

The interface captures project generation, revision and lane selection;
replacement and history intent discard the pending review. It prevents
duplicate submissions and only closes the request whose result succeeded.
The native lane comparison is not a new session-generation IPC protocol;
general in-flight document replacement remains governed by the existing
dispatch/session machinery.

Before the user changed priorities, seven native transaction tests passed
and the bindings/shared Rust simulator were regenerated. The subsequent UI
tests were authored but have not been executed. Remaining QA includes the
complete rack/UI run, TypeScript/lint, composed native consumers, visual and
native-window interaction, save/play/export workflows and broader regressions.
No GitHub CI or Actions was used.
