import { describe, expect, it } from "vitest"

import type { Note } from "@/bindings"

import { BarColumns } from "./lane-bars"
import {
  baselineY,
  LANE_PAD_PX,
  laneUpdates,
  nearestBar,
  paintValues,
  scaleValues,
  valueAtY,
  yOfValue,
} from "./lane-math"

function note(id: number, start: number, velocity = 0.8, pan = 0): Note {
  return { id, start, length: 240, key: 60, velocity, pan }
}

const HEIGHT = 110

describe("lane geometry", () => {
  it("puts full velocity at the top and silence at the bottom", () => {
    expect(valueAtY(LANE_PAD_PX, HEIGHT, "velocity")).toBe(1)
    expect(valueAtY(HEIGHT - LANE_PAD_PX, HEIGHT, "velocity")).toBe(0)
    expect(valueAtY(HEIGHT / 2, HEIGHT, "velocity")).toBe(0.5)
  })

  it("clamps above and below the lane", () => {
    expect(valueAtY(-40, HEIGHT, "velocity")).toBe(1)
    expect(valueAtY(999, HEIGHT, "velocity")).toBe(0)
    expect(valueAtY(-40, HEIGHT, "pan")).toBe(1)
    expect(valueAtY(999, HEIGHT, "pan")).toBe(-1)
  })

  it("centers pan and grows its bars from the middle", () => {
    expect(valueAtY(HEIGHT / 2, HEIGHT, "pan")).toBe(0)
    expect(baselineY(HEIGHT, "pan")).toBe(HEIGHT / 2)
    expect(baselineY(HEIGHT, "velocity")).toBe(HEIGHT - LANE_PAD_PX)
  })

  it("draws a value where a click on it would read it back", () => {
    for (const value of [0, 0.25, 0.8, 1]) {
      const y = yOfValue(value, HEIGHT, "velocity")
      expect(valueAtY(y, HEIGHT, "velocity")).toBe(value)
    }
  })
})

describe("picking a bar", () => {
  const notes = [note(1, 0), note(2, 480), note(3, 480), note(4, 960)]

  it("takes the nearest bar within reach", () => {
    expect(nearestBar(notes, 30, 100, new Set())?.id).toBe(1)
    expect(nearestBar(notes, 900, 100, new Set())?.id).toBe(4)
    expect(nearestBar(notes, 250, 100, new Set())).toBeNull()
  })

  it("prefers a selected note among notes on the same tick", () => {
    expect(nearestBar(notes, 480, 50, new Set())?.id).toBe(2)
    expect(nearestBar(notes, 480, 50, new Set([3]))?.id).toBe(3)
  })
})

describe("painting values", () => {
  const notes = [note(1, 0), note(2, 240), note(3, 480), note(4, 960)]

  it("sets every bar the drag passed, along a straight line", () => {
    const values = new Map<number, number>()
    paintValues(
      notes,
      "velocity",
      { tick: 0, value: 1 },
      { tick: 480, value: 0 },
      0,
      values
    )
    expect([...values]).toEqual([
      [1, 1],
      [2, 0.5],
      [3, 0],
    ])
  })

  it("works right to left as well", () => {
    const values = new Map<number, number>()
    paintValues(
      notes,
      "velocity",
      { tick: 480, value: 0.2 },
      { tick: 0, value: 0.6 },
      0,
      values
    )
    expect(values.get(1)).toBe(0.6)
    expect(values.get(2)).toBe(0.4)
    expect(values.get(3)).toBe(0.2)
  })

  it("keeps hold of the bar under a drag that only goes up and down", () => {
    const values = new Map<number, number>()
    const here = { tick: 250, value: 0.3 }
    paintValues(notes, "velocity", here, here, 20, values)
    expect([...values]).toEqual([[2, 0.3]])
  })

  it("leaves unselected bars alone when told which ones may change", () => {
    const values = new Map<number, number>()
    paintValues(
      notes,
      "velocity",
      { tick: 0, value: 0.1 },
      { tick: 960, value: 0.1 },
      0,
      values,
      new Set([2, 4])
    )
    expect([...values.keys()]).toEqual([2, 4])
  })
})

describe("scaling values", () => {
  it("keeps the proportions of velocities", () => {
    const notes = [note(1, 0, 0.8), note(2, 240, 0.4)]
    const values = scaleValues(notes, "velocity", 0.8, 0.4)
    expect(values.get(1)).toBe(0.4)
    expect(values.get(2)).toBe(0.2)
  })

  it("clamps at full velocity", () => {
    const notes = [note(1, 0, 0.5), note(2, 240, 0.9)]
    const values = scaleValues(notes, "velocity", 0.5, 1)
    expect(values.get(1)).toBe(1)
    expect(values.get(2)).toBe(1)
  })

  it("adds when the grabbed bar is at zero", () => {
    const notes = [note(1, 0, 0), note(2, 240, 0.5)]
    const values = scaleValues(notes, "velocity", 0, 0.25)
    expect(values.get(1)).toBe(0.25)
    expect(values.get(2)).toBe(0.75)
  })

  it("shifts pans by the same amount", () => {
    const notes = [note(1, 0, 0.8, -0.5), note(2, 240, 0.8, 0.25)]
    const values = scaleValues(notes, "pan", -0.5, 0)
    expect(values.get(1)).toBe(0)
    expect(values.get(2)).toBe(0.75)
  })
})

describe("lane updates", () => {
  it("lists only the notes whose value changed", () => {
    const notes = [note(1, 0, 0.8), note(2, 240, 0.4)]
    const values = new Map([
      [1, 0.8],
      [2, 0.6],
    ])
    expect(laneUpdates(notes, "velocity", values)).toEqual([
      { id: 2, patch: { velocity: 0.6 } },
    ])
    expect(laneUpdates(notes, "pan", new Map([[1, -1]]))).toEqual([
      { id: 1, patch: { pan: -1 } },
    ])
  })
})

describe("bars merged by pixel column", () => {
  it("keeps one bar per column, from the lowest to the highest value", () => {
    const columns = new BarColumns()
    columns.reset(100)
    columns.add(10, 40, 90)
    columns.add(10, 20, 90)
    columns.add(10, 60, 90)
    columns.add(11, 95, 90)
    const drawn: number[][] = []
    columns.forEach((x, top, bottom) => drawn.push([x, top, bottom]))
    expect(drawn).toEqual([
      [10, 20, 90],
      [11, 90, 95],
    ])
  })

  it("ignores bars outside the lane and starts clean each frame", () => {
    const columns = new BarColumns()
    columns.reset(50)
    columns.add(-1, 10, 40)
    columns.add(50, 10, 40)
    columns.add(5, 10, 40)
    expect(columns.count).toBe(1)
    columns.reset(50)
    expect(columns.count).toBe(0)
    columns.add(5, 30, 40)
    const drawn: number[][] = []
    columns.forEach((x, top, bottom) => drawn.push([x, top, bottom]))
    expect(drawn).toEqual([[5, 30, 40]])
  })
})
