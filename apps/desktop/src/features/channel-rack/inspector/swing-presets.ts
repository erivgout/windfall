export const SWING_PRESETS = [
  { label: "Straight", value: 0 },
  { label: "Light", value: 0.25 },
  { label: "Half", value: 0.5 },
  { label: "Full", value: 1 },
] as const

export function nextSwing(current: number, preset: number): number | null {
  return Math.abs(current - preset) < 0.001 ? null : preset
}
