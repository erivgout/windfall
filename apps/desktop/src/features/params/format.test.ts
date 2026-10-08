import { describe, expect, it } from "vitest"

import type { ParamInfo, ParamUnit } from "@/bindings"
import { faderTaper } from "@/components/audio"

import {
  EFFECT_KINDS,
  effectDescriptor,
  INSTRUMENT_KINDS,
  instrumentDescriptor,
  paramInfo,
  type ParamDescriptor,
} from "./descriptors"
import { formatParam, paramIsBipolar, paramScale, parseParam } from "./format"

const synth = instrumentDescriptor("subtractiveSynth")
const compressor = effectDescriptor("compressor")
const reverb = effectDescriptor("reverb")
const delay = effectDescriptor("delay")
const eq = effectDescriptor("eq")
const limiter = effectDescriptor("limiter")

const ALL: ParamDescriptor[] = [
  ...EFFECT_KINDS.map((kind) => effectDescriptor(kind)),
  ...INSTRUMENT_KINDS.map((kind) => instrumentDescriptor(kind)),
]

function text(descriptor: ParamDescriptor, id: string, value: number) {
  return formatParam(paramInfo(descriptor, id), value)
}

function read(descriptor: ParamDescriptor, id: string, typed: string) {
  return parseParam(paramInfo(descriptor, id), typed)
}

describe("formatParam", () => {
  it("writes decibels with a sign where the range goes below zero", () => {
    expect(text(compressor, "thresholdDb", -18)).toBe("−18.0 dB")
    expect(text(compressor, "makeupDb", 3)).toBe("+3.0 dB")
    expect(text(compressor, "makeupDb", 0)).toBe("0.0 dB")
    expect(text(limiter, "ceilingDb", -0.3)).toBe("−0.3 dB")
    // A knee is a width, not an offset.
    expect(text(compressor, "kneeDb", 6)).toBe("6.0 dB")
  })

  it("writes a linear gain as decibels", () => {
    expect(text(synth, "gain", 1)).toBe("0.0 dB")
    expect(text(synth, "gain", 0.25)).toBe("−12.0 dB")
    expect(text(synth, "gain", 2)).toBe("+6.0 dB")
    expect(text(synth, "gain", 0)).toBe("−∞ dB")
  })

  it("writes frequencies in Hz and kHz, finer where the numbers are small", () => {
    expect(text(synth, "lfos.0.rateHz", 0.01)).toBe("0.01 Hz")
    expect(text(synth, "lfos.0.rateHz", 5)).toBe("5.00 Hz")
    expect(text(synth, "filter.cutoffHz", 20)).toBe("20.0 Hz")
    expect(text(synth, "filter.cutoffHz", 440)).toBe("440 Hz")
    expect(text(synth, "filter.cutoffHz", 1200)).toBe("1.20 kHz")
    expect(text(synth, "filter.cutoffHz", 20000)).toBe("20.0 kHz")
  })

  it("writes times in ms and s", () => {
    expect(text(compressor, "attackMs", 0.05)).toBe("0.05 ms")
    expect(text(synth, "ampEnvelope.attackMs", 0)).toBe("0 ms")
    expect(text(synth, "ampEnvelope.attackMs", 2)).toBe("2 ms")
    expect(text(synth, "ampEnvelope.attackMs", 2.5)).toBe("2.5 ms")
    expect(text(synth, "ampEnvelope.decayMs", 250)).toBe("250 ms")
    expect(text(synth, "ampEnvelope.releaseMs", 1250)).toBe("1.25 s")
    expect(text(synth, "ampEnvelope.releaseMs", 10000)).toBe("10.0 s")
    expect(text(delay, "stereoOffsetMs", 12)).toBe("+12 ms")
    expect(text(delay, "stereoOffsetMs", -12)).toBe("−12 ms")
    expect(text(reverb, "decayS", 1.8)).toBe("1.80 s")
    expect(text(reverb, "decayS", 0.1)).toBe("100 ms")
  })

  it("writes a fraction as a percentage", () => {
    expect(text(synth, "filter.resonance", 0.1)).toBe("10%")
    expect(text(synth, "unisonSpread", 0.699999988079071)).toBe("70%")
    expect(text(synth, "oscillators.0.pulseWidth", 0.5)).toBe("50%")
    expect(text(synth, "lfos.0.pulseWidth", 0.2)).toBe("+20%")
    expect(text(synth, "lfos.0.pulseWidth", -0.2)).toBe("−20%")
    expect(text(synth, "lfos.0.pulseWidth", 0)).toBe("0%")
  })

  it("reads rounded short modulation delays and signed feedback consistently", () => {
    const flanger = effectDescriptor("flanger")
    const phaser = effectDescriptor("phaser")
    for (const value of [1.02, 4.9999999, 5.02, 9.99]) {
      const shown = text(flanger, "delayMs", value)
      expect(text(flanger, "delayMs", read(flanger, "delayMs", shown)!)).toBe(
        shown
      )
    }
    expect(text(flanger, "feedback", -0.9)).toBe("−90%")
    expect(text(flanger, "feedback", 0.9)).toBe("+90%")
    expect(read(flanger, "feedback", "−70%")).toBeCloseTo(-0.7)
    expect(text(phaser, "feedback", 0.85)).toBe("+85%")
  })

  it("writes pitch offsets in semitones, cents and octaves", () => {
    expect(text(synth, "oscillators.0.coarse", 7)).toBe("+7 st")
    expect(text(synth, "oscillators.0.coarse", -12)).toBe("−12 st")
    expect(text(synth, "oscillators.0.coarse", 0)).toBe("0 st")
    expect(text(synth, "lfos.0.pitchSemitones", 0.25)).toBe("+0.25 st")
    expect(text(synth, "oscillators.0.fineCents", 5)).toBe("+5.0 ct")
    expect(text(synth, "unisonDetuneCents", 20)).toBe("20.0 ct")
    expect(text(synth, "filter.envelopeOctaves", 2)).toBe("+2.00 oct")
    expect(text(synth, "lfos.1.cutoffOctaves", -1.5)).toBe("−1.50 oct")
  })

  it("writes a ratio to one, and infinity at the top of the range", () => {
    expect(text(compressor, "ratio", 4)).toBe("4:1")
    expect(text(compressor, "ratio", 2.5)).toBe("2.5:1")
    expect(text(compressor, "ratio", 1)).toBe("1:1")
    expect(text(compressor, "ratio", 20)).toBe("20:1")
    expect(text(compressor, "ratio", 100)).toBe("∞:1")
  })

  it("writes pan as left, center and right", () => {
    expect(text(synth, "pan", 0)).toBe("C")
    expect(text(synth, "oscillators.1.pan", -0.3)).toBe("L30")
    expect(text(synth, "pan", 1)).toBe("R100")
  })

  it("writes plain numbers with digits that suit their size", () => {
    expect(text(synth, "polyphony", 16)).toBe("16")
    expect(text(synth, "unisonVoices", 3)).toBe("3")
    expect(text(eq, "peak1.q", 0.7071067690849304)).toBe("0.71")
    expect(text(eq, "peak1.q", 12)).toBe("12.0")
  })

  it("names toggles and choices", () => {
    expect(text(compressor, "autoMakeup", 1)).toBe("On")
    expect(text(compressor, "autoMakeup", 0)).toBe("Off")
    expect(text(synth, "filter.mode", 1)).toBe("Band-pass")
    expect(text(synth, "oscillators.0.waveform", 5)).toBe("White noise")
    expect(text(delay, "division", 8)).toBe("1/8")
  })

  it("covers every unit the bindings name", () => {
    const seen = new Set<ParamUnit>()
    for (const descriptor of ALL) {
      for (const info of descriptor.params) seen.add(info.unit)
    }
    const units: Record<ParamUnit, true> = {
      none: true,
      decibels: true,
      hertz: true,
      milliseconds: true,
      seconds: true,
      fraction: true,
      semitones: true,
      cents: true,
      octaves: true,
      ratio: true,
      gain: true,
      pan: true,
    }
    // Every unit is in use, so the tests above and below exercise them all.
    expect([...seen].sort()).toEqual(Object.keys(units).sort())
  })
})

