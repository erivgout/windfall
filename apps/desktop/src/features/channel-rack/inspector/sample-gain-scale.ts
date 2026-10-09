import { MAX_GAIN } from "@/lib/units"

export function scaledSampleGain(
  gain: number,
  factor: "half" | "double"
): number {
  return factor === "half" ? gain / 2 : Math.min(gain * 2, MAX_GAIN)
}

export function nextSampleGainScale(
  gain: number,
  factor: "half" | "double"
): number | null {
  const next = scaledSampleGain(gain, factor)
  return Math.abs(next - gain) < 0.001 ? null : next
}
