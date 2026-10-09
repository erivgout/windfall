import { describe, expect, it } from "vitest"

import { nextRootKey, ROOT_KEY_PRESETS } from "./root-key-presets"

describe("root key presets", () => {
  it("lists the four presets as whole MIDI keys", () => {
    expect(ROOT_KEY_PRESETS).toEqual([
      { label: "C3", rootKey: 36 },
      { label: "C4", rootKey: 48 },
      { label: "C5", rootKey: 60 },
      { label: "C6", rootKey: 72 },
    ])
  })

  it("returns null when the root is already that key", () => {
    for (const { rootKey } of ROOT_KEY_PRESETS) {
      expect(nextRootKey(rootKey, rootKey)).toBeNull()
    }
  })

  it("returns the preset when the root differs", () => {
    for (const { rootKey } of ROOT_KEY_PRESETS) {
      for (const current of [0, rootKey - 1, rootKey + 1, 127]) {
        expect(nextRootKey(current, rootKey)).toBe(rootKey)
      }
    }
  })
})
