import { toast } from "sonner"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { AutomationPoint, AutomationTarget } from "@/bindings"
import type { ContextItem, InlineAction } from "@/components/context-actions"
import {
  rangeNormalized,
  rangeValue,
  TEMPO_RANGE,
} from "@/lib/automation/curve"
import { FULL_VIEW, type ViewRange } from "@/lib/automation/view-range"
import type { Backend } from "@/lib/ipc"
import { dispatch, receivePatch, undo } from "@/lib/store/project"
import { settle } from "@/test/harness"

import {
  answerText,
  BAR,
  project,
  PX_PER_TICK,
  startPlaylist,
  startSession,
  ui,
} from "../test-utils"
import { automationMenu } from "./menu"
import { setCurve, typePointValue } from "./ops"
import {
  currentView,
  fitViewToCurve,
  parseViewRange,
  selectedViewRangeItems,
  showFullRange,
  typeViewRange,
  viewRangeItems,
} from "./view-menu"
import { chosenView, useCurveViews } from "./view-store"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

/** Rows tall enough to draw a curve in: its area runs from y 18 to y 88. */
const ROW = 92
const TOP = 18
const BOTTOM = 88
const HEIGHT = BOTTOM - TOP

let backend: Backend
let stop: () => void

beforeEach(async () => {
  ;({ backend, stop } = await startPlaylist())
  useCurveViews.setState({ views: {} })
  vi.mocked(toast.error).mockClear()
})
afterEach(() => stop())

const bpm = (value: number) => rangeNormalized(TEMPO_RANGE, value)
const toBpm = (value: number) => rangeValue(TEMPO_RANGE, value)
const inBpm = (view: ViewRange) => [
  Math.round(toBpm(view.lo) * 100) / 100,
  Math.round(toBpm(view.hi) * 100) / 100,
]
const automations = () => project().automations
const points = (index = 0) => automations()[index].points
const point = (tick: number, value: number): AutomationPoint => ({
  tick,
  value,
  curve: 0,
  hold: false,
})

async function automate(target: AutomationTarget) {
  const result = await backend.automate(target)
  receivePatch(result.patch)
  await settle()
  const [automation, track, clip] = result.created
  return { automation, track, clip }
}

/** A tempo curve at the project's 128 bpm becomes one at 120. */
async function tempoCurve() {
  await dispatch({ type: "updateSettings", patch: { tempoBpm: 120 } })
  const made = await automate({ type: "tempo" })
  await setCurve(made.automation, [point(0, bpm(120)), point(BAR, bpm(120))])
  await settle()
  return made
}

function tallSession() {
  const made = startSession()
  made.surface.setViewport({ ...made.surface.viewport, rowHeight: ROW })
  return made
}

/** The y a share of the way up the curve's area, 0 at the bottom. */
const yAt = (share: number) => BOTTOM - HEIGHT * share
const at = (tick: number, y: number) => ({
  x: tick * PX_PER_TICK,
  y,
  button: 0,
  shift: false,
  mod: false,
  alt: true,
})

const entries = (items: ContextItem[]) => {
  const [range] = items.filter(
    (item): item is { submenu: string; items: ContextItem[] } =>
      typeof item === "object" &&
      "submenu" in item &&
      item.submenu === "Range shown"
  )
  return (range?.items ?? []) as InlineAction[]
}
const entry = (items: ContextItem[], title: string) => {
  const found = entries(items).find((item) => item.title === title)
  if (!found) throw new Error(`no entry "${title}"`)
  return found
}

