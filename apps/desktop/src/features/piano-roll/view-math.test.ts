import { describe, expect, it } from "vitest"

import type { Clip, Note, PlaylistTrack, TimeSignature } from "@/bindings"
import { gridLevels, type Viewport, type ViewportLimits } from "@/lib/canvas"

import { pressIntent } from "./intents"
import {
  gridSpecFor,
  SNAP_OPTIONS,
  snapCeil,
  snapFloor,
  snapRound,
  snapTicks,
} from "./snap"
import {
  clampRowHeight,
  contentTicks,
  fitViewport,
  MAX_ROW_HEIGHT,
  MIN_ROW_HEIGHT,
  openingViewport,
  patternTickInSong,
  startAtThumbOffset,
  steppedRowHeight,
  thumbGeometry,
} from "./view-math"

const FOUR_FOUR: TimeSignature = { numerator: 4, denominator: 4 }
const SIX_EIGHT: TimeSignature = { numerator: 6, denominator: 8 }

const LIMITS: ViewportLimits = {
  rowCount: 128,
  contentTicks: 3840 * 64,
  minPxPerTick: 0.0012,
  maxPxPerTick: 2,
  minRowHeight: MIN_ROW_HEIGHT,
  maxRowHeight: MAX_ROW_HEIGHT,
}

const VIEWPORT: Viewport = {
  width: 1000,
  height: 400,
  dpr: 1,
  scrollTick: 0,
  scrollRow: 0,
  pxPerTick: 0.08,
  rowHeight: 16,
}

function note(start: number, key: number, length = 240): Note {
  return { id: start + key, start, length, key, velocity: 0.8, pan: 0 }
}

describe("snap", () => {
  it("offers FL's set, from none to a bar", () => {
    expect(SNAP_OPTIONS.map((option) => option.id)).toEqual([
      "none",
      "step/6",
      "step/4",
      "step/3",
      "step/2",
      "step",
      "beat/6",
      "beat/4",
      "beat/3",
      "beat/2",
      "beat",
      "bar",
    ])
  })

  it("turns each choice into ticks", () => {
    const ticks = (id: (typeof SNAP_OPTIONS)[number]["id"]) =>
      snapTicks(id, FOUR_FOUR)
    expect(ticks("none")).toBe(0)
    expect(ticks("step/6")).toBe(40)
    expect(ticks("step/3")).toBe(80)
    expect(ticks("step")).toBe(240)
    expect(ticks("beat/3")).toBe(320)
    expect(ticks("beat")).toBe(960)
    expect(ticks("bar")).toBe(3840)
  })

  it("follows the time signature for beats and bars", () => {
    expect(snapTicks("beat", SIX_EIGHT)).toBe(480)
    expect(snapTicks("bar", SIX_EIGHT)).toBe(2880)
    expect(snapTicks("step", SIX_EIGHT)).toBe(240)
  })

  it("rounds down, up and to the nearest line", () => {
    expect(snapFloor(479, 240)).toBe(240)
    expect(snapCeil(241, 240)).toBe(480)
    expect(snapRound(359, 240)).toBe(240)
    expect(snapRound(360, 240)).toBe(480)
    expect(snapFloor(12.6, 0)).toBe(13)
  })
})

describe("grid lines", () => {
  it("draws the snap while its lines are far enough apart", () => {
    const spec = gridSpecFor(60, 0.2, FOUR_FOUR)
    expect(spec).toEqual({ ticksPerStep: 60, stepsPerBeat: 16, beatsPerBar: 4 })
    expect(gridLevels(0.2, spec)).toEqual({ minor: 60, mid: 960, strong: 3840 })
  })

  it("falls back to steps, then beats, when zoomed out", () => {
    expect(gridSpecFor(60, 0.08, FOUR_FOUR).ticksPerStep).toBe(240)
    expect(gridSpecFor(60, 0.01, FOUR_FOUR).ticksPerStep).toBe(960)
  })

  it("shows triplets as three lines to the beat", () => {
    expect(gridSpecFor(320, 0.08, FOUR_FOUR)).toEqual({
      ticksPerStep: 320,
      stepsPerBeat: 3,
      beatsPerBar: 4,
    })
  })

  it("shows steps when snap is off or coarser than a beat", () => {
    expect(gridSpecFor(0, 0.08, FOUR_FOUR).ticksPerStep).toBe(240)
    expect(gridSpecFor(3840, 0.08, FOUR_FOUR).ticksPerStep).toBe(240)
    expect(gridSpecFor(960, 0.08, FOUR_FOUR).ticksPerStep).toBe(960)
  })

  it("counts beats to the bar from the time signature", () => {
    expect(gridSpecFor(240, 0.08, SIX_EIGHT)).toEqual({
      ticksPerStep: 240,
      stepsPerBeat: 2,
      beatsPerBar: 6,
    })
  })
})

