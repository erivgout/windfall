import { describe, expect, it } from "vitest"

import { CUT_GROUP_PRESETS, nextCutGroup } from "./cut-group-presets"

describe("cut group presets", () => {
  it("lists the four presets as whole cut groups", () => {
    expect(CUT_GROUP_PRESETS).toEqual([
      { label: "None", cutGroup: 0 },
      { label: "1", cutGroup: 1 },
      { label: "2", cutGroup: 2 },
      { label: "3", cutGroup: 3 },
    ])
  })

  it("returns null when the group is already that number", () => {
    for (const { cutGroup } of CUT_GROUP_PRESETS) {
      expect(nextCutGroup(cutGroup, cutGroup)).toBeNull()
    }
  })

  it("returns the preset when the group differs", () => {
    for (const { cutGroup } of CUT_GROUP_PRESETS) {
      for (const current of [0, 1, 2, 3, 4, 255]) {
        if (current !== cutGroup) {
          expect(nextCutGroup(current, cutGroup)).toBe(cutGroup)
        }
      }
    }
  })
})
