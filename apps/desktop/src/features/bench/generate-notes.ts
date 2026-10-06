import type { Note } from "@/bindings/Note"

export const TICKS_PER_STEP = 240
export const TICKS_PER_BEAT = 960
export const TICKS_PER_BAR = 3840
export const SONG_BARS = 200
export const SONG_TICKS = SONG_BARS * TICKS_PER_BAR
/** The 88 keys of a piano, A0 to C8. */
export const LOWEST_KEY = 21
export const HIGHEST_KEY = 108

/** mulberry32: small, fast, and the same sequence in every engine. */
export function seededRandom(seed: number): () => number {
  let state = seed >>> 0
  return () => {
    state = (state + 0x6d2b79f5) >>> 0
    let t = state
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

const MINOR_SCALE = [0, 2, 3, 5, 7, 8, 10]
// i, VI, III, VII in A minor, as semitones above A.
const PROGRESSION = [0, 8, 3, 10]
const TRIAD = [0, 3, 7]
const MAJOR_TRIAD = [0, 4, 7]
const ROOT = 9

function clampKey(key: number): number {
  let k = key
  while (k < LOWEST_KEY) k += 12
  while (k > HIGHEST_KEY) k -= 12
  return k
}

/**
 * A deterministic song-shaped note set: a bass line, held chords, a melody,
 * sixteenth-note arpeggios and a few notes scattered across the whole
 * keyboard, over a four-chord progression. Density is uniform per bar, so
 * 10,000 notes over 200 bars is 50 notes per bar.
 */
export function generateNotes(
  count: number,
  seed: number,
  bars = SONG_BARS
): Note[] {
  const random = seededRandom(seed)
  const pick = (max: number) => Math.floor(random() * max)
  const notes: Note[] = []
  const songTicks = bars * TICKS_PER_BAR

  for (let bar = 0; bar < bars; bar++) {
    const quota =
      Math.floor(((bar + 1) * count) / bars) - Math.floor((bar * count) / bars)
    const chord = PROGRESSION[bar % PROGRESSION.length]
    // The minor tonic is the only minor chord of the four.
    const triad = chord === 0 ? TRIAD : MAJOR_TRIAD
    const barStart = bar * TICKS_PER_BAR
    const chordTone = () => ROOT + chord + triad[pick(3)]
    const scaleTone = () => ROOT + MINOR_SCALE[pick(7)]

    for (let n = 0; n < quota; n++) {
      const voice = random()
      let key: number
      let start: number
      let length: number
      let velocity: number

      if (voice < 0.1) {
        // Bass: roots and fifths on the beat, held.
        key = 24 + ROOT + chord + (random() < 0.7 ? 0 : 7) + 12 * pick(2)
        start = pick(4) * TICKS_PER_BEAT
        length = (1 + pick(4)) * TICKS_PER_BEAT
        velocity = 0.7 + random() * 0.3
      } else if (voice < 0.4) {
        // Chords: mid-register chord tones, a beat to a bar long.
        key = 48 + chordTone() + 12 * pick(2)
        start = pick(8) * (TICKS_PER_BEAT / 2)
        length = (1 + pick(8)) * (TICKS_PER_BEAT / 2)
        velocity = 0.45 + random() * 0.3
      } else if (voice < 0.7) {
        // Melody: scale tones, eighths and sixteenths, some triplets.
        key = 60 + scaleTone() + 12 * pick(3)
        const triplet = random() < 0.12
        start = triplet ? pick(12) * 320 : pick(16) * TICKS_PER_STEP
        length = triplet ? 320 : (1 + pick(4)) * TICKS_PER_STEP
        velocity = 0.55 + random() * 0.45
      } else if (voice < 0.93) {
        // Arpeggio: chord tones on every sixteenth, short.
        key = 36 + chordTone() + 12 * pick(5)
        start = pick(16) * TICKS_PER_STEP
        length = random() < 0.8 ? TICKS_PER_STEP : TICKS_PER_STEP / 2
        velocity = 0.3 + random() * 0.5
      } else {
        // Anywhere on the keyboard, so every row gets used.
        key = LOWEST_KEY + pick(HIGHEST_KEY - LOWEST_KEY + 1)
        start = pick(32) * (TICKS_PER_STEP / 2)
        length = (1 + pick(6)) * (TICKS_PER_STEP / 2)
        velocity = 0.2 + random() * 0.8
      }

      const absolute = barStart + start
      notes.push({
        id: notes.length + 1,
        start: absolute,
        length: Math.max(60, Math.min(length, songTicks - absolute)),
        key: clampKey(key),
        velocity: Math.min(1, velocity),
        pan: 0,
      })
    }
  }
  return notes
}
