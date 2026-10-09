import { describe, expect, it } from "vitest"

import { KEYMAP_PRESETS } from "@/lib/actions/keymap"

import { nextKeymapPreset } from "./keymap-preset-step"

const presets = KEYMAP_PRESETS.map((preset, index) => ({ ...preset, index }))

describe("keymap preset stepping", () => {
  it("starts at Windfall and ends at FL Studio", () => {
    expect(KEYMAP_PRESETS[0].id).toBe("windfall")
    expect(KEYMAP_PRESETS[KEYMAP_PRESETS.length - 1].id).toBe("fl")
  })

  it.each(presets)(
    "steps previous from $name without wrapping",
    ({ id, index }) => {
      expect(nextKeymapPreset(id, "previous")).toBe(
        KEYMAP_PRESETS[index - 1]?.id ?? null
      )
    }
  )

  it.each(presets)(
    "steps next from $name without wrapping",
    ({ id, index }) => {
      expect(nextKeymapPreset(id, "next")).toBe(
        KEYMAP_PRESETS[index + 1]?.id ?? null
      )
    }
  )

  it.each(["previous", "next"] as const)(
    "leaves another shortcut list alone when stepping %s",
    (direction) => {
      expect(nextKeymapPreset("other", direction)).toBeNull()
    }
  )
})
