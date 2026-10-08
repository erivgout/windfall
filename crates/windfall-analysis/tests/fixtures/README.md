# Authored lifecycle CPU fixture v1

These bytes and the interleaved four-frame stereo PCM sequence in
`analysis_jobs.rs` are authored for Windfall and dedicated to CC0-1.0.
They contain no third-party audio, weights or runtime code. The adapter exists
only in the integration test executable. It streams the selected PCM unchanged.

Pinned contract: fixture version 1, CPU, 48,000 Hz, stereo, selection `[1, 3)`,
frame origin 1, exact Float32 output `[0.5, -0.5, 0.75, -0.75]`. SHA-256 checks
pin both the model fixture and encoded output below in the tests. This proves
job/file lifecycle only. No separation, denoising, pitch or MIDI quality is
measured. A real adapter must add its own fixed model/runtime versions, exact
pre/postprocessing and licensed CPU references before claiming inference.
