import type { TimeSignature } from "@/bindings"
import {
  clamp,
  MAX_TEMPO_BPM,
  MIN_TEMPO_BPM,
  PPQ,
  TICKS_PER_STEP,
} from "@/lib/units"

export function ticksPerBeat(signature: TimeSignature): number {
  return (PPQ * 4) / signature.denominator
}

export function ticksPerBar(signature: TimeSignature): number {
  return signature.numerator * ticksPerBeat(signature)
}

export function ticksToSeconds(tick: number, tempoBpm: number): number {
  return (tick / PPQ) * (60 / tempoBpm)
}

export type Position = {
  /** All three count from 1, the way musicians do. */
  bar: number
  beat: number
  /** Sixteenth-note step inside the beat. */
  step: number
}

export function tickToPosition(
  tick: number,
  signature: TimeSignature
): Position {
  const whole = Math.max(0, Math.floor(tick))
  const bar = ticksPerBar(signature)
  const beat = ticksPerBeat(signature)
  const inBar = whole % bar
  return {
    bar: Math.floor(whole / bar) + 1,
    beat: Math.floor(inBar / beat) + 1,
    step: Math.floor((inBar % beat) / TICKS_PER_STEP) + 1,
  }
}

/** `001:01:1`, always the same width so the readout does not jitter. */
export function formatPosition(tick: number, signature: TimeSignature): string {
  const { bar, beat, step } = tickToPosition(tick, signature)
  return `${String(bar).padStart(3, "0")}:${String(beat).padStart(2, "0")}:${step}`
}

/** Minutes, seconds and hundredths: `1:07.25`. */
export function formatClock(seconds: number): string {
  const hundredths = Math.max(0, Math.floor(seconds * 100))
  const minutes = Math.floor(hundredths / 6000)
  const rest = hundredths % 6000
  const secs = String(Math.floor(rest / 100)).padStart(2, "0")
  const cents = String(rest % 100).padStart(2, "0")
  return `${minutes}:${secs}.${cents}`
}

export function formatTempo(tempoBpm: number): string {
  return tempoBpm.toFixed(2)
}

/** Keeps a tempo in range and to three decimals, which is all the UI shows. */
export function normalizeTempo(tempoBpm: number): number {
  return Math.round(clamp(tempoBpm, MIN_TEMPO_BPM, MAX_TEMPO_BPM) * 1000) / 1000
}

/** Reads what a person typed into the tempo field. Null when it is not a number. */
export function parseTempo(text: string): number | null {
  const value = Number(text.trim().replace(",", "."))
  if (text.trim() === "" || !Number.isFinite(value)) return null
  return normalizeTempo(value)
}

export function formatSampleRate(hz: number): string {
  const khz = hz / 1000
  return `${Number.isInteger(khz) ? khz : khz.toFixed(1)} kHz`
}

/** The last part of a path, with either kind of slash. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path
}
