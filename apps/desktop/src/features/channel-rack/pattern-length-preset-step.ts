import { LENGTH_PRESETS } from "./actions"

export function nextPatternLengthPreset(
  steps: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(steps)) return null

  const index = LENGTH_PRESETS.findIndex((preset) => preset === steps)
  if (index !== -1) {
    const offset = direction === "previous" ? -1 : 1
    return LENGTH_PRESETS[index + offset] ?? null
  }

  let previous: number | null = null
  for (const preset of LENGTH_PRESETS) {
    if (preset > steps) {
      return direction === "previous" ? previous : preset
    }
    previous = preset
  }
  return direction === "previous" ? previous : null
}
