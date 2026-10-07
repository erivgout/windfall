import { describe, expect, it } from "vitest"

import { instrumentDescriptor } from "@/features/params"
import { DEFAULT_CHANNEL_VOLUME } from "@/lib/units"

import { chordPeakDb, notePeak, notePeakDb, synthPeak } from "./peak-estimate"
import { SYNTH_PRESETS, synthPreset, type SynthSettings } from "./presets"

/**
 * One full-velocity note may reach this by the bound, in dBFS. The bound
 * takes the worst case at every stage. It was -9 while the gains were set
 * by the bound alone; matched to renders, the Soft pad's bound is -7.3.
 */
const NOTE_LIMIT_DB = -7
/** Where one full-velocity note of every sound is meant to peak, in dBFS. */
const NOTE_TARGET_DB = -14
const NOTE_TOLERANCE_DB = 1
/** A chord of four full-velocity notes must stay at or under this, in dBFS. */
const CHORD_LIMIT_DB = -6
const CHORD_NOTES = 4
/** Full scale, which nothing may reach. */
const FULL_SCALE_DB = 0

/**
 * What renders of the real engine measured, on a new channel with every
 * fader where it starts: the peak of one full-velocity C5 and of a chord of
 * four, in dBFS, with the `gain` each sound had when it was measured. The
 * synth's output scales with `gain`, so a sound's peak at another gain is
 * the measured one plus the change of the gain in dB.
 */
const MEASURED: Record<string, { gain: number; note: number; chord: number }> =
  {
    init: { gain: 0.25, note: -12.85, chord: -4.5 },
    softPad: { gain: 0.095, note: -18.98, chord: -13.15 },
    pluck: { gain: 0.13, note: -16.9, chord: -11.3 },
    bass: { gain: 0.07, note: -13.57, chord: -13.58 },
    lead: { gain: 0.15, note: -13.94, chord: -14.1 },
    keys: { gain: 0.17, note: -14.8, chord: -7.93 },
    brass: { gain: 0.095, note: -15.83, chord: -7.48 },
    sub: { gain: 0.29, note: -11.52, chord: -11.53 },
  }

/** How much louder a sound is now than when it was measured, in dB. */
function changeDb(id: string, params: SynthSettings): number {
  return 20 * Math.log10(params.gain / MEASURED[id].gain)
}

/** The sounds whose level is set here: all but Init, which is the core's. */
const OWN_SOUNDS = SYNTH_PRESETS.filter((sound) => sound.id !== "init").map(
  (sound) => [sound.name, sound.id, sound.params] as const
)

const defaults = instrumentDescriptor("subtractiveSynth").defaults

function preset(id: string): SynthSettings {
  const found = synthPreset(id)
  if (!found) throw new Error(`There is no sound called "${id}"`)
  return found.params
}

// What a render of the real engine measured: one C5 at full velocity, on a
// new channel with every fader where it starts.
const MEASURED_INIT_DB = -12.9
const MEASURED_OLD_BASS_DB = 0.41
/** The Bass as it shipped, when one note of it clipped. */
const oldBass: SynthSettings = { ...preset("bass"), gain: 0.35 }

describe("the level of the built-in sounds", () => {
  it.each(SYNTH_PRESETS.map((sound) => [sound.name, sound.params] as const))(
    "keeps one full-velocity note of %s at or under -7 dBFS by the worst case",
    (_, params) => {
      expect(notePeakDb(params)).toBeLessThanOrEqual(NOTE_LIMIT_DB)
    }
  )

  it("has a measurement for every sound", () => {
    expect(Object.keys(MEASURED).sort()).toEqual(
      SYNTH_PRESETS.map((sound) => sound.id).sort()
    )
  })

  it.each(OWN_SOUNDS)(
    "puts one full-velocity note of %s within 1 dB of -14 dBFS",
    (_, id, params) => {
      const peak = MEASURED[id].note + changeDb(id, params)
      expect(Math.abs(peak - NOTE_TARGET_DB)).toBeLessThanOrEqual(
        NOTE_TOLERANCE_DB
      )
    }
  )

  it("matches the sounds to each other: no two are more than 1 dB apart", () => {
    const peaks = OWN_SOUNDS.map(
      ([, id, params]) => MEASURED[id].note + changeDb(id, params)
    )
    expect(Math.max(...peaks) - Math.min(...peaks)).toBeLessThanOrEqual(1)
    // As they shipped before they were matched, they spanned 7.5 dB.
    const before = Object.values(MEASURED).map((sound) => sound.note)
    expect(Math.max(...before) - Math.min(...before)).toBeCloseTo(7.46, 2)
  })

  it.each(OWN_SOUNDS)(
    "keeps a chord of four notes of %s at or under -6 dBFS",
    (_, id, params) => {
      expect(MEASURED[id].chord + changeDb(id, params)).toBeLessThanOrEqual(
        CHORD_LIMIT_DB
      )
    }
  )

  it("has the init sound, whose level is the core's, at the limit for a chord", () => {
    // Init is the core's default settings, so its volume is not set here.
    // The bound puts four notes of it a hair over full scale (+0.005 dB),
    // and the bound is the worst case: the render came out 0.86 dB under
    // it, which leaves a real chord of four just under 0 dBFS.
    expect(preset("init")).toEqual(defaults)
    expect(chordPeakDb(defaults, CHORD_NOTES)).toBeLessThan(0.05)
    expect(MEASURED_INIT_DB + 20 * Math.log10(CHORD_NOTES)).toBeLessThan(
      FULL_SCALE_DB
    )
    // The measured chord of four is 4.5 dB under full scale.
    expect(MEASURED.init.chord).toBeLessThan(FULL_SCALE_DB)
  })

  it("counts one note for a sound that plays one at a time", () => {
    for (const id of ["bass", "lead", "sub"]) {
      expect(chordPeakDb(preset(id), CHORD_NOTES)).toBe(notePeakDb(preset(id)))
    }
    expect(chordPeakDb(defaults, CHORD_NOTES)).toBeCloseTo(
      notePeakDb(defaults) + 20 * Math.log10(CHORD_NOTES),
      6
    )
  })
})

