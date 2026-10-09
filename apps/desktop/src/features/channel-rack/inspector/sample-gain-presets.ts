export const SAMPLE_GAIN_PRESETS = [
  { label: "Quiet", gain: 0.5 },
  { label: "Unity", gain: 1 },
  { label: "Loud", gain: 1.5 },
] as const

export function nextSampleGain(current: number, preset: number): number | null {
  return Math.abs(current - preset) < 0.001 ? null : preset
}
