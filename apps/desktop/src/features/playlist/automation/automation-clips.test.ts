import { toast } from "sonner"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { AutomationPoint, AutomationTarget, Clip } from "@/bindings"
import { curveValue } from "@/lib/automation/curve"
import type { Backend } from "@/lib/ipc"
import { dispatch, receivePatch, redo, undo } from "@/lib/store/project"
import { settle } from "@/test/harness"

import {
  answerText,
  BAR,
  history,
  labels,
  project,
  PX_PER_TICK,
  startPlaylist,
  startSession,
  tracks,
  ui,
} from "../test-utils"
import { pointMenu } from "./menu"
import { deletePointAt, setCurve } from "./ops"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

/** Rows tall enough to draw a curve in. */
const ROW = 92
/** The curve's area inside a clip on row 0: under the title bar. */
const TOP = 18
const BOTTOM = 88

let backend: Backend
let stop: () => void

beforeEach(async () => {
  ;({ backend, stop } = await startPlaylist())
  vi.mocked(toast.error).mockClear()
})
afterEach(() => stop())

const steps = () => history().entries.length
const clips = () => project().playlist.clips
const clipOf = (id: number) => clips().find((clip) => clip.id === id) as Clip
const automations = () => project().automations
const points = (index = 0) => automations()[index].points
const fader = (): AutomationTarget => ({
  type: "trackVolume",
  track: project().mixer.tracks[1].id,
})

const point = (
  tick: number,
  value: number,
  more: Partial<AutomationPoint> = {}
): AutomationPoint => ({ tick, value, curve: 0, hold: false, ...more })

/** An automation of the Kick track's fader with a clip from the top. */
async function automate(target: AutomationTarget = fader()) {
  const result = await backend.automate(target)
  receivePatch(result.patch)
  await settle()
  const [automation, track, clip] = result.created
  return { automation, track, clip }
}

/** Replaces the curve outside the history under test. */
async function curve(id: number, next: AutomationPoint[]) {
  await setCurve(id, next)
  await settle()
}

function tallSession() {
  const made = startSession()
  made.surface.setViewport({ ...made.surface.viewport, rowHeight: ROW })
  return made
}

/** The y of a value in a clip on a row. */
const yOf = (value: number, row = 0) =>
  row * ROW + BOTTOM - (BOTTOM - TOP) * value

/** A pointer position at a tick and a value of the curve of a clip on `row`. */
const spot = (
  tick: number,
  value: number,
  more: Partial<{
    row: number
    shift: boolean
    alt: boolean
    mod: boolean
    button: number
  }> = {}
) => ({
  x: tick * PX_PER_TICK,
  y: yOf(value, more.row ?? 0),
  button: more.button ?? 0,
  shift: more.shift ?? false,
  mod: more.mod ?? false,
  alt: more.alt ?? false,
})

type Session = ReturnType<typeof tallSession>["session"]

async function press(session: Session, at: ReturnType<typeof spot>) {
  session.pointerDown(at)
  await session.pointerUp(at)
  await settle()
}

async function pull(
  session: Session,
  from: ReturnType<typeof spot>,
  to: ReturnType<typeof spot>
) {
  session.pointerDown(from)
  for (const part of [0.5, 1]) {
    session.pointerMove({
      ...to,
      x: from.x + (to.x - from.x) * part,
      y: from.y + (to.y - from.y) * part,
    })
  }
  await session.pointerUp(to)
  await settle()
}

