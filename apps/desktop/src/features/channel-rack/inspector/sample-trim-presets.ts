export const SAMPLE_TRIM_PRESETS = [
  { label: "Whole", start: 0, end: 1 },
  { label: "First half", start: 0, end: 0.5 },
  { label: "Second half", start: 0.5, end: 1 },
  { label: "Middle", start: 0.25, end: 0.75 },
] as const

export function nextSampleTrim(
  start: number,
  end: number,
  preset: { start: number; end: number }
): { start: number; end: number } | null {
  return Math.abs(start - preset.start) < 0.001 &&
    Math.abs(end - preset.end) < 0.001
    ? null
    : preset
}
