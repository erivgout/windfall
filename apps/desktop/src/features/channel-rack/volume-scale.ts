import { MAX_GAIN } from "@/lib/units"

export function scaledChannelVolume(
  volume: number,
  factor: "half" | "double"
): number {
  return factor === "half" ? volume / 2 : Math.min(volume * 2, MAX_GAIN)
}

export function nextChannelVolumeScale(
  volume: number,
  factor: "half" | "double"
): number | null {
  const next = scaledChannelVolume(volume, factor)
  return Math.abs(next - volume) < 0.001 ? null : next
}
