import { describe, expect, it } from "vitest"

import type { AutomationPoint } from "@/bindings"

import { innerHit } from "../inner"
import type { Viewport } from "@/lib/canvas"
import {
  canEditCurve,
  curveTickAt,
  DOT_REACH,
  curveX,
  hitCurve,
  valueAt,
  valueY,
  type CurveView,
} from "./hit"

const BAR = 3840

const point = (
  tick: number,
  value: number,
  more: Partial<AutomationPoint> = {}
): AutomationPoint => ({ tick, value, curve: 0, hold: false, ...more })

/**
 * A clip two bars long, drawn 200 pixels wide from x 100, with its curve
 * between y 20 and y 100.
 */
const view: CurveView = {
  window: { start: 4 * BAR, length: 2 * BAR, offset: 0 },
  area: { left: 101, right: 299, top: 20, bottom: 100 },
  pxPerTick: 200 / (2 * BAR),
}

const hill = [point(0, 0), point(BAR, 1), point(2 * BAR, 0.5)]

describe("where a curve is on screen", () => {
  it("puts ticks across the clip and values up its height", () => {
    expect(curveX(view, 0)).toBe(100)
    expect(curveX(view, BAR)).toBe(200)
    expect(curveTickAt(view, 150)).toBe(BAR / 2)
    expect(valueY(view, 0)).toBe(100)
    expect(valueY(view, 1)).toBe(20)
    expect(valueY(view, 0.5)).toBe(60)
    expect(valueAt(view, 60)).toBe(0.5)
    // Above and below the area the value stops at its ends.
    expect(valueAt(view, -40)).toBe(1)
    expect(valueAt(view, 400)).toBe(0)
  })

  it("follows the clip's offset into the curve", () => {
    const trimmed: CurveView = {
      ...view,
      window: { start: 4 * BAR, length: 2 * BAR, offset: BAR },
    }
    expect(curveX(trimmed, BAR)).toBe(100)
    expect(curveTickAt(trimmed, 100)).toBe(BAR)
  })

  it("can be edited only in a clip with a title bar and some width", () => {
    expect(canEditCurve({ left: 0, right: 200, top: 1, bottom: 92 })).toBe(true)
    expect(canEditCurve({ left: 0, right: 200, top: 1, bottom: 20 })).toBe(
      false
    )
    expect(canEditCurve({ left: 0, right: 8, top: 1, bottom: 92 })).toBe(false)
  })
})

describe("hitCurve", () => {
  it("takes a point within a few pixels of it", () => {
    expect(hitCurve(hill, view, 200, 20)).toEqual({ kind: "point", index: 1 })
    expect(hitCurve(hill, view, 204, 23)).toEqual({ kind: "point", index: 1 })
    // The first point sits on the clip's left edge and is still taken.
    expect(hitCurve(hill, view, 98, 100)).toEqual({ kind: "point", index: 0 })
  })

  it("reaches only as far as it is told", () => {
    // Six pixels by default, which is what a press inside the clip gets.
    expect(hitCurve(hill, view, 95, 100)).toEqual({ kind: "point", index: 0 })
    // Outside the clip a point is its dot and no more.
    expect(hitCurve(hill, view, 95, 100, DOT_REACH)).toBeNull()
    expect(hitCurve(hill, view, 97, 100, DOT_REACH)).toEqual({
      kind: "point",
      index: 0,
    })
    expect(hitCurve(hill, view, 150, 65, DOT_REACH)).toEqual({ kind: "area" })
    expect(hitCurve(hill, view, 150, 63, DOT_REACH)).toEqual({
      kind: "bend",
      index: 0,
    })
  })

  it("takes the nearest of several, and the later of two on one spot", () => {
    const close = [point(0, 0.5), point(100, 0.5), point(2 * BAR, 0.5)]
    expect(hitCurve(close, view, 104, 60)).toEqual({ kind: "point", index: 1 })
    const jump = [point(BAR, 0.5), point(BAR, 0.5)]
    expect(hitCurve(jump, view, 200, 60)).toEqual({ kind: "point", index: 1 })
  })

  it("takes the handle in the middle of a stretch that is long enough", () => {
    expect(hitCurve(hill, view, 150, 60)).toEqual({ kind: "bend", index: 0 })
    expect(hitCurve(hill, view, 250, 40)).toEqual({ kind: "bend", index: 1 })
    // Two points 20 pixels apart have no handle between them.
    const tight = [point(0, 0), point(384, 1), point(2 * BAR, 1)]
    expect(hitCurve(tight, view, 110, 60)).toEqual({ kind: "area" })
  })

  it("is the open area anywhere else inside, and nothing outside", () => {
    expect(hitCurve(hill, view, 150, 90)).toEqual({ kind: "area" })
    expect(hitCurve(hill, view, 150, 10)).toBeNull()
    expect(hitCurve(hill, view, 400, 60)).toBeNull()
  })

  it("leaves points outside the clip's window to other clips", () => {
    const narrow: CurveView = {
      ...view,
      window: { start: 4 * BAR, length: BAR / 2, offset: 0 },
      area: { ...view.area, right: 149 },
    }
    // The point at one bar lies past the end of this clip.
    expect(hitCurve(hill, narrow, 200, 20)).toBeNull()
    expect(hitCurve(hill, narrow, 100, 100)).toEqual({
      kind: "point",
      index: 0,
    })
  })
})

describe("innerHit on an automation clip", () => {
  const viewport: Viewport = {
    width: 960,
    height: 400,
    dpr: 1,
    scrollTick: 0,
    scrollRow: 0,
    pxPerTick: 200 / (2 * BAR),
    rowHeight: 92,
  }
  const clip = {
    id: 1,
    track: 1,
    start: 0,
    length: 2 * BAR,
    offset: 0,
    muted: false,
    content: { type: "automation", automation: 9 },
  } as const

  it("finds the points under the title bar, and leaves the bar to the clip", () => {
    // Row 0: the title bar ends at 15, the curve runs from 18 to 88.
    expect(innerHit(viewport, clip, 0, hill, 100, 18)).toEqual({
      kind: "point",
      index: 1,
    })
    expect(innerHit(viewport, clip, 0, hill, 50, 80)).toEqual({ kind: "curve" })
    expect(innerHit(viewport, clip, 0, hill, 50, 6)).toBeNull()
    // On a row too short for a title bar the whole clip is the clip's.
    expect(
      innerHit({ ...viewport, rowHeight: 20 }, clip, 0, hill, 50, 12)
    ).toBeNull()
  })

  it("takes a point on the clip's edge from the half of its dot that hangs over", () => {
    // The clip starts two bars in, at x 200. Its first point is on that
    // edge, at the bottom of the curve.
    const later = { ...clip, start: 2 * BAR }
    for (const x of [197, 198, 200, 203]) {
      expect(innerHit(viewport, later, 0, hill, x, 88), `x ${x}`).toEqual({
        kind: "point",
        index: 0,
      })
    }
    // Past the dot it is the grid's again. Inside the clip a press may be
    // a little further off.
    expect(innerHit(viewport, later, 0, hill, 195, 88)).toBeNull()
    expect(innerHit(viewport, later, 0, hill, 206, 88)).toEqual({
      kind: "point",
      index: 0,
    })
  })
})
