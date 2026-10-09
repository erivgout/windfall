import { MAX_SONG_TICKS } from "@/lib/units"

export function scaledWaveformLength(
  length: number,
  factor: "half" | "double"
): number {
  return factor === "half"
    ? Math.max(1, Math.floor(length / 2))
    : Math.min(MAX_SONG_TICKS, length * 2)
}

export function nextWaveformLengthScale(
  length: number,
  factor: "half" | "double"
): number | null {
  const next = scaledWaveformLength(length, factor)
  return next === length ? null : next
}
