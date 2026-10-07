import type { CutSlope, EqBand, EqCutBand, EqParams } from "@/bindings"

/*
 * The equaliser's curve, computed the way the engine computes it
 * (`crates/windfall-dsp/src/eq.rs` and `blocks/biquad.rs`), so the display
 * shows what is heard. The coefficient formulas are the ones in Robert
 * Bristow-Johnson's "Audio EQ Cookbook"
 * (https://webaudio.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html).
 */

/** The bands in signal order, which is also left to right on the display. */
export const EQ_BAND_IDS = [
  "lowCut",
  "lowShelf",
  "peak1",
  "peak2",
  "peak3",
  "highShelf",
  "highCut",
] as const

export type EqBandId = (typeof EQ_BAND_IDS)[number]
export type EqBandShape =
  "lowCut" | "lowShelf" | "peak" | "highShelf" | "highCut"

export const EQ_BAND_SHAPES: Record<EqBandId, EqBandShape> = {
  lowCut: "lowCut",
  lowShelf: "lowShelf",
  peak1: "peak",
  peak2: "peak",
  peak3: "peak",
  highShelf: "highShelf",
  highCut: "highCut",
}

export function isCutBand(id: EqBandId): id is "lowCut" | "highCut" {
  return id === "lowCut" || id === "highCut"
}

/** One second-order section, already divided by `a0`. */
export type Section = {
  b0: number
  b1: number
  b2: number
  a1: number
  a2: number
}

/** Lowest and highest Q a section is built with, as in the engine. */
const Q_MIN = 0.025
const Q_MAX = 40

/** A section never reads lower than this, which is -200 dB. */
const MAGNITUDE_FLOOR = 1e-10

const CUT_STAGES: Record<CutSlope, number> = { db12: 1, db24: 2, db48: 4 }

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value))
}

/** The cookbook's `cos(w0)` and `alpha`. */
function prototype(frequencyHz: number, q: number, sampleRate: number) {
  const rate = Math.max(1, sampleRate)
  // The formulas break down at 0 Hz and at half the sample rate.
  const frequency = clamp(frequencyHz, 1, 0.49 * rate)
  const omega = (2 * Math.PI * frequency) / rate
  return {
    cos: Math.cos(omega),
    alpha: Math.sin(omega) / (2 * clamp(q, Q_MIN, Q_MAX)),
  }
}

function normalized(b: number[], a: number[]): Section {
  const scale = 1 / a[0]
  return {
    b0: b[0] * scale,
    b1: b[1] * scale,
    b2: b[2] * scale,
    a1: a[1] * scale,
    a2: a[2] * scale,
  }
}

/** The cookbook's `A`: the square root of the linear gain. */
function amplitude(gainDb: number) {
  return 10 ** (gainDb / 40)
}

export function lowPass(frequencyHz: number, q: number, sampleRate: number) {
  const { cos, alpha } = prototype(frequencyHz, q, sampleRate)
  const edge = (1 - cos) / 2
  return normalized([edge, 1 - cos, edge], [1 + alpha, -2 * cos, 1 - alpha])
}

export function highPass(frequencyHz: number, q: number, sampleRate: number) {
  const { cos, alpha } = prototype(frequencyHz, q, sampleRate)
  const edge = (1 + cos) / 2
  return normalized([edge, -(1 + cos), edge], [1 + alpha, -2 * cos, 1 - alpha])
}

export function peak(
  frequencyHz: number,
  q: number,
  gainDb: number,
  sampleRate: number
) {
  const { cos, alpha } = prototype(frequencyHz, q, sampleRate)
  const a = amplitude(gainDb)
  return normalized(
    [1 + alpha * a, -2 * cos, 1 - alpha * a],
    [1 + alpha / a, -2 * cos, 1 - alpha / a]
  )
}

export function lowShelf(
  frequencyHz: number,
  q: number,
  gainDb: number,
  sampleRate: number
) {
  const { cos, alpha } = prototype(frequencyHz, q, sampleRate)
  const a = amplitude(gainDb)
  const root = 2 * Math.sqrt(a) * alpha
  return normalized(
    [
      a * (a + 1 - (a - 1) * cos + root),
      2 * a * (a - 1 - (a + 1) * cos),
      a * (a + 1 - (a - 1) * cos - root),
    ],
    [
      a + 1 + (a - 1) * cos + root,
      -2 * (a - 1 + (a + 1) * cos),
      a + 1 + (a - 1) * cos - root,
    ]
  )
}

export function highShelf(
  frequencyHz: number,
  q: number,
  gainDb: number,
  sampleRate: number
) {
  const { cos, alpha } = prototype(frequencyHz, q, sampleRate)
  const a = amplitude(gainDb)
  const root = 2 * Math.sqrt(a) * alpha
  return normalized(
    [
      a * (a + 1 + (a - 1) * cos + root),
      -2 * a * (a - 1 + (a + 1) * cos),
      a * (a + 1 + (a - 1) * cos - root),
    ],
    [
      a + 1 - (a - 1) * cos + root,
      2 * (a - 1 - (a + 1) * cos),
      a + 1 - (a - 1) * cos - root,
    ]
  )
}

/**
 * The sections of a cut filter. Together they are a Butterworth filter of
 * order `2 * stages`; the band's Q scales the sharpest section, so 0.707
 * gives the pure Butterworth response and more adds a peak at the corner.
 */
