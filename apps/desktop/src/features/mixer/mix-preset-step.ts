import { MIX_PRESETS } from "./mix-presets"

export function nextMixPreset(
  mix: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(mix)) return null

  const index = MIX_PRESETS.findIndex(
    (item) => Math.abs(mix - item.value) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return MIX_PRESETS[index + step]?.value ?? null
  }

  let previous: number | null = null
  for (const item of MIX_PRESETS) {
    if (item.value > mix) {
      return direction === "previous" ? previous : item.value
    }
    previous = item.value
  }
  return direction === "previous" ? previous : null
}
