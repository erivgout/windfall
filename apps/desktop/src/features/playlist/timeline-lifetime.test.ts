import { afterEach, beforeEach, expect, it, vi } from "vitest"
import {
  announceProjectReplaced,
  getProjectGeneration,
} from "@/lib/store/replaced"
import { useTransportStore } from "@/lib/store/transport"
import { startPlaylist } from "./test-utils"
import { addClips } from "./ops"
import { usePlaylistStore } from "./store"
import {
  clearTimelineSelection,
  playTimelineSelection,
} from "./timeline-actions"
import {
  applyPlaybackRegion,
  selectTimelineRegion,
  refreshTimelineState,
  useTimelineStore,
} from "./timeline-store"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { settle } from "@/test/harness"

function gate() {
  let release = () => {}
  const promise = new Promise<void>((resolve) => {
    release = resolve
  })
  return { promise, release }
}
let rig: Awaited<ReturnType<typeof startPlaylist>>
let saved: string
beforeEach(async () => {
  rig = await startPlaylist()
  announceProjectReplaced()
  await addClips(
    [
      {
        row: 0,
        start: 0,
        length: 3840,
        offset: 0,
        muted: false,
        content: {
          type: "pattern",
          pattern: useProjectStore.getState().project.patterns[0].id,
        },
      },
    ],
    "Seed"
  )
  saved = await rig.backend.projectSave("/timeline-lifetime")
  await selectTimelineRegion({ start: 17, end: 839 })
})
afterEach(() => rig.stop())

for (const stage of ["set", "seek", "play"] as const) {
  for (const delivery of ["before commit", "after commit"] as const) {
    for (const replacement of ["New", "Open"] as const) {
      it(`refuses an old selection action with ${stage} pending ${delivery} across ${replacement}`, async () => {
        const entered = gate(),
          resume = gate()
        const delayed = async <T>(commit: () => Promise<T>) => {
          if (delivery === "before commit") {
            entered.release()
            await resume.promise
            return commit()
          }
          const reply = await commit()
          entered.release()
          await resume.promise
          return reply
        }
        const set = rig.backend.transportSet.bind(rig.backend)
        const seek = rig.backend.transportSeek.bind(rig.backend)
        const play = rig.backend.transportPlay.bind(rig.backend)
        const setSpy = vi.spyOn(rig.backend, "transportSet")
        const seekSpy = vi.spyOn(rig.backend, "transportSeek")
        const playSpy = vi.spyOn(rig.backend, "transportPlay")
        if (stage === "set")
          setSpy.mockImplementation((...args) => delayed(() => set(...args)))
        if (stage === "seek")
          seekSpy.mockImplementation((...args) => delayed(() => seek(...args)))
        if (stage === "play")
          playSpy.mockImplementation((...args) => delayed(() => play(...args)))
        const generation = getProjectGeneration()
        const action = playTimelineSelection(false)
        await entered.promise
        if (replacement === "New") await rig.backend.projectNew()
        else await rig.backend.projectOpen(saved)
        const replacementState = await rig.backend.transportState()
        expect(getProjectGeneration()).not.toBe(generation)
        resume.release()
        await action
        expect(await rig.backend.transportState()).toEqual(replacementState)
        expect(useTransportStore.getState()).toMatchObject(replacementState)
        expect(usePlaylistStore.getState().cursorTick).toBe(0)
        expect((await rig.backend.timelineState()).region).toBeNull()
        expect(useTimelineStore.getState().active).toBe(false)
        if (stage === "set") expect(seekSpy).not.toHaveBeenCalled()
        if (stage !== "play") expect(playSpy).not.toHaveBeenCalled()
      })
    }
  }
}

for (const delivery of ["before commit", "after commit"] as const) {
  for (const next of [null, { start: 91, end: 210 }]) {
    it(`reconciles a pending first arm ${delivery} when selection becomes ${JSON.stringify(next)}`, async () => {
      const entered = gate(),
        resume = gate()
      const publish = rig.backend.timelineRegion.bind(rig.backend)
      vi.spyOn(rig.backend, "timelineRegion").mockImplementationOnce(
        async (...args) => {
          if (delivery === "before commit") {
            entered.release()
            await resume.promise
            return publish(...args)
          }
          const reply = await publish(...args)
          entered.release()
          await resume.promise
          return reply
        }
      )
      const pending = applyPlaybackRegion({ start: 17, end: 839 })
      await entered.promise
      await selectTimelineRegion(next)
      resume.release()
      expect(await pending).toBe(false)
      expect((await rig.backend.timelineState()).region).toEqual(next)
      expect(useTimelineStore.getState()).toMatchObject({
        selection: next,
        active: next !== null,
      })
    })
  }
}