export function cutSections(
  band: EqCutBand,
  highPassFilter: boolean,
  sampleRate: number
): Section[] {
  const stages = CUT_STAGES[band.slope]
  const sections: Section[] = []
  for (let stage = 0; stage < stages; stage += 1) {
    const angle = (Math.PI * (2 * stage + 1)) / (4 * stages)
    let q = 1 / (2 * Math.cos(angle))
    if (stage + 1 === stages) q *= band.q / Math.SQRT1_2
    sections.push(
      highPassFilter
        ? highPass(band.frequencyHz, q, sampleRate)
        : lowPass(band.frequencyHz, q, sampleRate)
    )
  }
  return sections
}

function bellSection(
  shape: "lowShelf" | "peak" | "highShelf",
  band: EqBand,
  sampleRate: number
): Section {
  const { frequencyHz, q, gainDb } = band
  if (shape === "lowShelf") return lowShelf(frequencyHz, q, gainDb, sampleRate)
  if (shape === "highShelf") {
    return highShelf(frequencyHz, q, gainDb, sampleRate)
  }
  return peak(frequencyHz, q, gainDb, sampleRate)
}

/**
 * The sections one band puts in the signal path: none while it is off, and
 * none for a shelf or bell at 0 dB, which changes nothing.
 */
export function bandSections(
  params: EqParams,
  id: EqBandId,
  sampleRate: number
): Section[] {
  if (isCutBand(id)) {
    const band = params[id]
    return band.enabled ? cutSections(band, id === "lowCut", sampleRate) : []
  }
  const band = params[id]
  if (!band.enabled || band.gainDb === 0) return []
  const shape = EQ_BAND_SHAPES[id]
  if (shape === "lowCut" || shape === "highCut") return []
  return [bellSection(shape, band, sampleRate)]
}

/**
 * The frequencies a curve is drawn at, with what every section needs from
 * each of them worked out once.
 */
export type ResponseGrid = {
  frequenciesHz: Float64Array
  /** `sin^2(w / 2)` at each frequency. */
  phi: Float64Array
}

export function responseGrid(
  frequenciesHz: ArrayLike<number>,
  sampleRate: number
): ResponseGrid {
  const rate = Math.max(1, sampleRate)
  const frequencies = Float64Array.from(frequenciesHz)
  const phi = new Float64Array(frequencies.length)
  for (let index = 0; index < frequencies.length; index += 1) {
    const half = (Math.PI * frequencies[index]) / rate
    phi[index] = Math.sin(half) ** 2
  }
  return { frequenciesHz: frequencies, phi }
}

/** `count` frequencies from `minHz` to `maxHz`, evenly spaced in pitch. */
export function logFrequencies(
  count: number,
  minHz = 20,
  maxHz = 20_000
): Float64Array {
  const frequencies = new Float64Array(count)
  const ratio = maxHz / minHz
  for (let index = 0; index < count; index += 1) {
    frequencies[index] = minHz * ratio ** (index / Math.max(1, count - 1))
  }
  return frequencies
}

/**
 * `|c0 + c1 z^-1 + c2 z^-2|^2` on the unit circle, written in terms of
 * `phi = sin^2(w / 2)`. The textbook form in `cos(w)` subtracts nearly
 * equal numbers at low frequencies and loses most of its digits there.
 */
function power(c0: number, c1: number, c2: number, phi: number) {
  const sum = c0 + c1 + c2
  return (
    sum * sum -
    4 * (c0 * c1 + 4 * c0 * c2 + c1 * c2) * phi +
    16 * c0 * c2 * phi * phi
  )
}

/** Gain of one section in dB at `phi = sin^2(w / 2)`. */
export function sectionGainDb(section: Section, phi: number): number {
  const numerator = power(section.b0, section.b1, section.b2, phi)
  const denominator = power(1, section.a1, section.a2, phi)
  const magnitude = Math.sqrt(
    Math.max(0, numerator) / Math.max(1e-300, denominator)
  )
  return 20 * Math.log10(Math.max(MAGNITUDE_FLOOR, magnitude))
}

function addSections(
  sections: Section[],
  grid: ResponseGrid,
  out: Float32Array
) {
  for (const section of sections) {
    for (let index = 0; index < out.length; index += 1) {
      out[index] += sectionGainDb(section, grid.phi[index])
    }
  }
}

/**
 * What one band alone does, in dB at each frequency of the grid. All zero
 * for a band that is off.
 */
export function bandResponseDb(
  params: EqParams,
  id: EqBandId,
  sampleRate: number,
  grid: ResponseGrid,
  out = new Float32Array(grid.phi.length)
): Float32Array {
  out.fill(0)
  addSections(bandSections(params, id, sampleRate), grid, out)
  return out
}

/** The whole equaliser, output gain included, in dB at each frequency. */
export function eqResponseDb(
  params: EqParams,
  sampleRate: number,
  grid: ResponseGrid,
  out = new Float32Array(grid.phi.length)
): Float32Array {
  out.fill(params.outputGainDb)
  for (const id of EQ_BAND_IDS) {
    addSections(bandSections(params, id, sampleRate), grid, out)
  }
  return out
}

/** Gain of the whole equaliser at one frequency, in dB. */
export function eqGainAt(
  params: EqParams,
  sampleRate: number,
  frequencyHz: number
): number {
  return eqResponseDb(
    params,
    sampleRate,
    responseGrid([frequencyHz], sampleRate)
  )[0]
}