describe("drawing a tempo curve by hand", () => {
  it("shows 100 to 140 bpm in the clip, so 120 is in the middle", async () => {
    const { automation } = await tempoCurve()
    const { session, stop: end } = tallSession()
    const scene = session["scene"]
    expect(inBpm(scene.viewOf(automation))).toEqual([100, 140])
    expect(scene.viewLabelsOf(automation)).toEqual({
      top: "140.00 bpm",
      bottom: "100.00 bpm",
    })
    // The point at 120 bpm sits half way up, where it can be taken.
    session.pointerDown(at(BAR, yAt(0.5)))
    expect(session.busy).toBe(true)
    session.cancel()
    end()
  })

  it("moves a point by bpm a hand can aim at: a quarter of the clip is 10 bpm", async () => {
    await tempoCurve()
    const { session, stop: end } = tallSession()
    // 17.5 pixels up. Across the whole range that was 128 bpm.
    session.pointerDown(at(BAR, yAt(0.5)))
    session.pointerMove(at(BAR, yAt(0.6)))
    session.pointerMove(at(BAR, yAt(0.75)))
    expect(session.badgeText).toMatch(/^130\.00 bpm/)
    await session.pointerUp(at(BAR, yAt(0.75)))
    await settle()
    expect(toBpm(points()[1].value)).toBeCloseTo(130, 4)
    end()
  })

  it("carries on past the top of the view, which grows to show the point", async () => {
    const { automation } = await tempoCurve()
    const { session, stop: end } = tallSession()
    const scene = session["scene"]
    session.pointerDown(at(BAR, yAt(0.5)))
    session.pointerMove(at(BAR, yAt(0.8)))
    // A quarter of the clip's height above its top edge: 10 bpm past 140.
    session.pointerMove(at(BAR, yAt(1.25)))
    expect(session.badgeText).toMatch(/^150\.00 bpm/)
    // While the button is down the view is the one of the press, grown.
    expect(inBpm(scene.viewOf(automation))).toEqual([100, 150])
    await session.pointerUp(at(BAR, yAt(1.25)))
    await settle()
    expect(toBpm(points()[1].value)).toBeCloseTo(150, 4)
    // Let go, the view is about the tempo and the points again.
    expect(inBpm(scene.viewOf(automation))).toEqual([100, 170])
    expect(chosenView(automation)).toBeUndefined()
    end()
  })

  it("does not let the view slide under the pointer while a point is dragged down", async () => {
    const { automation } = await tempoCurve()
    await setCurve(automation, [point(0, bpm(120)), point(BAR, bpm(160))])
    await settle()
    const { session, stop: end } = tallSession()
    const scene = session["scene"]
    // 100 to 180: the point at 160 is three quarters of the way up.
    expect(inBpm(scene.viewOf(automation))).toEqual([100, 180])
    session.pointerDown(at(BAR, yAt(0.75)))
    session.pointerMove(at(BAR, yAt(0.6)))
    session.pointerMove(at(BAR, yAt(0.5)))
    // Half way up the view of the press is 140 bpm, whatever the default
    // view of the curve as it is now would be.
    expect(session.badgeText).toMatch(/^140\.00 bpm/)
    expect(inBpm(scene.viewOf(automation))).toEqual([100, 180])
    await session.pointerUp(at(BAR, yAt(0.5)))
    await settle()
    expect(toBpm(points()[1].value)).toBeCloseTo(140, 4)
    end()
  })

  it("adds a point at the value under the pointer, in the view", async () => {
    await tempoCurve()
    const { session, stop: end } = tallSession()
    const spot = at(2 * BAR, yAt(0.25))
    session.pointerDown(spot)
    await session.pointerUp(spot)
    await settle()
    expect(toBpm(points()[2].value)).toBeCloseTo(110, 4)
    end()
  })

  it("stores nothing but values from 0 to 1, and undo leaves the view alone", async () => {
    const { automation } = await tempoCurve()
    fitViewToCurve(automation)
    const view = chosenView(automation)
    const before = points()
    expect(before.every((item) => item.value >= 0 && item.value <= 1)).toBe(
      true
    )
    await undo()
    await settle()
    expect(chosenView(automation)).toBe(view)
  })
})

