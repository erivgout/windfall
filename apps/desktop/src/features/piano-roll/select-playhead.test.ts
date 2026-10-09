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

import { PIANO_ROLL_ACTIONS } from "./actions"
import { NOTE_MENU, PANEL_MENU } from "./menu"
import { notesAtTick } from "./select-playhead"
import {
  channel,
  currentPattern,
  notesOf,
  startRoll,
  undoSteps,
} from "./test-utils"

const ACTION_ID = "pianoRoll.selectNotesAtPlayhead"

function note(id: number, start: number, length: number): Note {
  return { id, start, length, key: 60, velocity: 0.5, pan: 0 }
}

describe("notesAtTick", () => {
  it("includes a covering note and excludes a note ending on the tick", () => {
    expect(notesAtTick([note(1, 0, 241), note(2, 0, 240)], 240)).toEqual([1])
  })

  it("includes a note starting on the tick", () => {
    expect(notesAtTick([note(1, 240, 240)], 240)).toEqual([1])
  })

  it("keeps notes in the given order", () => {
    expect(
      notesAtTick([note(3, 120, 240), note(1, 0, 480), note(2, 240, 240)], 240)
    ).toEqual([3, 1, 2])
  })

  it("excludes notes elsewhere in the pattern", () => {
    expect(notesAtTick([note(1, 0, 120), note(2, 480, 240)], 240)).toEqual([])
  })

  it("uses fractional ticks without flooring them", () => {
    expect(notesAtTick([note(1, 240.5, 1)], 240.75)).toEqual([1])
  })
})

describe("select notes at the playhead action", () => {
  let roll: Awaited<ReturnType<typeof startRoll>>

  beforeEach(async () => {
    roll = await startRoll()
    await dispatch({ type: "addChannel", name: "Lead" })
    roll.show("Lead")
    await dispatch({
      type: "addNotes",
      pattern: currentPattern().id,
      channel: channel("Lead").id,
      notes: [
        { start: 0, key: 60, length: 240, velocity: 0.5 },
        { start: 120, key: 64, length: 121, velocity: 0.5 },
        { start: 240, key: 67, length: 240, velocity: 0 },
      ],
    })
  })

  afterEach(() => {
    roll.stop()
    vi.restoreAllMocks()
  })

  function action() {
    const found = registry.get(ACTION_ID)
    if (!found)
      throw new Error("Select notes at the playhead is not registered")
    return found
  }

  it("selects the open editor notes at the floored session playhead without dispatching", async () => {
    const registered = action()
    expect(registered.title).toBe("Select notes at the playhead")
    expect(shortcutLabel(ACTION_ID)).toBeUndefined()
    expect(
      PIANO_ROLL_ACTIONS[
        PIANO_ROLL_ACTIONS.findIndex(
          (item) => item.id === "pianoRoll.restoreMutedNotes"
        ) + 1
      ].id
    ).toBe(ACTION_ID)
    for (const menu of [NOTE_MENU, PANEL_MENU]) {
      expect(menu[menu.indexOf("pianoRoll.restoreMutedNotes") + 1]).toBe(
        ACTION_ID
      )
    }
    const notes = notesOf("Lead")
    roll.editor.setSelection([notes[0].id])
    roll.session.setPlayhead(240.75)
    expect(isEnabled(registered, getAppState())).toBe(true)
    const select = vi.spyOn(roll.editor, "setSelection")
    const send = vi.spyOn(roll.backend, "dispatch")
    const before = undoSteps()

    await runAction(ACTION_ID)

    expect(select).toHaveBeenCalledExactlyOnceWith([notes[1].id, notes[2].id])
    expect(roll.editor.selection).toEqual(new Set([notes[1].id, notes[2].id]))
    expect(notesOf("Lead")).toEqual(notes)
    expect(undoSteps()).toBe(before)
    expect(send).not.toHaveBeenCalled()
    roll.show("Kick")
    expect(isEnabled(registered, getAppState())).toBe(false)
  })

  it.each([null, 480])(
    "keeps the selection when the playhead is %s",
    async (playhead) => {
      const notes = notesOf("Lead")
      roll.editor.setSelection([notes[0].id])
      roll.session.setPlayhead(playhead)
      const registered = action()
      expect(isEnabled(registered, getAppState())).toBe(false)
      const select = vi.spyOn(roll.editor, "setSelection")
      const send = vi.spyOn(roll.backend, "dispatch")

      await registered.run()

      expect(select).not.toHaveBeenCalled()
      expect(roll.editor.selection).toEqual(new Set([notes[0].id]))
      expect(notesOf("Lead")).toEqual(notes)
      expect(send).not.toHaveBeenCalled()
    }
  )
})