it("refuses a late old arm and clear after a newer rearm has committed", async () => {
  const armEntered = gate(),
    armResume = gate(),
    clearEntered = gate(),
    clearResume = gate()
  const publish = rig.backend.timelineRegion.bind(rig.backend)
  const publication = vi
    .spyOn(rig.backend, "timelineRegion")
    .mockImplementationOnce(async (...args) => {
      armEntered.release()
      await armResume.promise
      return publish(...args)
    })
    .mockImplementationOnce(async (...args) => {
      clearEntered.release()
      await clearResume.promise
      return publish(...args)
    })
  const old = applyPlaybackRegion({ start: 17, end: 839 })
  await armEntered.promise
  const clear = selectTimelineRegion(null)
  await settle()
  expect(publication).toHaveBeenCalledTimes(2)
  // Starting the rearm does not depend on the old arm/clear reply arriving.
  await selectTimelineRegion({ start: 91, end: 210 })
  const rearm = applyPlaybackRegion({ start: 91, end: 210 })
  await clearEntered.promise
  expect(await rearm).toBe(true)
  clearResume.release()
  await clear
  armResume.release()
  expect(await old).toBe(false)
  expect((await rig.backend.timelineState()).region).toEqual({
    start: 91,
    end: 210,
  })
  expect(useTimelineStore.getState()).toMatchObject({
    selection: { start: 91, end: 210 },
    active: true,
  })
})

for (const replacement of ["New", "Open"] as const) {
  for (const stage of [
    "source query",
    "region commit",
    "region reply",
  ] as const) {
    it(`cancels a first pending ${stage} across ${replacement} and permits a fresh arm`, async () => {
      const entered = gate(),
        resume = gate()
      if (stage === "source query") {
        const read = rig.backend.timelineState.bind(rig.backend)
        vi.spyOn(rig.backend, "timelineState").mockImplementationOnce(
          async () => {
            const result = await read()
            entered.release()
            await resume.promise
            return result
          }
        )
      } else {
        const publish = rig.backend.timelineRegion.bind(rig.backend)
        vi.spyOn(rig.backend, "timelineRegion").mockImplementationOnce(
          async (...args) => {
            const result =
              stage === "region reply" ? await publish(...args) : null
            entered.release()
            await resume.promise
            return result ?? publish(...args)
          }
        )
      }
      const old = applyPlaybackRegion({ start: 17, end: 839 })
      await entered.promise
      if (replacement === "New") await rig.backend.projectNew()
      else await rig.backend.projectOpen(saved)
      await selectTimelineRegion({ start: 91, end: 210 })
      expect(await applyPlaybackRegion({ start: 91, end: 210 })).toBe(true)
      resume.release()
      expect(await old).toBe(false)
      expect((await rig.backend.timelineState()).region).toEqual({
        start: 91,
        end: 210,
      })
      expect(useTimelineStore.getState()).toMatchObject({
        selection: { start: 91, end: 210 },
        active: true,
      })
    })
  }
}

for (const stage of ["set", "seek", "play"] as const) {
  it(`clearing selection invalidates a ${stage} awaiting native commit`, async () => {
    const entered = gate(),
      resume = gate()
    const delayed = async <T>(commit: () => Promise<T>) => {
      entered.release()
      await resume.promise
      return commit()
    }
    const set = rig.backend.transportSet.bind(rig.backend)
    const seek = rig.backend.transportSeek.bind(rig.backend)
    const play = rig.backend.transportPlay.bind(rig.backend)
    if (stage === "set")
      vi.spyOn(rig.backend, "transportSet").mockImplementation((...args) =>
        delayed(() => set(...args))
      )
    if (stage === "seek")
      vi.spyOn(rig.backend, "transportSeek").mockImplementation((...args) =>
        delayed(() => seek(...args))
      )
    if (stage === "play")
      vi.spyOn(rig.backend, "transportPlay").mockImplementation((...args) =>
        delayed(() => play(...args))
      )
    const action = playTimelineSelection(false)
    await entered.promise
    await selectTimelineRegion(null)
    const cancelled = await rig.backend.transportState()
    const cursor = usePlaylistStore.getState().cursorTick
    resume.release()
    await action
    expect(await rig.backend.transportState()).toEqual(cancelled)
    expect(usePlaylistStore.getState().cursorTick).toBe(cursor)
    expect((await rig.backend.timelineState()).region).toBeNull()
    expect(useTimelineStore.getState()).toMatchObject({
      selection: null,
      active: false,
    })
  })
}

