import { SAMPLE_GAIN_PRESETS } from "./sample-gain-presets"

export function nextSampleGainPreset(
  gain: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(gain)) return null

  const index = SAMPLE_GAIN_PRESETS.findIndex(
    (item) => Math.abs(gain - item.gain) < 0.001
  )
  if (index !== -1) {
    const nextIndex = index + (direction === "previous" ? -1 : 1)
    return SAMPLE_GAIN_PRESETS[nextIndex]?.gain ?? null
  }

  let previous: number | null = null
  for (const item of SAMPLE_GAIN_PRESETS) {
    if (item.gain > gain) {
      return direction === "previous" ? previous : item.gain
    }
    previous = item.gain
  }
  return direction === "previous" ? previous : null
}
