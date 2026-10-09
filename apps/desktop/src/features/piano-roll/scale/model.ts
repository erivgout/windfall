/** Highlight choices are separate from the editor's pitch-snapping policy. */
export const HIGHLIGHT_SCALES = [
  { id: "off", label: "Off", intervals: [] },
  { id: "major", label: "Major", intervals: [0, 2, 4, 5, 7, 9, 11] },
  {
    id: "natural-minor",
    label: "Natural minor",
    intervals: [0, 2, 3, 5, 7, 8, 10],
  },
  {
    id: "harmonic-minor",
    label: "Harmonic minor",
    intervals: [0, 2, 3, 5, 7, 8, 11],
  },
  {
    id: "pentatonic-major",
    label: "Pentatonic major",
    intervals: [0, 2, 4, 7, 9],
  },
] as const

export const PITCH_CLASSES = [
  "C",
  "C#",
  "D",
  "D#",
  "E",
  "F",
  "F#",
  "G",
  "G#",
  "A",
  "A#",
  "B",
] as const

export type HighlightScaleId = (typeof HIGHLIGHT_SCALES)[number]["id"]
export type ScaleHighlight = { root: number; scale: HighlightScaleId }

export function isHighlightScaleId(value: string): value is HighlightScaleId {
  return HIGHLIGHT_SCALES.some((choice) => choice.id === value)
}

/** Accepts MIDI pitches as well as pitch classes; Off includes every pitch. */
export function isInScale(key: number, choice: ScaleHighlight): boolean {
  if (choice.scale === "off") return true
  const interval = (((key - choice.root) % 12) + 12) % 12
  const definition = HIGHLIGHT_SCALES.find(
    (scale) => scale.id === choice.scale
  )!
  return definition.intervals.some((member) => member === interval)
}
