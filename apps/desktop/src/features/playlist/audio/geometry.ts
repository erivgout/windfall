import type { ClipStretch } from "@/bindings"
import { dbToGain, gainToDb, MAX_GAIN, PPQ } from "@/lib/units"

/*
 * Where the audio of a clip lies on the timeline. A clip starts on its tick
 * and then runs in seconds, at its own speed, so what part of the file is
 * under a given tick depends on the tempo, the pitch, the offset and the
 * direction. This mirrors the timing rule in `ClipContent::Audio`.
 */

/** What of a clip decides where its audio lies. */
export type AudioTiming = {
  /** Clip start on the timeline, in ticks. */
  readonly start: number
  readonly length: number
  /** Ticks of audio skipped at the clip's start, at the stored tempo. */
  readonly offset: number
  /** Semitones. Tape-style: it changes the speed too. */
  readonly pitch: number
  readonly stretch?: ClipStretch
  readonly reverse: boolean
}

/** Seconds of the file that go by for every second of the song. */
export function playbackSpeed(
  timing: Pick<AudioTiming, "pitch" | "stretch">
): number {
  return timing.stretch?.mode === "spectral"
    ? 1 / timing.stretch.ratio
    : speedOf(timing.pitch)
}

export function speedOf(pitch: number): number {
  return 2 ** (pitch / 12)
}

/** Ticks in a second at a tempo: 960 a beat is 16 for each beat a minute. */
export function ticksPerSecond(tempoBpm: number): number {
  return (tempoBpm * PPQ) / 60
}

/**
 * How many ticks the audio lasts from the clip's start: the length of a
 * clip that ends exactly with its audio. Zero or less when the offset
 * skips all of it.
 */
export function naturalTicks(
  durationSecs: number,
  timing: Pick<AudioTiming, "offset" | "pitch" | "stretch">,
  tempoBpm: number
): number {
  return (
    (durationSecs / playbackSpeed(timing)) * ticksPerSecond(tempoBpm) -
    timing.offset
  )
}

/** The length to give a new clip of a whole file, as the shell does. */
export function wholeClipTicks(durationSecs: number, tempoBpm: number): number {
  return Math.max(1, Math.ceil(durationSecs * ticksPerSecond(tempoBpm)))
}

/**
 * The tick the sound of a clip stops on: the clip's end, or sooner when the
 * audio runs out first. `length` only ever cuts.
 */
export function audioEndTick(
  timing: AudioTiming,
  durationSecs: number,
  tempoBpm: number
): number {
  const natural = Math.max(0, naturalTicks(durationSecs, timing, tempoBpm))
  return timing.start + Math.min(timing.length, natural)
}

/**
 * The place in the file, as a fraction of its length from 0 to 1, that
 * plays at a tick of the song. Outside 0 to 1 there is no audio: before the
 * file's first frame or after its last. A reversed clip reads from the end.
 */
export function filePosition(
  timing: AudioTiming,
  tick: number,
  durationSecs: number,
  tempoBpm: number
): number {
  if (durationSecs <= 0) return -1
  const played =
    ((timing.offset + (tick - timing.start)) / ticksPerSecond(tempoBpm)) *
    playbackSpeed(timing)
  const forward = played / durationSecs
  return timing.reverse ? 1 - forward : forward
}

/** Fractions of the file a stretch of the timeline shows, in drawing order. */
export type FileWindow = {
  /** The file position at the left end and at the right end. */
  readonly from: number
  readonly to: number
}

/** The part of the file under the ticks `tick0` to `tick1`. */
export function fileWindow(
  timing: AudioTiming,
  tick0: number,
  tick1: number,
  durationSecs: number,
  tempoBpm: number
): FileWindow {
  return {
    from: filePosition(timing, tick0, durationSecs, tempoBpm),
    to: filePosition(timing, tick1, durationSecs, tempoBpm),
  }
}

/** The level of an equal-power fade in, `part` (0 to 1) of the way through. */
export function fadeInGain(part: number): number {
  return Math.sin((Math.min(1, Math.max(0, part)) * Math.PI) / 2)
}

/** The level of an equal-power fade out, `part` (0 to 1) of the way through. */
export function fadeOutGain(part: number): number {
  return Math.cos((Math.min(1, Math.max(0, part)) * Math.PI) / 2)
}

/**
 * The level the clip's own fades give at a tick, 0 to 1. The fade in runs
 * over the `fadeIn` ticks after the clip's start and the fade out over the
 * `fadeOut` ticks before its end. Where they overlap both apply.
 */
export function fadeGainAt(
  clip: { start: number; length: number; fadeIn: number; fadeOut: number },
  tick: number
): number {
  const into = tick - clip.start
  const left = clip.start + clip.length - tick
  let gain = 1
  if (clip.fadeIn > 0 && into < clip.fadeIn)
    gain *= fadeInGain(into / clip.fadeIn)
  if (clip.fadeOut > 0 && left < clip.fadeOut) {
    gain *= fadeOutGain(1 - left / clip.fadeOut)
  }
  return into < 0 || left < 0 ? 0 : gain
}

/** A fade length a drag asks for, held inside the clip and on the grid. */
export function clampFade(
  ticks: number,
  clipLength: number,
  snap: number
): number {
  const snapped = snap > 0 ? Math.round(ticks / snap) * snap : Math.round(ticks)
  return Math.min(Math.max(0, snapped), Math.max(0, clipLength))
}

/**
 * The fade a drag of a fade handle to `pointerTick` gives: the distance
 * from the clip's start for the fade in, and from its end for the fade out.
 */
export function fadeFromPointer(
  edge: "in" | "out",
  clip: { start: number; length: number },
  pointerTick: number,
  snap: number
): number {
  const raw =
    edge === "in"
      ? pointerTick - clip.start
      : clip.start + clip.length - pointerTick
  return clampFade(raw, clip.length, snap)
}

/** Decibels a gain drag moves for every pixel the pointer travels up. */
export const GAIN_DB_PER_PIXEL = 0.25
/** The quietest level a gain drag reaches before it falls to silence. */
export const GAIN_FLOOR_DB = -60

/**
 * The gain a vertical drag gives: a quarter of a decibel a pixel, a tenth
 * of that with `fine`. Below the floor it is silence, and coming back up
 * from silence starts at the floor.
 */
export function gainFromDrag(
  startGain: number,
  pixelsUp: number,
  fine: boolean
): number {
  const startDb = Math.max(GAIN_FLOOR_DB, gainToDb(startGain))
  const db = startDb + pixelsUp * GAIN_DB_PER_PIXEL * (fine ? 0.1 : 1)
  if (db < GAIN_FLOOR_DB) return 0
  return Math.min(MAX_GAIN, dbToGain(Math.round(db * 10) / 10))
}

/** A fade length as text: "1/2 beat · 250 ms", in beats and in time. */
export function describeFade(ticks: number, tempoBpm: number): string {
  if (ticks <= 0) return "No fade"
  const beats = ticks / PPQ
  const ms = (ticks / ticksPerSecond(tempoBpm)) * 1000
  const beatText =
    beats >= 10
      ? beats.toFixed(0)
      : Number.isInteger(beats * 4)
        ? String(beats)
        : beats.toFixed(2)
  const timeText =
    ms >= 1000 ? `${(ms / 1000).toFixed(2)} s` : `${Math.round(ms)} ms`
  return `${beatText} ${beats === 1 ? "beat" : "beats"}, ${timeText}`
}
