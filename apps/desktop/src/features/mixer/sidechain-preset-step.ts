import { SIDECHAIN_PRESETS } from "./sidechain-presets"

export function nextSidechainPreset(
  gain: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(gain)) return null

  const index = SIDECHAIN_PRESETS.findIndex(
    (item) => Math.abs(gain - item.value) < 0.001
  )
  if (index !== -1) {
    const nextIndex = index + (direction === "previous" ? -1 : 1)
    return SIDECHAIN_PRESETS[nextIndex]?.value ?? null
  }

  let previous: number | null = null
  for (const item of SIDECHAIN_PRESETS) {
    if (item.value > gain) {
      return direction === "previous" ? previous : item.value
    }
    previous = item.value
  }
  return direction === "previous" ? previous : null
}
