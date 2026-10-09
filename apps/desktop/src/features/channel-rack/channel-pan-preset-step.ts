import { CHANNEL_PAN_PRESETS } from "./pan-presets"

export function nextChannelPanPreset(
  pan: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(pan)) return null

  const index = CHANNEL_PAN_PRESETS.findIndex(
    (preset) => Math.abs(pan - preset.value) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return CHANNEL_PAN_PRESETS[index + step]?.value ?? null
  }

  let previous: number | null = null
  for (const preset of CHANNEL_PAN_PRESETS) {
    if (preset.value > pan) {
      return direction === "previous" ? previous : preset.value
    }
    previous = preset.value
  }
  return direction === "previous" ? previous : null
}
