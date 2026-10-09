import { SWING_PRESETS } from "./swing-presets"

export function nextSwingMixPreset(
  mix: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(mix)) return null

  const index = SWING_PRESETS.findIndex(
    (item) => Math.abs(mix - item.value) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return SWING_PRESETS[index + step]?.value ?? null
  }

  let previous: number | null = null
  for (const item of SWING_PRESETS) {
    if (item.value > mix) {
      return direction === "previous" ? previous : item.value
    }
    previous = item.value
  }
  return direction === "previous" ? previous : null
}
