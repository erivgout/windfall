export const PITCH_NAMES = [
  "C",
  "C♯",
  "D",
  "D♯",
  "E",
  "F",
  "F♯",
  "G",
  "G♯",
  "A",
  "A♯",
  "B",
] as const

export const TRIADS = [
  { quality: "major", label: "Major", intervals: [0, 4, 7] },
  { quality: "minor", label: "Minor", intervals: [0, 3, 7] },
  { quality: "diminished", label: "Diminished", intervals: [0, 3, 6] },
  { quality: "augmented", label: "Augmented", intervals: [0, 4, 8] },
  { quality: "sus2", label: "Suspended second", intervals: [0, 2, 7] },
  { quality: "sus4", label: "Suspended fourth", intervals: [0, 5, 7] },
] as const

export type TriadQuality = (typeof TRIADS)[number]["quality"]
export type ChordDetection = {
  kind: "triad" | "unknown" | "empty"
  label: string
  root: number | null
  quality: TriadQuality | null
  pitchClasses: number[]
}

export function pitchClass(key: number): number {
  return ((key % 12) + 12) % 12
}

/** Ignores voicing, inversions and octave doublings. Ambiguities use the
 * lowest pitch-class root, then the order of TRIADS, regardless of input order. */
export function detectChord(keys: readonly number[]): ChordDetection {
  const pitchClasses = [
    ...new Set(keys.filter(Number.isInteger).map(pitchClass)),
  ].sort((a, b) => a - b)
  if (!pitchClasses.length)
    return {
      kind: "empty",
      label: "No notes selected",
      root: null,
      quality: null,
      pitchClasses,
    }
  if (pitchClasses.length === 3) {
    for (const root of pitchClasses) {
      for (const triad of TRIADS) {
        if (
          triad.intervals.every((interval) =>
            pitchClasses.includes(pitchClass(root + interval))
          )
        ) {
          return {
            kind: "triad",
            label: `${PITCH_NAMES[root]} ${triad.label.toLowerCase()}`,
            root,
            quality: triad.quality,
            pitchClasses,
          }
        }
      }
    }
  }
  return {
    kind: "unknown",
    label: `Unknown chord (${pitchClasses.map((key) => PITCH_NAMES[key]).join(", ")})`,
    root: null,
    quality: null,
    pitchClasses,
  }
}
