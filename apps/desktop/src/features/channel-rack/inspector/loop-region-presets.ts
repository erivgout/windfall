export const LOOP_REGION_PRESETS = [
  { label: "Whole", start: 0, end: 1 },
  { label: "First half", start: 0, end: 0.5 },
  { label: "Second half", start: 0.5, end: 1 },
  { label: "Last quarter", start: 0.75, end: 1 },
] as const

export function nextLoopRegion(
  start: number,
  end: number,
  preset: { start: number; end: number }
): { start: number; end: number } | null {
  return Math.abs(start - preset.start) < 0.001 &&
    Math.abs(end - preset.end) < 0.001
    ? null
    : preset
}
