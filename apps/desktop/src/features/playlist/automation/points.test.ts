import { describe, expect, it } from "vitest"

import type { AutomationPoint } from "@/bindings"
import { curveValue } from "@/lib/automation/curve"
import { MAX_AUTOMATION_POINTS, MAX_SONG_TICKS } from "@/lib/units"

import {
  bendHandle,
  bendSegment,
  bendThrough,
  constrainToAxis,
  deletePoint,
  insertPoint,
  inWindow,
  isBendable,
  movePoint,
  samePoints,
  setPointValue,
  straighten,
  tickLimits,
  toCurveTick,
  toggleHold,
  toSongTick,
  windowPath,
  type CurveWindow,
} from "./points"

const BAR = 3840

const point = (
  tick: number,
  value: number,
  more: Partial<AutomationPoint> = {}
): AutomationPoint => ({ tick, value, curve: 0, hold: false, ...more })

/** A rise over a bar and a fall over the next. */
const hill = [point(0, 0), point(BAR, 1), point(2 * BAR, 0)]

describe("a clip as a window onto a curve", () => {
  const window: CurveWindow = { start: 8 * BAR, length: BAR, offset: 960 }

  it("maps ticks of the song to ticks of the curve and back", () => {
    expect(toCurveTick(window, 8 * BAR)).toBe(960)
    expect(toCurveTick(window, 8 * BAR + 100)).toBe(1060)
    expect(toSongTick(window, 960)).toBe(8 * BAR)
    expect(toSongTick(window, toCurveTick(window, 12345))).toBe(12345)
  })

  it("shows the ticks from its offset to its end, the ends included", () => {
    expect(inWindow(window, 959)).toBe(false)
    expect(inWindow(window, 960)).toBe(true)
    expect(inWindow(window, 960 + BAR)).toBe(true)
    expect(inWindow(window, 961 + BAR)).toBe(false)
  })
})

describe("insertPoint", () => {
  it("puts the point in order, with whole ticks and a value from 0 to 1", () => {
    const added = insertPoint(hill, BAR / 2 + 0.4, 1.7)
    expect(added?.index).toBe(1)
    expect(added?.points).toEqual([
      point(0, 0),
      point(BAR / 2, 1),
      point(BAR, 1),
      point(2 * BAR, 0),
    ])
    // The curve it was given is left as it was.
    expect(hill).toHaveLength(3)
  })

  it("goes after a point on the same tick, which makes a jump", () => {
    const added = insertPoint(hill, BAR, 0.25)
    expect(added?.index).toBe(2)
    expect(curveValue(added!.points, BAR - 1)).toBeCloseTo(1, 3)
    expect(curveValue(added!.points, BAR)).toBe(0.25)
  })

  it("goes before the first point and after the last", () => {
    const late = [point(BAR, 0.5)]
    expect(insertPoint(late, 0, 0.1)?.index).toBe(0)
    expect(insertPoint(late, 9 * BAR, 0.1)?.index).toBe(1)
    expect(insertPoint(late, -50, 0.1)?.points[0].tick).toBe(0)
    expect(insertPoint(late, MAX_SONG_TICKS + 9, 0)?.points[1].tick).toBe(
      MAX_SONG_TICKS
    )
  })

  it("has no room in a curve that is full", () => {
    const full = Array.from({ length: MAX_AUTOMATION_POINTS }, (_, index) =>
      point(index, 0.5)
    )
    expect(insertPoint(full, 10, 0.5)).toBeNull()
  })
})