it("allocates from the canonical watermark after a UI reset and refuses inexact source numbers", async () => {
  const source = await rig.backend.timelineState()
  await rig.backend.timelineRegion(
    null,
    source.generation,
    source.revision,
    100_000
  )
  announceProjectReplaced()
  const publication = vi.spyOn(rig.backend, "timelineRegion")
  await selectTimelineRegion({ start: 91, end: 210 })
  expect(await applyPlaybackRegion({ start: 91, end: 210 })).toBe(true)
  expect(publication.mock.calls[0][3]).toBeGreaterThan(100_000)
  const canonical = await rig.backend.timelineState()
  vi.spyOn(rig.backend, "timelineState").mockResolvedValueOnce({
    ...canonical,
    request: Number.MAX_SAFE_INTEGER + 1,
  })
  expect(await applyPlaybackRegion({ start: 17, end: 839 })).toBe(false)
  expect(publication).toHaveBeenCalledOnce()
  expect((await rig.backend.timelineState()).region).toEqual({
    start: 91,
    end: 210,
  })
  expect(useTimelineStore.getState().error).toMatch(/invalid timeline request/)
  expect(await applyPlaybackRegion({ start: 17, end: 839 })).toBe(true)
})

it("clears an arm committed before an edit after its stale reply was discarded", async () => {
  const entered = gate(),
    resume = gate()
  const publish = rig.backend.timelineRegion.bind(rig.backend)
  vi.spyOn(rig.backend, "timelineRegion").mockImplementationOnce(
    async (...args) => {
      const reply = await publish(...args)
      entered.release()
      await resume.promise
      return reply
    }
  )
  const arm = applyPlaybackRegion({ start: 17, end: 839 })
  await entered.promise
  await dispatch({
    type: "addMeterChange",
    tick: 4001,
    signature: { numerator: 7, denominator: 8 },
  })
  resume.release()
  expect(await arm).toBe(false)
  expect(useTimelineStore.getState()).toMatchObject({
    active: true,
    selection: { start: 17, end: 839 },
  })
  await selectTimelineRegion(null)
  expect((await rig.backend.timelineState()).region).toBeNull()
  expect(useTimelineStore.getState()).toMatchObject({
    selection: null,
    active: false,
  })
})

for (const ordinary of ["Play", "Seek", "Set", "Stop"] as const) {
  it(`does not stop ordinary ${ordinary} that supersedes a cancelled pending selection Play`, async () => {
    const entered = gate(),
      resume = gate(),
      queryEntered = gate(),
      queryResume = gate()
    const play = rig.backend.transportPlay.bind(rig.backend)
    vi.spyOn(rig.backend, "transportPlay").mockImplementationOnce(
      async (...args) => {
        entered.release()
        await resume.promise
        return play(...args)
      }
    )
    const action = playTimelineSelection(false)
    await entered.promise
    const read = rig.backend.timelineState.bind(rig.backend)
    vi.spyOn(rig.backend, "timelineState").mockImplementationOnce(async () => {
      queryEntered.release()
      await queryResume.promise
      return read()
    })
    const clear = clearTimelineSelection()
    await queryEntered.promise
    resume.release()
    await action
    if (ordinary === "Play") await rig.backend.transportPlay()
    if (ordinary === "Seek") await rig.backend.transportSeek(99)
    if (ordinary === "Set") await rig.backend.transportSet({ loopSong: true })
    if (ordinary === "Stop") {
      await rig.backend.transportStop()
      await rig.backend.transportPlay()
    }
    const normal = await rig.backend.transportState()
    expect(normal.playing).toBe(true)
    queryResume.release()
    await clear
    expect(await rig.backend.transportState()).toEqual(normal)
    expect((await rig.backend.timelineState()).region).toBeNull()
    // A fresh selection Play still works after the cancelled action settles.
    await selectTimelineRegion({ start: 91, end: 210 })
    await playTimelineSelection(true)
    expect(await rig.backend.transportState()).toMatchObject({
      playing: true,
      loopSong: true,
    })
    expect((await rig.backend.timelineState()).region).toEqual({
      start: 91,
      end: 210,
    })
  })
}

it("clears a settled selection without stopping normal playback", async () => {
  await playTimelineSelection(false)
  expect((await rig.backend.transportState()).playing).toBe(true)
  await clearTimelineSelection()
  expect((await rig.backend.timelineState()).region).toBeNull()
  expect((await rig.backend.transportState()).playing).toBe(true)
})

it("ignores a delayed hydration snapshot after a newer Clear has committed", async () => {
  expect(await applyPlaybackRegion({ start: 17, end: 839 })).toBe(true)
  const entered = gate(),
    resume = gate()
  const read = rig.backend.timelineState.bind(rig.backend)
  vi.spyOn(rig.backend, "timelineState").mockImplementationOnce(async () => {
    const state = await read()
    entered.release()
    await resume.promise
    return state
  })
  const hydration = refreshTimelineState()
  await entered.promise
  await clearTimelineSelection()
  resume.release()
  await hydration
  expect((await rig.backend.timelineState()).region).toBeNull()
  expect(useTimelineStore.getState()).toMatchObject({
    selection: null,
    active: false,
  })
})

