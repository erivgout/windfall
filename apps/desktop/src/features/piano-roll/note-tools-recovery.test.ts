import { afterEach, beforeEach, expect, it, vi } from "vitest"
import type { DocumentSnapshot } from "@/bindings"
import {
  dispatch,
  loadSnapshot,
  refetchSnapshot,
  useProjectStore,
} from "@/lib/store/project"
import { settle } from "@/test/harness"
import {
  applyNoteTool,
  closeNoteTools,
  openNoteTools,
  useNoteTools,
} from "./note-tools"
import { channel, currentPattern, notesOf, startRoll } from "./test-utils"

let roll: Awaited<ReturnType<typeof startRoll>>
beforeEach(async () => {
  roll = await startRoll()
  await dispatch({ type: "addChannel", name: "Lead" })
  await dispatch({
    type: "addNotes",
    pattern: currentPattern().id,
    channel: channel("Lead").id,
    notes: [{ start: 0, length: 480, key: 60, velocity: 0.7, pan: 0 }],
  })
  roll.show("Lead")
  roll.editor.selectAll()
})
afterEach(() => {
  closeNoteTools()
  roll.stop()
  vi.restoreAllMocks()
})

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

it("selects every chopped piece after deferred revision-gap recovery", async () => {
  const old = await roll.backend.documentSnapshot()
  await roll.backend.dispatch({
    type: "updateSettings",
    patch: { name: "Missed event" },
  })
  // Reconstruct the mirror after an event was missed, using the real backend.
  loadSnapshot(old)
  const recovery = deferred<DocumentSnapshot>()
  const snapshot = vi
    .spyOn(roll.backend, "documentSnapshot")
    .mockReturnValueOnce(recovery.promise)
  openNoteTools("chop")
  const work = applyNoteTool(useNoteTools.getState().request!, {
    type: "chop",
    grid: 240,
  })
  await settle()
  expect(snapshot).toHaveBeenCalledTimes(1)
  const resumed = refetchSnapshot()
  // Fetch the committed backend document without resolving the held UI fetch.
  snapshot.mockRestore()
  recovery.resolve(await roll.backend.documentSnapshot())
  expect(await work).toBe(true)
  await resumed
  expect(notesOf("Lead")).toHaveLength(2)
  expect([...roll.editor.selection].sort()).toEqual(
    notesOf("Lead")
      .map((n) => n.id)
      .sort()
  )
  expect(useProjectStore.getState().project.settings.name).toBe("Missed event")
})

it("drops a tool reply when the project changes during revision-gap recovery", async () => {
  const old = await roll.backend.documentSnapshot()
  await roll.backend.dispatch({
    type: "updateSettings",
    patch: { name: "Missed event" },
  })
  loadSnapshot(old)
  const recovery = deferred<DocumentSnapshot>()
  const original = roll.backend.documentSnapshot.bind(roll.backend)
  vi.spyOn(roll.backend, "documentSnapshot").mockReturnValueOnce(
    recovery.promise
  )
  openNoteTools("chop")
  const work = applyNoteTool(useNoteTools.getState().request!, {
    type: "chop",
    grid: 240,
  })
  await settle()
  const committed = await original()
  await roll.backend.projectNew()
  const replacement = useProjectStore.getState()
  recovery.resolve(committed)
  expect(await work).toBe(false)
  expect(useProjectStore.getState().project).toEqual(replacement.project)
  expect(useProjectStore.getState().history).toEqual(replacement.history)
  expect(roll.editor.selectionCount).toBe(0)
})
