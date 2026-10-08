/** Constants that mirror `windfall-core` and `windfall-project`. */

/** Ticks per quarter note. */
export const PPQ = 960
/** A step is a sixteenth note. */
export const TICKS_PER_STEP = 240

export const DEFAULT_KEY = 60
export const DEFAULT_VELOCITY = 0.8
export const DEFAULT_CHANNEL_VOLUME = 0.8
export const DEFAULT_PATTERN_STEPS = 16
export const MAX_PATTERN_STEPS = 1024
export const MAX_PATTERN_TICKS = MAX_PATTERN_STEPS * TICKS_PER_STEP
/** The end of the longest song: a million quarter notes. No clip ends past it. */
export const MAX_SONG_TICKS = 1_000_000 * PPQ
/** Furthest an audio clip can be pitched up or down, in semitones. */
export const MAX_TUNE_SEMITONES = 48
/**
 * Audio clips the engine can sound at once. One more is not started and
 * stays silent for its whole length.
 */
export const MAX_AUDIO_CLIPS = 64
/** Most points one automation curve can have. */
export const MAX_AUTOMATION_POINTS = 4096

/** The tempo of an empty project. */
export const DEFAULT_TEMPO_BPM = 120
export const MIN_TEMPO_BPM = 10
export const MAX_TEMPO_BPM = 522

/** Highest linear gain a channel or mixer fader allows, about +6 dB. */
export const MAX_GAIN = 2
export const MAX_MIXER_TRACKS = 501
export const MASTER_TRACK = 0

export const FORMAT_VERSION = 1

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

/** `0xRRGGBB` to a CSS color. */
export function colorToCss(color: number): string {
  return `#${(color & 0xffffff).toString(16).padStart(6, "0")}`
}

/** Linear gain to decibels. Silence is `-Infinity`. */
export function gainToDb(gain: number): number {
  return gain <= 0 ? -Infinity : 20 * Math.log10(gain)
}

export function dbToGain(db: number): number {
  return db === -Infinity ? 0 : 10 ** (db / 20)
}
