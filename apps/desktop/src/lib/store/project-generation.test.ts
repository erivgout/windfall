import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { startTestApp } from "@/test/harness"
import type { MockBackend } from "@/lib/ipc/mock"
import {
  dispatch,
  undo,
  redo,
  historyJump,
  refetchSnapshot,
  useProjectStore,
} from "./project"

let backend: MockBackend
let stop: () => void
beforeEach(async () => {
  ;({ backend, stop } = await startTestApp())
})
afterEach(() => {
  stop()
  vi.restoreAllMocks()
})
function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

describe("document identity across asynchronous replies", () => {
  it("never rolls back a newer patch while a snapshot is in flight", async () => {
    const old = await backend.documentSnapshot()
    const reply = deferred<typeof old>()
    const snapshot = vi
      .spyOn(backend, "documentSnapshot")
      .mockReturnValueOnce(reply.promise)
    const work = refetchSnapshot()
    await backend.dispatch({
      type: "updateSettings",
      patch: { name: "Latest" },
    })
    const before = useProjectStore.getState()
    const revisions: number[] = []
    const unsubscribe = useProjectStore.subscribe((state) =>
      revisions.push(state.revision)
    )
    try {
      reply.resolve(old)
      await work
      expect(snapshot).toHaveBeenCalledTimes(2)
      expect(revisions).not.toContain(old.revision)
      expect(useProjectStore.getState().project).toEqual(before.project)
      expect(useProjectStore.getState().history).toEqual(before.history)
    } finally {
      unsubscribe()
    }
  })
  it("ignores old edit replies when the replacement reuses their revision", async () => {
    const result = await backend.dispatch({
      type: "updateSettings",
      patch: { name: "Old project" },
    })
    await backend.projectNew()
    const reply = deferred<typeof result>()
    vi.spyOn(backend, "dispatch").mockReturnValueOnce(reply.promise)
    const work = dispatch({
      type: "updateSettings",
      patch: { name: "Pending" },
    })
    await backend.projectNew()
    const before = useProjectStore.getState()
    expect(before.revision).toBe(0)
    expect(result.patch.revision).toBe(1)
    reply.resolve(result)
    expect(await work).toBeNull()
    expect(useProjectStore.getState()).toBe(before)
  })
  it.each(["undo", "redo", "jump"] as const)(
    "ignores old %s replies",
    async (operation) => {
      const result = await backend.dispatch({
        type: "updateSettings",
        patch: { name: "Old" },
      })
      await backend.projectNew()
      const reply = deferred<typeof result.patch>()
      let work: Promise<void>
      if (operation === "jump") {
        vi.spyOn(backend, "historyJump").mockReturnValueOnce(reply.promise)
        work = historyJump(0)
      } else {
        vi.spyOn(backend, operation).mockReturnValueOnce(reply.promise)
        work = operation === "undo" ? undo() : redo()
      }
      await backend.projectNew()
      const before = useProjectStore.getState()
      reply.resolve(result.patch)
      await work
      expect(useProjectStore.getState()).toBe(before)
    }
  )
  it("discards a stale snapshot and refetches the replacement at the same revision", async () => {
    await backend.dispatch({ type: "updateSettings", patch: { name: "Old" } })
    const old = await backend.documentSnapshot()
    const reply = deferred<typeof old>()
    const snapshot = vi
      .spyOn(backend, "documentSnapshot")
      .mockReturnValueOnce(reply.promise)
    const work = refetchSnapshot()
    await backend.projectNew()
    await backend.dispatch({
      type: "updateSettings",
      patch: { name: "Replacement" },
    })
    const before = useProjectStore.getState()
    expect(old.revision).toBe(before.revision)
    reply.resolve(old)
    await work
    expect(snapshot).toHaveBeenCalledTimes(2)
    expect(useProjectStore.getState().project).toEqual(before.project)
    expect(useProjectStore.getState().history).toEqual(before.history)
    expect(useProjectStore.getState().revision).toBe(before.revision)
  })
})
