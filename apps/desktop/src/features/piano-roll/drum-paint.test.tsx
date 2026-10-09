import { cleanup, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { NoteInit } from "@/bindings"
import { TooltipProvider } from "@/components/ui/tooltip"
import { runAction, shortcutLabel } from "@/lib/actions"
import { dispatch, redo, undo } from "@/lib/store/project"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { useUiStore } from "@/lib/store/ui"
import { settle } from "@/test/harness"
import PianoRollPanel from "./index"
import { usePianoRollStore } from "./store"
import {
  at,
  brief,
  channel,
  currentPattern,
  notesOf,
  startRoll,
  undoSteps,
} from "./test-utils"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let roll: Awaited<ReturnType<typeof startRoll>>
beforeEach(async () => {
  roll = await startRoll()
  await dispatch({ type: "addChannel", name: "Lead" })
  roll.show("Lead")
  await runAction("pianoRoll.toolPaint")
  await runAction("pianoRoll.drum")
})
afterEach(() => {
  cleanup()
  roll.stop()
  vi.restoreAllMocks()
})

const lead = () => brief(notesOf("Lead"))
async function add(notes: NoteInit[]) {
  await dispatch({
    type: "addNotes",
    pattern: currentPattern().id,
    channel: channel("Lead").id,
    notes,
  })
}

describe("piano-roll Drum Paint", () => {
  it("adds a snap-length note at an empty cell's start on release in one batch", async () => {
    usePianoRollStore.getState().rememberNote(960, 0.65)
    const before = undoSteps()
    const send = vi.spyOn(roll.backend, "dispatch")
    roll.editor.pointerDown(at(500, 64), "left")
    expect(lead()).toEqual([])
    expect(send).not.toHaveBeenCalled()
    roll.editor.pointerUp(at(500, 64))
    await settle()
    expect(notesOf("Lead")).toMatchObject([
      { start: 480, length: 240, key: 64, velocity: 0.65 },
    ])
    expect(send).toHaveBeenCalledTimes(1)
    expect(send.mock.calls[0][0]).toMatchObject({
      type: "batch",
      label: "Paint drum steps",
      commands: [{ type: "addNotes" }],
    })
    expect(undoSteps()).toBe(before + 1)
    await undo()
    expect(lead()).toEqual([])
    await redo()
    expect(lead()).toEqual(["480:64:240"])
  })

  it("deletes every note starting in a filled cell, including off-grid starts", async () => {
    await add([
      { start: 480, length: 240, key: 60 },
      { start: 600, length: 960, key: 60 },
      { start: 600, length: 240, key: 61 },
      { start: 720, length: 240, key: 60 },
    ])
    const original = notesOf("Lead")
    const before = undoSteps()
    const send = vi.spyOn(roll.backend, "dispatch")
    // The filled cell routes to Drum even at a note edge.
    await roll.click(at(480, 60))
    expect(lead()).toEqual(["600:61:240", "720:60:240"])
    expect(send).toHaveBeenCalledTimes(1)
    expect(send.mock.calls[0][0]).toMatchObject({
      type: "batch",
      commands: [{ type: "removeNotes" }],
    })
    expect(undoSteps()).toBe(before + 1)
    await undo()
    expect(notesOf("Lead")).toEqual(original)
  })

  it("keeps an empty first cell's add decision across filled cells and revisits", async () => {
    await add([{ start: 480, length: 240, key: 60 }])
    const before = undoSteps()
    await roll.drag(at(20, 60), [at(980, 60), at(20, 60), at(980, 60)])
    expect(lead()).toEqual([
      "0:60:240",
      "240:60:240",
      "480:60:240",
      "720:60:240",
      "960:60:240",
    ])
    expect(undoSteps()).toBe(before + 1)
  })

  it("keeps a filled first cell's delete decision across empty cells and revisits", async () => {
    await add([
      { start: 0, length: 240, key: 60 },
      { start: 480, length: 240, key: 60 },
      { start: 960, length: 240, key: 60 },
    ])
    const before = undoSteps()
    await roll.drag(at(20, 60), [at(980, 60), at(20, 60)])
    expect(lead()).toEqual([])
    expect(undoSteps()).toBe(before + 1)
  })

  it("tracks cells separately on different keys", async () => {
    await roll.drag(at(20, 60), [at(20, 61), at(20, 60)])
    expect(lead()).toEqual(["0:60:240", "0:61:240"])
  })

  it("counts only starts, allowing an empty cell under a preceding note's tail", async () => {
    await add([{ start: 0, length: 960, key: 60 }])
    await roll.click(at(500, 60))
    expect(lead()).toEqual(["0:60:960", "480:60:240"])
  })

  it.each(["add", "delete"])(
    "cancels a %s stroke without dispatching",
    async (decision) => {
      if (decision === "delete") await add([{ start: 0, length: 240, key: 60 }])
      const original = notesOf("Lead")
      const before = undoSteps()
      const send = vi.spyOn(roll.backend, "dispatch")
      roll.editor.pointerDown(at(20, 60), "left")
      roll.editor.pointerMove(at(980, 60))
      if (decision === "add")
        await runAction("pianoRoll.deselect") // Escape's action.
      else roll.editor.cancel() // Pointer cancellation/lost capture/blur's path.
      roll.editor.pointerUp(at(980, 60))
      await settle()
      expect(notesOf("Lead")).toEqual(original)
      expect(send).not.toHaveBeenCalled()
      expect(undoSteps()).toBe(before)
    }
  )

  it("leaves normal Paint's remembered length, spacing, and filled-note move intact when off", async () => {
    await runAction("pianoRoll.drum")
    usePianoRollStore.getState().rememberNote(960, 0.8)
    await roll.drag(at(500, 60), at(2500, 60))
    expect(lead()).toEqual(["480:60:960", "1440:60:960", "2400:60:960"])
    expect(roll.editor.pointerDown(at(800, 60), "left")).toEqual({
      kind: "move",
    })
    roll.editor.cancel()
    expect(lead()).toEqual(["480:60:960", "1440:60:960", "2400:60:960"])
  })

  it("leaves Draw as a normal remembered-length draw while Drum is on", async () => {
    await runAction("pianoRoll.toolDraw")
    usePianoRollStore.getState().rememberNote(960, 0.8)
    await roll.click(at(500, 64))
    expect(lead()).toEqual(["480:64:960"])
  })

  it("shows the Drum toolbar toggle and binds G in both keymap presets", async () => {
    render(
      <TooltipProvider>
        <PianoRollPanel />
      </TooltipProvider>
    )
    const button = screen.getByRole("button", { name: "Drum" })
    expect(button).toHaveAttribute("aria-pressed", "true")
    fireEvent.click(button)
    await settle()
    expect(button).toHaveAttribute("aria-pressed", "false")
    const root = document.createElement("div")
    root.tabIndex = 0
    root.dataset.shortcutScope = "pianoRoll"
    document.body.append(root)
    try {
      for (const keymap of ["windfall", "fl"] as const) {
        useUiStore.getState().setKeymap(keymap)
        expect(shortcutLabel("pianoRoll.drum")).toBe("G")
        root.focus()
        root.dispatchEvent(
          new KeyboardEvent("keydown", {
            key: "g",
            code: "KeyG",
            bubbles: true,
          })
        )
        await settle()
        expect(usePianoRollStore.getState().drum).toBe(keymap === "windfall")
      }
    } finally {
      root.remove()
    }
  })

  it("resets Drum when another project replaces the open project", () => {
    expect(usePianoRollStore.getState().drum).toBe(true)
    announceProjectReplaced()
    expect(usePianoRollStore.getState().drum).toBe(false)
  })
})
