import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { Note } from "@/bindings"
import {
  getAppState,
  isEnabled,
  registry,
  runAction,
  shortcutLabel,
} from "@/lib/actions"
import { dispatch, undo } from "@/lib/store/project"
import { announceProjectReplaced } from "@/lib/store/replaced"

import { PIANO_ROLL_ACTIONS } from "./actions"
import { NOTE_MENU, PANEL_MENU } from "./menu"
import { rememberedVelocity, toggledVelocity } from "./pointer-tools"
import { restoredVelocities } from "./restore-muted"
import { usePianoRollStore } from "./store"
import {
  channel,
  currentPattern,
  notesOf,
  startRoll,
  undoSteps,
} from "./test-utils"

const ACTION_ID = "pianoRoll.restoreMutedNotes"

function note(id: number, velocity: number): Note {
  return { id, start: 0, length: 240, key: 60, velocity, pan: 0 }
}

function restoreAction() {
  const action = registry.get(ACTION_ID)
  if (!action) throw new Error("Restore muted notes is not registered")
  return action
}

describe("restoredVelocities", () => {
  it("restores a silent note with memory and omits a sounding note", () => {
    const remembered = new Map([
      [1, 0.63],
      [2, 0.9],
    ])
    expect(
      restoredVelocities([note(1, 0), note(2, 0.5)], (id) => remembered.get(id))
    ).toEqual([{ id: 1, velocity: 0.63 }])
  })

  it("omits a silent note without positive memory", () => {
    const remembered = new Map([
      [2, 0],
      [3, -0.1],
    ])
    expect(
      restoredVelocities([note(1, 0), note(2, 0), note(3, 0)], (id) =>
        remembered.get(id)
      )
    ).toEqual([])
  })

  it("keeps updates in the given order", () => {
    const remembered = new Map([
      [3, 0.3],
      [1, 0.1],
      [2, 0.2],
    ])
    expect(
      restoredVelocities([note(3, 0), note(1, 0), note(2, 0)], (id) =>
        remembered.get(id)
      )
    ).toEqual([
      { id: 3, velocity: 0.3 },
      { id: 1, velocity: 0.1 },
      { id: 2, velocity: 0.2 },
    ])
  })
})

describe("rememberedVelocity", () => {
  beforeEach(() => announceProjectReplaced())

  it("returns the prior velocity after toggledVelocity silences a note", () => {
    expect(toggledVelocity(1, 2, note(3, 0.63), 0.8)).toBe(0)
    expect(rememberedVelocity(1, 2, 3)).toBe(0.63)
    expect(rememberedVelocity(2, 2, 3)).toBeUndefined()
    expect(rememberedVelocity(1, 3, 3)).toBeUndefined()
  })

  it("returns undefined for a note never muted by toggledVelocity", () => {
    expect(rememberedVelocity(1, 2, 3)).toBeUndefined()
    expect(toggledVelocity(1, 2, note(3, 0), 0)).toBe(0.8)
    expect(rememberedVelocity(1, 2, 3)).toBeUndefined()
  })

  it("forgets the remembered velocity when the project is replaced", () => {
    toggledVelocity(1, 2, note(3, 0.63), 0.8)
    announceProjectReplaced()
    expect(rememberedVelocity(1, 2, 3)).toBeUndefined()
  })
})

