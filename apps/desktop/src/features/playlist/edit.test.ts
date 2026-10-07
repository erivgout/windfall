import { describe, expect, it } from "vitest"

import type { Clip, ClipContent } from "@/bindings"

import {
  clampMove,
  clampNudge,
  clipboardFrom,
  clipInits,
  clipUpdates,
  cloneMoved,
  duplicateRight,
  endResizeDelta,
  minResizeLength,
  moveChanges,
  paintStarts,
  pasteAt,
  resizeChanges,
  snapCell,
  snapNearest,
  songEnd,
  startTrimDelta,
  strokeBox,
  trackDropIndex,
  tracksNeeded,
  withoutStacked,
  wrapTicks,
  type NewClip,
} from "./edit"

const BAR = 3840

/** A clip of pattern 1. Track ids are 100 plus the row. */
function clip(id: number, row: number, start: number, length = BAR): Clip {
  return {
    id,
    track: 100 + row,
    start,
    length,
    offset: 0,
    muted: false,
    content: { type: "pattern", pattern: 1 },
  }
}

const rowOf = (track: number) => track - 100
const spot = (item: Clip) => ({ start: item.start, row: rowOf(item.track) })
const pass = () => BAR

describe("snapping", () => {
  it("rounds to the nearest grid line and never goes before the song", () => {
    expect(snapNearest(1900, BAR)).toBe(0)
    expect(snapNearest(1921, BAR)).toBe(BAR)
    expect(snapNearest(-5000, BAR)).toBe(0)
    expect(snapNearest(1234.4, 0)).toBe(1234)
  })

  it("finds the cell the pointer is in", () => {
    expect(snapCell(3839, BAR)).toBe(0)
    expect(snapCell(3840, BAR)).toBe(BAR)
    expect(snapCell(7000, BAR)).toBe(BAR)
    expect(snapCell(-10, BAR)).toBe(0)
    expect(snapCell(100.6, 0)).toBe(101)
  })

  it("wraps into the pattern, also from below zero", () => {
    expect(wrapTicks(4000, BAR)).toBe(160)
    expect(wrapTicks(-240, BAR)).toBe(BAR - 240)
    expect(wrapTicks(BAR, BAR)).toBe(0)
    expect(wrapTicks(5, 0)).toBe(0)
  })
})

describe("songEnd", () => {
  it("is the end of the clip that ends last", () => {
    expect(songEnd([])).toBe(0)
    expect(songEnd([clip(1, 0, 0, 8 * BAR), clip(2, 1, 4 * BAR, BAR)])).toBe(
      8 * BAR
    )
  })
})

describe("clampMove", () => {
  const items = [clip(1, 1, 2 * BAR), clip(2, 3, 5 * BAR)].map(spot)

  it("lands the grabbed clip on the grid and moves the rest as far", () => {
    expect(clampMove(items, 2 * BAR, BAR + 700, 1, BAR, 12)).toEqual({
      ticks: BAR,
      rows: 1,
    })
  })

  it("puts a clip that was off the grid onto it", () => {
    const off = [{ start: 2 * BAR + 100, row: 0 }]
    expect(clampMove(off, 2 * BAR + 100, 0, 0, BAR, 12).ticks).toBe(-100)
  })

  it("stops at the start of the song", () => {
    expect(clampMove(items, 2 * BAR, -9 * BAR, 0, BAR, 12).ticks).toBe(-2 * BAR)
  })

  it("keeps every clip on a row", () => {
    expect(clampMove(items, 2 * BAR, 0, -5, BAR, 12).rows).toBe(-1)
    expect(clampMove(items, 2 * BAR, 0, 50, BAR, 12).rows).toBe(8)
  })

  it("moves freely with the snap off", () => {
    expect(clampMove(items, 2 * BAR, 123.4, 0, 0, 12).ticks).toBe(123)
  })

  it("does nothing with nothing to move", () => {
    expect(clampMove([], 0, 500, 2, BAR, 12)).toEqual({ ticks: 0, rows: 0 })
  })
})

describe("clampNudge", () => {
  const items = [clip(1, 0, BAR), clip(2, 2, 3 * BAR)].map(spot)

  it("moves by the step it is given, without snapping", () => {
    expect(clampNudge(items, 240, 0, 12)).toEqual({ ticks: 240, rows: 0 })
  })

  it("stops at the start of the song and at the first and last row", () => {
    expect(clampNudge(items, -2 * BAR, -1, 12)).toEqual({
      ticks: -BAR,
      rows: 0,
    })
    expect(clampNudge(items, 0, 99, 12).rows).toBe(9)
  })
})