describe("choosing the range a clip shows", () => {
  it("offers it for the selected automation clips, and in the automation's own menu", async () => {
    const { clip, automation } = await tempoCurve()
    ui().select([clip])
    expect(entries(selectedViewRangeItems()).map((item) => item.title)).toEqual(
      ["Fit to curve", "Full range", "Set range…", "Around the tempo"]
    )
    expect(
      entries(automationMenu(automations()[0])).map((item) => item.title)
    ).toEqual(["Fit to curve", "Full range", "Set range…", "Around the tempo"])
    expect(entry(viewRangeItems([automation]), "Around the tempo").checked).toBe(
      true
    )

    // Another target zooms, and has no tempo to follow.
    const fader = await automate({
      type: "trackVolume",
      track: project().mixer.tracks[1].id,
    })
    expect(
      entries(viewRangeItems([fader.automation])).map((item) => item.title)
    ).toEqual(["Zoom to curve", "Full range", "Set range…"])
    expect(entry(viewRangeItems([fader.automation]), "Full range").checked).toBe(
      true
    )
    // Nothing for clips that are not automation.
    ui().clearSelection()
    expect(selectedViewRangeItems()).toEqual([])
  })

  it("fits the curve, shows the whole range, and goes back to the tempo's window", async () => {
    const { automation } = await tempoCurve()
    await setCurve(automation, [point(0, bpm(120)), point(BAR, bpm(200))])
    await settle()
    const view = () => currentView(automations()[0])
    const items = () => viewRangeItems([automation])

    await entry(items(), "Fit to curve").run()
    const [low, high] = inBpm(view())
    expect(low).toBeCloseTo(110.4, 1)
    expect(high).toBeCloseTo(209.6, 1)
    expect(entry(items(), "Around the tempo").checked).toBe(false)

    await entry(items(), "Full range").run()
    expect(view()).toEqual(FULL_VIEW)
    expect(entry(items(), "Full range").checked).toBe(true)

    await entry(items(), "Around the tempo").run()
    expect(inBpm(view())).toEqual([100, 220])
    expect(chosenView(automation)).toBeUndefined()
  })

  it("takes a typed lowest and highest value, in the target's own unit", async () => {
    const { automation } = await tempoCurve()
    const asked = typeViewRange(automation)
    await answerText("90 to 150")
    await asked
    expect(inBpm(currentView(automations()[0]))).toEqual([90, 150])

    // What cannot be read changes nothing, and says so.
    const again = typeViewRange(automation)
    await answerText("fast")
    await again
    expect(inBpm(currentView(automations()[0]))).toEqual([90, 150])
    expect(toast.error).toHaveBeenCalled()
  })

  it("reads a range typed in several ways", async () => {
    const { automation } = await tempoCurve()
    const tempo = automations().find((item) => item.id === automation)!
    const fader = await automate({
      type: "trackVolume",
      track: project().mixer.tracks[1].id,
    })
    const gain = automations().find((item) => item.id === fader.automation)!
    const read = (text: string) => {
      const view = parseViewRange(tempo, text)
      return view ? inBpm(view) : null
    }
    expect(read("100 to 140")).toEqual([100, 140])
    expect(read("140 bpm to 100 bpm")).toEqual([100, 140])
    expect(read("100-140")).toEqual([100, 140])
    expect(read("100 .. 140")).toEqual([100, 140])
    expect(read("100; 140")).toEqual([100, 140])
    expect(read("100")).toBeNull()
    expect(read("100 to 100")).toBeNull()
    expect(read("")).toBeNull()
    // A minus is a sign, not a dash, where it starts a value.
    const db = parseViewRange(gain, "-12 dB to 0 dB")!
    expect(db.lo).toBeCloseTo(Math.sqrt(10 ** (-12 / 20) / 2), 6)
    expect(db.hi).toBeCloseTo(Math.SQRT1_2, 6)
  })

  it("grows a chosen view when a point is dragged or typed out of it", async () => {
    const { automation } = await tempoCurve()
    const asked = typeViewRange(automation)
    await answerText("110 to 130")
    await asked
    const view = () => inBpm(currentView(automations()[0]))
    const { session, stop: end } = tallSession()

    // 120 is half way up 110 to 130. Dragged to half a clip above the top.
    session.pointerDown(at(BAR, yAt(0.5)))
    session.pointerMove(at(BAR, yAt(0.8)))
    session.pointerMove(at(BAR, yAt(1.5)))
    await session.pointerUp(at(BAR, yAt(1.5)))
    await settle()
    expect(toBpm(points()[1].value)).toBeCloseTo(140, 4)
    expect(view()).toEqual([110, 140])

    const typed = typePointValue({ clip: 0, automation, index: 0 })
    await answerText("95")
    await typed
    expect(view()).toEqual([95, 140])
    end()
  })

  it("shows the whole range of a fader again without keeping a view for it", async () => {
    const fader = await automate({
      type: "trackVolume",
      track: project().mixer.tracks[1].id,
    })
    fitViewToCurve(fader.automation)
    expect(chosenView(fader.automation)).toBeDefined()
    showFullRange(fader.automation)
    expect(chosenView(fader.automation)).toBeUndefined()
    expect(currentView(automations()[0])).toBe(FULL_VIEW)
  })

  it("is forgotten when another project is opened, whose ids start over", async () => {
    const { automation } = await tempoCurve()
    fitViewToCurve(automation)
    expect(chosenView(automation)).toBeDefined()
    await backend.projectNew()
    await settle()
    expect(useCurveViews.getState().views).toEqual({})
  })
})

describe("a point outside the view", () => {
  it("cannot be pressed where it is not drawn, and Fit brings it back", async () => {
    const { automation } = await tempoCurve()
    await setCurve(automation, [point(0, bpm(120)), point(BAR, bpm(300))])
    await settle()
    const asked = typeViewRange(automation)
    await answerText("100 to 140")
    await asked
    const { session, stop: end } = tallSession()
    const before = points()

    // The top edge, where a point held to the view would have been drawn.
    session.pointerDown(at(BAR, yAt(1)))
    session.pointerMove(at(BAR, yAt(0.5)))
    await session.pointerUp(at(BAR, yAt(0.5)))
    await settle()
    // That press was on the open curve: it added a point, and the one at
    // 300 bpm is where it was.
    expect(points()).toHaveLength(3)
    expect(points().some((item) => item.value === before[1].value)).toBe(true)

    fitViewToCurve(automation)
    const [low, high] = inBpm(currentView(automations()[0]))
    expect(low).toBeLessThan(120)
    expect(high).toBeGreaterThan(300)
    end()
  })
})