describe("parseParam", () => {
  it("reads what the readout prints, and looser spellings", () => {
    expect(read(compressor, "thresholdDb", "-24")).toBe(-24)
    expect(read(compressor, "thresholdDb", "−24.0 dB")).toBe(-24)
    expect(read(synth, "gain", "-6 dB")).toBeCloseTo(0.501, 3)
    expect(read(synth, "gain", "-inf")).toBe(0)
    expect(read(synth, "filter.cutoffHz", "1.2k")).toBe(1200)
    expect(read(synth, "filter.cutoffHz", "440 Hz")).toBe(440)
    expect(read(synth, "ampEnvelope.decayMs", "1.5 s")).toBe(1500)
    expect(read(synth, "ampEnvelope.decayMs", "80")).toBe(80)
    expect(read(reverb, "decayS", "2.5")).toBe(2.5)
    expect(read(reverb, "decayS", "2.5 s")).toBe(2.5)
    expect(read(reverb, "decayS", "500 ms")).toBe(0.5)
    expect(read(synth, "filter.resonance", "35")).toBeCloseTo(0.35, 6)
    expect(read(synth, "filter.resonance", "35%")).toBeCloseTo(0.35, 6)
    expect(read(synth, "lfos.0.pulseWidth", "−20%")).toBeCloseTo(-0.2, 6)
    expect(read(synth, "oscillators.0.coarse", "+7 st")).toBe(7)
    expect(read(synth, "oscillators.0.fineCents", "-12,5")).toBe(-12.5)
    expect(read(synth, "filter.envelopeOctaves", "+2 oct")).toBe(2)
    expect(read(synth, "pan", "L30")).toBeCloseTo(-0.3, 6)
    expect(read(synth, "pan", "c")).toBe(0)
  })

  it("reads a ratio with or without the one, and infinity as the top", () => {
    expect(read(compressor, "ratio", "4:1")).toBe(4)
    expect(read(compressor, "ratio", "2.5")).toBe(2.5)
    expect(read(compressor, "ratio", "∞:1")).toBe(100)
    expect(read(compressor, "ratio", "inf")).toBe(100)
  })

  it("keeps the result inside the range and whole for an integer", () => {
    expect(read(synth, "filter.cutoffHz", "5")).toBe(20)
    expect(read(synth, "filter.cutoffHz", "99k")).toBe(20000)
    expect(read(synth, "polyphony", "7.6")).toBe(8)
    expect(read(synth, "polyphony", "900")).toBe(32)
    expect(read(compressor, "thresholdDb", "-inf")).toBe(-60)
  })

  it("reads toggles and choices by name", () => {
    expect(read(compressor, "autoMakeup", "On")).toBe(1)
    expect(read(compressor, "autoMakeup", "off")).toBe(0)
    expect(read(compressor, "autoMakeup", "maybe")).toBeNull()
    expect(read(synth, "filter.mode", "High-pass")).toBe(2)
    expect(read(synth, "filter.mode", "bandPass")).toBe(1)
    expect(read(synth, "oscillators.0.waveform", "tri")).toBe(1)
    expect(read(delay, "division", "1/8")).toBe(8)
    expect(read(delay, "division", "1/8 dotted")).toBe(7)
    expect(read(synth, "filter.mode", "notch")).toBeNull()
  })

  it("gives null for text with no value in it", () => {
    expect(read(synth, "filter.cutoffHz", "bright")).toBeNull()
    expect(read(synth, "gain", "")).toBeNull()
    expect(read(synth, "filter.mode", "  ")).toBeNull()
  })

  it("reads back the readout of every setting, to the precision shown", () => {
    for (const descriptor of ALL) {
      for (const info of descriptor.params) {
        for (const value of probes(info)) {
          const shown = formatParam(info, value)
          const back = parseParam(info, shown)
          expect(
            back,
            `${descriptor.name} ${info.id}: "${shown}"`
          ).not.toBeNull()
          // Printing what was read prints the same text again.
          expect(formatParam(info, back ?? Number.NaN)).toBe(shown)
        }
      }
    }
  })
})