describe("moveChanges", () => {
  it("names only what changed", () => {
    const clips = [clip(1, 0, 0), clip(2, 2, BAR)]
    expect(moveChanges(clips, rowOf, BAR, 0)).toEqual([
      { id: 1, start: BAR },
      { id: 2, start: 2 * BAR },
    ])
    expect(moveChanges(clips, rowOf, 0, 1)).toEqual([
      { id: 1, row: 1 },
      { id: 2, row: 3 },
    ])
    expect(moveChanges(clips, rowOf, 0, 0)).toEqual([])
  })
})

describe("resizing", () => {
  it("measures how far the end has to move to meet the pointer", () => {
    const item = clip(1, 0, BAR, BAR)
    expect(endResizeDelta(item, 4 * BAR + 300, BAR)).toBe(2 * BAR)
    expect(endResizeDelta(item, BAR + 100, BAR)).toBe(-BAR)
  })

  it("lets a clip run past its pattern, which loops inside it", () => {
    expect(
      resizeChanges([clip(1, 0, 0)], 0, 3 * BAR, minResizeLength(BAR), pass)
    ).toEqual([{ id: 1, length: 4 * BAR }])
  })

  it("never shrinks a clip below one grid cell", () => {
    const long = clip(1, 0, 0, 4 * BAR)
    expect(
      resizeChanges([long], 0, -9 * BAR, minResizeLength(BAR), pass)
    ).toEqual([{ id: 1, length: BAR }])
  })

  it("leaves a clip that is already shorter than a cell as it is", () => {
    const short = clip(1, 0, 0, 480)
    expect(
      resizeChanges([short], 0, -9 * BAR, minResizeLength(BAR), pass)
    ).toEqual([])
  })

  it("uses a small minimum when the snap is off", () => {
    expect(minResizeLength(0)).toBe(60)
    expect(minResizeLength(960)).toBe(960)
  })

  it("resizes every selected clip by the same amount", () => {
    const clips = [clip(1, 0, 0, 2 * BAR), clip(2, 1, BAR, BAR)]
    expect(resizeChanges(clips, 0, BAR, BAR, pass)).toEqual([
      { id: 1, length: 3 * BAR },
      { id: 2, length: 2 * BAR },
    ])
  })
})

describe("trimming the start", () => {
  it("measures how far the start has to move to meet the pointer", () => {
    const item = clip(1, 0, 2 * BAR, 4 * BAR)
    expect(startTrimDelta([item], item, 3 * BAR + 90, BAR)).toBe(BAR)
    expect(startTrimDelta([item], item, BAR - 90, BAR)).toBe(-BAR)
  })

  it("does not pull any selected clip before the start of the song", () => {
    const grabbed = clip(1, 0, 4 * BAR)
    const early = clip(2, 1, BAR)
    expect(startTrimDelta([grabbed, early], grabbed, 0, BAR)).toBe(-BAR)
  })

  it("keeps the notes in place by moving the offset with the start", () => {
    const item = clip(1, 0, 2 * BAR, 4 * BAR)
    // A step into the clip: the clip now starts a step into its pattern.
    expect(resizeChanges([item], 240, 0, 240, pass)).toEqual([
      { id: 1, start: 2 * BAR + 240, offset: 240, length: 4 * BAR - 240 },
    ])
  })

  it("adds to an offset the clip already has, wrapping at the pattern", () => {
    const item = { ...clip(1, 0, 0, 4 * BAR), offset: BAR - 240 }
    expect(resizeChanges([item], 480, 0, 240, pass)).toEqual([
      { id: 1, start: 480, offset: 240, length: 4 * BAR - 480 },
    ])
  })

  it("uncovers the end of the previous pass when pulled to the left", () => {
    const item = clip(1, 0, 2 * BAR, BAR)
    expect(resizeChanges([item], -960, 0, 240, pass)).toEqual([
      { id: 1, start: 2 * BAR - 960, offset: BAR - 960, length: BAR + 960 },
    ])
  })

  it("stops before the clip is shorter than a grid cell", () => {
    const item = clip(1, 0, 0, 2 * BAR)
    expect(resizeChanges([item], 5 * BAR, 0, BAR, pass)).toEqual([
      { id: 1, start: BAR, offset: 0, length: BAR },
    ])
  })
})

describe("paintStarts", () => {
  it("lays passes back to back from the first cell to the pointer", () => {
    expect(paintStarts(2 * BAR, 2 * BAR + 10, BAR)).toEqual([2 * BAR])
    expect(paintStarts(2 * BAR, 4 * BAR + 10, BAR)).toEqual([
      2 * BAR,
      3 * BAR,
      4 * BAR,
    ])
  })

  it("uses the pattern's own length, whatever the grid is", () => {
    expect(paintStarts(0, 5 * BAR, 2 * BAR)).toEqual([0, 2 * BAR, 4 * BAR])
  })

  it("paints to the left too, but not before the start of the song", () => {
    expect(paintStarts(2 * BAR, BAR - 5, BAR)).toEqual([0, BAR, 2 * BAR])
    expect(paintStarts(BAR, -6 * BAR, BAR)).toEqual([0, BAR])
  })
})

