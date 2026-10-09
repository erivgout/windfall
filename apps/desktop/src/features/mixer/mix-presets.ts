export const MIX_PRESETS = [
  { label: "Dry", value: 0 },
  { label: "25%", value: 0.25 },
  { label: "Half", value: 0.5 },
  { label: "75%", value: 0.75 },
  { label: "Wet", value: 1 },
] as const

export function nextMix(current: number, preset: number): number | null {
  return Math.abs(current - preset) < 0.001 ? null : preset
}
