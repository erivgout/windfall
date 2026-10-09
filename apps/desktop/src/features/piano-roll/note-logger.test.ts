import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { getAppState, isEnabled, registry, runAction } from "@/lib/actions"
import { dispatch, undo } from "@/lib/store/project"
import * as realtime from "@/lib/store/realtime"
import { useSnapStore } from "@/lib/store/snap"
import { useUiStore } from "@/lib/store/ui"
import { settle } from "@/test/harness"

import { auditionOff, auditionOn } from "./audition"
import { dumpPlayedNotes } from "./dump-played-notes"
import { clearNoteLog, playedNotesFor, useNoteLogStore } from "./note-log"
import { usePianoRollStore } from "./store"
import {
  channel,
  currentPattern,
  notesOf,
  startRoll,
  undoSteps,
} from "./test-utils"

describe("audition note logger", () => {
  let roll: Awaited<ReturnType<typeof startRoll>>
  let position: number

  beforeEach(async () => {
    vi.useFakeTimers({ toFake: ["Date", "setTimeout", "clearTimeout"] })
    vi.setSystemTime(0)
    clearNoteLog()
    roll = await startRoll()
    await dispatch({ type: "updateSettings", patch: { tempoBpm: 120 } })
    await dispatch({
      type: "clearLane",
      pattern: currentPattern().id,
      channel: channel("Kick").id,
    })
    position = 960
    const frame = { ...realtime.realtimeFrame() }
    vi.spyOn(realtime, "realtimeFrame").mockImplementation(() => ({
      ...frame,
      tick: position,
    }))
    useSnapStore.getState().setSnap("step")
  })

  afterEach(() => {
    clearNoteLog()
    roll.stop()
    vi.restoreAllMocks()
    vi.useRealTimers()
  })

  function play(name: string, key: number, duration: number, velocity = 0.63) {
    auditionOn(channel(name).id, key, velocity)
    vi.advanceTimersByTime(duration)
    auditionOff(channel(name).id, key)
  }

  const enabled = () =>
    isEnabled(registry.get("pianoRoll.dumpPlayedNotes")!, getAppState())

  it("turns pairs into held notes at the transport position with shared snap and one addNotes", async () => {
    const on = vi.spyOn(roll.backend, "auditionNoteOn")
    const off = vi.spyOn(roll.backend, "auditionNoteOff")
    const version = registry.stateVersion()
    expect(enabled()).toBe(false)
    play("Kick", 60, 500)
    vi.advanceTimersByTime(90)
    play("Kick", 64, 20, 0.4)
    expect(on).toHaveBeenCalledWith(channel("Kick").id, 60, 0.63)
    expect(off).toHaveBeenCalledWith(channel("Kick").id, 60)
    expect(enabled()).toBe(true)
    expect(registry.stateVersion()).toBeGreaterThan(version)
    // The shared step wins over a piano-only half-step.
    usePianoRollStore.getState().setSnap("step/2")
    const before = undoSteps()
    const send = vi.spyOn(roll.backend, "dispatch")

    await runAction("pianoRoll.dumpPlayedNotes")

    expect(send).toHaveBeenCalledTimes(1)
    expect(send.mock.calls[0][0]).toEqual({
      type: "addNotes",
      pattern: currentPattern().id,
      channel: channel("Kick").id,
      notes: [
        { key: 60, start: 960, length: 960, velocity: 0.63, pan: 0 },
        { key: 64, start: 2160, length: 240, velocity: 0.4, pan: 0 },
      ],
    })
    expect(notesOf("Kick")).toMatchObject([
      { key: 60, start: 960, length: 960, velocity: 0.63 },
      { key: 64, start: 2160, length: 240, velocity: 0.4 },
    ])
    expect(undoSteps()).toBe(before + 1)
    expect(playedNotesFor(channel("Kick").id)).toEqual([])
    expect(enabled()).toBe(false)
    await undo()
    expect(notesOf("Kick")).toEqual([])
    expect(enabled()).toBe(false)
  })

  it("leaves another channel in the log and dumps it only when that channel is open", async () => {
    await dispatch({ type: "addChannel", name: "Lead" })
    play("Lead", 72, 250)
    play("Kick", 60, 125)
    const send = vi.spyOn(roll.backend, "dispatch")

    await runAction("pianoRoll.dumpPlayedNotes")

    expect(send).toHaveBeenCalledTimes(1)
    expect(notesOf("Kick")).toMatchObject([
      { key: 60, start: 960, length: 240 },
    ])
    expect(notesOf("Lead")).toEqual([])
    expect(playedNotesFor(channel("Lead").id)).toHaveLength(1)
    expect(enabled()).toBe(false)
    roll.show("Lead")
    expect(enabled()).toBe(true)
    await runAction("pianoRoll.dumpPlayedNotes")
    expect(notesOf("Lead")).toMatchObject([
      { key: 72, start: 960, length: 480 },
    ])
    expect(useNoteLogStore.getState().notes).toEqual([])
  })

  it("dispatches nothing for an empty log or an unmatched call", async () => {
    const send = vi.spyOn(roll.backend, "dispatch")
    await dumpPlayedNotes()
    auditionOff(channel("Kick").id, 60)
    auditionOn(channel("Kick").id, 64, 0.8)
    expect(enabled()).toBe(false)
    await dumpPlayedNotes()
    await runAction("pianoRoll.dumpPlayedNotes")
    expect(send).not.toHaveBeenCalled()
  })

  it.each(["new", "open"] as const)(
    "clears pairs and held keys on project %s",
    async (change) => {
      const path = await roll.backend.projectSave("/projects/note-log.windfall")
      const id = channel("Kick").id
      play("Kick", 60, 250)
      auditionOn(id, 64, 0.8)

      if (change === "open") await roll.backend.projectOpen(path)
      else await roll.backend.projectNew()
      await settle()

      expect(useNoteLogStore.getState().notes).toEqual([])
      expect(useNoteLogStore.getState().held).toEqual([])
      auditionOff(id, 64)
      expect(playedNotesFor(id)).toEqual([])
      const send = vi.spyOn(roll.backend, "dispatch")
      await dumpPlayedNotes()
      expect(send).not.toHaveBeenCalled()
    }
  )

  it("requires the piano roll and a pattern channel to be open", () => {
    play("Kick", 60, 250)
    expect(registry.get("pianoRoll.dumpPlayedNotes")?.title).toBe(
      "Dump played notes"
    )
    useUiStore.getState().showCenterTab("playlist")
    expect(enabled()).toBe(false)
    useUiStore.getState().showCenterTab("pianoRoll")
    roll.editor.setContext(null)
    expect(enabled()).toBe(false)
  })

  it("expires old pairs and held onsets after 20 seconds, updating command availability", () => {
    play("Kick", 60, 250)
    auditionOn(channel("Kick").id, 64, 0.8)
    vi.advanceTimersByTime(19_750)
    expect(enabled()).toBe(true)
    vi.advanceTimersByTime(1)
    expect(enabled()).toBe(false)
    expect(useNoteLogStore.getState().notes).toEqual([])
    vi.advanceTimersByTime(250)
    auditionOff(channel("Kick").id, 64)
    expect(useNoteLogStore.getState().held).toEqual([])
    expect(playedNotesFor(channel("Kick").id)).toEqual([])
  })

  it("keeps at most 1,024 completed notes", () => {
    for (let index = 0; index < 1_025; index++) play("Kick", index % 128, 1)
    const notes = playedNotesFor(channel("Kick").id)
    expect(notes).toHaveLength(1_024)
    expect(notes[0].key).toBe(1)
    expect(notes.at(-1)?.key).toBe(0)
  })

  it("retains the log after a failed dump", async () => {
    play("Kick", 60, 250)
    vi.spyOn(roll.backend, "dispatch").mockRejectedValueOnce(
      new Error("failed")
    )
    await dumpPlayedNotes()
    expect(playedNotesFor(channel("Kick").id)).toHaveLength(1)
    expect(enabled()).toBe(true)
    expect(notesOf("Kick")).toEqual([])
  })
})
