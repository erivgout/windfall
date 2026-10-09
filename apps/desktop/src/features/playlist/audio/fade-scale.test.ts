import { describe, expect, it } from "vitest"

import { fadeScaleUpdates, scaledFade } from "./fade-scale"

describe("fade scale", () => {
  it("halves both fades from 40 to 20", () => {
    expect(
      fadeScaleUpdates(
        [{ id: 1, length: 100, fadeIn: 40, fadeOut: 40 }],
        "half"
      )
    ).toEqual([{ id: 1, patch: { fadeIn: 20, fadeOut: 20 } }])
  })

  it("doubles both fades from 40 to 80", () => {
    expect(
      fadeScaleUpdates(
        [{ id: 1, length: 100, fadeIn: 40, fadeOut: 40 }],
        "double"
      )
    ).toEqual([{ id: 1, patch: { fadeIn: 80, fadeOut: 80 } }])
  })

  it("rounds half of 5 down to 2 whole ticks", () => {
    expect(scaledFade(5, 100, "half")).toBe(2)
    expect(
      fadeScaleUpdates([{ id: 1, length: 100, fadeIn: 5, fadeOut: 0 }], "half")
    ).toEqual([{ id: 1, patch: { fadeIn: 2 } }])
  })

  it("leaves zero fade in out of a patch that halves fade out", () => {
    expect(scaledFade(0, 100, "half")).toBe(0)
    expect(
      fadeScaleUpdates([{ id: 1, length: 100, fadeIn: 0, fadeOut: 16 }], "half")
    ).toEqual([{ id: 1, patch: { fadeOut: 8 } }])
  })

  it("omits a clip when doubled fades are already at length and zero", () => {
    expect(scaledFade(100, 100, "double")).toBe(100)
    expect(
      fadeScaleUpdates(
        [{ id: 1, length: 100, fadeIn: 100, fadeOut: 0 }],
        "double"
      )
    ).toEqual([])
  })

  it("stops doubled fades of 40 at a clip length of 50", () => {
    expect(
      fadeScaleUpdates(
        [{ id: 1, length: 50, fadeIn: 40, fadeOut: 40 }],
        "double"
      )
    ).toEqual([{ id: 1, patch: { fadeIn: 50, fadeOut: 50 } }])
  })

  it("halves full fades on a seven-tick clip to three ticks", () => {
    expect(
      fadeScaleUpdates([{ id: 1, length: 7, fadeIn: 7, fadeOut: 7 }], "half")
    ).toEqual([{ id: 1, patch: { fadeIn: 3, fadeOut: 3 } }])
  })

  it("scales unequal fade sides independently", () => {
    const clips = [{ id: 1, length: 100, fadeIn: 5, fadeOut: 16 }]
    expect(fadeScaleUpdates(clips, "half")).toEqual([
      { id: 1, patch: { fadeIn: 2, fadeOut: 8 } },
    ])
    expect(fadeScaleUpdates(clips, "double")).toEqual([
      { id: 1, patch: { fadeIn: 10, fadeOut: 32 } },
    ])
  })

  it("caps each clip's doubled fades at its own length", () => {
    expect(
      fadeScaleUpdates(
        [
          { id: 1, length: 100, fadeIn: 40, fadeOut: 40 },
          { id: 2, length: 50, fadeIn: 40, fadeOut: 40 },
        ],
        "double"
      )
    ).toEqual([
      { id: 1, patch: { fadeIn: 80, fadeOut: 80 } },
      { id: 2, patch: { fadeIn: 50, fadeOut: 50 } },
    ])
  })

  it("omits a saturated fade out while doubling fade in", () => {
    expect(
      fadeScaleUpdates(
        [{ id: 1, length: 100, fadeIn: 16, fadeOut: 100 }],
        "double"
      )
    ).toEqual([{ id: 1, patch: { fadeIn: 32 } }])
  })

  it("omits unchanged clips while updating their neighbors", () => {
    expect(
      fadeScaleUpdates(
        [
          { id: 1, length: 100, fadeIn: 100, fadeOut: 0 },
          { id: 2, length: 50, fadeIn: 40, fadeOut: 0 },
        ],
        "double"
      )
    ).toEqual([{ id: 2, patch: { fadeIn: 50 } }])
  })

  it("patches only fades without changing the input", () => {
    const clips = [
      Object.freeze({
        id: 1,
        length: 100,
        fadeIn: 40,
        fadeOut: 16,
        gain: 0.5,
        pan: -0.25,
        pitch: 12,
      }),
    ]
    const updates = fadeScaleUpdates(Object.freeze(clips), "double")
    expect(updates).toEqual([{ id: 1, patch: { fadeIn: 80, fadeOut: 32 } }])
    for (const key of ["gain", "pan", "pitch", "length"]) {
      expect(updates[0].patch).not.toHaveProperty(key)
    }
    expect(clips[0].fadeIn).toBe(40)
    expect(clips[0].fadeOut).toBe(16)
  })

  it("omits zero fades and empty selections for either factor", () => {
    for (const factor of ["half", "double"] as const) {
      expect(
        fadeScaleUpdates(
          [{ id: 1, length: 100, fadeIn: 0, fadeOut: 0 }],
          factor
        )
      ).toEqual([])
      expect(fadeScaleUpdates([], factor)).toEqual([])
    }
  })
})
