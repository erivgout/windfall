import type { EffectSlot } from "@/bindings"

/**
 * The frames a limiter delays its signal by: its look-ahead at the sample
 * rate, and never less than one frame. This is `latency_samples` in
 * `crates/windfall-dsp/src/limiter.rs`.
 */
export function lookaheadFrames(lookaheadMs: number, sampleRate: number) {
  return Math.max(1, Math.round((lookaheadMs * Math.max(1, sampleRate)) / 1000))
}

/**
 * The frames a chain of effects delays its track by. Only a limiter adds
 * any, and it adds the same whether it is on, off or half mixed.
 */
export function chainLatencyFrames(
  effects: readonly EffectSlot[],
  sampleRate: number
): number {
  let frames = 0
  for (const slot of effects) {
    if (slot.params.type === "limiter") {
      frames += lookaheadFrames(slot.params.lookaheadMs, sampleRate)
    }
  }
  return frames
}

/** Frames as milliseconds at a sample rate. */
export function framesToMs(frames: number, sampleRate: number): number {
  return (frames * 1000) / Math.max(1, sampleRate)
}

/** "5.0 ms (240 samples)". */
export function formatLatency(frames: number, sampleRate: number): string {
  const ms = framesToMs(frames, sampleRate)
  const unit = frames === 1 ? "sample" : "samples"
  return `${ms.toFixed(ms < 10 ? 1 : 0)} ms (${frames} ${unit})`
}
