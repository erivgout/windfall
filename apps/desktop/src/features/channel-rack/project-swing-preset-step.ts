import { SWING_PRESETS } from "./swing-presets"

export function nextProjectSwingPreset(
  swing: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(swing)) return null

  const index = SWING_PRESETS.findIndex(
    (preset) => Math.abs(swing - preset.value) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return SWING_PRESETS[index + step]?.value ?? null
  }

  let previous: number | null = null
  for (const preset of SWING_PRESETS) {
    if (preset.value > swing) {
      return direction === "previous" ? previous : preset.value
    }
    previous = preset.value
  }
  return direction === "previous" ? previous : null
}
