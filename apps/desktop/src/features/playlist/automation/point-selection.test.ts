import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { AutomationPoint } from "@/bindings"
import { deviceTransform, rgbaToCss } from "@/lib/canvas"
import type { Backend } from "@/lib/ipc"
import { dispatch, receivePatch } from "@/lib/store/project"
import { settle } from "@/test/harness"

import type { PointerInput } from "../session"
import {
  BAR,
  BEAT,
  click,
  project,
  PX_PER_TICK,
  startPlaylist,
  startSession,
  ui,
} from "../test-utils"
import { togglePointHold, setCurve } from "./ops"
import { paintAutomation } from "./paint"

const point = (tick: number, value: number): AutomationPoint => ({
  tick,
  value,
  curve: 0,
  hold: false,
})
const original = [
  point(0, 0.1),
  point(BAR, 0.3),
  point(2 * BAR, 0.5),
  point(4 * BAR, 0.8),
]
const spot = (
  tick: number,
  value: number,
  more: Partial<PointerInput> = {},
  row = 0
): PointerInput => ({
  x: tick * PX_PER_TICK,
  y: row * 92 + 88 - 70 * value,
  button: 0,
  shift: false,
  alt: false,
  mod: false,
  ...more,
})

let backend: Backend
let stop: () => void
let made: ReturnType<typeof startSession>
let automation: number
let clip: number
const points = () =>
  project().automations.find((item) => item.id === automation)!.points
const selected = () =>
  [...(made.session.pointSelection?.indices ?? [])].sort((a, b) => a - b)

async function automate() {
  const result = await backend.automate({
    type: "trackVolume",
    track: project().mixer.tracks[1].id,
  })
  receivePatch(result.patch)
  await settle()
  const [automation, track, clip] = result.created
  await setCurve(automation, original)
  await dispatch({
    type: "updateClips",
    updates: [{ id: clip, patch: { length: 6 * BAR } }],
  })
  return { automation, track, clip }
}

beforeEach(async () => {
  ;({ backend, stop } = await startPlaylist())
  ;({ automation, clip } = await automate())
  made = startSession()
  made.surface.setViewport({ ...made.surface.viewport, rowHeight: 92 })
  ui().setSnap("beat")
  ui().setTool("select")
})

afterEach(() => {
  made.stop()
  stop()
  vi.restoreAllMocks()
})

async function selectPair() {
  await click(made.session, spot(BAR, 0.3))
  await click(made.session, spot(2 * BAR, 0.5, { shift: true }))
}

