import type { CompressorParams } from "@/bindings"

/*
 * The compressor's static curve, as `crates/windfall-dsp/src/compressor.rs`
 * computes it: level in dB through threshold, ratio and a parabolic soft
 * knee (Giannoulis, Massberg and Reiss, "Digital Dynamic Range Compressor
 * Design", JAES 2012).
 */

/** A ratio at or above this is infinity to one. */
export const COMPRESSOR_MAX_RATIO = 100

type Curve = Pick<CompressorParams, "thresholdDb" | "ratio" | "kneeDb">
type Makeup = Curve & Pick<CompressorParams, "makeupDb" | "autoMakeup">
type Transfer = Makeup & Pick<CompressorParams, "mix">

/**
 * The share of every dB over the threshold that is taken away: 0 at a
 * ratio of 1, 1 at infinity.
 */
export function reductionSlope(ratio: number): number {
  if (ratio >= COMPRESSOR_MAX_RATIO) return 1
  return 1 - 1 / Math.max(1, ratio)
}

/**
 * Reduction in dB, zero or more, for a signal `overDb` above the threshold.
 * Inside the knee the slope eases in along a parabola.
 */
export function reductionDb(
  overDb: number,
  slope: number,
  kneeDb: number
): number {
  if (2 * overDb <= -kneeDb) return 0
  if (2 * overDb < kneeDb) {
    const intoKnee = overDb + kneeDb / 2
    return (slope * intoKnee * intoKnee) / (2 * kneeDb)
  }
  return slope * overDb
}

/**
 * The gain change in dB, zero or negative, applied to a steady signal at
 * `levelDb`, before makeup gain.
 */
export function staticGainDb(params: Curve, levelDb: number): number {
  const reduction = reductionDb(
    levelDb - params.thresholdDb,
    reductionSlope(params.ratio),
    params.kneeDb
  )
  // Keeps a level below the knee at 0 and not at -0.
  return reduction === 0 ? 0 : -reduction
}

/**
 * The makeup gain in effect, in dB. Auto makeup adds half of what a
 * full-scale signal is turned down by.
 */
export function totalMakeupDb(params: Makeup): number {
  if (!params.autoMakeup) return params.makeupDb
  return params.makeupDb - staticGainDb(params, 0) / 2
}

/**
 * The level that comes out for a steady signal at `levelDb`, with makeup
 * and the compressor's own dry/wet mix: the dry signal and the compressed
 * one are in phase, so they add as amplitudes.
 */
export function outputLevelDb(params: Transfer, levelDb: number): number {
  const wetDb = staticGainDb(params, levelDb) + totalMakeupDb(params)
  const gain = 1 + (10 ** (wetDb / 20) - 1) * params.mix
  return levelDb + 20 * Math.log10(Math.max(1e-10, gain))
}

/** "4.0:1", "1.5:1", "20:1", and "∞:1" from the top of the range. */
export function formatRatio(ratio: number): string {
  if (ratio >= COMPRESSOR_MAX_RATIO) return "∞:1"
  return `${ratio >= 10 ? ratio.toFixed(0) : ratio.toFixed(1)}:1`
}