describe("a point on the edge of its clip", () => {
  /** A clip moved two bars in, so there is grid to the left of its edge. */
  async function edgeClip() {
    const made = await automate()
    await dispatch({
      type: "updateClips",
      updates: [{ id: made.clip, patch: { start: 2 * BAR } }],
    })
    await settle()
    return made
  }
  const pattern = () =>
    clips().filter((clip) => clip.content.type === "pattern")
  /** Two pixels outside the clip, on the outer half of the first point's dot. */
  const outside = (more: Parameters<typeof spot>[2] = {}) => ({
    ...spot(2 * BAR, points()[0].value, more),
    x: 2 * BAR * PX_PER_TICK - 2,
  })

  it("is taken from the half of its dot that hangs over the edge, in the Draw tool", async () => {
    const { clip } = await edgeClip()
    const first = points()[0]
    const { session, clock, stop: end } = tallSession()
    ui().setTool("draw")
    const before = steps()

    // A click there is a click on the point: nothing is placed under the
    // automation clip, and the clip is what gets selected.
    await press(session, outside())
    expect(pattern()).toEqual([])
    expect(clips()).toHaveLength(1)
    expect(steps()).toBe(before)
    expect([...ui().selection]).toEqual([clip])

    // A drag from there, later, moves the point.
    clock.time += 2000
    const from = outside()
    await pull(session, from, { ...from, y: yOf(1) })
    expect(points()).toEqual([{ ...first, value: 1 }])
    expect(labels().at(-1)).toBe("Change automation curve")
    expect(clipOf(clip).start).toBe(2 * BAR)
    end()
  })

  it("says so with the cursor, and gives the grid back past the dot", async () => {
    await edgeClip()
    const { session, stop: end } = tallSession()
    ui().setTool("draw")
    const cursors: string[] = []
    session.onCursor = (cursor) => cursors.push(cursor)
    session.pointerMove(outside())
    expect(cursors.at(-1)).toBe("move")

    // Seven pixels out is empty grid, where the Draw tool places a clip.
    const far = { ...outside(), x: 2 * BAR * PX_PER_TICK - 7 }
    session.pointerMove(far)
    expect(cursors.at(-1)).toBe("crosshair")
    await press(session, far)
    expect(pattern()).toHaveLength(1)
    end()
  })

  it("is deleted by a right-click on that half too, and the clip stays", async () => {
    const { clip, automation } = await edgeClip()
    await curve(automation, [point(0, 0.5), point(BAR, 1)])
    const { session, stop: end } = tallSession()
    ui().setTool("draw")
    await press(session, outside({ button: 2 }))
    expect(points()).toEqual([point(BAR, 1)])
    expect(clipOf(clip)).toBeDefined()
    end()
  })

  it("wins over the body of a clip that lies beside it", async () => {
    const { clip } = await edgeClip()
    const track = clipOf(clip).track
    const added = await dispatch({
      type: "addClips",
      clips: [
        {
          track,
          start: 0,
          length: 2 * BAR,
          content: { type: "pattern", pattern: project().patterns[0].id },
        },
      ],
    })
    await settle()
    const neighbour = added!.created[0]
    const { session, stop: end } = tallSession()
    ui().setTool("draw")
    const from = outside()
    await pull(session, from, { ...from, y: yOf(1) })
    // The point moved, not the pattern clip the pointer was over.
    expect(points()[0].value).toBe(1)
    expect(clipOf(neighbour).start).toBe(0)
    expect([...ui().selection]).toEqual([clip])
    end()
  })
})

