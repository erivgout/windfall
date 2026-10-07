import { describe, expect, it } from "vitest"

import { PREVIEW_BRAND_SHARE, PREVIEW_WAVE } from "./preview-colors"

/**
 * The oklch lightness of the tokens involved, as `index.css` sets them for
 * each theme. The stylesheet is not loaded in these tests, so they are
 * written out here.
 */
const TOKENS = {
  light: {
    brand: 0.592,
    displayForeground: 0.96,
    display: 0.235,
    chassis: 0.948,
  },
  dark: {
    brand: 0.66,
    displayForeground: 0.95,
    display: 0.125,
    chassis: 0.165,
  },
}

/** The lightness of the brand color mixed with the display's foreground. */
const mixed = (theme: keyof typeof TOKENS, brandShare: number) =>
  brandShare * TOKENS[theme].brand +
  (1 - brandShare) * TOKENS[theme].displayForeground

describe("the waveform in the browser's preview", () => {
  it("is mostly the brand color", () => {
    expect(PREVIEW_WAVE).toBe(
      "color-mix(in oklch, var(--wf-brand) 82%, var(--wf-display-foreground))"
    )
  })

  it.each(["light", "dark"] as const)(
    "stands out from its window and from the panel around it in the %s theme",
    (theme) => {
      const wave = mixed(theme, PREVIEW_BRAND_SHARE)
      // The dark window it is drawn in.
      expect(Math.abs(wave - TOKENS[theme].display)).toBeGreaterThan(0.35)
      // The panel the window sits in. A waveform that fills the window
      // must not look like a hole in it.
      expect(Math.abs(wave - TOKENS[theme].chassis)).toBeGreaterThan(0.25)
    }
  )

  it("was the color of the light theme's panel when drawn in the display's own foreground", () => {
    // What it used to be: no brand color at all.
    expect(Math.abs(mixed("light", 0) - TOKENS.light.chassis)).toBeLessThan(
      0.02
    )
  })
})