describe("the bound the sounds are held to", () => {
  it("is not under what a render of the real engine measured", () => {
    expect(notePeakDb(defaults)).toBeGreaterThanOrEqual(MEASURED_INIT_DB)
    expect(notePeakDb(oldBass)).toBeGreaterThanOrEqual(MEASURED_OLD_BASS_DB)
  })

  it.each(
    SYNTH_PRESETS.map((sound) => [sound.name, sound.id, sound.params] as const)
  )(
    "is not under the measured note or chord of %s",
    (_, id, params) => {
      const change = changeDb(id, params)
      expect(notePeakDb(params)).toBeGreaterThanOrEqual(
        MEASURED[id].note + change
      )
      expect(chordPeakDb(params, CHORD_NOTES)).toBeGreaterThanOrEqual(
        MEASURED[id].chord + change
      )
    }
  )

  it("would have caught the Bass that clipped on one note", () => {
    expect(notePeakDb(oldBass)).toBeGreaterThan(FULL_SCALE_DB)
    expect(notePeakDb(oldBass)).toBeGreaterThan(NOTE_LIMIT_DB)
  })

  it("follows the synth's volume and the channel's default volume", () => {
    expect(synthPeak({ ...defaults, gain: 0.5 })).toBeCloseTo(
      2 * synthPeak(defaults),
      9
    )
    expect(notePeak(defaults)).toBeCloseTo(
      synthPeak(defaults) * DEFAULT_CHANNEL_VOLUME,
      9
    )
  })

  it("adds the oscillators up, and a narrow pulse reaches further than a square", () => {
    const [first, second, third] = defaults.oscillators
    const withSquare: SynthSettings = {
      ...defaults,
      oscillators: [
        first,
        { ...second, waveform: "square", level: 0.5 },
        third,
      ],
    }
    expect(synthPeak(withSquare)).toBeCloseTo(1.5 * synthPeak(defaults), 9)
    const withPulse: SynthSettings = {
      ...defaults,
      oscillators: [
        first,
        { ...second, waveform: "pulse", level: 0.5, pulseWidth: 0.3 },
        third,
      ],
    }
    // High 30% of the time, the pulse swings up to 1.4.
    expect(synthPeak(withPulse)).toBeCloseTo(1.7 * synthPeak(defaults), 9)
  })

  it("grows with unison by the square root of the copies, less what the spread takes", () => {
    const stacked = (unisonSpread: number): SynthSettings => ({
      ...defaults,
      unisonVoices: 4,
      unisonSpread,
    })
    expect(synthPeak(stacked(0))).toBeCloseTo(2 * synthPeak(defaults), 9)
    expect(synthPeak(stacked(1))).toBeLessThan(synthPeak(stacked(0)))
    expect(synthPeak(stacked(1))).toBeGreaterThan(synthPeak(defaults))
  })

  it("rises with resonance and with drive, as the sound does", () => {
    const filtered = (filter: Partial<SynthSettings["filter"]>) =>
      synthPeak({ ...defaults, filter: { ...defaults.filter, ...filter } })
    expect(filtered({ resonance: 0.5 })).toBeGreaterThan(filtered({}))
    expect(filtered({ resonance: 0.9 })).toBeGreaterThan(
      filtered({ resonance: 0.5 })
    )
    // Two stages that share the Q can swing further than one with all of it.
    expect(filtered({ resonance: 0.25, slope: "db24" })).toBeGreaterThan(
      filtered({ resonance: 0.25 })
    )
    expect(filtered({ drive: 0.3 })).toBeGreaterThan(filtered({}))
    // With resonance at 0 the filter still overshoots an edge, by 4%.
    expect(filtered({ resonance: 0 })).toBeGreaterThan(defaults.gain)
    expect(filtered({ resonance: 0 })).toBeLessThan(1.1 * defaults.gain)
  })
})