describe("adding points", () => {
  it("adds a point where the curve's area is clicked, as one undo step", async () => {
    const { automation, clip } = await automate()
    const start = points()[0]
    const { session, stop: end } = tallSession()
    const before = steps()

    // A little past bar 3: the point lands on the bar line.
    await press(session, spot(2 * BAR + 300, 1))
    expect(points()).toEqual([start, point(2 * BAR, 1)])
    expect(steps()).toBe(before + 1)
    expect(labels().at(-1)).toBe("Change automation curve")
    // The clip the point was put in is the selection.
    expect([...ui().selection]).toEqual([clip])

    await undo()
    expect(points()).toEqual([start])
    await redo()
    expect(points()).toHaveLength(2)
    expect(automations()[0].id).toBe(automation)
    end()
  })

  it("lets go of the snap with Alt", async () => {
    await automate()
    const { session, stop: end } = tallSession()
    await press(session, spot(2 * BAR + 300, 0.25, { alt: true }))
    expect(points()[1]).toMatchObject({ tick: 2 * BAR + 300 })
    expect(points()[1].value).toBeCloseTo(0.25, 5)
    end()
  })

  it("adds the point and drags it on, still as one step", async () => {
    await automate()
    const { session, stop: end } = tallSession()
    const before = steps()
    await pull(session, spot(BAR, 0.2), spot(3 * BAR, 0.9))
    expect(points()[1].tick).toBe(3 * BAR)
    expect(points()[1].value).toBeCloseTo(0.9, 5)
    expect(steps()).toBe(before + 1)
    end()
  })

  it("moves the clip, not the curve, by its title bar", async () => {
    const { clip } = await automate()
    const { session, stop: end } = tallSession()
    const before = points()
    const bar = { ...spot(BAR / 2, 0), y: 7 }
    await pull(session, bar, { ...bar, x: bar.x + BAR * PX_PER_TICK })
    expect(clipOf(clip).start).toBe(BAR)
    expect(points()).toEqual(before)
    expect(labels().at(-1)).toBe("Move clip")
    end()
  })

  it("leaves the curve alone on a row too short to draw it in", async () => {
    const { clip } = await automate()
    const { session, stop: end } = startSession()
    const at = {
      x: BAR * PX_PER_TICK,
      y: 12,
      button: 0,
      shift: false,
      mod: false,
      alt: false,
    }
    await press(session, at)
    expect(points()).toHaveLength(1)
    expect([...ui().selection]).toEqual([clip])
    end()
  })
})

describe("moving points", () => {
  it("drags a point to a tick and a value, as one undo step", async () => {
    const { automation } = await automate()
    await curve(automation, [point(0, 0), point(BAR, 0.5), point(3 * BAR, 1)])
    const { session, stop: end } = tallSession()
    const before = steps()

    session.pointerDown(spot(BAR, 0.5))
    session.pointerMove(spot(2 * BAR - 100, 0.25))
    // While it is dragged the curve is a draft, and the project is not told.
    expect(session.draftPoints?.[1]).toMatchObject({ tick: 2 * BAR })
    expect(points()[1].tick).toBe(BAR)
    // The value in the fader's own unit, and where the point is.
    // A quarter of the way up a fader is an eighth of unity gain.
    expect(session.badgeText).toBe("−18.1 dB  ·  3.1.1")
    await session.pointerUp(spot(2 * BAR - 100, 0.25))
    await settle()

    expect(points()[1].tick).toBe(2 * BAR)
    expect(points()[1].value).toBeCloseTo(0.25, 5)
    expect(steps()).toBe(before + 1)
    expect(labels().at(-1)).toBe("Change automation curve")
    expect(session.draftPoints).toBeNull()
    await undo()
    expect(points()[1]).toEqual(point(BAR, 0.5))
    end()
  })

  it("stops a point at its neighbours, so points never change order", async () => {
    const { automation } = await automate()
    await curve(automation, [point(0, 0), point(BAR, 0.5), point(2 * BAR, 1)])
    const { session, stop: end } = tallSession()
    await pull(session, spot(BAR, 0.5), spot(3 * BAR + 500, 0.5))
    expect(points().map((item) => item.tick)).toEqual([0, 2 * BAR, 2 * BAR])
    await pull(session, spot(2 * BAR, 0.5), spot(-900, 0.5))
    expect(points().map((item) => item.tick)).toEqual([0, 0, 2 * BAR])
    end()
  })

  it("holds the value inside 0 to 1 when the pointer leaves the clip", async () => {
    const { automation } = await automate()
    await curve(automation, [point(0, 0.5), point(BAR, 0.5)])
    const { session, stop: end } = tallSession()
    await pull(session, spot(BAR, 0.5), { ...spot(BAR, 1), y: -60 })
    expect(points()[1].value).toBe(1)
    await pull(session, spot(BAR, 1), { ...spot(BAR, 0), y: 400 })
    expect(points()[1].value).toBe(0)
    end()
  })

  it("keeps a drag to one axis with Shift", async () => {
    const { automation } = await automate()
    await curve(automation, [point(0, 0), point(BAR, 0.5), point(3 * BAR, 1)])
    const { session, stop: end } = tallSession()
    // Mostly sideways: the value stays.
    await pull(session, spot(BAR, 0.5), spot(2 * BAR, 0.6, { shift: true }))
    expect(points()[1]).toEqual(point(2 * BAR, 0.5))
    // Mostly up: the tick stays.
    await pull(
      session,
      spot(2 * BAR, 0.5),
      spot(2 * BAR + 200, 1, { shift: true, alt: true })
    )
    expect(points()[1]).toEqual(point(2 * BAR, 1))
    end()
  })

  it("does nothing on a click that does not move", async () => {
    const { automation } = await automate()
    await curve(automation, [point(0, 0), point(BAR, 0.5)])
    const { session, clock, stop: end } = tallSession()
    const before = steps()
    await press(session, spot(BAR, 0.5))
    expect(steps()).toBe(before)
    // A second click long after is not a double-click either.
    clock.time += 5000
    await press(session, spot(BAR, 0.5))
    expect(steps()).toBe(before)
    expect(points()[1].hold).toBe(false)
    end()
  })
})