describe("movePoint", () => {
  it("holds the tick between the point's neighbours", () => {
    expect(movePoint(hill, 1, 100, 0.5)[1]).toMatchObject({
      tick: 100,
      value: 0.5,
    })
    expect(movePoint(hill, 1, -900, 0.5)[1].tick).toBe(0)
    expect(movePoint(hill, 1, 9 * BAR, 0.5)[1].tick).toBe(2 * BAR)
    expect(tickLimits(hill, 1)).toEqual({ min: 0, max: 2 * BAR })
  })

  it("lets the first point go back to 0 and the last out to the end", () => {
    const pair = [point(BAR, 0.2), point(2 * BAR, 0.8)]
    expect(movePoint(pair, 0, -5, 0.2)[0].tick).toBe(0)
    expect(tickLimits(pair, 1)).toEqual({ min: BAR, max: MAX_SONG_TICKS })
  })

  it("holds the value to 0 to 1 and the tick to whole numbers", () => {
    expect(movePoint(hill, 1, BAR + 0.6, 1.4)[1]).toMatchObject({
      tick: BAR + 1,
      value: 1,
    })
    expect(movePoint(hill, 1, BAR, -3)[1].value).toBe(0)
    expect(movePoint(hill, 1, BAR, Number.NaN)[1].value).toBe(0)
  })

  it("keeps the point inside the window of the clip it is dragged in", () => {
    const window: CurveWindow = { start: 0, length: BAR, offset: BAR / 2 }
    // The clip shows the curve from half a bar to a bar and a half.
    expect(movePoint(hill, 1, 0, 1, window)[1].tick).toBe(BAR / 2)
    expect(movePoint(hill, 1, 9 * BAR, 1, window)[1].tick).toBe(BAR + BAR / 2)
    expect(tickLimits(hill, 1, window)).toEqual({
      min: BAR / 2,
      max: BAR + BAR / 2,
    })
  })

  it("keeps its bend and its hold, and leaves the other points alone", () => {
    const bent = [point(0, 0, { curve: 0.4, hold: true }), point(BAR, 1)]
    const moved = movePoint(bent, 0, 50, 0.3)
    expect(moved[0]).toEqual({ tick: 50, value: 0.3, curve: 0.4, hold: true })
    expect(moved[1]).toBe(bent[1])
    expect(movePoint(bent, 7, 0, 0)).toEqual(bent)
  })
})

describe("bending a stretch", () => {
  it("sets the bend of the stretch that leaves a point, from -1 to 1", () => {
    expect(bendSegment(hill, 0, 0.5)[0].curve).toBe(0.5)
    expect(bendSegment(hill, 0, 9)[0].curve).toBe(1)
    expect(bendSegment(hill, 0, -9)[0].curve).toBe(-1)
    expect(bendSegment(hill, 0, Number.NaN)[0].curve).toBe(0)
  })

  it("finds the bend that puts the middle of the stretch on a value", () => {
    for (const wanted of [0.1, 0.3, 0.5, 0.7, 0.9]) {
      const curve = bendThrough(0, 1, wanted)!
      const bent = bendSegment(hill, 0, curve)
      expect(curveValue(bent, BAR / 2)).toBeCloseTo(wanted, 6)
    }
    // A stretch that falls bends the same way round.
    const falling = bendThrough(1, 0, 0.8)!
    expect(
      curveValue(bendSegment(hill, 1, falling), BAR + BAR / 2)
    ).toBeCloseTo(0.8, 6)
  })

  it("stops at the strongest bend there is, and has none between level points", () => {
    expect(bendThrough(0, 1, 0)).toBe(1)
    expect(bendThrough(0, 1, 1)).toBe(-1)
    expect(bendThrough(0, 1, 0.5)).toBeCloseTo(0, 9)
    expect(bendThrough(0.4, 0.4, 0.9)).toBeNull()
  })

  it("has a handle in the middle of every stretch that can bend", () => {
    expect(bendHandle(hill, 0)).toEqual({ tick: BAR / 2, value: 0.5 })
    const curved = bendSegment(hill, 0, 1)
    expect(bendHandle(curved, 0)?.value).toBeCloseTo(0.0474, 3)
    // Not after the last point, on a step, or between two points on a tick.
    expect(bendHandle(hill, 2)).toBeNull()
    expect(bendHandle(toggleHold(hill, 0), 0)).toBeNull()
    const jump = [point(0, 0), point(0, 1), point(BAR, 0)]
    expect(bendHandle(jump, 0)).toBeNull()
    expect(isBendable(jump[1], jump[2])).toBe(true)
  })
})

