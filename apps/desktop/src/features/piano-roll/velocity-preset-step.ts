import { VELOCITY_PRESETS } from "./velocity-presets"

export function nextNoteVelocityPreset(
  velocity: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(velocity)) return null

  const index = VELOCITY_PRESETS.findIndex(
    (preset) => Math.abs(velocity - preset.velocity) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return VELOCITY_PRESETS[index + step]?.velocity ?? null
  }

  let previous: number | null = null
  for (const preset of VELOCITY_PRESETS) {
    if (preset.velocity > velocity) {
      return direction === "previous" ? previous : preset.velocity
    }
    previous = preset.velocity
  }
  return direction === "previous" ? previous : null
}
