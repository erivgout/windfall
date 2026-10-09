import { describe, expect, it } from "vitest"

import { ENVELOPE_PRESETS, nextEnvelope } from "./envelope-presets"

describe("envelope presets", () => {
  it("lists Pluck, Keys, Organ, and Pad with their full shapes", () => {
    expect(ENVELOPE_PRESETS).toEqual([
      {
        label: "Pluck",
        envelope: { attackMs: 1, decayMs: 180, sustain: 0, releaseMs: 80 },
      },
      {
        label: "Keys",
        envelope: { attackMs: 8, decayMs: 400, sustain: 0.7, releaseMs: 250 },
      },
      {
        label: "Organ",
        envelope: { attackMs: 8, decayMs: 0, sustain: 1, releaseMs: 30 },
      },
      {
        label: "Pad",
        envelope: {
          attackMs: 500,
          decayMs: 300,
          sustain: 0.85,
          releaseMs: 800,
        },
      },
    ])
  })

  it("returns null when every field already matches", () => {
    for (const { envelope } of ENVELOPE_PRESETS) {
      expect(nextEnvelope({ ...envelope }, envelope)).toBeNull()
    }
  })

  it("counts a sustain difference smaller than 0.001 as a match", () => {
    for (const { envelope } of ENVELOPE_PRESETS) {
      for (const difference of [-0.0009, 0.0009]) {
        expect(
          nextEnvelope(
            { ...envelope, sustain: envelope.sustain + difference },
            envelope
          )
        ).toBeNull()
      }
    }
  })

  it("returns the preset for a different attack without mutating the input", () => {
    for (const { envelope } of ENVELOPE_PRESETS) {
      const current = Object.freeze({
        ...envelope,
        attackMs: envelope.attackMs + 1,
      })
      expect(nextEnvelope(current, envelope)).toBe(envelope)
      expect(current).toEqual({ ...envelope, attackMs: envelope.attackMs + 1 })
    }
  })
})