describe("withoutStacked", () => {
  const fresh = (row: number, start: number, pattern = 1): NewClip => ({
    row,
    start,
    length: BAR,
    offset: 0,
    muted: false,
    content: { type: "pattern", pattern },
  })

  it("drops a clip that would sit exactly on the same thing", () => {
    const existing = [clip(1, 0, BAR)]
    expect(
      withoutStacked([fresh(0, 0), fresh(0, BAR)], existing, rowOf)
    ).toEqual([fresh(0, 0)])
  })

  it("keeps clips that only overlap, or play another pattern", () => {
    const existing = [clip(1, 0, BAR)]
    const candidates = [fresh(0, BAR + 240), fresh(1, BAR), fresh(0, BAR, 2)]
    expect(withoutStacked(candidates, existing, rowOf)).toEqual(candidates)
  })
})

describe("copies", () => {
  const first = { ...clip(1, 0, 2 * BAR, 2 * BAR), offset: 240, muted: true }
  const second = clip(2, 2, 3 * BAR, BAR)

  it("clones clips where a drag put them", () => {
    expect(cloneMoved([first], rowOf, BAR, 1)).toEqual([
      {
        row: 1,
        start: 3 * BAR,
        length: 2 * BAR,
        offset: 240,
        muted: true,
        content: { type: "pattern", pattern: 1 },
      },
    ])
  })

  it("duplicates a selection right after itself", () => {
    const copies = duplicateRight([first, second], rowOf, BAR)
    expect(copies.map((copy) => [copy.row, copy.start])).toEqual([
      [0, 4 * BAR],
      [2, 5 * BAR],
    ])
  })

  it("keeps duplicates on the grid when the selection is not a whole cell", () => {
    const odd = clip(1, 0, 0, BAR + 960)
    expect(duplicateRight([odd], rowOf, BAR)[0].start).toBe(2 * BAR)
    expect(duplicateRight([odd], rowOf, 0)[0].start).toBe(BAR + 960)
    expect(duplicateRight([], rowOf, BAR)).toEqual([])
  })

  it("copies with times counted from the earliest clip", () => {
    expect(
      clipboardFrom([first, second], rowOf).map((item) => item.start)
    ).toEqual([0, BAR])
  })

  it("pastes at a snapped tick, on the rows the clips came from", () => {
    const copied = clipboardFrom([first, second], rowOf)
    const pasted = pasteAt(copied, 8 * BAR + 500, BAR, () => true)
    expect(pasted.map((item) => [item.row, item.start])).toEqual([
      [0, 8 * BAR],
      [2, 9 * BAR],
    ])
    expect(pasted[0]).toMatchObject({ offset: 240, muted: true })
  })

  it("leaves out clips whose pattern is gone", () => {
    const copied: NewClip[] = [
      {
        ...clipboardFrom([first], rowOf)[0],
        content: { type: "pattern", pattern: 7 },
      },
      ...clipboardFrom([second], rowOf),
    ]
    const exists = (content: ClipContent) =>
      content.type === "pattern" && content.pattern === 1
    expect(pasteAt(copied, 0, BAR, exists)).toHaveLength(1)
  })
})

describe("tracks for rows", () => {
  it("counts the tracks that must be added", () => {
    expect(tracksNeeded(0, [0])).toBe(1)
    expect(tracksNeeded(2, [4, 1])).toBe(3)
    expect(tracksNeeded(5, [4, 1])).toBe(0)
    expect(tracksNeeded(3, [])).toBe(0)
  })

  it("turns rows into track ids and keeps numbers whole and in range", () => {
    const made: NewClip = {
      row: 1,
      start: -5.4,
      length: 0,
      offset: 0,
      muted: false,
      content: { type: "pattern", pattern: 3 },
    }
    expect(clipInits([made], [10, 11])).toEqual([
      {
        track: 11,
        start: 0,
        length: 1,
        content: { type: "pattern", pattern: 3 },
      },
    ])
  })

  it("gives a copy its offset and its mute in the one command", () => {
    const plain: NewClip = {
      row: 0,
      start: 0,
      length: BAR,
      offset: 0,
      muted: false,
      content: { type: "pattern", pattern: 1 },
    }
    const inits = clipInits(
      [plain, { ...plain, offset: 240.4 }, { ...plain, muted: true }],
      [7]
    )
    expect(inits.map((init) => [init.offset, init.muted])).toEqual([
      [undefined, undefined],
      [240, undefined],
      [undefined, true],
    ])
  })

  it("carries an audio clip's own settings into its copy", () => {
    const content: ClipContent = {
      type: "audio",
      sample: 4,
      mixerTrack: 9,
      gain: 0.5,
      pan: -1,
      fadeIn: 240,
      fadeOut: 480,
      reverse: true,
      pitch: -12,
    }
    const audio: Clip = { ...clip(5, 1, BAR, 2 * BAR), offset: 960, content }
    const [copy] = cloneMoved([audio], rowOf, BAR, 0)
    expect(clipInits([copy], [10, 11])).toEqual([
      { track: 11, start: 2 * BAR, length: 2 * BAR, offset: 960, content },
    ])
  })

  it("builds updates with only the changed fields", () => {
    expect(
      clipUpdates(
        [
          { id: 1, row: 2, start: 100.6 },
          { id: 2, length: 0, offset: -3 },
          { id: 3, muted: true },
        ],
        [10, 11, 12]
      )
    ).toEqual([
      { id: 1, patch: { track: 12, start: 101 } },
      { id: 2, patch: { length: 1, offset: 0 } },
      { id: 3, patch: { muted: true } },
    ])
  })
})

