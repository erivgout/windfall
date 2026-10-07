import { describe, expect, it } from "vitest"

import { gridLevels } from "@/lib/canvas"

import {
  contentTicksFor,
  EDGE_SCROLL_MAX,
  edgePull,
  MIN_ROWS,
  MIN_SONG_BARS,
  rowCountFor,
  SPARE_ROWS,
  TAIL_BARS,
} from "./layout"
import { describePatternLength } from "./pattern-picker"
import { barsPerLabel } from "./ruler"
import { startForThumb, thumbBox } from "./scrollbar"
import { gridSpecFor, nudgeTicks, snapTicks } from "./snap"

const BAR = 3840
const FOUR_FOUR = { numerator: 4, denominator: 4 }
const SIX_EIGHT = { numerator: 6, denominator: 8 }

describe("rows and timeline length", () => {
  it("shows rows to place clips on before any track exists", () => {
    expect(rowCountFor(0, 0)).toBe(MIN_ROWS)
  })

  it("keeps spare rows below the last track", () => {
    expect(rowCountFor(200, 10)).toBe(200 + SPARE_ROWS)
  })

  it("never has fewer rows than fill the view", () => {
    expect(rowCountFor(2, 30.2)).toBe(31)
  })

  it("always leaves empty bars after the last clip", () => {
    expect(contentTicksFor(0, BAR)).toBe(MIN_SONG_BARS * BAR)
    expect(contentTicksFor(100 * BAR, BAR)).toBe((100 + TAIL_BARS) * BAR)
    expect(contentTicksFor(100 * BAR + 1, BAR)).toBe((101 + TAIL_BARS) * BAR)
  })
})

describe("snap", () => {
  it("measures each setting in ticks from the time signature", () => {
    expect(snapTicks("none", FOUR_FOUR)).toBe(0)
    expect(snapTicks("step", FOUR_FOUR)).toBe(240)
    expect(snapTicks("beat", FOUR_FOUR)).toBe(960)
    expect(snapTicks("bar", FOUR_FOUR)).toBe(BAR)
    expect(snapTicks("beat", SIX_EIGHT)).toBe(480)
    expect(snapTicks("bar", SIX_EIGHT)).toBe(2880)
  })

  it("nudges by a step when the snap is off", () => {
    expect(nudgeTicks("none", FOUR_FOUR)).toBe(240)
    expect(nudgeTicks("bar", SIX_EIGHT)).toBe(2880)
  })

  it("draws no line finer than the snap, however far in the zoom is", () => {
    const zoomedIn = 0.4
    expect(gridLevels(zoomedIn, gridSpecFor("bar", FOUR_FOUR)).minor).toBe(BAR)
    expect(gridLevels(zoomedIn, gridSpecFor("beat", FOUR_FOUR)).minor).toBe(960)
    expect(gridLevels(zoomedIn, gridSpecFor("step", FOUR_FOUR)).minor).toBe(240)
    expect(gridLevels(zoomedIn, gridSpecFor("none", FOUR_FOUR)).minor).toBe(240)
  })

  it("marks bars and groups of four bars above the snap", () => {
    expect(gridLevels(0.4, gridSpecFor("bar", FOUR_FOUR))).toEqual({
      minor: BAR,
      mid: BAR,
      strong: 4 * BAR,
    })
    expect(gridLevels(0.4, gridSpecFor("beat", SIX_EIGHT))).toEqual({
      minor: 480,
      mid: 2880,
      strong: 4 * 2880,
    })
    expect(gridLevels(0.4, gridSpecFor("step", SIX_EIGHT))).toEqual({
      minor: 240,
      mid: 480,
      strong: 2880,
    })
  })

  it("thins the lines out by itself when zoomed far out", () => {
    const far = gridLevels(0.0008, gridSpecFor("step", FOUR_FOUR))
    expect(far.minor).toBeGreaterThanOrEqual(BAR)
  })
})

describe("ruler labels", () => {
  it("numbers every bar when there is room, then every 2, 4, 8", () => {
    expect(barsPerLabel(72)).toBe(1)
    expect(barsPerLabel(30)).toBe(2)
    expect(barsPerLabel(12)).toBe(4)
    expect(barsPerLabel(3)).toBe(16)
  })
})

describe("pattern length", () => {
  it("reads in bars when it is a whole number of them", () => {
    expect(describePatternLength(16, BAR)).toBe("1 bar")
    expect(describePatternLength(64, BAR)).toBe("4 bars")
    expect(describePatternLength(12, 2880)).toBe("1 bar")
  })

  it("reads in steps otherwise", () => {
    expect(describePatternLength(12, BAR)).toBe("12 steps")
    expect(describePatternLength(1, BAR)).toBe("1 step")
  })
})

describe("scrollbar thumb", () => {
  it("is as long as the share of the content in view", () => {
    expect(thumbBox({ start: 0, visible: 25, total: 100 }, 400)).toEqual({
      offset: 0,
      size: 100,
    })
    expect(thumbBox({ start: 75, visible: 25, total: 100 }, 400)).toEqual({
      offset: 300,
      size: 100,
    })
  })

  it("never gets too small to grab", () => {
    expect(thumbBox({ start: 0, visible: 1, total: 1000 }, 400)?.size).toBe(28)
  })

  it("is hidden when everything fits", () => {
    expect(thumbBox({ start: 0, visible: 100, total: 100 }, 400)).toBeNull()
    expect(thumbBox({ start: 0, visible: 10, total: 100 }, 0)).toBeNull()
  })

  it("turns a thumb position back into a scroll position", () => {
    const extent = { start: 0, visible: 25, total: 100 }
    expect(startForThumb(extent, 400, 150)).toBe(37.5)
    expect(startForThumb(extent, 400, -50)).toBe(0)
    expect(startForThumb(extent, 400, 9999)).toBe(75)
  })
})

describe("scrolling at the edges", () => {
  it("leaves the view alone while the drag is in the middle", () => {
    expect(edgePull(400, 800)).toBe(0)
    expect(edgePull(20, 800)).toBe(0)
    expect(edgePull(780, 800)).toBe(0)
  })

  it("scrolls back near the start and on near the end", () => {
    expect(edgePull(10, 800)).toBeLessThan(0)
    expect(edgePull(790, 800)).toBeGreaterThan(0)
  })

  it("scrolls faster the further out the pointer is, up to a limit", () => {
    expect(edgePull(830, 800)).toBeGreaterThan(edgePull(790, 800))
    expect(edgePull(5000, 800)).toBe(EDGE_SCROLL_MAX)
    expect(edgePull(-5000, 800)).toBe(-EDGE_SCROLL_MAX)
  })
})
