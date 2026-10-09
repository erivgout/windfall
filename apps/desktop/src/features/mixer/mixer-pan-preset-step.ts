import { PAN_PRESETS } from "./pan-presets"

export function nextMixerPanPreset(
  pan: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(pan)) return null

  const index = PAN_PRESETS.findIndex(
    (preset) => Math.abs(pan - preset.value) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return PAN_PRESETS[index + step]?.value ?? null
  }

  let previous: number | null = null
  for (const preset of PAN_PRESETS) {
    if (preset.value > pan) {
      return direction === "previous" ? previous : preset.value
    }
    previous = preset.value
  }
  return direction === "previous" ? previous : null
}
