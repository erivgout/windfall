import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { Backend } from "@/lib/ipc"
import { dispatch, undo } from "@/lib/store/project"
import { startTestApp } from "@/test/harness"

import { resetLevels } from "./operations"
import { history, trackNamed, tracks } from "./test-utils"

let stop: () => void
let backend: Backend

beforeEach(async () => {
  ;({ stop, backend } = await startTestApp())
})

afterEach(() => {
  stop()
  vi.restoreAllMocks()
})

describe("reset mixer levels", () => {
  it("patches both changed fields and omits tracks already at unity", async () => {
    const kick = trackNamed("Kick").id
    await dispatch({
      type: "updateMixerTrack",
      id: kick,
      patch: { volume: 0.5, pan: -0.2 },
    })
    const dispatched = vi.spyOn(backend, "dispatch")

    await resetLevels()

    expect(dispatched).toHaveBeenCalledTimes(1)
    expect(dispatched.mock.calls[0][0]).toEqual({
      type: "batch",
      label: "Reset mixer levels",
      commands: [
        { type: "updateMixerTrack", id: kick, patch: { volume: 1, pan: 0 } },
      ],
    })
    expect(trackNamed("Kick").volume).toBe(1)
    expect(trackNamed("Kick").pan).toBe(0)
  })

  it("dispatches nothing when every track is already at unity", async () => {
    expect(
      tracks().every((track) => track.volume === 1 && track.pan === 0)
    ).toBe(true)
    const dispatched = vi.spyOn(backend, "dispatch")

    await resetLevels()

    expect(dispatched).not.toHaveBeenCalled()
  })

  it("leaves mute and solo out of the changed track's patch", async () => {
    const kick = trackNamed("Kick").id
    await dispatch({
      type: "updateMixerTrack",
      id: kick,
      patch: { volume: 0.5, pan: -0.2, muted: true, solo: true },
    })
    const dispatched = vi.spyOn(backend, "dispatch")

    await resetLevels()

    expect(dispatched).toHaveBeenCalledTimes(1)
    expect(dispatched.mock.calls[0][0]).toEqual({
      type: "batch",
      label: "Reset mixer levels",
      commands: [
        { type: "updateMixerTrack", id: kick, patch: { volume: 1, pan: 0 } },
      ],
    })
    expect(trackNamed("Kick").muted).toBe(true)
    expect(trackNamed("Kick").solo).toBe(true)
  })

  it("includes the master, patches only changed fields, and undoes in one step", async () => {
    const master = trackNamed("Master").id
    const kick = trackNamed("Kick").id
    await dispatch({
      type: "batch",
      commands: [
        { type: "updateMixerTrack", id: master, patch: { volume: 0.5 } },
        { type: "updateMixerTrack", id: kick, patch: { pan: -0.2 } },
      ],
    })
    const before = history().entries.length
    const dispatched = vi.spyOn(backend, "dispatch")

    await resetLevels()

    expect(dispatched).toHaveBeenCalledTimes(1)
    expect(dispatched.mock.calls[0][0]).toEqual({
      type: "batch",
      label: "Reset mixer levels",
      commands: [
        { type: "updateMixerTrack", id: master, patch: { volume: 1 } },
        { type: "updateMixerTrack", id: kick, patch: { pan: 0 } },
      ],
    })
    expect(trackNamed("Master").volume).toBe(1)
    expect(trackNamed("Kick").pan).toBe(0)
    expect(history().entries).toHaveLength(before + 1)
    expect(history().entries.at(-1)?.label).toBe("Reset mixer levels")

    await undo()

    expect(trackNamed("Master").volume).toBe(0.5)
    expect(trackNamed("Kick").pan).toBe(-0.2)
    expect(trackNamed("Master").pan).toBe(0)
    expect(trackNamed("Kick").volume).toBe(1)
  })
})
