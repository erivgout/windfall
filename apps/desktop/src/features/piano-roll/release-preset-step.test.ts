import { describe, expect, it } from "vitest"

import { nextNoteReleasePreset } from "./release-preset-step"
import { RELEASE_PRESETS } from "./release-presets"

describe("note release preset stepping", () => {
  it("starts at Short 0 and ends at Long 1", () => {
    expect(RELEASE_PRESETS[0].release).toBe(0)
    expect(RELEASE_PRESETS[RELEASE_PRESETS.length - 1].release).toBe(1)
  })

  it.each([
    { release: 0, previous: null, next: 0.5 },
    { release: 0.5, previous: 0, next: 1 },
    { release: 1, previous: 0.5, next: null },
    { release: 0.25, previous: 0, next: 0.5 },
    { release: -0.1, previous: null, next: 0 },
    { release: 1.2, previous: 1, next: null },
    { release: 0.5004, previous: 0, next: 1 },
    { release: 0.4996, previous: 0, next: 1 },
    { release: NaN, previous: null, next: null },
    { release: Infinity, previous: null, next: null },
    { release: -Infinity, previous: null, next: null },
  ])(
    "steps from $release to previous $previous and next $next",
    ({ release, previous, next }) => {
      expect(nextNoteReleasePreset(release, "previous")).toBe(previous)
      expect(nextNoteReleasePreset(release, "next")).toBe(next)
    }
  )
})
