import { describe, expect, it } from "vitest"

import { scaledVelocity, velocityScaleUpdates } from "./velocity-scale"

describe("velocity scaling", () => {
  it("halves 0.5 to 0.25 and doubles 0.5 to 1", () => {
    expect(scaledVelocity(0.5, "half")).toBe(0.25)
    expect(scaledVelocity(0.5, "double")).toBe(1)
  })

  it("caps doubled 0.6 at full velocity", () => {
    expect(scaledVelocity(0.6, "double")).toBe(1)
  })

  it("omits full velocity when doubled", () => {
    expect(velocityScaleUpdates([{ id: 1, velocity: 1 }], "double")).toEqual([])
  })

  it("omits silence when halved", () => {
    expect(scaledVelocity(0, "half")).toBe(0)
    expect(velocityScaleUpdates([{ id: 1, velocity: 0 }], "half")).toEqual([])
  })

  it("omits changes smaller than 0.001", () => {
    expect(
      velocityScaleUpdates([{ id: 1, velocity: 0.001 }], "half")
    ).toEqual([])
    expect(
      velocityScaleUpdates(
        [
          { id: 2, velocity: 0.0005 },
          { id: 3, velocity: 0.9995 },
        ],
        "double"
      )
    ).toEqual([])
  })

  it("keeps updates in the given order with only id and velocity", () => {
    const notes = [
      { id: 9, velocity: 0.25, pan: -0.5 },
      { id: 3, velocity: 1, pan: 0 },
      { id: 7, velocity: 0.4, pan: 0.5 },
    ]

    expect(velocityScaleUpdates(notes, "double")).toEqual([
      { id: 9, velocity: 0.5 },
      { id: 7, velocity: 0.8 },
    ])
  })

  it("does not round velocities", () => {
    expect(scaledVelocity(0.12345, "half")).toBe(0.12345 / 2)
    expect(scaledVelocity(0.12345, "double")).toBe(0.12345 * 2)
  })

  it("includes changes exactly at the 0.001 threshold", () => {
    expect(velocityScaleUpdates([{ id: 1, velocity: 0.002 }], "half")).toEqual([
      { id: 1, velocity: 0.001 },
    ])
  })

  it("does not mutate the input", () => {
    const notes = Object.freeze([Object.freeze({ id: 1, velocity: 0.6 })])

    expect(velocityScaleUpdates(notes, "half")).toEqual([
      { id: 1, velocity: 0.3 },
    ])
    expect(notes).toEqual([{ id: 1, velocity: 0.6 }])
  })
})
