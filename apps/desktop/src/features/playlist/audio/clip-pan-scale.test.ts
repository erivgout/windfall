import { describe, expect, it } from "vitest"

import { clipPanScaleUpdates, scaledClipPan } from "./clip-pan-scale"

describe("clip pan scale", () => {
  it("halves pan 0.5 to 0.25 and doubles it to 1", () => {
    expect(scaledClipPan(0.5, "half")).toBe(0.25)
    expect(scaledClipPan(0.5, "double")).toBe(1)
  })

  it("halves pan -0.5 to -0.25 and doubles it to -1", () => {
    expect(scaledClipPan(-0.5, "half")).toBe(-0.25)
    expect(scaledClipPan(-0.5, "double")).toBe(-1)
  })

  it("clamps doubled pan 0.6 and -0.6 at hard right and hard left", () => {
    expect(scaledClipPan(0.6, "double")).toBe(1)
    expect(scaledClipPan(-0.6, "double")).toBe(-1)
    expect(
      clipPanScaleUpdates(
        [
          { id: 1, pan: 0.6 },
          { id: 2, pan: -0.6 },
        ],
        "double"
      )
    ).toEqual([
      { id: 1, patch: { pan: 1 } },
      { id: 2, patch: { pan: -1 } },
    ])
  })

  it("omits pan 1 and -1 when doubled", () => {
    expect(
      clipPanScaleUpdates(
        [
          { id: 1, pan: 1 },
          { id: 2, pan: -1 },
        ],
        "double"
      )
    ).toEqual([])
  })

  it("omits centered clips for both factors", () => {
    expect(clipPanScaleUpdates([{ id: 1, pan: 0 }], "half")).toEqual([])
    expect(clipPanScaleUpdates([{ id: 1, pan: 0 }], "double")).toEqual([])
  })

  it("omits changes smaller than 0.001 in either direction", () => {
    expect(
      clipPanScaleUpdates(
        [
          { id: 1, pan: 0.001 },
          { id: 2, pan: -0.001 },
        ],
        "half"
      )
    ).toEqual([])
    expect(
      clipPanScaleUpdates(
        [
          { id: 1, pan: 0.0005 },
          { id: 2, pan: -0.0005 },
          { id: 3, pan: 0.9995 },
          { id: 4, pan: -0.9995 },
        ],
        "double"
      )
    ).toEqual([])
  })

  it("keeps the given order and patches only pan", () => {
    expect(
      clipPanScaleUpdates(
        [
          { id: 9, pan: 0.25 },
          { id: 3, pan: 1 },
          { id: 7, pan: -0.25 },
        ],
        "double"
      )
    ).toEqual([
      { id: 9, patch: { pan: 0.5 } },
      { id: 7, patch: { pan: -0.5 } },
    ])
  })

  it("does not round the scaled pan", () => {
    expect(scaledClipPan(0.123456789, "half")).toBe(0.123456789 / 2)
    expect(scaledClipPan(-0.123456789, "double")).toBe(-0.123456789 * 2)
  })

  it("includes changes of exactly 0.001", () => {
    expect(clipPanScaleUpdates([{ id: 1, pan: 0.002 }], "half")).toEqual([
      { id: 1, patch: { pan: 0.001 } },
    ])
    expect(clipPanScaleUpdates([{ id: 1, pan: -0.001 }], "double")).toEqual([
      { id: 1, patch: { pan: -0.002 } },
    ])
  })

  it("does not mutate the input", () => {
    const clips = Object.freeze([Object.freeze({ id: 1, pan: -0.5 })])
    expect(clipPanScaleUpdates(clips, "half")).toEqual([
      { id: 1, patch: { pan: -0.25 } },
    ])
    expect(clipPanScaleUpdates(clips, "double")).toEqual([
      { id: 1, patch: { pan: -1 } },
    ])
    expect(clips).toEqual([{ id: 1, pan: -0.5 }])
  })
})