describe("bending, holding and deleting", () => {
  it("bends a stretch by its handle so the curve passes under the pointer", async () => {
    const { automation } = await automate()
    await curve(automation, [point(0, 0), point(2 * BAR, 1)])
    const { session, stop: end } = tallSession()
    const before = steps()

    session.pointerDown(spot(BAR, 0.5))
    session.pointerMove(spot(BAR, 0.2))
    expect(session.badgeText).toMatch(/^Bend \d+%$/)
    await session.pointerUp(spot(BAR, 0.2))
    await settle()

    expect(points()[0].curve).toBeGreaterThan(0)
    expect(curveValue(points(), BAR)).toBeCloseTo(0.2, 4)
    expect(points()).toHaveLength(2)
    expect(steps()).toBe(before + 1)
    expect(labels().at(-1)).toBe("Change automation curve")
    end()
  })

  it("makes a point a step with a double-click, and a slope with another", async () => {
    const { automation } = await automate()
    await curve(automation, [point(0, 0), point(BAR, 0.5), point(2 * BAR, 1)])
    const { session, clock, stop: end } = tallSession()
    const before = steps()

    await press(session, spot(BAR, 0.5))
    clock.time += 150
    await press(session, spot(BAR, 0.5))
    expect(points()[1].hold).toBe(true)
    expect(steps()).toBe(before + 1)

    clock.time += 2000
    await press(session, spot(BAR, 0.5))
    clock.time += 150
    await press(session, spot(BAR, 0.5))
    expect(points()[1].hold).toBe(false)
    expect(steps()).toBe(before + 2)
    end()
  })

  it("deletes a point with a right-click in the Draw tool", async () => {
    const { automation, clip } = await automate()
    await curve(automation, [point(0, 0), point(BAR, 0.5), point(2 * BAR, 1)])
    const { session, stop: end } = tallSession()
    const before = steps()
    await press(session, spot(BAR, 0.5, { button: 2 }))
    expect(points()).toEqual([point(0, 0), point(2 * BAR, 1)])
    expect(steps()).toBe(before + 1)
    // A right-click that misses the points does not delete the clip.
    await press(session, spot(BAR, 0.1, { button: 2 }))
    expect(clipOf(clip)).toBeDefined()
    expect(steps()).toBe(before + 1)
    // On the title bar it deletes the clip, as on any other.
    await press(session, { ...spot(BAR, 0, { button: 2 }), y: 7 })
    expect(clips()).toEqual([])
    end()
  })

  it("keeps the last point of a curve, and says so", async () => {
    await automate()
    const { session, stop: end } = tallSession()
    const before = steps()
    await press(session, spot(0, points()[0].value, { button: 2 }))
    expect(points()).toHaveLength(1)
    expect(steps()).toBe(before)
    expect(toast.error).toHaveBeenCalledWith(
      "A curve keeps at least one point",
      expect.objectContaining({ description: expect.any(String) })
    )
    end()
  })

  it("opens a menu on a point in the Select tool", async () => {
    const { automation, clip } = await automate()
    await curve(automation, [
      point(0, 0, { curve: 0.5 }),
      point(BAR, 0.5),
      point(2 * BAR, 1),
    ])
    ui().setTool("select")
    const { session, stop: end } = tallSession()
    const before = steps()
    await press(session, spot(0, 0, { button: 2 }))
    const ref = { clip, automation, index: 0 }
    expect(ui().menuPoint).toEqual(ref)
    expect(steps()).toBe(before)

    const titles = () =>
      pointMenu(ref).flatMap((item) =>
        typeof item === "object" && "title" in item ? [item.title] : []
      )
    expect(titles()).toEqual(["Type value…", "Hold", "Straighten", "Delete"])
    const entry = (title: string) => {
      const found = pointMenu(ref).find(
        (item) =>
          typeof item === "object" && "title" in item && item.title === title
      )
      if (!found || typeof found !== "object" || !("run" in found)) {
        throw new Error(`no entry ${title}`)
      }
      return found
    }

    await entry("Straighten").run()
    await settle()
    expect(points()[0].curve).toBe(0)
    expect(entry("Straighten").disabled).toBe(true)

    await entry("Hold").run()
    await settle()
    expect(points()[0].hold).toBe(true)
    expect(entry("Hold").checked).toBe(true)

    // The value is typed in the fader's unit: 0 dB is 0.7071.
    void entry("Type value…").run()
    await answerText("0 dB")
    expect(points()[0].value).toBeCloseTo(Math.SQRT1_2, 5)
    void entry("Type value…").run()
    await answerText("loud")
    expect(toast.error).toHaveBeenCalledTimes(1)

    await entry("Delete").run()
    await settle()
    expect(points()).toHaveLength(2)
    expect(steps()).toBe(before + 4)

    // A press anywhere else puts the point's menu away.
    await press(session, { ...spot(9 * BAR, 0.5, { button: 2 }), y: 300 })
    expect(ui().menuPoint).toBeNull()
    end()
  })

  it("refuses to delete the only point from the menu too", async () => {
    const { automation, clip } = await automate()
    const remove = pointMenu({ clip, automation, index: 0 }).at(-1)
    expect(remove).toMatchObject({
      title: "Delete",
      disabled: true,
      reason: "The only point",
    })
    await deletePointAt({ clip, automation, index: 0 })
    expect(points()).toHaveLength(1)
  })
})