describe("press intents", () => {
  it("draws, paints or selects on empty space by tool", () => {
    expect(pressIntent("draw", "left", null, { ctrl: false }).kind).toBe("draw")
    expect(pressIntent("paint", "left", null, { ctrl: false }).kind).toBe(
      "paint"
    )
    expect(pressIntent("select", "left", null, { ctrl: false }).kind).toBe(
      "marquee"
    )
    expect(pressIntent("erase", "left", null, { ctrl: false }).kind).toBe(
      "erase"
    )
  })

  it("moves a note by its body and resizes it by its edges", () => {
    for (const tool of ["draw", "paint", "select"] as const) {
      expect(pressIntent(tool, "left", "body", { ctrl: false })).toEqual({
        kind: "move",
      })
      expect(pressIntent(tool, "left", "end-edge", { ctrl: false })).toEqual({
        kind: "resize",
        edge: "end",
      })
      expect(pressIntent(tool, "left", "start-edge", { ctrl: false })).toEqual({
        kind: "resize",
        edge: "start",
      })
    }
    expect(pressIntent("erase", "left", "body", { ctrl: false }).kind).toBe(
      "erase"
    )
  })

  it("selects with Ctrl in every tool, even on a note", () => {
    for (const tool of ["draw", "paint", "select", "erase"] as const) {
      expect(pressIntent(tool, "left", "body", { ctrl: true }).kind).toBe(
        "marquee"
      )
    }
  })

  it("deletes with the right button, except in the select tool", () => {
    expect(pressIntent("draw", "right", "body", { ctrl: false }).kind).toBe(
      "erase"
    )
    expect(pressIntent("paint", "right", null, { ctrl: false }).kind).toBe(
      "erase"
    )
    expect(pressIntent("select", "right", "body", { ctrl: false }).kind).toBe(
      "menu"
    )
  })
})

describe("row height", () => {
  it("stays whole and in range", () => {
    expect(clampRowHeight(15.6)).toBe(16)
    expect(clampRowHeight(1)).toBe(MIN_ROW_HEIGHT)
    expect(clampRowHeight(500)).toBe(MAX_ROW_HEIGHT)
  })

  it("always moves at least a pixel per zoom notch", () => {
    expect(steppedRowHeight(6, 1.1)).toBe(7)
    expect(steppedRowHeight(7, 1 / 1.1)).toBe(6)
    expect(steppedRowHeight(20, 1.15)).toBe(23)
    expect(steppedRowHeight(MAX_ROW_HEIGHT, 1.15)).toBe(MAX_ROW_HEIGHT)
  })
})

describe("scrollable extent", () => {
  it("leaves room to the right of the pattern and of the last note", () => {
    expect(contentTicks(16, 0, 3840)).toBe(16 * 3840)
    expect(contentTicks(160, 0, 3840)).toBe(160 * 240 + 8 * 3840)
    expect(contentTicks(16, 100_000, 3840)).toBe(100_000 + 8 * 3840)
  })
})

