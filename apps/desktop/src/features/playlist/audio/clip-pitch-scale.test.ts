import { describe, expect, it } from "vitest"

import { clipPitchScaleUpdates, scaledClipPitch } from "./clip-pitch-scale"

describe("clip pitch scale", () => {
  it("halves pitch 12 to 6 and doubles it to 24", () => {
    expect(scaledClipPitch(12, "half")).toBe(6)
    expect(scaledClipPitch(12, "double")).toBe(24)
  })

  it("halves pitch -12 to -6 and doubles it to -24", () => {
    expect(scaledClipPitch(-12, "half")).toBe(-6)
    expect(scaledClipPitch(-12, "double")).toBe(-24)
  })

  it("halves pitch 7 to 3.5 without rounding", () => {
    expect(scaledClipPitch(7, "half")).toBe(3.5)
    expect(clipPitchScaleUpdates([{ id: 1, pitch: 7 }], "half")).toEqual([
      { id: 1, patch: { pitch: 3.5 } },
    ])
  })

  it("omits unison clips for both factors", () => {
    expect(clipPitchScaleUpdates([{ id: 1, pitch: 0 }], "half")).toEqual([])
    expect(clipPitchScaleUpdates([{ id: 1, pitch: 0 }], "double")).toEqual([])
  })

  it("omits pitch 48 and -48 when doubled", () => {
    expect(
      clipPitchScaleUpdates(
        [
          { id: 1, pitch: 48 },
          { id: 2, pitch: -48 },
        ],
        "double"
      )
    ).toEqual([])
  })

  it("clamps doubled pitch 30 and -30 at the tune limits", () => {
    expect(scaledClipPitch(30, "double")).toBe(48)
    expect(scaledClipPitch(-30, "double")).toBe(-48)
    expect(
      clipPitchScaleUpdates(
        [
          { id: 1, pitch: 30 },
          { id: 2, pitch: -30 },
        ],
        "double"
      )
    ).toEqual([
      { id: 1, patch: { pitch: 48 } },
      { id: 2, patch: { pitch: -48 } },
    ])
  })

  it("patches only pitch", () => {
    const clips = [
      {
        id: 1,
        pitch: 12,
        gain: 1,
        pan: 0.5,
        fadeIn: 4,
        fadeOut: 8,
        reverse: true,
      },
    ]
    expect(clipPitchScaleUpdates(clips, "double")).toEqual([
      { id: 1, patch: { pitch: 24 } },
    ])
  })

  it("omits a unison clip while halving its neighbor from 4 to 2", () => {
    expect(
      clipPitchScaleUpdates(
        [
          { id: 1, pitch: 0 },
          { id: 2, pitch: 4 },
        ],
        "half"
      )
    ).toEqual([{ id: 2, patch: { pitch: 2 } }])
  })

  it("omits changes smaller than 0.001 in either direction", () => {
    expect(
      clipPitchScaleUpdates(
        [
          { id: 1, pitch: 0.001 },
          { id: 2, pitch: -0.001 },
        ],
        "half"
      )
    ).toEqual([])
    expect(
      clipPitchScaleUpdates(
        [
          { id: 1, pitch: 0.0005 },
          { id: 2, pitch: -0.0005 },
          { id: 3, pitch: 47.9995 },
          { id: 4, pitch: -47.9995 },
        ],
        "double"
      )
    ).toEqual([])
  })

  it("includes changes of exactly 0.001", () => {
    expect(clipPitchScaleUpdates([{ id: 1, pitch: 0.002 }], "half")).toEqual([
      { id: 1, patch: { pitch: 0.001 } },
    ])
    expect(clipPitchScaleUpdates([{ id: 1, pitch: -0.001 }], "double")).toEqual([
      { id: 1, patch: { pitch: -0.002 } },
    ])
  })
})