describe("several clips of one curve", () => {
  /** A second clip of the same automation, a window further into the curve. */
  async function twoClips() {
    const made = await automate()
    await curve(made.automation, [point(0, 0), point(4 * BAR, 1)])
    await dispatch({
      type: "updateClips",
      updates: [{ id: made.clip, patch: { length: 2 * BAR } }],
    })
    await dispatch({ type: "addPlaylistTrack" })
    const added = await dispatch({
      type: "addClips",
      clips: [
        {
          track: tracks()[1].id,
          start: 4 * BAR,
          length: 2 * BAR,
          offset: 2 * BAR,
          content: { type: "automation", automation: made.automation },
        },
      ],
    })
    await settle()
    return { ...made, second: added!.created[0] }
  }

  it("edits the curve all of them show, through the window of the one pressed", async () => {
    const { second } = await twoClips()
    const { session, stop: end } = tallSession()
    const before = steps()
    // In the second clip, a bar in: tick 3 bars of the curve.
    await press(session, spot(5 * BAR, 0.25, { row: 1 }))

    expect(points()).toEqual([
      point(0, 0),
      point(3 * BAR, 0.25),
      point(4 * BAR, 1),
    ])
    expect(steps()).toBe(before + 1)
    expect([...ui().selection]).toEqual([second])
    // There is one automation, so the first clip shows the new curve too.
    expect(automations()).toHaveLength(1)
    end()
  })

  it("cannot take a point that lies outside the window of the clip pressed", async () => {
    const { clip } = await twoClips()
    const { session, stop: end } = tallSession()
    // The point at four bars belongs to the second clip's window. Where it
    // would be on the first clip's row there is only empty grid.
    session.pointerDown(spot(4 * BAR, 1))
    expect(session.draftPoints).toBeNull()
    session.cancel()
    // The first clip's own points are its to edit.
    await pull(session, spot(0, 0), spot(0, 0.5))
    expect(points()[0].value).toBeCloseTo(0.5, 5)
    expect([...ui().selection]).toEqual([clip])
    end()
  })

  it("keeps a dragged point inside the window it is dragged in", async () => {
    await twoClips()
    const { session, stop: end } = tallSession()
    await press(session, spot(5 * BAR, 0.5, { row: 1 }))
    // Dragged far left, it stops at the left edge of the second clip.
    await pull(
      session,
      spot(5 * BAR, 0.5, { row: 1 }),
      spot(0, 0.5, { row: 1 })
    )
    expect(points()[1].tick).toBe(2 * BAR)
    end()
  })
})

