import { describe, expect, it } from "vitest"

import type { AutomationPoint } from "@/bindings"

import { rangeNormalized, rangeValue, TEMPO_RANGE } from "./curve"
import {
  extendView,
  fitView,
  fittedView,
  FULL_VIEW,
  inView,
  isFullView,
  makeView,
  tempoView,
  viewFraction,
  viewOfAutomation,
  viewValue,
} from "./view-range"

const bpm = (value: number) => rangeNormalized(TEMPO_RANGE, value)
const inBpm = (view: { lo: number; hi: number }) => [
  Math.round(rangeValue(TEMPO_RANGE, view.lo) * 100) / 100,
  Math.round(rangeValue(TEMPO_RANGE, view.hi) * 100) / 100,
]
const point = (value: number, tick = 0): AutomationPoint => ({
  tick,
  value,
  curve: 0,
  hold: false,
})

describe("a view of a curve's range", () => {
  it("places a value between its ends, and gives it back", () => {
    const view = { lo: 0.2, hi: 0.6 }
    expect(viewFraction(view, 0.2)).toBe(0)
    expect(viewFraction(view, 0.4)).toBeCloseTo(0.5, 12)
    expect(viewFraction(view, 0.6)).toBeCloseTo(1, 12)
    // Outside the view it is not held: what is out of sight is cut off.
    expect(viewFraction(view, 0.8)).toBeCloseTo(1.5, 12)
    expect(viewValue(view, 0.5)).toBeCloseTo(0.4, 12)
    // Past the view's edge the scale carries on, as far as the range goes.
    expect(viewValue(view, 1.5)).toBeCloseTo(0.8, 12)
    expect(viewValue(view, 4)).toBe(1)
    expect(viewValue(view, -3)).toBe(0)
    // The whole range is the plain mapping a clip always had.
    expect(viewFraction(FULL_VIEW, 0.37)).toBe(0.37)
  })

  it("is made in order, inside the range and never flat", () => {
    expect(makeView(0.8, 0.3)).toEqual({ lo: 0.3, hi: 0.8 })
    expect(makeView(-2, 5)).toEqual(FULL_VIEW)
    const thin = makeView(0.5, 0.5, 0.1)
    expect(thin.hi - thin.lo).toBeCloseTo(0.1, 12)
    expect((thin.lo + thin.hi) / 2).toBeCloseTo(0.5, 12)
    // At an end of the range it moves over instead of leaving it.
    expect(makeView(0, 0, 0.1)).toEqual({ lo: 0, hi: 0.1 })
    const top = makeView(1, 1, 0.1)
    expect(top.hi).toBe(1)
    expect(top.lo).toBeCloseTo(0.9, 12)
  })

  it("grows to show a value, and only then", () => {
    const view = { lo: 0.2, hi: 0.6 }
    expect(extendView(view, 0.4)).toBe(view)
    expect(extendView(view, 0.9)).toEqual({ lo: 0.2, hi: 0.9 })
    expect(extendView(view, 0.1, 0.7)).toEqual({ lo: 0.1, hi: 0.7 })
    expect(extendView(view, 3)).toEqual({ lo: 0.2, hi: 1 })
    expect(inView(view, 0.6)).toBe(true)
    expect(inView(view, 0.61)).toBe(false)
  })

  it("fits a curve with air around it, and a level one with room to move", () => {
    const fitted = fitView([0.4, 0.5, 0.6])
    expect(fitted.lo).toBeCloseTo(0.376, 12)
    expect(fitted.hi).toBeCloseTo(0.624, 12)
    const level = fitView([0.5])
    expect(level).toEqual({ lo: 0.4, hi: 0.6 })
    expect(fitView([])).toEqual(FULL_VIEW)
    expect(isFullView(fitView([0, 1]))).toBe(true)
  })
})

describe("the default view of a tempo curve", () => {
  it("is 20 bpm either side of the tempo, 40 tall", () => {
    expect(inBpm(tempoView([bpm(120)], 120))).toEqual([100, 140])
    // No points yet: about the project's tempo all the same.
    expect(inBpm(tempoView([], 128))).toEqual([108, 148])
  })

  it("takes in the curve's points and the tempo", () => {
    expect(inBpm(tempoView([bpm(120), bpm(140)], 120))).toEqual([100, 160])
    // The stored tempo is what plays outside the clips, so it shows too.
    expect(inBpm(tempoView([bpm(170), bpm(180)], 120))).toEqual([100, 200])
    expect(inBpm(tempoView([bpm(90)], 120))).toEqual([70, 140])
  })

  it("stops at the ends of the tempo's range and stays 40 bpm tall", () => {
    expect(inBpm(tempoView([bpm(10)], 20))).toEqual([10, 50])
    expect(inBpm(tempoView([bpm(522)], 515))).toEqual([482, 522])
  })

  it("makes 120 to 140 bpm half a clip, where the whole range made it 4%", () => {
    const view = tempoView([bpm(120)], 120)
    const rise = viewFraction(view, bpm(140)) - viewFraction(view, bpm(120))
    expect(rise).toBeCloseTo(0.5, 6)
    expect(bpm(140) - bpm(120)).toBeCloseTo(20 / 512, 12)
  })
})

describe("the view of an automation", () => {
  const tempo = {
    target: { type: "tempo" } as const,
    points: [point(bpm(120))],
  }
  const fader = {
    target: { type: "trackVolume", track: 1 } as const,
    points: [point(0.5), point(0.7, 960)],
  }

  it("is the window about the tempo for a tempo curve, the whole range for the rest", () => {
    expect(inBpm(viewOfAutomation(tempo, 120, undefined))).toEqual([100, 140])
    expect(viewOfAutomation(fader, 120, undefined)).toBe(FULL_VIEW)
  })

  it("is the chosen one where one was chosen", () => {
    const chosen = { lo: 0.1, hi: 0.3 }
    expect(viewOfAutomation(tempo, 120, chosen)).toBe(chosen)
    expect(viewOfAutomation(fader, 120, chosen)).toBe(chosen)
  })

  it("fits a tempo curve no tighter than 40 bpm, and another curve to its points", () => {
    const [low, high] = inBpm(fittedView(tempo))
    expect(high - low).toBeCloseTo(40, 6)
    expect((low + high) / 2).toBeCloseTo(120, 6)
    const fitted = fittedView(fader)
    expect(fitted.lo).toBeCloseTo(0.476, 12)
    expect(fitted.hi).toBeCloseTo(0.724, 12)
  })
})
