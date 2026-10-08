// Provisionally synchronized from Rust; artifact generation is deferred.
export type MixerWaveform = { track: import("./TrackId").TrackId; epoch: number; serial: number; sampleRate: number; bucketFrames: number; points: [number, number, number, number][] }
