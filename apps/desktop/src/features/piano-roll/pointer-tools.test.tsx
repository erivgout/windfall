import { fireEvent, render, screen, cleanup } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import type { NoteInit } from "@/bindings"
import { runAction } from "@/lib/actions"
import { DEFAULT_LIMITS, type TimeGridView } from "@/lib/canvas"
import { dispatch, redo, undo } from "@/lib/store/project"
import { settle } from "@/test/harness"
import { TooltipProvider } from "@/components/ui/tooltip"
import { attachGridInput } from "./grid-input"
import PianoRollPanel from "./index"
import { usePianoRollStore } from "./store"
import { sliceNoteCommand } from "./pointer-tools"
import { at, channel, currentPattern, notesOf, startRoll, undoSteps } from "./test-utils"

vi.mock("sonner", () => ({ toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }) }))

let roll: Awaited<ReturnType<typeof startRoll>>
let detach: (() => void) | undefined
beforeEach(async () => {
  roll = await startRoll()
  await dispatch({ type: "addChannel", name: "Lead" })
  roll.show("Lead")
})
afterEach(() => {
  detach?.()
  detach = undefined
  cleanup()
  roll.stop()
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})
async function add(notes: NoteInit[]) {
  return dispatch({ type: "addNotes", pattern: currentPattern().id, channel: channel("Lead").id, notes })
}

