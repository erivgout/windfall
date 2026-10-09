import { MAX_GAIN } from "@/lib/units"

export function scaledFaderVolume(
  volume: number,
  factor: "half" | "double"
): number {
  return factor === "half" ? volume / 2 : Math.min(volume * 2, MAX_GAIN)
}

export function nextFaderVolumeScale(
  volume: number,
  factor: "half" | "double"
): number | null {
  const next = scaledFaderVolume(volume, factor)
  return Math.abs(next - volume) < 0.001 ? null : next
}