describe("clips that do not loop", () => {
  const audio = (id: number, start: number, offset: number): Clip => ({
    ...clip(id, 0, start, 2 * BAR),
    offset,
    content: {
      type: "audio",
      sample: 1,
      mixerTrack: 0,
      gain: 1,
      pan: 0,
      fadeIn: 0,
      fadeOut: 0,
      reverse: false,
      pitch: 0,
    },
  })
  const noLoop = () => 0

  it("stops a left edge where the audio begins", () => {
    const trimmed = audio(1, 4 * BAR, 960)
    // There is a beat of audio before the clip's start, and no more.
    expect(startTrimDelta([trimmed], trimmed, 0, BAR, noLoop)).toBe(-960)
    expect(startTrimDelta([trimmed], trimmed, 5 * BAR, BAR, noLoop)).toBe(BAR)
    // A pattern clip in the same place loops, so it can go all the way.
    const looping = clip(2, 0, 4 * BAR)
    expect(startTrimDelta([looping], looping, 0, BAR, pass)).toBe(-4 * BAR)
    // The tightest clip of the selection holds the others back.
    const byKind = (item: Clip) => (item.content.type === "pattern" ? BAR : 0)
    expect(startTrimDelta([trimmed, looping], looping, 0, BAR, byKind)).toBe(
      -960
    )
  })

  it("moves the offset with the left edge, and never below zero", () => {
    const trimmed = audio(1, 4 * BAR, 960)
    expect(resizeChanges([trimmed], 480, 0, 60, noLoop)).toEqual([
      { id: 1, start: 4 * BAR + 480, offset: 1440, length: 2 * BAR - 480 },
    ])
    expect(resizeChanges([trimmed], -960, 0, 60, noLoop)).toEqual([
      { id: 1, start: 4 * BAR - 960, offset: 0, length: 2 * BAR + 960 },
    ])
    expect(resizeChanges([trimmed], -2000, 0, 60, noLoop)[0].offset).toBe(0)
  })

  it("lets the right edge run past the end of the audio", () => {
    const whole = audio(1, 0, 0)
    expect(resizeChanges([whole], 0, 6 * BAR, 60, noLoop)).toEqual([
      { id: 1, length: 8 * BAR },
    ])
  })
})

describe("trackDropIndex", () => {
  it("turns a gap between tracks into the place the track ends up in", () => {
    // Four tracks. Gap 0 is above the first, gap 4 below the last.
    expect(trackDropIndex(0, 4, 4)).toBe(3)
    expect(trackDropIndex(0, 2, 4)).toBe(1)
    expect(trackDropIndex(3, 0, 4)).toBe(0)
    expect(trackDropIndex(2, 1, 4)).toBe(1)
  })

  it("is null for the two gaps beside the track, and for no track", () => {
    expect(trackDropIndex(1, 1, 4)).toBeNull()
    expect(trackDropIndex(1, 2, 4)).toBeNull()
    expect(trackDropIndex(7, 0, 4)).toBeNull()
    expect(trackDropIndex(-1, 0, 4)).toBeNull()
  })
})

describe("strokeBox", () => {
  it("covers the path between two points, whichever way it went", () => {
    const box = strokeBox({ tick: 500, row: 3.2 }, { tick: 100, row: 1.5 })
    expect(box.tick0).toBe(100)
    expect(box.tick1).toBeGreaterThan(500)
    expect(box.row0).toBe(1.5)
    expect(box.row1).toBeGreaterThan(3.2)
  })

  it("still has an area when the pointer has not moved", () => {
    const point = { tick: 100, row: 2.5 }
    const box = strokeBox(point, point)
    expect(box.tick1).toBeGreaterThan(box.tick0)
    expect(box.row1).toBeGreaterThan(box.row0)
  })
})
