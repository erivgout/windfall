import { METRONOME_GAINS } from "./metronome-gain"

export function nextMetronomePreset(
  gain: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(gain)) return null

  const index = METRONOME_GAINS.findIndex(
    (preset) => Math.abs(gain - preset.value) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return METRONOME_GAINS[index + step]?.value ?? null
  }

  let previous: number | null = null
  for (const preset of METRONOME_GAINS) {
    if (preset.value > gain) return direction === "previous" ? previous : preset.value
    previous = preset.value
  }
  return direction === "previous" ? previous : null
}
