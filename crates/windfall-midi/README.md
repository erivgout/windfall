# windfall-midi

Pure Rust MIDI file reading, writing, project import plans and playlist export.
See the crate documentation for mapping and rounding rules.

The shell can import with `read_file(path)`, then `import(&song, &options)`.
Show the plan's channels, patterns, clips and adjustments to the user, then
dispatch `plan.command(document.project())` with no intervening project edit.
This command is a single undo step. Resolve any mapped drum sample paths through
the shell's usual sample-loading path before playback.

Export with `export_song(project, &options)` or
`export_pattern(project, pattern_id, &options)`, then
`write_file(path, &song, &write_options)`. The byte-only equivalents `read`
and `write` also work in WebAssembly. Recommended IPC operations are
`import_midi(path, options)` and `export_midi(path, options)`; an optional
preview endpoint can return the import plan without dispatching it.

No external parser or writer dependency is used. Runtime dependencies are the
workspace's `windfall-core` and `windfall-project`, both GPL-3.0-or-later.
Test dependencies `proptest` 1.11.0 and `tempfile` 3.27.0 are dual licensed
MIT OR Apache-2.0; their local registry `LICENSE-MIT` texts were inspected.
The MIT option permits their use with the workspace license. All MIDI fixture
bytes are authored in tests, with no downloaded MIDI files; those fixture bytes
are dedicated to the public domain under CC0-1.0.

Import reports unrepresentable controllers, program changes, pitch bend,
pressure, markers, key signatures and later meter changes. General MIDI
programs provide names, not matching synth presets. Long notes crossing
pattern splits retrigger; excess notes and tempo points are reported when
limited. Export omits audio and instrument/effect sound, reuses MIDI channels
beyond fifteen melodic tracks, and samples continuous tempo curves. A MIDI
write/read round trip compares normalized songs because overlapping same-key
notes have no individual identities in the file format.
