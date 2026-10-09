export const CHANNEL_PAN_PRESETS = [
  { label: "Hard left", value: -1 },
  { label: "Left", value: -0.5 },
  { label: "Center", value: 0 },
  { label: "Right", value: 0.5 },
  { label: "Hard right", value: 1 },
] as const

export function nextChannelPan(current: number, preset: number): number | null {
  return Math.abs(current - preset) < 0.001 ? null : preset
}
