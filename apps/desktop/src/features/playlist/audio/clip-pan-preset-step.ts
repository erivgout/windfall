import { PAN_PRESETS } from "./pan-presets"

export function nextClipPanPreset(
  pan: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(pan)) return null

  const index = PAN_PRESETS.findIndex(
    (item) => Math.abs(pan - item.pan) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return PAN_PRESETS[index + step]?.pan ?? null
  }

  let previous: number | null = null
  for (const item of PAN_PRESETS) {
    if (item.pan > pan) {
      return direction === "previous" ? previous : item.pan
    }
    previous = item.pan
  }
  return direction === "previous" ? previous : null
}
