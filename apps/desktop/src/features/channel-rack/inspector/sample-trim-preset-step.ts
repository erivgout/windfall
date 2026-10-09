import { nextSampleTrim, SAMPLE_TRIM_PRESETS } from "./sample-trim-presets"

export function nextSampleTrimPreset(
  start: number,
  end: number,
  direction: "previous" | "next"
): { start: number; end: number } | null {
  const index = SAMPLE_TRIM_PRESETS.findIndex(
    (preset) => nextSampleTrim(start, end, preset) === null
  )
  if (index === -1) return null

  const step = direction === "previous" ? -1 : 1
  const preset = SAMPLE_TRIM_PRESETS[index + step]
  return preset ? { start: preset.start, end: preset.end } : null
}
