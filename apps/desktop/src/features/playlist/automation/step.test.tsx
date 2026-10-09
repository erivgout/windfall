import { act, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { AutomationPoint } from "@/bindings"
import { shortcutLabel } from "@/lib/actions"
import type { Backend } from "@/lib/ipc"
import { dispatch, receivePatch } from "@/lib/store/project"
import { MAX_AUTOMATION_POINTS } from "@/lib/units"
import { settle } from "@/test/harness"

import { setActiveSession } from "../active"
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
import { PlaylistToolbar } from "../toolbar"
import { setCurve } from "./ops"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

const point = (tick: number, value: number): AutomationPoint => ({
  tick,
  value,
  curve: 0,
  hold: false,
})
const original = [point(0, 0.1), point(4 * BAR, 0.8)]
const spot = (tick: number, value: number): PointerInput => ({
  x: tick * PX_PER_TICK,
  y: 88 - 70 * value,
  button: 0,
  shift: false,
  alt: false,
  mod: false,
})

let backend: Backend
let stop: () => void
let made: ReturnType<typeof startSession>
let automation: number
let clip: number
const points = () =>
  project().automations.find((item) => item.id === automation)!.points

beforeEach(async () => {
  const app = await startPlaylist()
  backend = app.backend
  const result = await backend.automate({
    type: "trackVolume",
    track: project().mixer.tracks[1].id,
  })
  receivePatch(result.patch)
  await settle()
  ;[automation, , clip] = result.created
  await setCurve(automation, original)
  await dispatch({
    type: "updateClips",
    updates: [{ id: clip, patch: { length: 6 * BAR } }],
  })
  made = startSession()
  made.surface.setViewport({ ...made.surface.viewport, rowHeight: 92 })
  const deactivate = setActiveSession(made.session)
  stop = () => {
    deactivate()
    made.stop()
    app.stop()
  }
  ui().setSnap("beat")
  ui().toggleStep()
})

afterEach(() => {
  stop()
  vi.restoreAllMocks()
})

describe("playlist automation Step mode", () => {
  it("writes every crossed snap tick with held values in one release command", async () => {
    const send = vi.spyOn(backend, "dispatch")
    expect(made.session.pointerDown(spot(BEAT + 100, 0.2))).toBe(true)
    const to = spot(4 * BEAT + 100, 0.5)
    made.session.pointerMove(to)
    const draft = made.session.draftPoints!
    expect(draft.map((item) => item.tick)).toEqual([
      0,
      BEAT,
      2 * BEAT,
      3 * BEAT,
      4 * BEAT,
      4 * BAR,
    ])
    draft.slice(1, 5).forEach((item, index) => {
      expect(item.value).toBeCloseTo(0.2 + index * 0.1)
      expect(item.hold).toBe(true)
    })
    expect(draft[0]).toEqual(original[0])
    expect(draft.at(-1)).toEqual(original[1])
    expect(points()).toEqual(original)
    expect(send).not.toHaveBeenCalled()
    await made.session.pointerUp(to)
    expect(send).toHaveBeenCalledTimes(1)
    expect(send.mock.calls[0][0]).toEqual({
      type: "setAutomationPoints",
      id: automation,
      points: draft,
    })
    expect(points()).toEqual(draft)
    expect(made.session.draftPoints).toBeNull()
  })

  it("updates an existing tick instead of duplicating it and clears the whole curve's final hold", async () => {
    await setCurve(automation, [
      original[0],
      { ...point(2 * BEAT, 0.9), curve: 0.4 },
      { ...original[1], hold: true },
    ])
    made.session.pointerDown(spot(BEAT, 0.2))
    await made.session.pointerUp(spot(3 * BEAT, 0.6))
    const atTick = points().filter((item) => item.tick === 2 * BEAT)
    expect(atTick).toHaveLength(1)
    expect(atTick[0]).toMatchObject({ hold: true, curve: 0 })
    expect(atTick[0].value).toBeCloseTo(0.4)
    expect(points().at(-1)?.hold).toBe(false)
  })

  it("leaves a new final point of the whole curve unheld", async () => {
    made.session.pointerDown(spot(4 * BAR + BEAT, 0.2))
    await made.session.pointerUp(spot(4 * BAR + 3 * BEAT, 0.6))
    expect(
      points()
        .slice(-3)
        .map((item) => item.hold)
    ).toEqual([true, true, false])
  })

  it("fills backwards strokes and rewrites crossed ticks on a return pass", async () => {
    made.session.pointerDown(spot(4 * BEAT, 0.5))
    made.session.pointerMove(spot(BEAT, 0.2))
    await made.session.pointerUp(spot(3 * BEAT, 0.8))
    const written = points().filter((item) => item.hold)
    expect(written.map((item) => item.tick)).toEqual([
      BEAT,
      2 * BEAT,
      3 * BEAT,
      4 * BEAT,
    ])
    expect(written[1].value).toBeCloseTo(0.5)
    expect(written[2].value).toBeCloseTo(0.8)
  })

  it("Step off still adds one ordinary point", async () => {
    ui().toggleStep()
    const send = vi.spyOn(backend, "dispatch")
    await click(made.session, spot(BEAT + 100, 0.4))
    expect(points()).toHaveLength(3)
    expect(points()[1].tick).toBe(BEAT)
    expect(points()[1].value).toBeCloseTo(0.4)
    expect(points().every((item) => !item.hold)).toBe(true)
    expect(send).toHaveBeenCalledTimes(1)
  })

  it.each(["cancel", "Escape"])(
    "restores the curve and sends nothing on %s",
    async (cancel) => {
      const send = vi.spyOn(backend, "dispatch")
      const to = spot(4 * BEAT, 0.6)
      made.session.pointerDown(spot(BEAT, 0.2))
      made.session.pointerMove(to)
      expect(made.session.draftPoints).not.toEqual(original)
      if (cancel === "cancel") made.session.cancel()
      else {
        fireEvent.keyDown(document.body, { key: "Escape", code: "Escape" })
        await settle()
      }
      expect(made.session.draftPoints).toBeNull()
      expect(made.session.busy).toBe(false)
      await made.session.pointerUp(to)
      expect(points()).toEqual(original)
      expect(send).not.toHaveBeenCalled()
    }
  )

  it("refuses a full curve without a preview or command", async () => {
    const full = Array.from({ length: MAX_AUTOMATION_POINTS }, (_, index) =>
      point(index, 0.1)
    )
    await setCurve(automation, full)
    const send = vi.spyOn(backend, "dispatch")
    expect(made.session.pointerDown(spot(2 * BAR, 0.5))).toBe(false)
    await made.session.pointerUp(spot(3 * BAR, 0.6))
    expect(made.session.draftPoints).toBeNull()
    expect(points()).toEqual(full)
    expect(send).not.toHaveBeenCalled()
  })

  it("cancels the whole stroke if it runs out of capacity while drawing", async () => {
    const almostFull = Array.from(
      { length: MAX_AUTOMATION_POINTS - 1 },
      (_, index) => point(index, 0.1)
    )
    await setCurve(automation, almostFull)
    const send = vi.spyOn(backend, "dispatch")
    expect(made.session.pointerDown(spot(2 * BAR, 0.5))).toBe(true)
    expect(made.session.draftPoints).toHaveLength(MAX_AUTOMATION_POINTS)
    made.session.pointerMove(spot(3 * BAR, 0.6))
    expect(made.session.busy).toBe(false)
    expect(made.session.draftPoints).toBeNull()
    await made.session.pointerUp(spot(3 * BAR, 0.6))
    expect(points()).toEqual(almostFull)
    expect(send).not.toHaveBeenCalled()
  })

  it("writes song-grid ticks through a clip's offset and clamps to its window", async () => {
    await dispatch({
      type: "updateClips",
      updates: [
        { id: clip, patch: { start: BAR, offset: 100, length: 2 * BEAT } },
      ],
    })
    made.session.pointerDown(spot(BAR + BEAT, 0.2))
    await made.session.pointerUp(spot(BAR + 4 * BEAT, 0.6))
    expect(points().map((item) => item.tick)).toEqual([
      0,
      100 + BEAT,
      100 + 2 * BEAT,
      4 * BAR,
    ])
    expect(points()[1].value).toBeCloseTo(0.2)
    expect(points()[2].value).toBeCloseTo(0.6)
    expect(points().at(-1)).toEqual(original[1])
  })

  it("keeps an existing selected pair moving together with Step on", async () => {
    await setCurve(automation, [
      original[0],
      point(BAR, 0.3),
      point(2 * BAR, 0.5),
      original[1],
    ])
    await click(made.session, spot(BAR, 0.3))
    await click(made.session, { ...spot(2 * BAR, 0.5), shift: true })
    const send = vi.spyOn(backend, "dispatch")
    made.session.pointerDown(spot(BAR, 0.3))
    made.session.pointerMove(spot(BAR + BEAT, 0.4))
    await made.session.pointerUp(spot(BAR + BEAT, 0.4))
    expect(points().map((item) => item.tick)).toEqual([
      0,
      BAR + BEAT,
      2 * BAR + BEAT,
      4 * BAR,
    ])
    expect(points()[2].value).toBeCloseTo(0.6)
    expect(points().every((item) => !item.hold)).toBe(true)
    expect(send).toHaveBeenCalledTimes(1)
  })

  it("shows a Step toolbar toggle, binds H, and resets when another project opens", async () => {
    render(<PlaylistToolbar metrics={made.metrics} />)
    const button = screen.getByRole("button", { name: "Step" })
    expect(button).toHaveAttribute("aria-pressed", "true")
    fireEvent.click(button)
    expect(button).toHaveAttribute("aria-pressed", "false")
    fireEvent.keyDown(document.body, { key: "h", code: "KeyH" })
    expect(shortcutLabel("playlist.step")).toBe("H")
    expect(button).toHaveAttribute("aria-pressed", "true")
    const send = vi.spyOn(backend, "dispatch")
    made.session.pointerDown(spot(BEAT, 0.2))
    made.session.pointerMove(spot(3 * BEAT, 0.6))
    await act(async () => {
      await backend.projectNew()
      await settle()
    })
    expect(ui().step).toBe(false)
    expect(button).toHaveAttribute("aria-pressed", "false")
    expect(made.session.draftPoints).toBeNull()
    await made.session.pointerUp(spot(3 * BEAT, 0.6))
    expect(send).not.toHaveBeenCalled()
  })
})
