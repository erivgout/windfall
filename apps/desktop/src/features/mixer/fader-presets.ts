export const FADER_PRESETS = [
  { label: "Quiet", value: 0.5 },
  { label: "Unity", value: 1 },
  { label: "Loud", value: 1.5 },
] as const

export function nextFaderVolume(current: number, preset: number): number | null {
  return Math.abs(current - preset) < 0.001 ? null : preset
}
