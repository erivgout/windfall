import { describe, expect, it } from "vitest"

import { nextEnvelopePreset } from "./envelope-preset-step"
import { ENVELOPE_PRESETS } from "./envelope-presets"

describe("sampler envelope preset stepping", () => {
  it("starts at Pluck and ends at Pad", () => {
    expect(ENVELOPE_PRESETS[0].label).toBe("Pluck")
    expect(ENVELOPE_PRESETS[ENVELOPE_PRESETS.length - 1].label).toBe("Pad")
  })

  it.each(ENVELOPE_PRESETS)(
    "steps from $label in both directions without wrapping",
    (preset) => {
      const index = ENVELOPE_PRESETS.indexOf(preset)
      const current = Object.freeze({ ...preset.envelope })
      expect(nextEnvelopePreset(current, "previous")).toBe(
        ENVELOPE_PRESETS[index - 1]?.envelope ?? null
      )
      expect(nextEnvelopePreset(current, "next")).toBe(
        ENVELOPE_PRESETS[index + 1]?.envelope ?? null
      )
    }
  )

  it("leaves the sampler's first-on shape unchanged", () => {
    const current = { attackMs: 1, decayMs: 200, sustain: 1, releaseMs: 50 }
    expect(nextEnvelopePreset(current, "previous")).toBeNull()
    expect(nextEnvelopePreset(current, "next")).toBeNull()
  })

  it("matches Pluck with a sustain of 0.0004", () => {
    const current = { ...ENVELOPE_PRESETS[0].envelope, sustain: 0.0004 }
    expect(nextEnvelopePreset(current, "previous")).toBeNull()
    expect(nextEnvelopePreset(current, "next")).toBe(
      ENVELOPE_PRESETS[1].envelope
    )
  })

  it("leaves Pluck with an attack of 2 unchanged", () => {
    const current = { ...ENVELOPE_PRESETS[0].envelope, attackMs: 2 }
    expect(nextEnvelopePreset(current, "previous")).toBeNull()
    expect(nextEnvelopePreset(current, "next")).toBeNull()
  })

  it("leaves Keys with a sustain of 0.701 unchanged", () => {
    const current = { ...ENVELOPE_PRESETS[1].envelope, sustain: 0.701 }
    expect(nextEnvelopePreset(current, "previous")).toBeNull()
    expect(nextEnvelopePreset(current, "next")).toBeNull()
  })

  it("leaves a shape with a NaN attack unchanged", () => {
    const current = { ...ENVELOPE_PRESETS[0].envelope, attackMs: NaN }
    expect(nextEnvelopePreset(current, "previous")).toBeNull()
    expect(nextEnvelopePreset(current, "next")).toBeNull()
  })
})
