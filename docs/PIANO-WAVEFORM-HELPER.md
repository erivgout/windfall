# Piano waveform-helper source implementation

The piano roll toolbar, View menu and palette now offer Waveform helper.
The control selects existing project audio; on first open it prefers the
current sampler's source, then the project's first audio asset. Show/Hide
keeps the reference configured, and Clear reference removes it. Importing
audio through the existing channel/playlist workflows makes it available.

The reference appears behind the active notes and alongside ghost notes in
the renderer's underlay. Timing modes preserve the sample's duration at the
current project tempo, fit its overview to the pattern, or map it to a custom
tick length. Signed tick offset aligns transients. Center MIDI key, height in
rows and opacity determine its placement in the pitch grid. Tempo changes
update the original-duration mapping; pattern-length changes update pattern
fit. Its visible end can extend the editor's scrollable reference span up to
the existing maximum note-pattern length.

The helper uses the existing generation-aware sample-info cache and native
peak overview. Missing/loading status appears in its control. Rendering
aggregates min/max peak buckets into at most 2,048 flat rect columns in the
shared Canvas/WebGL/WebGPU underlay contract, with row-grid vertical precision.
Ghost notes retain their source ids for editable-ghost input; waveform rects
are reference-only and are not note hits. Theme changes rebuild colors through
the normal view lifecycle. Configuration is view state, cleared on project
replacement; no audio, source file or project note is edited by the helper.

No builds, tests, QA, reviews or browser/runtime checks ran for this source
pass. The renderer composition and native file interaction await deferred QA.
