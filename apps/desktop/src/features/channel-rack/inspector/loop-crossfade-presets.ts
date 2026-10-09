export const LOOP_CROSSFADE_PRESETS = [
  { label: "None", crossfade: 0 },
  { label: "Short", crossfade: 0.25 },
  { label: "Medium", crossfade: 0.5 },
  { label: "Long", crossfade: 1 },
] as const

export function nextLoopCrossfade(
  current: number,
  preset: number
): number | null {
  return Math.abs(current - preset) < 0.001 ? null : preset
}
