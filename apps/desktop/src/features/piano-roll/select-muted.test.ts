import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { Note } from "@/bindings"
import {
  getAppState,
  isEnabled,
  registry,
  runAction,
  shortcutLabel,
} from "@/lib/actions"
import { dispatch } from "@/lib/store/project"

import { NOTE_MENU, PANEL_MENU } from "./menu"
import { mutedNoteIds } from "./select-muted"
import { usePianoRollStore } from "./store"
import { channel, currentPattern, notesOf, startRoll, undoSteps } from "./test-utils"

function note(id: number, velocity: number): Note {
  return { id, start: 0, length: 240, key: 60, velocity, pan: 0 }
}

describe("mutedNoteIds", () => {
  it("keeps velocity 0 notes in order and drops notes with velocity above 0", () => {
    expect(mutedNoteIds([note(3, 0), note(1, 0.5), note(2, 0)])).toEqual([3, 2])
  })

  it("returns nothing when there are no muted notes", () => {
    expect(mutedNoteIds([note(1, 0.5), note(2, 1)])).toEqual([])
    expect(mutedNoteIds([])).toEqual([])
  })
})

describe("select muted notes", () => {
  let roll: Awaited<ReturnType<typeof startRoll>>

  beforeEach(async () => {
    roll = await startRoll()
    await dispatch({ type: "addChannel", name: "Lead" })
    roll.show("Lead")
  })

  afterEach(() => {
    roll.stop()
    vi.restoreAllMocks()
  })

  it("selects only the silent note through the action without dispatching", async () => {
    const id = "pianoRoll.selectMutedNotes"
    const action = registry.get(id)
    if (!action) throw new Error("Select muted notes is not registered")
    expect(action.title).toBe("Select muted notes")
    expect(shortcutLabel(id)).toBeUndefined()
    expect(isEnabled(action, getAppState())).toBe(false)
    for (const menu of [NOTE_MENU, PANEL_MENU]) {
      expect(menu[menu.indexOf("pianoRoll.selectMatchingPitches") + 1]).toBe(id)
    }

    await dispatch({
      type: "addNotes",
      pattern: currentPattern().id,
      channel: channel("Lead").id,
      notes: [
        { start: 0, key: 60, length: 240, velocity: 0 },
        { start: 240, key: 64, length: 240, velocity: 0.5 },
      ],
    })
    const notes = notesOf("Lead")
    roll.editor.setSelection([notes[1].id])
    expect(isEnabled(action, getAppState())).toBe(true)
    const send = vi.spyOn(roll.backend, "dispatch")
    const before = undoSteps()

    await runAction(id)

    expect(roll.editor.selection).toEqual(new Set([notes[0].id]))
    expect(usePianoRollStore.getState().selectionCount).toBe(1)
    expect(notesOf("Lead")).toEqual(notes)
    expect(undoSteps()).toBe(before)
    expect(send).not.toHaveBeenCalled()
    roll.show("Kick")
    expect(isEnabled(action, getAppState())).toBe(false)
  })

  it("clears the selection when the open channel has no muted notes", async () => {
    await dispatch({
      type: "addNotes",
      pattern: currentPattern().id,
      channel: channel("Lead").id,
      notes: [{ start: 0, key: 60, length: 240, velocity: 0.5 }],
    })
    const notes = notesOf("Lead")
    roll.editor.setSelection([notes[0].id])
    const action = registry.get("pianoRoll.selectMutedNotes")
    if (!action) throw new Error("Select muted notes is not registered")
    expect(isEnabled(action, getAppState())).toBe(false)
    const send = vi.spyOn(roll.backend, "dispatch")
    const before = undoSteps()

    roll.editor.selectMutedNotes()

    expect(roll.editor.selection.size).toBe(0)
    expect(usePianoRollStore.getState().selectionCount).toBe(0)
    expect(notesOf("Lead")).toEqual(notes)
    expect(undoSteps()).toBe(before)
    expect(send).not.toHaveBeenCalled()
  })
})