describe("holds, straightening and deleting", () => {
  it("makes a point a step and a slope again", () => {
    const held = toggleHold(hill, 0)
    expect(held[0].hold).toBe(true)
    expect(curveValue(held, BAR - 1)).toBe(0)
    expect(curveValue(held, BAR)).toBe(1)
    expect(toggleHold(held, 0)[0].hold).toBe(false)
  })

  it("takes the bend and the step out of a stretch", () => {
    const shaped = [point(0, 0, { curve: -0.7, hold: true }), point(BAR, 1)]
    expect(straighten(shaped, 0)[0]).toEqual(point(0, 0))
  })

  it("deletes a point, but never the last one", () => {
    expect(deletePoint(hill, 1)).toEqual([point(0, 0), point(2 * BAR, 0)])
    expect(deletePoint([point(0, 0.5)], 0)).toBeNull()
    expect(deletePoint(hill, 9)).toBeNull()
  })

  it("sets a point's value from a typed number", () => {
    expect(setPointValue(hill, 1, 0.25)[1]).toEqual(point(BAR, 0.25))
    expect(setPointValue(hill, 1, 4)[1].value).toBe(1)
  })

  it("tells two curves apart point for point", () => {
    expect(samePoints(hill, [...hill])).toBe(true)
    expect(samePoints(hill, hill.slice(0, 2))).toBe(false)
    expect(samePoints(hill, toggleHold(hill, 1))).toBe(false)
    expect(samePoints(hill, bendSegment(hill, 0, 0.1))).toBe(false)
  })
})

describe("constrainToAxis", () => {
  const from = { tick: 100, value: 0.5 }
  const to = { tick: 400, value: 0.9 }

  it("keeps the axis the pointer has moved further along", () => {
    expect(constrainToAxis(from, to, 30, 5)).toEqual({ tick: 400, value: 0.5 })
    expect(constrainToAxis(from, to, 5, -30)).toEqual({ tick: 100, value: 0.9 })
  })
})

describe("windowPath", () => {
  const ticks = (path: { tick: number; value: number }[]) =>
    path.map((corner) => [corner.tick, +corner.value.toFixed(4)])

  it("goes through the points in the window, from edge to edge", () => {
    const window: CurveWindow = { start: 0, length: 2 * BAR, offset: 0 }
    expect(ticks(windowPath(hill, window, 60))).toEqual([
      [0, 0],
      [BAR, 1],
      [2 * BAR, 0],
      [2 * BAR, 0],
    ])
  })

  it("starts and ends on the value at the window's edges", () => {
    const window: CurveWindow = { start: 0, length: BAR, offset: BAR / 2 }
    expect(ticks(windowPath(hill, window, 60))).toEqual([
      [BAR / 2, 0.5],
      [BAR, 1],
      [BAR + BAR / 2, 0.5],
    ])
  })

  it("is flat before the first point and after the last", () => {
    const late = [point(BAR, 0.3), point(2 * BAR, 0.9)]
    const window: CurveWindow = { start: 0, length: 4 * BAR, offset: 0 }
    expect(ticks(windowPath(late, window, 60))).toEqual([
      [0, 0.3],
      [BAR, 0.3],
      [2 * BAR, 0.9],
      [4 * BAR, 0.9],
    ])
    const outside: CurveWindow = { start: 0, length: BAR, offset: 5 * BAR }
    expect(ticks(windowPath(late, outside, 60))).toEqual([
      [5 * BAR, 0.9],
      [6 * BAR, 0.9],
    ])
  })

  it("draws a hold as a step and two points on a tick as a jump", () => {
    const window: CurveWindow = { start: 0, length: 2 * BAR, offset: 0 }
    const stepped = [point(0, 0.2, { hold: true }), point(BAR, 0.8)]
    expect(ticks(windowPath(stepped, window, 60))).toEqual([
      [0, 0.2],
      [BAR, 0.2],
      [BAR, 0.8],
      [2 * BAR, 0.8],
    ])
    const jump = [
      point(0, 0),
      point(BAR, 0.5),
      point(BAR, 1),
      point(2 * BAR, 0),
    ]
    const path = ticks(windowPath(jump, window, 60))
    expect(path).toContainEqual([BAR, 0.5])
    expect(path).toContainEqual([BAR, 1])
    expect(path.at(-1)).toEqual([2 * BAR, 0])
  })

  it("follows a bent stretch with corners a grain apart", () => {
    const bent = [point(0, 0, { curve: 1 }), point(960, 1)]
    const window: CurveWindow = { start: 0, length: 960, offset: 0 }
    const path = windowPath(bent, window, 240)
    expect(path.map((corner) => corner.tick)).toEqual([
      0, 240, 480, 720, 960, 960,
    ])
    for (const corner of path) {
      expect(corner.value).toBeCloseTo(curveValue(bent, corner.tick), 9)
    }
    // A window that opens in the middle of the bend follows it too.
    const inside: CurveWindow = { start: 0, length: 480, offset: 240 }
    expect(windowPath(bent, inside, 120).map((corner) => corner.tick)).toEqual([
      240, 360, 480, 600, 720,
    ])
  })
})
