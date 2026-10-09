import { describe, expect, it } from "vitest"

import { nextWaveformFit, WAVEFORM_FITS } from "./waveform-fit-step"

describe("piano-roll waveform time mapping stepping", () => {
  it("uses seconds and custom as the ends of WAVEFORM_FITS", () => {
    expect(WAVEFORM_FITS[0]).toBe("seconds")
    expect(WAVEFORM_FITS[WAVEFORM_FITS.length - 1]).toBe("custom")
  })

  it.each<{
    fit: string
    direction: "previous" | "next"
    expected: "seconds" | "pattern" | "custom" | null
  }>([
    { fit: "seconds", direction: "previous", expected: null },
    { fit: "seconds", direction: "next", expected: "pattern" },
    { fit: "pattern", direction: "previous", expected: "seconds" },
    { fit: "pattern", direction: "next", expected: "custom" },
    { fit: "custom", direction: "previous", expected: "pattern" },
    { fit: "custom", direction: "next", expected: null },
    { fit: "unknown", direction: "previous", expected: null },
    { fit: "unknown", direction: "next", expected: null },
  ])(
    "returns $expected for $fit moved $direction",
    ({ fit, direction, expected }) => {
      expect(nextWaveformFit(fit, direction)).toBe(expected)
    }
  )
})
