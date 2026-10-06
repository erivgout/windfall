import { describe, expect, it } from "vitest"

import {
  MAX_SEND_ROWS,
  scrollToReveal,
  SEND_ROW_HEIGHT,
  STRIP_WIDTH,
  stripLayout,
  visibleRange,
} from "./layout"

describe("stripLayout", () => {
  const mode = (height: number, sends = 0) => stripLayout(height, sends).mode

  it("shows everything inline when the panel is tall", () => {
    expect(stripLayout(460, 0)).toEqual({
      mode: "full",
      sendRows: 0,
      sparseScale: false,
    })
    expect(mode(293)).toBe("full")
  })

  it("moves the routing into a popover when the fader would get short", () => {
    expect(mode(292)).toBe("compact")
    expect(mode(196)).toBe("compact")
  })

  it("drops the chips and the peak readout to keep the fader upright", () => {
    expect(mode(195)).toBe("tight")
    expect(mode(154)).toBe("tight")
  })

  it("lays the fader flat at the lowest heights", () => {
    expect(mode(153)).toBe("flat")
    expect(mode(112)).toBe("flat")
    expect(mode(111)).toBe("mini")
    expect(mode(80, 4)).toBe("mini")
  })

  it("asks for more height before it shows send rows inline", () => {
    expect(stripLayout(293 + SEND_ROW_HEIGHT, 1)).toMatchObject({
      mode: "full",
      sendRows: 1,
    })
    expect(stripLayout(293 + SEND_ROW_HEIGHT - 1, 1)).toMatchObject({
      mode: "compact",
      sendRows: 0,
    })
  })

  it("never reserves more than a few send rows", () => {
    expect(stripLayout(900, 12).sendRows).toBe(MAX_SEND_ROWS)
  })

  it("labels fewer dB marks while the fader is short", () => {
    expect(stripLayout(293, 0).sparseScale).toBe(true)
    expect(stripLayout(330, 0).sparseScale).toBe(false)
    expect(stripLayout(200, 0).sparseScale).toBe(true)
    expect(stripLayout(270, 0).sparseScale).toBe(false)
    expect(stripLayout(160, 0).sparseScale).toBe(true)
  })

  it("uses the full layout until the panel has been measured", () => {
    expect(stripLayout(0, 2)).toEqual({
      mode: "full",
      sendRows: 2,
      sparseScale: false,
    })
  })
})

describe("visibleRange", () => {
  it("mounts the strips in view and two on each side", () => {
    // Strips 10 to 14 are in view.
    expect(visibleRange(10 * STRIP_WIDTH, 5 * STRIP_WIDTH, 127)).toEqual({
      start: 8,
      end: 17,
    })
  })

  it("stops at both ends of the mixer", () => {
    expect(visibleRange(0, 5 * STRIP_WIDTH, 127)).toEqual({ start: 0, end: 7 })
    expect(visibleRange(125 * STRIP_WIDTH, 5 * STRIP_WIDTH, 127)).toEqual({
      start: 123,
      end: 127,
    })
  })

  it("mounts every strip of a short mixer", () => {
    expect(visibleRange(0, 1200, 4)).toEqual({ start: 0, end: 4 })
    expect(visibleRange(0, 1200, 0)).toEqual({ start: 0, end: 0 })
  })

  it("counts a strip that is only partly in view", () => {
    expect(visibleRange(STRIP_WIDTH / 2, STRIP_WIDTH, 127)).toEqual({
      start: 0,
      end: 4,
    })
  })

  it("stays far below the track count however long the mixer is", () => {
    const range = visibleRange(40 * STRIP_WIDTH, 1200, 127)
    expect(range.end - range.start).toBeLessThanOrEqual(20)
  })
})

describe("scrollToReveal", () => {
  const view = 5 * STRIP_WIDTH

  it("leaves a strip that is fully in view alone", () => {
    expect(scrollToReveal(2, 0, view)).toBeNull()
    expect(scrollToReveal(4, 0, view)).toBeNull()
  })

  it("scrolls back to a strip on the left", () => {
    expect(scrollToReveal(3, 10 * STRIP_WIDTH, view)).toBe(3 * STRIP_WIDTH)
  })

  it("scrolls on just far enough for a strip on the right", () => {
    expect(scrollToReveal(5, 0, view)).toBe(STRIP_WIDTH)
    expect(scrollToReveal(20, 0, view)).toBe(16 * STRIP_WIDTH)
  })

  it("brings a strip that is cut off fully into view", () => {
    expect(scrollToReveal(5, 30, view)).toBe(STRIP_WIDTH)
    expect(scrollToReveal(0, 30, view)).toBe(0)
  })
})