describe("the clip machinery on automation clips", () => {
  it("holds the last value when the right edge is pulled past the last point", async () => {
    const { automation, clip } = await automate()
    await curve(automation, [point(0, 0), point(BAR, 1)])
    const { session, stop: end } = tallSession()
    const edge = { ...spot(4 * BAR - 60, 0), y: 50 }
    await pull(session, edge, { ...edge, x: 8 * BAR * PX_PER_TICK })
    expect(clipOf(clip)).toMatchObject({ start: 0, length: 8 * BAR, offset: 0 })
    expect(points()).toEqual([point(0, 0), point(BAR, 1)])
    expect(curveValue(points(), 7 * BAR)).toBe(1)
    end()
  })

  it("trims the left edge with the offset, and stops at the curve's start", async () => {
    const { automation, clip } = await automate()
    await curve(automation, [point(0, 0), point(4 * BAR, 1)])
    await dispatch({
      type: "updateClips",
      updates: [{ id: clip, patch: { start: 4 * BAR } }],
    })
    await settle()
    const { session, stop: end } = tallSession()
    const edge = { ...spot(4 * BAR + 40, 0), y: 50 }
    await pull(session, edge, { ...edge, x: 5 * BAR * PX_PER_TICK })
    expect(clipOf(clip)).toMatchObject({
      start: 5 * BAR,
      offset: BAR,
      length: 3 * BAR,
    })
    // Back out, and no further than tick 0 of the curve.
    const trimmed = { ...spot(5 * BAR + 40, 0), y: 50 }
    await pull(session, trimmed, { ...trimmed, x: 0 })
    expect(clipOf(clip)).toMatchObject({
      start: 4 * BAR,
      offset: 0,
      length: 4 * BAR,
    })
    end()
  })

  it("places more clips of a picked automation with Draw and Paint", async () => {
    const { automation } = await automate()
    await curve(automation, [point(0, 0), point(2 * BAR, 1)])
    ui().setBrush({ type: "automation", automation })
    const { session, stop: end } = tallSession()
    const row = { ...spot(8 * BAR + 50, 0), y: 3 * ROW + 40 }
    session.pointerDown(row)
    // As long as the curve, which is at least a bar.
    expect(session.ghosts).toEqual([
      expect.objectContaining({ row: 3, start: 8 * BAR, length: 2 * BAR }),
    ])
    await session.pointerUp(row)
    await settle()
    expect(clips()).toHaveLength(2)
    expect(clips()[1]).toMatchObject({
      start: 8 * BAR,
      length: 2 * BAR,
      content: { type: "automation", automation },
    })
    expect(labels().at(-1)).toBe("Add clip")

    ui().setTool("paint")
    const from = { ...spot(12 * BAR, 0), y: 3 * ROW + 40 }
    session.pointerDown(from)
    session.pointerMove({ ...from, x: (16 * BAR + 10) * PX_PER_TICK })
    await session.pointerUp({ ...from, x: (16 * BAR + 10) * PX_PER_TICK })
    await settle()
    expect(clips().map((item) => item.start)).toEqual([
      0,
      8 * BAR,
      12 * BAR,
      14 * BAR,
      16 * BAR,
    ])
    expect(labels().at(-1)).toBe("Paint clips")
    end()
  })
})

