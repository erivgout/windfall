import { clamp, DEFAULT_KEY } from "@/lib/units"

export function scaledWaveformCenter(
  key: number,
  factor: "half" | "double"
): number {
  const offset = key - DEFAULT_KEY
  return factor === "half"
    ? DEFAULT_KEY + Math.trunc(offset / 2)
    : clamp(DEFAULT_KEY + offset * 2, 0, 127)
}

export function nextWaveformCenterScale(
  key: number,
  factor: "half" | "double"
): number | null {
  const next = scaledWaveformCenter(key, factor)
  return next === key ? null : next
}
