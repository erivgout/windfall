import type { InstrumentParamsOf } from "@/features/params"
import { DEFAULT_CHANNEL_VOLUME, gainToDb } from "@/lib/units"

type SynthSettings = InstrumentParamsOf<"subtractiveSynth">
type Oscillator = SynthSettings["oscillators"][number]

/*
 * How loud one full-velocity note of the subtractive synth can get, worked
 * out from its settings. It follows a voice through `windfall-dsp`'s
 * `synth.rs` stage by stage and takes the worst case at each one, so the
 * real peak is at or below the answer. It is an upper bound to hold the
 * built-in sounds to, not a meter: only a render says what a sound peaks at.
 *
 * One thing is left out: the filter that brings the oversampled voices down
 * to the output rate can overshoot a hard edge by a few percent. That only
 * shows with the synth's own filter wide open, which is the Init sound, and
 * a render of Init came out under this bound all the same.
 */

/** The filter's Q with resonance at 1 (`MAX_RESONANCE_Q` in `synth.rs`). */
const MAX_RESONANCE_Q = 20
/** The drive stage's input gain at its lowest and its range (`DRIVE_FLOOR`, `DRIVE_RANGE`). */
const DRIVE_FLOOR = 0.02
const DRIVE_RANGE = 250

/**
 * The furthest an oscillator's waveform gets from zero. A pulse has its
 * average taken off (`pulse` in `blocks/oscillator.rs`), which lifts the
 * narrow half: a pulse that is high 30% of the time swings from -0.6 to 1.4.
 */
function wavePeak(oscillator: Oscillator): number {
  if (oscillator.waveform !== "pulse") return 1
  const width = oscillator.pulseWidth
  return Math.max(2 * width, 2 - 2 * width)
}

/**
 * The most the unison copies of one oscillator add up to in one channel,
 * when all of them peak together. `Patch::new` gives each of `voices`
 * copies `1 / sqrt(voices)` of the level and spreads them across the
 * stereo field, and `pan_gains` only ever turns the far side down.
 */
function unisonSum(voices: number, spread: number, pan: number): number {
  const count = Math.min(7, Math.max(1, Math.round(voices)))
  const share = 1 / Math.sqrt(count)
  let left = 0
  let right = 0
  for (let copy = 0; copy < count; copy += 1) {
    const position = count > 1 ? (2 * copy) / (count - 1) - 1 : 0
    const place = Math.min(1, Math.max(-1, pan + spread * position))
    left += place > 0 ? 1 - place : 1
    right += place < 0 ? 1 + place : 1
  }
  return share * Math.max(left, right)
}

/** `soft_clip` in `blocks/shaper.rs`: it rises all the way and never passes 1. */
function softClip(input: number): number {
  const x = Math.min(3, Math.max(-3, input))
  return Math.min(1, Math.max(-1, (x * (27 + x * x)) / (27 + 9 * x * x)))
}

/**
 * What the drive stage makes of a peak. Its gains are those of
 * `Patch::new`: quiet signals come out louder the further drive is turned
 * up, by about 4 dB at 0.3, and the curve in between only rises, so the
 * largest input gives the largest output.
 */
function driven(peak: number, drive: number): number {
  if (drive <= 0) return peak
  const gainIn = DRIVE_FLOOR * DRIVE_RANGE ** drive
  const gainOut = (DRIVE_FLOOR / gainIn) ** 0.7 / DRIVE_FLOOR
  return softClip(peak * gainIn) * gainOut
}

/**
 * The most one stage of the filter can raise a peak by, whatever goes in:
 * the sum of its impulse response's swings. For a resonant low-pass that is
 * `(1 + r) / (1 - r)`, where `r` is how much of each swing of its ringing
 * is left half a cycle later. A square wave at the cutoff comes close to
 * it. A filter damped too far to ring raises nothing; the synth's is not,
 * even with resonance at 0, where it still overshoots an edge by 4%.
 */
function stageBoost(q: number): number {
  const damping = 1 / (2 * q)
  if (damping >= 1) return 1
  const ring = Math.exp((-Math.PI * damping) / Math.sqrt(1 - damping ** 2))
  return (1 + ring) / (1 - ring)
}

/**
 * The most the filter can raise a peak by. `Patch::new` turns its input
 * down by `1 / (1 + resonance)` and, for the 24 dB slope, runs two stages
 * that share the Q between them.
 */
function filterBoost(filter: SynthSettings["filter"]): number {
  const q = Math.SQRT1_2 * (MAX_RESONANCE_Q / Math.SQRT1_2) ** filter.resonance
  const stages = filter.slope === "db24" ? 2 : 1
  const boost = stageBoost(stages === 2 ? Math.sqrt(q) : q) ** stages
  // The band-pass and high-pass outputs are the input less what the
  // low-pass keeps, so they can reach one more than it does.
  const shaped = filter.mode === "lowPass" ? boost : boost + 1
  return shaped / (1 + filter.resonance)
}

/**
 * An upper bound on the peak of one full-velocity note at the synth's own
 * output, as linear gain: every oscillator and unison copy at its extreme
 * at once, through the drive and the filter's worst case, at the top of the
 * volume envelope, times the synth's volume.
 */
export function synthPeak(params: SynthSettings): number {
  const mixed = params.oscillators.reduce(
    (sum, oscillator) =>
      sum +
      oscillator.level *
        wavePeak(oscillator) *
        // Noise is not stacked: `Patch::new` gives it one copy.
        (oscillator.waveform === "whiteNoise" ||
        oscillator.waveform === "pinkNoise"
          ? 1
          : unisonSum(
              params.unisonVoices,
              params.unisonSpread,
              oscillator.pan
            )),
    0
  )
  return (
    driven(mixed, params.filter.drive) *
    filterBoost(params.filter) *
    params.gain
  )
}

/**
 * The bound for a note on a new channel as it reaches the master: through
 * a channel at its default volume and a mixer track at 0 dB.
 */
export function notePeak(params: SynthSettings): number {
  return synthPeak(params) * DEFAULT_CHANNEL_VOLUME
}

/** How many notes can sound at once: one for the mono and legato modes. */
function voicesOf(params: SynthSettings, notes: number): number {
  return params.voiceMode === "poly" ? Math.min(notes, params.polyphony) : 1
}

/** `notePeak` in dBFS. */
export function notePeakDb(params: SynthSettings): number {
  return gainToDb(notePeak(params))
}

/**
 * The bound for a chord of `notes` keys in dBFS: every note at its peak at
 * the same instant. A sound that plays one note at a time plays one.
 */
export function chordPeakDb(params: SynthSettings, notes: number): number {
  return gainToDb(notePeak(params) * voicesOf(params, notes))
}
