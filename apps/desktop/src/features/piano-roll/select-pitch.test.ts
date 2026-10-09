import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import {
  getAppState,
  isEnabled,
  registry,
  runAction,
  shortcutLabel,
} from "@/lib/actions"
import { dispatch } from "@/lib/store/project"

import { NOTE_MENU, PANEL_MENU } from "./menu"
import { usePianoRollStore } from "./store"
import { channel, currentPattern, notesOf, startRoll, undoSteps } from "./test-utils"

let roll: Awaited<ReturnType<typeof startRoll>>

beforeEach(async () => {
  roll = await startRoll()
  await dispatch({ type: "addChannel", name: "Lead" })
  roll.show("Lead")
  await dispatch({
    type: "addNotes",
    pattern: currentPattern().id,
    channel: channel("Lead").id,
    notes: [60, 64, 60, 64, 62].map((key, index) => ({
      start: index * 240,
      key,
      length: 240,
    })),
  })
})

afterEach(() => {
  roll.stop()
  vi.restoreAllMocks()
})

describe("select matching pitches", () => {
  it("selects every C and E from two selected notes and leaves D unselected", () => {
    const notes = notesOf("Lead")
    roll.editor.setSelection([notes[0].id, notes[1].id])
    const send = vi.spyOn(roll.backend, "dispatch")
    const before = undoSteps()

    roll.editor.selectMatchingPitches()

    expect(roll.editor.selection).toEqual(
      new Set(notes.slice(0, 4).map((note) => note.id))
    )
    expect(roll.editor.selection.has(notes[4].id)).toBe(false)
    expect(usePianoRollStore.getState().selectionCount).toBe(4)
    expect(notesOf("Lead")).toEqual(notes)
    expect(undoSteps()).toBe(before)
    expect(send).not.toHaveBeenCalled()
  })

  it("leaves an empty selection empty and dispatches nothing", () => {
    const send = vi.spyOn(roll.backend, "dispatch")
    const before = undoSteps()
    const invalidations = roll.surface.invalidations
    const selectionChanged = vi.fn()
    const unsubscribe = roll.editor.subscribe(selectionChanged)

    roll.editor.selectMatchingPitches()

    expect(roll.editor.selection.size).toBe(0)
    expect(roll.surface.invalidations).toBe(invalidations)
    expect(selectionChanged).not.toHaveBeenCalled()
    expect(undoSteps()).toBe(before)
    expect(send).not.toHaveBeenCalled()
    unsubscribe()
  })

  it("runs through the action when selected, with both menu entries and no shortcut", async () => {
    const id = "pianoRoll.selectMatchingPitches"
    const action = registry.get(id)
    if (!action) throw new Error("Select matching pitches is not registered")
    expect(action.title).toBe("Select matching pitches")
    expect(shortcutLabel(id)).toBeUndefined()
    expect(isEnabled(action, getAppState())).toBe(false)
    for (const menu of [NOTE_MENU, PANEL_MENU]) {
      expect(menu[menu.indexOf("pianoRoll.selectAll") + 1]).toBe(id)
    }

    const notes = notesOf("Lead")
    roll.editor.setSelection([notes[0].id, notes[1].id])
    expect(isEnabled(action, getAppState())).toBe(true)
    const send = vi.spyOn(roll.backend, "dispatch")

    await runAction(id)

    expect(roll.editor.selection).toEqual(
      new Set(notes.slice(0, 4).map((note) => note.id))
    )
    expect(send).not.toHaveBeenCalled()
    roll.editor.setSelection([])
    expect(isEnabled(action, getAppState())).toBe(false)
  })
})
