import { describe, expect, it } from "vitest"

import { gainScaleUpdates, scaledGain } from "./gain-scale"

describe("gain scale", () => {
  it("halves gain 1 to 0.5 and doubles it to 2", () => {
    expect(scaledGain(1, "half")).toBe(0.5)
    expect(scaledGain(1, "double")).toBe(2)
  })

  it("caps doubled gain 1.5 at 2", () => {
    expect(scaledGain(1.5, "double")).toBe(2)
    expect(gainScaleUpdates([{ id: 1, gain: 1.5 }], "double")).toEqual([
      { id: 1, patch: { gain: 2 } },
    ])
  })

  it("omits gain 2 when doubled", () => {
    expect(gainScaleUpdates([{ id: 1, gain: 2 }], "double")).toEqual([])
  })

  it("omits gain 0 when halved", () => {
    expect(gainScaleUpdates([{ id: 1, gain: 0 }], "half")).toEqual([])
  })

  it("omits changes smaller than 0.001", () => {
    expect(gainScaleUpdates([{ id: 1, gain: 0.001 }], "half")).toEqual([])
    expect(
      gainScaleUpdates(
        [
          { id: 1, gain: 0.0005 },
          { id: 2, gain: 1.9995 },
        ],
        "double"
      )
    ).toEqual([])
  })

  it("keeps the given order and patches only gain", () => {
    expect(
      gainScaleUpdates(
        [
          { id: 9, gain: 0.25 },
          { id: 3, gain: 2 },
          { id: 7, gain: 0.75 },
        ],
        "double"
      )
    ).toEqual([
      { id: 9, patch: { gain: 0.5 } },
      { id: 7, patch: { gain: 1.5 } },
    ])
  })

  it("does not round the scaled gain", () => {
    expect(scaledGain(0.123456789, "half")).toBe(0.123456789 / 2)
    expect(scaledGain(0.123456789, "double")).toBe(0.123456789 * 2)
  })

  it("includes a change of exactly 0.001", () => {
    expect(gainScaleUpdates([{ id: 1, gain: 0.002 }], "half")).toEqual([
      { id: 1, patch: { gain: 0.001 } },
    ])
  })

  it("does not mutate the input", () => {
    const clips = Object.freeze([Object.freeze({ id: 1, gain: 0.5 })])
    expect(gainScaleUpdates(clips, "half")).toEqual([
      { id: 1, patch: { gain: 0.25 } },
    ])
    expect(gainScaleUpdates(clips, "double")).toEqual([
      { id: 1, patch: { gain: 1 } },
    ])
    expect(clips).toEqual([{ id: 1, gain: 0.5 }])
  })
})
