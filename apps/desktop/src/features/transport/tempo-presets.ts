export const TEMPO_PRESETS = [80, 100, 120, 128, 140, 160, 174] as const

export function nextTempo(current: number, preset: number): number | null {
  return Math.abs(current - preset) < 0.001 ? null : preset
}
