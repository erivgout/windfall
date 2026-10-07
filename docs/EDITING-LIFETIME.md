# Clip editing lifetime

Native Apply publishes its project patch before its IPC reply. A replacement
patch removes the selected source clip, so the clip inspector temporarily has
no selected audio. That transition must not end the operation awaiting its
reply.

AudioEditorButton and SliceControls occupy stable positions in the inspector
strip, outside its selected/idle settings branch. Idle triggers are invisible
and inert; their dialog portals retain their normal visibility. Open captures
the source, selection identity and project generation. Each opening gets a
distinct local ticket and a new form, so a source prop changing during its own
publication cannot reopen or retarget the running operation.

Apply accepts a reply only while its form is mounted, the generation still
matches and the original selection identity is current. Publication does not
change selection; an explicit selection change abandons the session, including
leaving and returning to the same IDs. A successful current Apply merges the
reply patch idempotently, selects its result and closes. The editor selects the
last created ID (the derived clip); slicing selects every created clip.

Hiding/unmounting the inspector, closing a dialog or changing selection releases
retained preparation/review tokens. Late preparation results discard their own
tokens. Project replacement also releases retained tokens immediately; the
actual inspector starts a new strip keyed by the existing generation signal.
Standalone controls show the stale-project error and require reopening. Busy
state belongs to the opening ticket, so an old reply or cleanup cannot unlock
or close a newer editor.

`apps/desktop/src/features/playlist/audio/editing-lifetime.test.tsx` renders the
actual inspector with connected stores and the real WASM document. Its two
event-order regressions publish replacement events, flush React and rerender
the inspector before resolving Apply without changing generation. They verify
result selection, continued mounting, token ownership and unchanged history on
the duplicate reply. Additional cases cover selection changes, inspector
closure, slice cancellation, reopened projects with reused IDs/revisions and
old replies during newer operations. Audio file rendering is stubbed at the IPC
boundary; native rendering/publication safety remains covered by session tests.

The base commit's generated bindings predate its merged piano-roll transforms.
Generate local bindings for typechecking, and exclude generated files from this
lifetime-only commit. No store dispatch/history/refetch behavior changes here.
