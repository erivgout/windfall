import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { dispatch, undo } from "@/lib/store/project"
import { DEFAULT_LIMITS, deriveGridTheme, rgba, type TimeGridView } from "@/lib/canvas"
import { settle } from "@/test/harness"
import { attachGridInput } from "./grid-input"
import { usePianoRollStore } from "./store"
import { CHORD_STAMPS } from "./stamps"
import { at, channel, currentPattern, notesOf, startRoll, undoSteps } from "./test-utils"

let roll: Awaited<ReturnType<typeof startRoll>>
let detach: (() => void) | undefined
let canvas: HTMLCanvasElement
beforeEach(async () => {
  roll = await startRoll()
  await dispatch({ type: "addChannel", name: "Ghost" })
  await dispatch({ type: "addNotes", pattern: currentPattern().id, channel: channel("Ghost").id,
    notes: [{ start: 0, length: 960, key: 60, velocity: 0.6 }] })
  usePianoRollStore.setState({ ghosts: true, editGhosts: true, tool: "draw" })
  canvas = document.createElement("canvas")
  document.body.append(canvas)
  canvas.setPointerCapture = vi.fn()
  canvas.hasPointerCapture = () => false
  canvas.releasePointerCapture = vi.fn()
  vi.stubGlobal("requestAnimationFrame", () => 1)
  vi.stubGlobal("cancelAnimationFrame", vi.fn())
  const view = Object.assign(roll.surface, { element: canvas, limits: DEFAULT_LIMITS,
    theme: deriveGridTheme({ background: rgba(20, 20, 22), foreground: rgba(240, 240, 240),
      mutedForeground: rgba(160, 160, 160), brand: rgba(230, 60, 140), playhead: rgba(90, 150, 240),
      gridLine: rgba(255, 255, 255, 18), gridLineStrong: rgba(255, 255, 255, 51) }),
    localPoint: (event: MouseEvent) => ({ x: event.clientX, y: event.clientY }), panBy: vi.fn() })
  detach = attachGridInput(roll.session, view as unknown as TimeGridView)
})
afterEach(() => {
  detach?.()
  canvas.remove()
  roll.stop()
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

function pointer(type: string, button = 2, tick = 300) {
  const point = at(tick, 60)
  const event = new MouseEvent(type, { button, clientX: point.x, clientY: point.y, bubbles: true })
  Object.defineProperty(event, "pointerId", { value: 1 })
  canvas.dispatchEvent(event)
}

it("right-click erases an editable ghost in its source lane with one undo step", async () => {
  const original = notesOf("Ghost")
  const current = notesOf("Kick")
  const before = undoSteps()
  const viewport = { ...roll.surface.viewport }
  pointer("pointerdown")
  pointer("pointerup")
  await settle()
  expect(notesOf("Ghost")).toEqual([])
  expect(notesOf("Kick")).toEqual(current)
  expect(undoSteps()).toBe(before + 1)
  expect(roll.surface.viewport).toEqual(viewport)
  await undo()
  expect(notesOf("Ghost")).toEqual(original)
})

it("keeps read-only ghosts untouched by right-click erase", async () => {
  usePianoRollStore.setState({ editGhosts: false })
  const original = notesOf("Ghost")
  const before = undoSteps()
  pointer("pointerdown")
  pointer("pointerup")
  await settle()
  expect(notesOf("Ghost")).toEqual(original)
  expect(undoSteps()).toBe(before)
})

it("moves an editable ghost in its source lane without copying it or changing the viewport", async () => {
  const original = notesOf("Ghost")
  const current = notesOf("Kick")
  const before = undoSteps()
  const viewport = { ...roll.surface.viewport }
  pointer("pointerdown", 0)
  pointer("pointermove", 0, 540)
  pointer("pointerup", 0, 540)
  await settle()
  expect(notesOf("Ghost")).toHaveLength(1)
  expect(notesOf("Ghost")[0]).toMatchObject({ id: original[0].id, start: 240 })
  expect(notesOf("Kick")).toEqual(current)
  expect(undoSteps()).toBe(before + 1)
  expect(roll.surface.viewport).toEqual(viewport)
  await undo()
  expect(notesOf("Ghost")).toEqual(original)
})

it("right-click cancels an armed stamp over a ghost without switching lanes or erasing", async () => {
  const original = notesOf("Ghost")
  const editing = roll.session.editing
  const before = undoSteps()
  roll.editor.armStamp(CHORD_STAMPS[0])
  pointer("pointerdown")
  pointer("pointerup")
  await settle()
  expect(roll.editor.stampState).toBeNull()
  expect(roll.session.editing).toEqual(editing)
  expect(notesOf("Ghost")).toEqual(original)
  expect(undoSteps()).toBe(before)
})
