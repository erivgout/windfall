export const SWING_PRESETS = [
  { label: "Straight", value: 0 },
  { label: "Light", value: 0.25 },
  { label: "Medium", value: 0.5 },
  { label: "Heavy", value: 0.75 },
  { label: "Full", value: 1 },
] as const

export function nextSwing(
  current: number,
  preset: { value: number }
): number | null {
  return Math.abs(current - preset.value) < 0.001 ? null : preset.value
}
