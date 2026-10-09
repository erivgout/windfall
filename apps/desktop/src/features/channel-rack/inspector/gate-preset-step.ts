import { GATE_PRESETS } from "./gate-presets"

export function nextGatePreset(
  ticks: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(ticks)) return null

  const index = GATE_PRESETS.findIndex((item) => ticks === item.ticks)
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return GATE_PRESETS[index + step]?.ticks ?? null
  }

  let previous: number | null = null
  for (const item of GATE_PRESETS) {
    if (item.ticks > ticks) {
      return direction === "previous" ? previous : item.ticks
    }
    previous = item.ticks
  }
  return direction === "previous" ? previous : null
}