describe("what an automation lives with", () => {
  it("goes with the effect it moves, clips and all, and comes back on undo", async () => {
    const track = project().mixer.tracks[1].id
    const added = await dispatch({ type: "addEffect", track, kind: "reverb" })
    const effect = added!.created[0]
    const param = await automate({
      type: "effectParam",
      track,
      effect,
      param: 1,
    })
    const mix = await automate({ type: "effectMix", track, effect })
    const other = await automate({ type: "trackPan", track })
    expect(automations()).toHaveLength(3)
    expect(clips()).toHaveLength(3)

    const before = steps()
    await dispatch({ type: "removeEffect", track, effect })
    expect(automations().map((item) => item.id)).toEqual([other.automation])
    expect(clips().map((clip) => clip.id)).toEqual([other.clip])
    expect(steps()).toBe(before + 1)

    await undo()
    expect(automations().map((item) => item.id)).toEqual([
      param.automation,
      mix.automation,
      other.automation,
    ])
    expect(
      clips()
        .map((clip) => clip.id)
        .sort()
    ).toEqual([param.clip, mix.clip, other.clip].sort())
  })

  it("goes when its effect is replaced by another", async () => {
    const track = project().mixer.tracks[1].id
    const added = await dispatch({ type: "addEffect", track, kind: "delay" })
    const effect = added!.created[0]
    await automate({ type: "effectMix", track, effect })
    await dispatch({ type: "replaceEffect", track, effect, kind: "reverb" })
    expect(automations()).toEqual([])
    expect(clips()).toEqual([])
    await undo()
    expect(automations()).toHaveLength(1)
    expect(clips()).toHaveLength(1)
  })

  it("follows an effect that moves to another track", async () => {
    const [, first, second] = project().mixer.tracks
    const added = await dispatch({
      type: "addEffect",
      track: first.id,
      kind: "reverb",
    })
    const effect = added!.created[0]
    await automate({ type: "effectMix", track: first.id, effect })
    await dispatch({
      type: "moveEffect",
      track: first.id,
      effect,
      toTrack: second.id,
      index: 0,
    })
    expect(automations()[0].target).toEqual({
      type: "effectMix",
      track: second.id,
      effect,
    })
    expect(clips()).toHaveLength(1)
  })

  it("goes with its channel, its track and its send", async () => {
    const kick = project().channels[0]
    const [, track, bus] = project().mixer.tracks
    await dispatch({ type: "setSend", from: track.id, to: bus.id, gain: 0.5 })
    await automate({ type: "channelPan", channel: kick.id })
    await automate({ type: "sendGain", track: track.id, target: bus.id })
    await automate({ type: "trackVolume", track: bus.id })
    expect(automations()).toHaveLength(3)

    await dispatch({ type: "setSend", from: track.id, to: bus.id })
    expect(automations().map((item) => item.target.type)).toEqual([
      "channelPan",
      "trackVolume",
    ])
    await dispatch({ type: "removeChannel", id: kick.id })
    await dispatch({ type: "removeMixerTrack", id: bus.id })
    expect(automations()).toEqual([])
    expect(clips()).toEqual([])
    await undo()
    await undo()
    await undo()
    expect(automations()).toHaveLength(3)
    expect(clips()).toHaveLength(3)
  })
})

describe("where the clip of a new automation goes", () => {
  it("covers the song from the top, at least four bars", async () => {
    const { clip, track } = await automate()
    expect(clipOf(clip)).toMatchObject({
      start: 0,
      length: 4 * BAR,
      track,
      content: { type: "automation" },
    })
    expect(tracks().at(-1)?.id).toBe(track)
    expect(labels()).toEqual(["Create automation clip"])
    // One point, at the value the fader has, so nothing changes yet.
    expect(points()).toHaveLength(1)
    expect(points()[0]).toMatchObject({ tick: 0, curve: 0, hold: false })
    expect(points()[0].value).toBeCloseTo(Math.SQRT1_2, 6)
  })
})
