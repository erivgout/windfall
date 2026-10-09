import { TEMPO_PRESETS } from "./tempo-presets"

export function nextTempoPreset(
  tempo: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(tempo)) return null

  const index = TEMPO_PRESETS.findIndex(
    (preset) => Math.abs(tempo - preset) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return TEMPO_PRESETS[index + step] ?? null
  }

  let previous: number | null = null
  for (const preset of TEMPO_PRESETS) {
    if (preset > tempo) return direction === "previous" ? previous : preset
    previous = preset
  }
  return direction === "previous" ? previous : null
}
