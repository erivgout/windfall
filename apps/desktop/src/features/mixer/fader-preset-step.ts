import { FADER_PRESETS } from "./fader-presets"

export function nextFaderPreset(
  volume: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(volume)) return null

  const index = FADER_PRESETS.findIndex(
    (preset) => Math.abs(volume - preset.value) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return FADER_PRESETS[index + step]?.value ?? null
  }

  let previous: number | null = null
  for (const preset of FADER_PRESETS) {
    if (preset.value > volume) {
      return direction === "previous" ? previous : preset.value
    }
    previous = preset.value
  }
  return direction === "previous" ? previous : null
}
