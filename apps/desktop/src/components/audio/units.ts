// SPDX-License-Identifier: MIT

// Readouts use the real minus sign so "−6.0" lines up with "+6.0" in tabular
// figures. The parsers accept a plain hyphen as well.
const MINUS = "−"

/** Levels at or below this many dB read as silence. */
export const SILENCE_DB = -120

export type ValueUnit = {
  format: (value: number) => string
  parse: (text: string) => number | null
  /**
   * What the number is: `"gain"`, `"pan"`, `"hertz"`. Two controls with
   * the same kind hold the same sort of number, so a value copied from one
   * means the same in the other.
   */
  unitKind?: string
}

/** The kind of a control that holds a bare number, with no unit. */
export const PLAIN_NUMBER = "number"

export function gainToDb(gain: number): number {
  return gain > 0 ? 20 * Math.log10(gain) : -Infinity
}

export function dbToGain(db: number): number {
  if (Number.isNaN(db) || db === -Infinity) {
    return 0
  }
  return 10 ** (db / 20)
}

function signed(value: number, digits: number, plus: boolean): string {
  const fixed = Math.abs(value).toFixed(digits)
  if (Number(fixed) === 0) {
    return fixed
  }
  if (value < 0) {
    return `${MINUS}${fixed}`
  }
  return plus ? `+${fixed}` : fixed
}

/** `formatDb(-6)` is "−6.0 dB", `formatDb(-Infinity)` is "−∞ dB". */
export function formatDb(db: number, digits = 1): string {
  if (!(db > SILENCE_DB)) {
    return `${MINUS}∞ dB`
  }
  return `${signed(db, digits, true)} dB`
}

/** Formats a linear gain as decibels. */
export function formatGain(gain: number, digits = 1): string {
  return formatDb(gainToDb(gain), digits)
}

/** Pan from -1 (left) to 1 (right): "L50", "C", "R100". */
export function formatPan(pan: number): string {
  const amount = Math.round(pan * 100)
  if (amount === 0) {
    return "C"
  }
  return amount < 0 ? `L${-amount}` : `R${amount}`
}

/** A 0 to 1 fraction as a percentage: `formatPercent(0.5)` is "50%". */
export function formatPercent(value: number, digits = 0): string {
  return `${signed(value * 100, digits, false)}%`
}

export function formatMs(ms: number): string {
  const size = Math.abs(ms)
  if (size >= 10000) {
    return `${signed(ms / 1000, 1, false)} s`
  }
  if (size >= 1000) {
    return `${signed(ms / 1000, 2, false)} s`
  }
  if (size >= 10 || Number.isInteger(ms)) {
    return `${signed(ms, 0, false)} ms`
  }
  return `${signed(ms, 1, false)} ms`
}

export function formatSemitones(semitones: number, digits = 0): string {
  return `${signed(semitones, digits, true)} st`
}

export function formatHz(hz: number): string {
  if (Math.abs(hz) >= 1000) {
    return `${signed(hz / 1000, 2, false)} kHz`
  }
  return `${signed(hz, hz < 100 ? 1 : 0, false)} Hz`
}

function normalize(text: string): string {
  return text.trim().toLowerCase().replace(/[−–—]/g, "-")
}

/**
 * Reads the first number in a string and ignores any unit around it.
 * Understands "-6", "−6 dB", "+3,5", ".5", "-inf" and "∞". Returns null when
 * there is no number.
 */
export function parseNumber(text: string): number | null {
  const clean = normalize(text)
  const infinite = /^([+-]?)\s*(?:inf(?:inity)?|∞)/.exec(clean)
  if (infinite) {
    return infinite[1] === "-" ? -Infinity : Infinity
  }
  const match = /([+-]?)\s*(\d+(?:[.,]\d*)?|[.,]\d+)/.exec(clean)
  if (!match) {
    return null
  }
  const parsed = Number(match[2].replace(",", "."))
  if (Number.isNaN(parsed)) {
    return null
  }
  return match[1] === "-" ? -parsed : parsed
}

/** Text in decibels to decibels. "-inf" gives -Infinity. */
export function parseDb(text: string): number | null {
  return parseNumber(text)
}

/** Text in decibels to linear gain: "-6 dB" is about 0.501. */
export function parseGain(text: string): number | null {
  const db = parseNumber(text)
  return db === null ? null : dbToGain(db)
}

/** "50%" or "50" to 0.5. */
export function parsePercent(text: string): number | null {
  const percent = parseNumber(text)
  return percent === null ? null : percent / 100
}

/** "L50", "50L", "c", "R", "-25" and "25%" to a pan from -1 to 1. */
export function parsePan(text: string): number | null {
  const clean = normalize(text)
  if (/^(c|center|centre)$/.test(clean)) {
    return 0
  }
  const amount = parseNumber(clean)
  const side = /[lr]/.exec(clean)?.[0]
  if (side) {
    const size = amount === null ? 1 : Math.abs(amount) / 100
    return side === "l" ? -size : size
  }
  return amount === null ? null : amount / 100
}

/** "250", "250 ms" and "1.2 s" to milliseconds. */
export function parseMs(text: string): number | null {
  const amount = parseNumber(text)
  if (amount === null) {
    return null
  }
  return /\d\s*s/.test(normalize(text)) ? amount * 1000 : amount
}

export function parseSemitones(text: string): number | null {
  return parseNumber(text)
}

/** "440", "440 Hz" and "1.2k" to hertz. */
export function parseHz(text: string): number | null {
  const amount = parseNumber(text)
  if (amount === null) {
    return null
  }
  return /\d\s*k/.test(normalize(text)) ? amount * 1000 : amount
}

// Matching format and parse pairs. Spread one into a control:
// <Knob {...percentUnit} />.
export const dbUnit: ValueUnit = {
  format: formatDb,
  parse: parseDb,
  unitKind: "decibels",
}
export const gainUnit: ValueUnit = {
  format: formatGain,
  parse: parseGain,
  unitKind: "gain",
}
export const panUnit: ValueUnit = {
  format: formatPan,
  parse: parsePan,
  unitKind: "pan",
}
export const percentUnit: ValueUnit = {
  format: formatPercent,
  parse: parsePercent,
  unitKind: "fraction",
}
export const msUnit: ValueUnit = {
  format: formatMs,
  parse: parseMs,
  unitKind: "milliseconds",
}
export const semitonesUnit: ValueUnit = {
  format: formatSemitones,
  parse: parseSemitones,
  unitKind: "semitones",
}
export const hzUnit: ValueUnit = {
  format: formatHz,
  parse: parseHz,
  unitKind: "hertz",
}
