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

export const MIN_TEMPO_BPM = 10
export const MAX_TEMPO_BPM = 522

/** Highest linear gain a channel or mixer fader allows, about +6 dB. */
export const MAX_GAIN = 2
export const MAX_MIXER_TRACKS = 128
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
