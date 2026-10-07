import { describe, expect, it } from "vitest"

import type { Box } from "../clip-box"
import {
  FADE_HANDLE,
  fadeHandleLeft,
  gainHandleBox,
  hasFadeHandles,
  hasGainHandle,
  hitAudioHandle,
} from "./handles"

/** A clip 200 pixels wide on a 38 pixel row. */
const box: Box = { left: 100, right: 300, top: 1, bottom: 38 }

describe("which clips have handles", () => {
  it("needs a title bar, and room across", () => {
    expect(hasFadeHandles(box)).toBe(true)
    expect(hasGainHandle(box)).toBe(true)
    // A row too short for a title bar.
    expect(hasFadeHandles({ ...box, bottom: 20 })).toBe(false)
    expect(hasGainHandle({ ...box, bottom: 20 })).toBe(false)
    // A title bar and a sliver of waveform: the gain handle alone.
    expect(hasFadeHandles({ ...box, bottom: 27 })).toBe(false)
    expect(hasGainHandle({ ...box, bottom: 27 })).toBe(true)
    // Narrow clips lose the gain handle first, then the fades.
    expect(hasGainHandle({ ...box, right: 150 })).toBe(false)
    expect(hasFadeHandles({ ...box, right: 150 })).toBe(true)
    expect(hasFadeHandles({ ...box, right: 120 })).toBe(false)
  })
})

describe("where the fade handles sit", () => {
  it("is in the corner while there is no fade", () => {
    expect(fadeHandleLeft("in", box, 0)).toBe(100)
    expect(fadeHandleLeft("out", box, 0)).toBe(300 - FADE_HANDLE)
  })

  it("is centred on the end of the fade, and stays inside the clip", () => {
    expect(fadeHandleLeft("in", box, 50)).toBe(150 - FADE_HANDLE / 2)
    expect(fadeHandleLeft("out", box, 50)).toBe(250 - FADE_HANDLE / 2)
    expect(fadeHandleLeft("in", box, 5000)).toBe(300 - FADE_HANDLE)
    expect(fadeHandleLeft("out", box, 5000)).toBe(100)
  })
})

describe("hitAudioHandle", () => {
  it("takes the fade handles at the top corners of the waveform", () => {
    expect(hitAudioHandle(box, 0, 0, 103, 18)).toBe("fade-in")
    expect(hitAudioHandle(box, 0, 0, 297, 18)).toBe("fade-out")
    expect(hitAudioHandle(box, 60, 0, 160, 18)).toBe("fade-in")
    // Above them is the title bar, under them the edge of the clip.
    expect(hitAudioHandle(box, 0, 0, 103, 6)).toBeNull()
    expect(hitAudioHandle(box, 0, 0, 103, 30)).toBeNull()
  })

  it("takes the gain handle at the right end of the title bar", () => {
    const handle = gainHandleBox(box)
    expect(handle.right).toBe(294)
    const x = (handle.left + handle.right) / 2
    const y = (handle.top + handle.bottom) / 2
    expect(hitAudioHandle(box, 0, 0, x, y)).toBe("gain")
    expect(hitAudioHandle(box, 0, 0, x, 30)).toBeNull()
    expect(hitAudioHandle(box, 0, 0, x - 30, y)).toBeNull()
  })

  it("gives the nearer handle where two fades meet", () => {
    // Both fades end in the middle of the clip.
    expect(hitAudioHandle(box, 100, 100, 198, 18)).toBe("fade-in")
    expect(hitAudioHandle(box, 96, 96, 203, 18)).toBe("fade-out")
  })

  it("has nothing to take on a clip too small for handles", () => {
    const small: Box = { left: 0, right: 20, top: 1, bottom: 18 }
    expect(hitAudioHandle(small, 0, 0, 2, 4)).toBeNull()
    expect(hitAudioHandle(small, 0, 0, 2, 16)).toBeNull()
  })
})
