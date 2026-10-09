import { GAIN_PRESETS } from "./gain-presets"

export function nextClipGainPreset(
  gain: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(gain)) return null

  const index = GAIN_PRESETS.findIndex(
    (item) => Math.abs(gain - item.gain) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return GAIN_PRESETS[index + step]?.gain ?? null
  }

  let previous: number | null = null
  for (const item of GAIN_PRESETS) {
    if (item.gain > gain) {
      return direction === "previous" ? previous : item.gain
    }
    previous = item.gain
  }
  return direction === "previous" ? previous : null
}
