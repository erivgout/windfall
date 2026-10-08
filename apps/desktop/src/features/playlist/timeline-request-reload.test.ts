import { expect, it, vi } from "vitest"
import { startPlaylist } from "./test-utils"

it("rebases actual reloaded UI functions at the last two exact native request numbers without wrap", async () => {
  const rig = await startPlaylist()
  const original = await rig.backend.documentSnapshot()
  const source = await rig.backend.timelineState()
  const max = Number.MAX_SAFE_INTEGER
  await rig.backend.timelineRegion(
    null,
    source.generation,
    source.revision,
    max - 2
  )
  const publish = vi.spyOn(rig.backend, "timelineRegion")
  let disconnect = () => {}
  const reload = async () => {
    disconnect()
    // A window reload creates new module-local request/UI state but keeps
    // the canonical native session and document alive.
    vi.resetModules()
    const { setBackend } = await import("@/lib/ipc")
    setBackend(rig.backend)
    const { connectStores } = await import("@/lib/store/connect")
    const { settle } = await import("@/test/harness")
    const { useProjectStore } = await import("@/lib/store/project")
    const timeline = await import("./timeline-store")
    disconnect = connectStores()
    await settle()
    expect(useProjectStore.getState().revision).toBe(original.revision)
    return timeline
  }
  try {
    let timeline = await reload()
    expect(await timeline.applyPlaybackRegion({ start: 17, end: 839 })).toBe(
      true
    )
    expect(publish.mock.calls[0].slice(1)).toEqual([
      source.generation,
      source.revision,
      max - 1,
    ])
    timeline = await reload()
    expect(await timeline.applyPlaybackRegion(null)).toBe(true)
    expect(publish.mock.calls[1].slice(1)).toEqual([
      source.generation,
      source.revision,
      max,
    ])
    expect(await timeline.applyPlaybackRegion({ start: 91, end: 210 })).toBe(
      false
    )
    expect(publish).toHaveBeenCalledTimes(2)
    expect((await rig.backend.timelineState()).request).toBe(max)
    expect((await rig.backend.timelineState()).region).toBeNull()
    expect(timeline.useTimelineStore.getState().error).toMatch(/request limit/)
    // Reloading the same exhausted session must refuse, never reuse zero.
    timeline = await reload()
    expect(await timeline.applyPlaybackRegion({ start: 91, end: 210 })).toBe(
      false
    )
    expect(publish).toHaveBeenCalledTimes(2)
    const after = await rig.backend.documentSnapshot()
    expect(after.project).toEqual(original.project)
    expect(after.history).toEqual(original.history)
    expect(after.revision).toBe(original.revision)
    expect(after.dirty).toBe(original.dirty)
  } finally {
    disconnect()
    rig.stop()
  }
})