describe("playlist automation point selection", () => {
  it("selects two points with Shift, toggles either off, and collapses a plain click", async () => {
    const send = vi.spyOn(backend, "dispatch")
    await selectPair()
    expect(made.session.pointSelection).toMatchObject({ clip, automation })
    expect(selected()).toEqual([1, 2])
    await click(made.session, spot(BAR, 0.3, { shift: true }))
    expect(selected()).toEqual([2])
    await click(made.session, spot(BAR, 0.3, { shift: true }))
    expect(selected()).toEqual([1, 2])
    await click(made.session, spot(BAR, 0.3))
    expect(selected()).toEqual([1])
    expect(points()).toEqual(original)
    expect(send).not.toHaveBeenCalled()
  })

  it("previews a snapped shared delta and releases both points in one command", async () => {
    await selectPair()
    const send = vi.spyOn(backend, "dispatch")
    const from = spot(BAR, 0.3)
    const to = spot(BAR + BEAT - 100, 0.4)
    expect(made.session.pointerDown(from)).toBe(true)
    made.session.pointerMove(to)
    expect(made.session.draftPoints?.map((item) => item.tick)).toEqual([
      0,
      BAR + BEAT,
      2 * BAR + BEAT,
      4 * BAR,
    ])
    expect(points()).toEqual(original)
    expect(send).not.toHaveBeenCalled()
    const draft = made.session.draftPoints
    await made.session.pointerUp(to)
    expect(points()[1].value).toBeCloseTo(0.4)
    expect(points()[2].value).toBeCloseTo(0.6)
    expect(points()[0]).toEqual(original[0])
    expect(points()[3]).toEqual(original[3])
    expect(send).toHaveBeenCalledTimes(1)
    expect(send.mock.calls[0][0]).toEqual({
      type: "setAutomationPoints",
      id: automation,
      points: draft,
    })
    expect(selected()).toEqual([1, 2])
    expect(made.session.draftPoints).toBeNull()
  })

  it("clamps a contiguous group one tick before an unselected point", async () => {
    await selectPair()
    const send = vi.spyOn(backend, "dispatch")
    const to = spot(5 * BAR, 0.3)
    made.session.pointerDown(spot(BAR, 0.3))
    made.session.pointerMove(to)
    await made.session.pointerUp(to)
    expect(points().map((item) => item.tick)).toEqual([
      0,
      3 * BAR - 1,
      4 * BAR - 1,
      4 * BAR,
    ])
    expect(send).toHaveBeenCalledTimes(1)
    expect(
      points().every(
        (item, index, all) => index === 0 || item.tick > all[index - 1].tick
      )
    ).toBe(true)
  })

  it("clamps nonadjacent selected points against the point between them", async () => {
    await click(made.session, spot(BAR, 0.3))
    await click(made.session, spot(4 * BAR, 0.8, { shift: true }))
    const to = spot(3 * BAR, 0.3)
    made.session.pointerDown(spot(BAR, 0.3))
    made.session.pointerMove(to)
    await made.session.pointerUp(to)
    expect(points().map((item) => item.tick)).toEqual([
      0,
      2 * BAR - 1,
      2 * BAR,
      5 * BAR - 1,
    ])
    expect(points()[2]).toEqual(original[2])
  })

  it("keeps a shared value delta when one selected point reaches the value limit", async () => {
    await selectPair()
    const to = spot(BAR, 1)
    made.session.pointerDown(spot(BAR, 0.3))
    made.session.pointerMove(to)
    await made.session.pointerUp(to)
    expect(points()[1].value).toBeCloseTo(0.8)
    expect(points()[2].value).toBe(1)
    expect(points().map((item) => item.tick)).toEqual(
      original.map((item) => item.tick)
    )
  })

  it("leaves the tick of an existing jump alone during a vertical drag", async () => {
    await setCurve(automation, [
      point(0, 0.1),
      point(BAR, 0.3),
      point(BAR, 0.5),
      point(4 * BAR, 0.8),
    ])
    made.session.pointerDown(spot(BAR, 0.3))
    made.session.pointerMove(spot(BAR, 0.4))
    await made.session.pointerUp(spot(BAR, 0.4))
    expect(points().map((item) => item.tick)).toEqual([0, BAR, BAR, 4 * BAR])
    expect(points()[1].value).toBeCloseTo(0.4)
    expect(points()[2].value).toBe(0.5)
  })

  it("dispatches nothing when order clamps the entire move to zero", async () => {
    await setCurve(automation, [
      point(0, 0.1),
      point(BAR, 0.3),
      point(BAR + 1, 0.5),
      point(4 * BAR, 0.8),
    ])
    await click(made.session, spot(BAR, 0.3))
    await click(made.session, spot(0, 0.1, { shift: true }))
    const before = points()
    const send = vi.spyOn(backend, "dispatch")
    const to = spot(2 * BAR, 0.3)
    made.session.pointerDown(spot(BAR, 0.3))
    made.session.pointerMove(to)
    await made.session.pointerUp(to)
    expect(points()).toEqual(before)
    expect(send).not.toHaveBeenCalled()
  })

  it("restores the curve on cancel and ignores the later release", async () => {
    await selectPair()
    const send = vi.spyOn(backend, "dispatch")
    const to = spot(BAR + BEAT, 0.4)
    made.session.pointerDown(spot(BAR, 0.3))
    made.session.pointerMove(to)
    expect(made.session.draftPoints).not.toEqual(original)
    made.session.cancel()
    expect(made.session.draftPoints).toBeNull()
    expect(made.session.busy).toBe(false)
    await made.session.pointerUp(to)
    expect(points()).toEqual(original)
    expect(selected()).toEqual([1, 2])
    expect(send).not.toHaveBeenCalled()
  })

  it("replaces the selection on another automation, even with Shift", async () => {
    await selectPair()
    const other = await automate()
    const row = project().playlist.tracks.findIndex(
      (track) => track.id === other.track
    )
    await click(made.session, spot(BAR, 0.3, { shift: true }, row))
    expect(made.session.pointSelection).toMatchObject({
      clip: other.clip,
      automation: other.automation,
    })
    expect(selected()).toEqual([1])
  })

  it("limits point-menu actions to the pressed point", async () => {
    await selectPair()
    made.session.pointerDown(spot(BAR, 0.3, { button: 2 }))
    expect(ui().menuPoint).toEqual({ clip, automation, index: 1 })
    await togglePointHold(ui().menuPoint!)
    expect(points().map((item) => item.hold)).toEqual([
      false,
      true,
      false,
      false,
    ])
    expect(selected()).toEqual([1, 2])
  })

  it("keeps selection in one clip even when another clip shares the curve", async () => {
    await dispatch({
      type: "addClips",
      clips: [
        {
          track: project().playlist.clips[0].track,
          start: 8 * BAR,
          length: 6 * BAR,
          content: { type: "automation", automation },
        },
      ],
    })
    const other = project().playlist.clips.find(
      (item) => item.start === 8 * BAR
    )!
    await selectPair()
    await click(made.session, spot(9 * BAR, 0.3, { shift: true }))
    expect(made.session.pointSelection).toMatchObject({
      clip: other.id,
      automation,
    })
    expect(selected()).toEqual([1])
    expect(points()).toEqual(original)
  })

  it("clears selection and any draft when another project opens", async () => {
    await selectPair()
    made.session.pointerDown(spot(BAR, 0.3))
    made.session.pointerMove(spot(BAR + BEAT, 0.4))
    await backend.projectNew()
    await settle()
    expect(made.session.pointSelection).toBeNull()
    expect(made.session.draftPoints).toBeNull()
    expect(made.session.busy).toBe(false)
    const send = vi.spyOn(backend, "dispatch")
    await made.session.pointerUp(spot(BAR + BEAT, 0.4))
    expect(send).not.toHaveBeenCalled()
  })

  it("fills selected point markers with curve ink and leaves others hollow", () => {
    const fills: string[] = []
    const ctx = {
      beginPath: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      closePath: vi.fn(),
      arc: vi.fn(),
      stroke: vi.fn(),
      fillStyle: "",
      fill(this: { fillStyle: string }) {
        fills.push(this.fillStyle)
      },
    } as unknown as CanvasRenderingContext2D
    const viewport = made.surface.viewport
    const color = "#abcdef"
    paintAutomation(
      ctx,
      {
        viewport,
        transform: deviceTransform(viewport),
        theme: made.surface.theme,
      },
      {
        id: clip,
        style: 0,
        content: { type: "automation", automation },
        name: "Curve",
        pattern: undefined,
        span: { start: 0, length: 6 * BAR, offset: 0 },
        x0: 0,
        x1: 6 * BAR * PX_PER_TICK,
        y0: 0,
        y1: 92,
        ghost: false,
        selected: false,
        muted: false,
      },
      {
        body: color,
        border: color,
        band: color,
        bandInk: color,
        bodyInk: color,
        note: color,
      },
      {
        points: original,
        selected: new Set([1, 2]),
        area: { left: 0, right: 6 * BAR * PX_PER_TICK, top: 18, bottom: 88 },
        active: false,
        budget: { marks: 100 },
      }
    )
    const hollow = rgbaToCss(made.surface.theme.background)
    expect(fills.slice(1)).toEqual([hollow, color, color, hollow])
  })
})
