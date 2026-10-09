import { describe, expect, it } from "vitest"

import { PAN_PRESETS, panPresetUpdates } from "./pan-presets"

describe("pan presets", () => {
  it("lists Hard left, Left, Center, Right, and Hard right", () => {
    expect(PAN_PRESETS).toEqual([
      { label: "Hard left", pan: -1 },
      { label: "Left", pan: -0.5 },
      { label: "Center", pan: 0 },
      { label: "Right", pan: 0.5 },
      { label: "Hard right", pan: 1 },
    ])
  })

  it("omits a clip already at the preset", () => {
    expect(panPresetUpdates([{ id: 1, pan: -0.5 }], -0.5)).toEqual([])
  })

  it("counts a difference smaller than 0.001 as a match in either direction", () => {
    expect(
      panPresetUpdates(
        [
          { id: 1, pan: -0.0005 },
          { id: 2, pan: 0.0005 },
        ],
        0
      )
    ).toEqual([])
  })

  it("includes a differing clip with a patch of only pan", () => {
    expect(panPresetUpdates([{ id: 1, pan: -1 }], 0.5)).toEqual([
      { id: 1, patch: { pan: 0.5 } },
    ])
  })

  it("keeps updates in the given order while omitting matching clips", () => {
    expect(
      panPresetUpdates(
        [
          { id: 9, pan: -1 },
          { id: 3, pan: 0 },
          { id: 7, pan: 1 },
        ],
        0
      )
    ).toEqual([
      { id: 9, patch: { pan: 0 } },
      { id: 7, patch: { pan: 0 } },
    ])
  })

  it("does not mutate the input", () => {
    const clip = Object.freeze({ id: 1, pan: -1 })
    const clips = Object.freeze([clip])
    expect(panPresetUpdates(clips, 1)).toEqual([{ id: 1, patch: { pan: 1 } }])
    expect(clips).toEqual([{ id: 1, pan: -1 }])
  })

  it("includes a clip at the 0.001 tolerance boundary", () => {
    expect(panPresetUpdates([{ id: 1, pan: 0.001 }], 0)).toEqual([
      { id: 1, patch: { pan: 0 } },
    ])
  })
})