it("reconciles a Play committed before its reply and refuses late cancellation after a newer Play", async () => {
  const entered = gate(),
    resume = gate()
  const play = rig.backend.transportPlay.bind(rig.backend)
  vi.spyOn(rig.backend, "transportPlay").mockImplementationOnce(
    async (...args) => {
      const state = await play(...args)
      entered.release()
      await resume.promise
      return state
    }
  )
  const old = playTimelineSelection(false)
  await entered.promise
  expect((await rig.backend.transportState()).playing).toBe(true)
  await clearTimelineSelection()
  expect((await rig.backend.transportState()).playing).toBe(false)
  await selectTimelineRegion({ start: 91, end: 210 })
  await playTimelineSelection(true)
  const current = await rig.backend.timelineState()
  resume.release()
  await old
  expect(await rig.backend.timelineState()).toEqual(current)
  expect((await rig.backend.transportState()).playing).toBe(true)
})

it("reconciles a cancelled selection Play that commits while Clear awaits its source query", async () => {
  const playEntered = gate(),
    playResume = gate(),
    queryEntered = gate(),
    queryResume = gate()
  const play = rig.backend.transportPlay.bind(rig.backend)
  vi.spyOn(rig.backend, "transportPlay").mockImplementationOnce(
    async (...args) => {
      playEntered.release()
      await playResume.promise
      return play(...args)
    }
  )
  const action = playTimelineSelection(false)
  await playEntered.promise
  const read = rig.backend.timelineState.bind(rig.backend)
  vi.spyOn(rig.backend, "timelineState").mockImplementationOnce(async () => {
    queryEntered.release()
    await queryResume.promise
    return read()
  })
  const clear = clearTimelineSelection()
  await queryEntered.promise
  playResume.release()
  await action
  expect((await rig.backend.transportState()).playing).toBe(true)
  queryResume.release()
  await clear
  expect((await rig.backend.transportState()).playing).toBe(false)
  expect((await rig.backend.timelineState()).region).toBeNull()
  expect(useTimelineStore.getState()).toMatchObject({
    selection: null,
    active: false,
  })
})

it("retains a refused active range visibly and lets the user retry Clear", async () => {
  expect(await applyPlaybackRegion({ start: 17, end: 839 })).toBe(true)
  vi.spyOn(rig.backend, "timelineRegion").mockRejectedValueOnce(
    new Error("Stop recording before changing the timeline region.")
  )
  await clearTimelineSelection()
  expect((await rig.backend.timelineState()).region).toEqual({
    start: 17,
    end: 839,
  })
  expect(useTimelineStore.getState()).toMatchObject({
    selection: { start: 17, end: 839 },
    active: true,
    error: "Stop recording before changing the timeline region.",
  })
  await clearTimelineSelection()
  expect((await rig.backend.timelineState()).region).toBeNull()
  expect(useTimelineStore.getState()).toMatchObject({
    selection: null,
    active: false,
    error: null,
  })
})

for (const stage of ["source query", "native commit"] as const) {
  it(`retains the canonical active range and a retry error when an edit cancels Clear during ${stage}`, async () => {
    expect(await applyPlaybackRegion({ start: 17, end: 839 })).toBe(true)
    const entered = gate(),
      resume = gate()
    if (stage === "source query") {
      const read = rig.backend.timelineState.bind(rig.backend)
      vi.spyOn(rig.backend, "timelineState").mockImplementationOnce(
        async () => {
          const state = await read()
          entered.release()
          await resume.promise
          return state
        }
      )
    } else {
      const publish = rig.backend.timelineRegion.bind(rig.backend)
      vi.spyOn(rig.backend, "timelineRegion").mockImplementationOnce(
        async (...args) => {
          entered.release()
          await resume.promise
          return publish(...args)
        }
      )
    }
    const clear = clearTimelineSelection()
    await entered.promise
    await dispatch({
      type: "addMeterChange",
      tick: 4001,
      signature: { numerator: 7, denominator: 8 },
    })
    resume.release()
    await clear
    expect((await rig.backend.timelineState()).region).toEqual({
      start: 17,
      end: 839,
    })
    expect(useTimelineStore.getState()).toMatchObject({
      selection: { start: 17, end: 839 },
      active: true,
    })
    expect(useTimelineStore.getState().error).toMatch(/changed|retry/i)
    await clearTimelineSelection()
    expect((await rig.backend.timelineState()).region).toBeNull()
    expect(useTimelineStore.getState()).toMatchObject({
      selection: null,
      active: false,
      error: null,
    })
  })
}
