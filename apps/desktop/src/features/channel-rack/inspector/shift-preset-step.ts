import { SHIFT_PRESETS } from "./shift-presets"

export function nextShiftPreset(
  ticks: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(ticks)) return null

  const index = SHIFT_PRESETS.findIndex((item) => ticks === item.ticks)
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return SHIFT_PRESETS[index + step]?.ticks ?? null
  }

  let previous: number | null = null
  for (const item of SHIFT_PRESETS) {
    if (item.ticks > ticks) {
      return direction === "previous" ? previous : item.ticks
    }
    previous = item.ticks
  }
  return direction === "previous" ? previous : null
}