describe("restore muted notes action", () => {
  let roll: Awaited<ReturnType<typeof startRoll>>

  beforeEach(async () => {
    announceProjectReplaced()
    roll = await startRoll()
    await dispatch({ type: "addChannel", name: "Lead" })
    roll.show("Lead")
  })

  afterEach(() => {
    roll.stop()
    vi.restoreAllMocks()
  })

  it("restores velocities in one undo step without changing selection", async () => {
    const action = restoreAction()
    expect(action.title).toBe("Restore muted notes")
    expect(shortcutLabel(ACTION_ID)).toBeUndefined()
    expect(isEnabled(action, getAppState())).toBe(false)
    expect(
      PIANO_ROLL_ACTIONS[
        PIANO_ROLL_ACTIONS.findIndex(
          (item) => item.id === "pianoRoll.selectMutedNotes"
        ) + 1
      ].id
    ).toBe(ACTION_ID)
    for (const menu of [NOTE_MENU, PANEL_MENU]) {
      expect(menu[menu.indexOf("pianoRoll.selectMutedNotes") + 1]).toBe(
        ACTION_ID
      )
    }

    const target = { pattern: currentPattern().id, channel: channel("Lead").id }
    await dispatch({
      type: "addNotes",
      ...target,
      notes: [
        { start: 0, key: 60, length: 240, velocity: 0.63 },
        { start: 240, key: 64, length: 240, velocity: 0.42 },
        { start: 480, key: 67, length: 240, velocity: 0.5 },
        { start: 720, key: 69, length: 240, velocity: 0 },
      ],
    })
    const original = notesOf("Lead")
    await dispatch({
      type: "updateNotes",
      ...target,
      updates: original.slice(0, 2).map((item) => ({
        id: item.id,
        patch: {
          velocity: toggledVelocity(target.pattern, target.channel, item, 0.8),
        },
      })),
    })
    // A sounding note can have memory too; it must remain untouched.
    toggledVelocity(target.pattern, target.channel, original[2], 0.8)
    const muted = notesOf("Lead")
    roll.editor.setSelection([original[2].id])
    expect(isEnabled(action, getAppState())).toBe(true)
    const send = vi.spyOn(roll.backend, "dispatch")
    const before = undoSteps()

    await runAction(ACTION_ID)

    expect(send).toHaveBeenCalledExactlyOnceWith(
      {
        type: "updateNotes",
        ...target,
        updates: original
          .slice(0, 2)
          .map(({ id, velocity }) => ({ id, patch: { velocity } })),
      },
      undefined
    )
    expect(notesOf("Lead")).toEqual(original)
    expect(undoSteps()).toBe(before + 1)
    expect(roll.editor.selection).toEqual(new Set([original[2].id]))
    expect(usePianoRollStore.getState().selectionCount).toBe(1)
    expect(isEnabled(action, getAppState())).toBe(false)
    await undo()
    expect(notesOf("Lead")).toEqual(muted)
    expect(roll.editor.selection).toEqual(new Set([original[2].id]))
  })

  it("dispatches nothing when silent notes have no memory", async () => {
    await dispatch({
      type: "addNotes",
      pattern: currentPattern().id,
      channel: channel("Lead").id,
      notes: [{ start: 0, key: 60, length: 240, velocity: 0 }],
    })
    const notes = notesOf("Lead")
    roll.editor.setSelection([notes[0].id])
    const action = restoreAction()
    expect(isEnabled(action, getAppState())).toBe(false)
    const send = vi.spyOn(roll.backend, "dispatch")
    const before = undoSteps()

    await action.run()

    expect(send).not.toHaveBeenCalled()
    expect(notesOf("Lead")).toEqual(notes)
    expect(undoSteps()).toBe(before)
    expect(roll.editor.selection).toEqual(new Set([notes[0].id]))
  })

  it("uses only the open channel and dispatches nothing with no channel open", async () => {
    const target = { pattern: currentPattern().id, channel: channel("Lead").id }
    await dispatch({
      type: "addNotes",
      ...target,
      notes: [{ start: 0, key: 60, length: 240, velocity: 0.63 }],
    })
    const original = notesOf("Lead")[0]
    await dispatch({
      type: "updateNotes",
      ...target,
      updates: [
        {
          id: original.id,
          patch: {
            velocity: toggledVelocity(
              target.pattern,
              target.channel,
              original,
              0.8
            ),
          },
        },
      ],
    })
    const action = restoreAction()
    expect(isEnabled(action, getAppState())).toBe(true)
    roll.show("Kick")
    expect(isEnabled(action, getAppState())).toBe(false)
    const send = vi.spyOn(roll.backend, "dispatch")
    await action.run()
    expect(notesOf("Lead")[0].velocity).toBe(0)
    roll.editor.setContext(null)
    expect(isEnabled(action, getAppState())).toBe(false)
    await action.run()
    expect(send).not.toHaveBeenCalled()
  })
})