/** Values across a setting's range, along the scale its knob uses. */
function probes(info: ParamInfo): number[] {
  if (info.kind === "toggle") return [0, 1]
  if (info.kind === "choice") return info.choices.map((_, index) => index)
  const shares = [0, 0.13, 0.5, 0.77, 1]
  const values = shares.map((share) =>
    info.scale === "logarithmic" && info.min > 0
      ? info.min * (info.max / info.min) ** share
      : info.min + share * (info.max - info.min)
  )
  values.push(info.default)
  return info.kind === "integer" ? values.map(Math.round) : values
}

describe("knob set-up", () => {
  it("spreads logarithmic settings evenly per octave", () => {
    expect(paramScale(paramInfo(synth, "filter.cutoffHz"))).toBe("log")
    expect(paramScale(paramInfo(compressor, "ratio"))).toBe("log")
    expect(paramScale(paramInfo(synth, "filter.resonance"))).toBe("linear")
    expect(paramScale(paramInfo(synth, "oscillators.0.coarse"))).toBe("linear")
  })

  it("gives the output level the fader's taper", () => {
    expect(paramScale(paramInfo(synth, "gain"))).toBe(faderTaper)
  })

  it("leaves room for the first milliseconds of a long time from zero", () => {
    const attack = paramInfo(synth, "ampEnvelope.attackMs")
    const scale = paramScale(attack)
    if (typeof scale === "string") throw new Error("expected a curve")
    // A quarter of the way round is well under a second of ten.
    expect(scale.fromNormalized(0.25, attack.min, attack.max)).toBeLessThan(200)
    expect(scale.fromNormalized(1, attack.min, attack.max)).toBe(10000)
    expect(paramScale(paramInfo(synth, "glideMs"))).toBe(scale)
    // A short time from zero stays linear.
    expect(paramScale(paramInfo(reverb, "preDelayMs"))).toBe("linear")
    expect(paramScale(paramInfo(delay, "stereoOffsetMs"))).toBe("linear")
  })

  it("draws offsets around zero from the middle out", () => {
    const bipolar = (descriptor: ParamDescriptor, id: string) =>
      paramIsBipolar(paramInfo(descriptor, id))
    expect(bipolar(synth, "pan")).toBe(true)
    expect(bipolar(synth, "oscillators.0.coarse")).toBe(true)
    expect(bipolar(synth, "filter.envelopeOctaves")).toBe(true)
    expect(bipolar(compressor, "makeupDb")).toBe(true)
    expect(bipolar(synth, "filter.cutoffHz")).toBe(false)
    expect(bipolar(compressor, "thresholdDb")).toBe(false)
    expect(bipolar(limiter, "inputGainDb")).toBe(false)
    expect(bipolar(synth, "filter.mode")).toBe(false)
  })
})
