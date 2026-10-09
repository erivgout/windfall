import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { Backend } from "@/lib/ipc"
import { dispatch, undo } from "@/lib/store/project"
import { startTestApp } from "@/test/harness"

import {
  resetChannelLevels,
  unmuteAllChannels,
  unsoloAllChannels,
} from "./channel-ops"
import { channel, history, project } from "./test-utils"

let stop: () => void
let backend: Backend

beforeEach(async () => {
  ;({ stop, backend } = await startTestApp())
  // The default project's channel volumes are not all at unity.
  await dispatch({
    type: "batch",
    commands: project().channels.map((item) => ({
      type: "updateChannel",
      id: item.id,
      patch: { volume: 1, pan: 0 },
    })),
  })
})

afterEach(() => {
  stop()
  vi.restoreAllMocks()
})

describe("channel rack levels", () => {
  it("unmutes muted channels and omits channels that were not muted", async () => {
    const kick = channel("Kick").id
    await dispatch({
      type: "updateChannel",
      id: kick,
      patch: { muted: true },
    })
    expect(project().channels.some((item) => !item.muted)).toBe(true)
    const before = history().entries.length
    const dispatched = vi.spyOn(backend, "dispatch")

    await unmuteAllChannels()

    expect(dispatched).toHaveBeenCalledTimes(1)
    expect(dispatched.mock.calls[0][0]).toEqual({
      type: "batch",
      label: "Unmute channels",
      commands: [{ type: "updateChannel", id: kick, patch: { muted: false } }],
    })
    expect(channel("Kick").muted).toBe(false)
    expect(history().entries).toHaveLength(before + 1)
    expect(history().entries.at(-1)?.label).toBe("Unmute channels")

    await undo()

    expect(channel("Kick").muted).toBe(true)
  })

  it("dispatches nothing when nobody is muted", async () => {
    expect(project().channels.every((item) => !item.muted)).toBe(true)
    const dispatched = vi.spyOn(backend, "dispatch")

    await unmuteAllChannels()

    expect(dispatched).not.toHaveBeenCalled()
  })

  it("unsolos solo channels without turning any channel on", async () => {
    const kick = channel("Kick").id
    await dispatch({
      type: "updateChannel",
      id: kick,
      patch: { muted: true, solo: true },
    })
    const before = history().entries.length
    const dispatched = vi.spyOn(backend, "dispatch")

    await unsoloAllChannels()

    expect(dispatched).toHaveBeenCalledTimes(1)
    expect(dispatched.mock.calls[0][0]).toEqual({
      type: "batch",
      label: "Unsolo channels",
      commands: [{ type: "updateChannel", id: kick, patch: { solo: false } }],
    })
    expect(project().channels.every((item) => !item.solo)).toBe(true)
    expect(channel("Kick").muted).toBe(true)
    expect(history().entries).toHaveLength(before + 1)
    expect(history().entries.at(-1)?.label).toBe("Unsolo channels")

    await undo()

    expect(channel("Kick").solo).toBe(true)
  })

  it("dispatches nothing when nobody is solo", async () => {
    expect(project().channels.every((item) => !item.solo)).toBe(true)
    const dispatched = vi.spyOn(backend, "dispatch")

    await unsoloAllChannels()

    expect(dispatched).not.toHaveBeenCalled()
  })

  it("resets both changed fields, omitting mute, solo, and channels at unity", async () => {
    const kick = channel("Kick").id
    await dispatch({
      type: "updateChannel",
      id: kick,
      patch: { volume: 0.5, pan: -0.2, muted: true, solo: true },
    })
    expect(
      project().channels.some((item) => item.volume === 1 && item.pan === 0)
    ).toBe(true)
    const original = { ...channel("Kick") }
    const dispatched = vi.spyOn(backend, "dispatch")

    await resetChannelLevels()

    expect(dispatched).toHaveBeenCalledTimes(1)
    expect(dispatched.mock.calls[0][0]).toEqual({
      type: "batch",
      label: "Reset channel levels",
      commands: [
        { type: "updateChannel", id: kick, patch: { volume: 1, pan: 0 } },
      ],
    })
    expect(channel("Kick")).toEqual({ ...original, volume: 1, pan: 0 })
  })

  it("patches only changed fields across channels and undoes in one step", async () => {
    const kick = channel("Kick").id
    const other = project().channels.find((item) => item.id !== kick)!
    await dispatch({
      type: "batch",
      commands: [
        { type: "updateChannel", id: kick, patch: { volume: 0.5 } },
        { type: "updateChannel", id: other.id, patch: { pan: -0.2 } },
      ],
    })
    const before = history().entries.length
    const dispatched = vi.spyOn(backend, "dispatch")

    await resetChannelLevels()

    expect(dispatched).toHaveBeenCalledTimes(1)
    expect(dispatched.mock.calls[0][0]).toEqual({
      type: "batch",
      label: "Reset channel levels",
      commands: [
        { type: "updateChannel", id: kick, patch: { volume: 1 } },
        { type: "updateChannel", id: other.id, patch: { pan: 0 } },
      ],
    })
    expect(channel("Kick").volume).toBe(1)
    expect(channel(other.name).pan).toBe(0)
    expect(history().entries).toHaveLength(before + 1)
    expect(history().entries.at(-1)?.label).toBe("Reset channel levels")

    await undo()

    expect(channel("Kick").volume).toBe(0.5)
    expect(channel("Kick").pan).toBe(0)
    expect(channel(other.name).pan).toBe(-0.2)
    expect(channel(other.name).volume).toBe(1)
  })

  it("dispatches nothing when every channel is already at unity", async () => {
    expect(
      project().channels.every((item) => item.volume === 1 && item.pan === 0)
    ).toBe(true)
    const dispatched = vi.spyOn(backend, "dispatch")

    await resetChannelLevels()

    expect(dispatched).not.toHaveBeenCalled()
  })
})
