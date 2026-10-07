/*
 * What each setting of the subtractive synth does, in one sentence for the
 * status bar. The three oscillators and the two LFOs share theirs, so the
 * keys here are the last part of a setting's id, or the whole id.
 */
const DESCRIPTIONS: Record<string, string> = {
  waveform: "The shape of the wave, which sets the raw tone",
  level: "How loud the oscillator is. At 0% it is off",
  coarse: "Tuning in semitones. 12 is an octave up",
  fineCents:
    "Fine tuning in cents. A few cents apart, two oscillators beat and thicken",
  "oscillator.pulseWidth":
    "How much of each cycle the pulse spends high. 50% is a square; either end is thinner",
  "oscillator.pan": "Where the oscillator sits between left and right",

  unisonVoices: "Copies of each oscillator stacked on every note",
  unisonDetuneCents: "How far apart the stacked copies are tuned",
  unisonSpread: "How wide the stacked copies sit between left and right",

  "filter.mode":
    "Which part of the sound the filter keeps: below, around or above the cutoff",
  "filter.slope":
    "How steeply the filter cuts. 24 dB per octave is darker and tighter",
  "filter.cutoffHz":
    "Where the filter starts to cut, before the envelope, keys and LFOs move it",
  "filter.resonance":
    "Emphasis at the cutoff. Towards 100% the filter rings and whistles",
  "filter.keyTracking":
    "How much the cutoff follows the key played. At 100% every note is equally bright",
  "filter.envelopeOctaves":
    "How far the filter envelope opens the cutoff. Below zero it closes it",
  "filter.velocity": "How much playing softly darkens the sound",
  "filter.drive": "Overdrive before the filter: louder, fuller and grittier",

  "ampEnvelope.attackMs": "How long a note takes to reach full volume",
  "ampEnvelope.decayMs": "How long it takes to fall to the sustain level",
  "ampEnvelope.sustain": "The volume held while the key stays down",
  "ampEnvelope.releaseMs": "How long a note rings on after the key is let go",
  "filterEnvelope.attackMs": "How long the filter takes to open fully",
  "filterEnvelope.decayMs":
    "How long the filter takes to fall back to its sustain level",
  "filterEnvelope.sustain":
    "How far the filter stays open while the key is held",
  "filterEnvelope.releaseMs":
    "How long the filter takes to close after the key is let go",

  shape: "The shape of the slow wave. Random jumps to a new value each cycle",
  rateHz: "How fast the LFO moves",
  pitchSemitones: "How far the LFO bends the pitch: vibrato",
  cutoffOctaves: "How far the LFO moves the filter cutoff",
  amp: "How much the LFO dips the volume: tremolo",
  "lfo.pulseWidth":
    "How far the LFO moves the pulse width of every pulse oscillator",

  voiceMode:
    "Poly plays chords. Mono plays one note at a time. Legato also glides between overlapping notes without restarting them",
  glideMs: "How long the pitch takes to slide from the previous note",
  polyphony:
    "The most notes that sound at once. One more takes over the oldest",
  ampVelocity:
    "How much playing softly lowers the volume. At 0% every note is at full level",
  gain: "The synth's output level, before the channel volume",
  pan: "Where the synth sits between left and right",
}

/** The status-bar sentence for a setting of the synth, if it has one. */
export function describeSynthParam(id: string): string | undefined {
  const parts = id.split(".")
  const last = parts[parts.length - 1]
  const family =
    parts[0] === "oscillators"
      ? "oscillator"
      : parts[0] === "lfos"
        ? "lfo"
        : null
  return (
    DESCRIPTIONS[id] ??
    (family === null ? undefined : DESCRIPTIONS[`${family}.${last}`]) ??
    DESCRIPTIONS[last]
  )
}
