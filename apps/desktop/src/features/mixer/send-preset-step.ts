import { SEND_PRESETS } from "./send-presets"

export function nextSendPreset(
  gain: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(gain)) return null

  const current = SEND_PRESETS.findIndex(
    (preset) => Math.abs(gain - preset.value) < 0.001
  )
  if (current !== -1) {
    const index = current + (direction === "previous" ? -1 : 1)
    return SEND_PRESETS[index]?.value ?? null
  }

  let previous: number | null = null
  for (const preset of SEND_PRESETS) {
    if (direction === "next" && preset.value > gain) return preset.value
    if (preset.value < gain) previous = preset.value
  }
  return direction === "previous" ? previous : null
}