describe("fitting notes", () => {
  it("shows a box of notes with air around it", () => {
    const extent = { start: 3840, end: 7680, lowKey: 60, highKey: 72 }
    const fitted = fitViewport(VIEWPORT, extent, LIMITS)
    const left = (extent.start - fitted.scrollTick) * fitted.pxPerTick
    const right = (extent.end - fitted.scrollTick) * fitted.pxPerTick
    expect(left).toBeCloseTo(24)
    expect(right).toBeCloseTo(VIEWPORT.width - 24)
    const top = (127 - extent.highKey - fitted.scrollRow) * fitted.rowHeight
    const bottom = (128 - extent.lowKey - fitted.scrollRow) * fitted.rowHeight
    expect(top).toBeGreaterThan(0)
    expect(bottom).toBeLessThan(VIEWPORT.height)
    expect(Number.isInteger(fitted.rowHeight)).toBe(true)
  })

  it("does not blow a single note up to fill the view", () => {
    const fitted = fitViewport(
      VIEWPORT,
      { start: 0, end: 240, lowKey: 60, highKey: 60 },
      LIMITS
    )
    expect(fitted.rowHeight).toBeLessThanOrEqual(22)
    expect(fitted.pxPerTick).toBeLessThanOrEqual(LIMITS.maxPxPerTick)
  })

  it("opens an empty lane around C5", () => {
    const opened = openingViewport(VIEWPORT, [], LIMITS)
    const middleRow = opened.scrollRow + VIEWPORT.height / opened.rowHeight / 2
    expect(middleRow).toBeCloseTo(127 - 60 + 0.5)
    expect(opened.scrollTick).toBe(0)
  })

  it("opens a lane on its notes", () => {
    const opened = openingViewport(
      { ...VIEWPORT, scrollTick: 9000 },
      [note(0, 36), note(240, 40)],
      LIMITS
    )
    const middleRow = opened.scrollRow + VIEWPORT.height / opened.rowHeight / 2
    expect(middleRow).toBeCloseTo(127 - 38 + 0.5)
    expect(opened.scrollTick).toBe(0)
  })

  it("starts from the top note when the notes are taller than the view", () => {
    const opened = openingViewport(
      VIEWPORT,
      [note(0, 20), note(240, 110)],
      LIMITS
    )
    expect(opened.scrollRow).toBe(127 - 110 - 1)
  })
})

describe("scrollbar thumb", () => {
  it("sizes and places the thumb from the visible range", () => {
    expect(thumbGeometry(0.25, 0.5, 400, 20)).toEqual({
      offset: (0.25 / 0.75) * 300,
      size: 100,
    })
  })

  it("never gets too small to grab and still reaches both ends", () => {
    expect(thumbGeometry(0, 0.001, 400, 28)).toEqual({ offset: 0, size: 28 })
    expect(thumbGeometry(0.999, 1, 400, 28)).toEqual({ offset: 372, size: 28 })
  })

  it("fills the track when everything is visible", () => {
    expect(thumbGeometry(0, 1, 400, 28)).toEqual({ offset: 0, size: 400 })
  })

  it("turns a thumb position back into the same scroll position", () => {
    const { offset } = thumbGeometry(0.3, 0.5, 400, 28)
    expect(startAtThumbOffset(offset, 0.2, 400, 28)).toBeCloseTo(0.3)
    expect(startAtThumbOffset(-50, 0.2, 400, 28)).toBe(0)
    expect(startAtThumbOffset(9999, 0.2, 400, 28)).toBeCloseTo(0.8)
  })
})

describe("the song's playhead inside a pattern", () => {
  const tracks: PlaylistTrack[] = [
    { id: 10, name: "Track 1", muted: false },
    { id: 11, name: "Track 2", muted: true },
  ]
  function clip(patch: Partial<Clip>): Clip {
    return {
      id: 1,
      track: 10,
      start: 0,
      length: 3840,
      offset: 0,
      muted: false,
      content: { type: "pattern", pattern: 5 },
      ...patch,
    }
  }

  it("is the position inside the clip that plays the pattern", () => {
    const clips = [clip({ start: 7680, length: 3840 })]
    expect(patternTickInSong(7680 + 500, clips, tracks, 5, 3840)).toBe(500)
  })

  it("wraps as the clip loops the pattern, and honors the clip offset", () => {
    const clips = [clip({ start: 0, length: 3840 * 3, offset: 960 })]
    expect(patternTickInSong(100, clips, tracks, 5, 3840)).toBe(1060)
    expect(patternTickInSong(3840, clips, tracks, 5, 3840)).toBe(960)
  })

  it("is nothing outside the pattern's clips", () => {
    const clips = [clip({ start: 3840, length: 3840 })]
    expect(patternTickInSong(100, clips, tracks, 5, 3840)).toBeNull()
    expect(patternTickInSong(7680, clips, tracks, 5, 3840)).toBeNull()
    expect(patternTickInSong(4000, clips, tracks, 6, 3840)).toBeNull()
  })

  it("skips muted clips and clips on muted tracks", () => {
    expect(
      patternTickInSong(100, [clip({ muted: true })], tracks, 5, 3840)
    ).toBeNull()
    expect(
      patternTickInSong(100, [clip({ track: 11 })], tracks, 5, 3840)
    ).toBeNull()
  })
})