describe("remaining piano-roll tools", () => {
  it("makes every tool selectable from the piano-roll toolbar", async () => {
    render(<TooltipProvider><PianoRollPanel /></TooltipProvider>)
    for (const [tool, label] of [["mute", "Mute"], ["slice", "Slice"], ["zoom", "Zoom"], ["playback", "Playback"]] as const) {
      fireEvent.click(screen.getByRole("button", { name: `${label} tool` }))
      await settle()
      expect(usePianoRollStore.getState().tool).toBe(tool)
    }
  })

  it("toggles mute through note updates with one undo step per gesture", async () => {
    await add([{ start: 0, length: 960, key: 60, velocity: 0.63 }])
    await runAction("pianoRoll.toolMute")
    const before = undoSteps()
    await roll.drag(at(240, 60), [at(400, 60), at(600, 60)])
    expect(notesOf("Lead")[0].velocity).toBe(0)
    expect(undoSteps()).toBe(before + 1)
    await undo()
    expect(notesOf("Lead")[0].velocity).toBeCloseTo(0.63)
    await redo()
    await roll.click(at(240, 60))
    expect(notesOf("Lead")[0].velocity).toBeCloseTo(0.63)
    const after = undoSteps()
    await roll.click(at(2000, 62))
    expect(undoSteps()).toBe(after)
  })

  it.each([1, 239, 480, 959])("conserves split length at tick %s and preserves expression", async (tick) => {
    const expression = { release: 0.4, finePitchCents: 17, modulationX: 0.3, modulationY: 0.7, articulation: "portamento" as const, glideTicks: 100, colorGroup: 2 }
    await add([{ start: 0, length: 960, key: 60, velocity: 0.7, pan: -0.2, expression }])
    const original = notesOf("Lead")[0]
    await runAction("pianoRoll.toolSlice")
    const before = undoSteps()
    await roll.click(at(tick, 60, { alt: true }))
    const notes = notesOf("Lead")
    expect(notes).toHaveLength(2)
    expect(notes.map((note) => note.length)).toEqual([tick, 960 - tick])
    expect(notes[1].start).toBe(tick)
    expect(notes[0].length + notes[1].length).toBe(original.length)
    for (const note of notes) {
      expect(note).toMatchObject({ key: original.key, velocity: original.velocity, pan: original.pan })
      expect(note.expression).toEqual(original.expression)
    }
    expect(undoSteps()).toBe(before + 1)
    await undo()
    expect(notesOf("Lead")).toEqual([original])
    await redo()
    expect(notesOf("Lead")).toHaveLength(2)
  })

  it("ignores boundary slices and cancels an unfinished mute", async () => {
    await add([{ start: 0, length: 960, key: 60 }])
    await runAction("pianoRoll.toolSlice")
    const before = undoSteps()
    await roll.click(at(1, 60)) // snap rounds onto the existing start
    await roll.click(at(950, 60)) // snap rounds onto the existing end
    expect(undoSteps()).toBe(before)
    await runAction("pianoRoll.toolMute")
    roll.editor.pointerDown(at(200, 60), "left")
    roll.editor.cancel()
    roll.editor.pointerUp(at(200, 60))
    await settle()
    expect(undoSteps()).toBe(before)
  })

  it("preserves scalar expression and crops expression curves in the slice command", () => {
    // The checked-in WASM simulator predates expression/curve commands.
    // Verify their payload at the command boundary without dropping fields.
    const original = { id: 42, start: 0, length: 960, key: 60, velocity: 0.7, pan: -0.2,
      expression: { release: 0.4, finePitchCents: 17, modulationX: 0.3, modulationY: 0.7, articulation: "portamento" as const, glideTicks: 100, colorGroup: 2 } }
    const command = sliceNoteCommand({ pattern: 1, channel: 2 }, original, 480, [{ note: original.id, parameter: "pan", points: [
        { position: 0, value: -1, curve: 0, hold: false }, { position: 1, value: 1, curve: 0, hold: false },
      ] }])
    expect(command?.type).toBe("batch")
    if (command?.type !== "batch") throw new Error("Expected slice batch")
    const [leftCurves, update, right] = command.commands
    if (leftCurves.type !== "setNoteExpressionCurves" || right.type !== "addNotesWithCurves") throw new Error("Expected expression commands")
    expect(leftCurves.expected).toEqual([original])
    expect(leftCurves.curves[0].points.map((point) => [point.position, point.value])).toEqual([[0, -1], [1, 0]])
    expect(right.curves[0].points.map((point) => [point.position, point.value])).toEqual([[0, 0], [1, 1]])
    expect(right.notes[0]).toMatchObject({ start: 480, length: 480, key: 60, velocity: 0.7, pan: -0.2, expression: original.expression })
    expect(update).toMatchObject({ type: "updateNotes", updates: [{ id: 42, patch: { length: 480 } }] })
  })

  it("fits a dragged rectangle and restores zoom on another click or Escape", async () => {
    await runAction("pianoRoll.toolZoom")
    const original = { ...roll.surface.viewport }
    const before = undoSteps()
    await roll.drag(at(1200, 65, {}, 0), at(5200, 56, {}, 0))
    expect(roll.surface.viewport).toMatchObject({ scrollTick: 1200, scrollRow: 62, pxPerTick: 800 / 4000, rowHeight: 320 / 9 })
    expect(roll.editor.canRestoreZoom).toBe(true)
    await roll.click(at(1000, 60))
    expect(roll.surface.viewport).toEqual(original)
    await roll.drag(at(5200, 56, {}, 0), at(1200, 65, {}, 0))
    await runAction("pianoRoll.deselect")
    expect(roll.surface.viewport).toEqual(original)
    expect(roll.editor.canRestoreZoom).toBe(false)
    expect(undoSteps()).toBe(before)
  })

  it("does not zoom a zero-height rectangle or a canceled drag", async () => {
    await runAction("pianoRoll.toolZoom")
    const original = { ...roll.surface.viewport }
    await roll.drag(at(100, 60), at(1000, 60))
    expect(roll.surface.viewport).toEqual(original)
    roll.editor.pointerDown(at(100, 65), "left")
    roll.editor.pointerMove(at(1000, 56))
    roll.editor.cancel()
    expect(roll.surface.marquee).toBeNull()
    expect(roll.surface.viewport).toEqual(original)
  })

  it.each(["pointerup", "blur", "Escape", "pointercancel", "lostpointercapture"])("seeks while held and releases scrub audition on %s", async (release) => {
    await add([{ start: 0, length: 960, key: 60, velocity: 0.7 }])
    await runAction("pianoRoll.toolPlayback")
    const seek = vi.spyOn(roll.backend, "transportSeek")
    const on = vi.spyOn(roll.backend, "auditionNoteOn")
    const off = vi.spyOn(roll.backend, "auditionNoteOff")
    const canvas = document.createElement("canvas")
    document.body.append(canvas)
    canvas.setPointerCapture = vi.fn()
    canvas.hasPointerCapture = () => false
    canvas.releasePointerCapture = vi.fn()
    vi.stubGlobal("requestAnimationFrame", () => 1)
    vi.stubGlobal("cancelAnimationFrame", vi.fn())
    const view = Object.assign(roll.surface, { element: canvas, limits: DEFAULT_LIMITS,
      localPoint: (event: MouseEvent) => ({ x: event.clientX, y: event.clientY }), panBy: vi.fn() })
    detach = attachGridInput(roll.session, view as unknown as TimeGridView)
    const pointer = (type: string, tick: number) => {
      const input = at(tick, 60)
      const event = new MouseEvent(type, { button: 0, clientX: input.x, clientY: input.y, bubbles: true })
      Object.defineProperty(event, "pointerId", { value: 1 })
      canvas.dispatchEvent(event)
    }
    const before = undoSteps()
    pointer("pointerdown", 200)
    pointer("pointermove", 500)
    expect(seek).toHaveBeenLastCalledWith(500)
    expect(roll.session.playhead).toBe(500)
    expect(on).toHaveBeenCalledOnce()
    if (release === "blur") window.dispatchEvent(new Event("blur"))
    else if (release === "Escape") window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", cancelable: true }))
    else pointer(release, 500)
    await settle()
    expect(off).toHaveBeenCalledOnce()
    expect(roll.editor.scrubbing).toBe(false)
    const seeks = seek.mock.calls.length
    pointer("pointermove", 700)
    expect(seek).toHaveBeenCalledTimes(seeks)
    expect(undoSteps()).toBe(before)
  })
})
