export const METRONOME_GAINS = [
  { label: "Quiet", value: 0.25 },
  { label: "Medium", value: 0.5 },
  { label: "Loud", value: 1 },
] as const

export function nextMetronomeGain(current: number, preset: number): number | null {
  return Math.abs(current - preset) < 0.001 ? null : preset
}
