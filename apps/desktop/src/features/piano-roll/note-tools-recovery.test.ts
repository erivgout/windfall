import { afterEach, beforeEach, expect, it, vi } from "vitest"
import type { DocumentSnapshot, NoteTransform } from "@/bindings"
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
  let reject!: (reason: unknown) => void
  const promise = new Promise<T>((done, fail) => {
    resolve = done
    reject = fail
  })
  return { promise, resolve, reject }
}

const RHYTHM_RESULTS: { transform: NoteTransform; count: number }[] = [
  { transform: { type: "chop", grid: 240 }, count: 2 },
  {
    transform: {
      type: "chopPattern",
      origin: 0,
      period: 480,
      steps: [
        { tick: 0, gate: 1, velocity: 1 },
        { tick: 120, gate: 0.5, velocity: 1 },
      ],
    },
    count: 2,
  },
  {
    transform: {
      type: "arpeggiate",
      rate: 120,
      gate: 1,
      octaves: 1,
      repetitions: 0,
      direction: "ascending",
    },
    count: 4,
  },
  {
    transform: { type: "flam", interval: 30, velocity: 0.5, position: "after" },
    count: 2,
  },
  {
    transform: {
      type: "rhythmReshape",
      origin: 0,
      step: 240,
      period: 2,
      phase: 0,
      offset: 240,
      mode: "add",
    },
    count: 2,
  },
]

it.each(RHYTHM_RESULTS)(
  "selects every $transform.type result after deferred revision-gap recovery",
  async ({ transform, count }) => {
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
    openNoteTools(transform.type)
    const work = applyNoteTool(useNoteTools.getState().request!, transform)
    await settle()
    expect(snapshot).toHaveBeenCalledTimes(1)
    const resumed = refetchSnapshot()
    // Fetch the committed backend document without resolving the held UI fetch.
    snapshot.mockRestore()
    recovery.resolve(await roll.backend.documentSnapshot())
    expect(await work).toBe(true)
    await resumed
    expect(notesOf("Lead")).toHaveLength(count)
    expect([...roll.editor.selection].sort()).toEqual(
      notesOf("Lead")
        .map((n) => n.id)
        .sort()
    )
    expect(useProjectStore.getState().project.settings.name).toBe(
      "Missed event"
    )
  }
)

it.each(RHYTHM_RESULTS)(
  "drops a $transform.type reply when the project changes during revision-gap recovery",
  async ({ transform }) => {
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
    openNoteTools(transform.type)
    let completed: boolean | undefined
    const work = applyNoteTool(
      useNoteTools.getState().request!,
      transform
    ).then((result) => {
      completed = result
      return result
    })
    await settle()
    const committed = await original()
    const resumed = refetchSnapshot()
    await roll.backend.projectNew()
    const replacement = useProjectStore.getState()
    await settle()
    try {
      // Replacement ends the wait even while the old fetch remains pending.
      expect(completed).toBe(false)
      expect(useProjectStore.getState().project).toEqual(replacement.project)
      expect(useProjectStore.getState().history).toEqual(replacement.history)
      expect(roll.editor.selectionCount).toBe(0)
    } finally {
      recovery.resolve(committed)
      await resumed
      await work
    }
    expect(useProjectStore.getState().project).toEqual(replacement.project)
  }
)

it("ends a failed recovery without following IDs and permits a later retry", async () => {
  const original = roll.backend.documentSnapshot.bind(roll.backend)
  const old = await original()
  await roll.backend.dispatch({
    type: "updateSettings",
    patch: { name: "Missed event" },
  })
  loadSnapshot(old)
  const recovery = deferred<DocumentSnapshot>()
  vi.spyOn(roll.backend, "documentSnapshot").mockReturnValueOnce(
    recovery.promise
  )
  openNoteTools("chop")
  const work = applyNoteTool(useNoteTools.getState().request!, {
    type: "chop",
    grid: 240,
  })
  await settle()
  recovery.reject(new Error("Snapshot unavailable"))
  expect(await work).toBe(false)
  expect(roll.editor.busy).toBe(false)
  expect(notesOf("Lead")).toHaveLength(1)
  await refetchSnapshot()
  expect(notesOf("Lead")).toHaveLength(2)
  expect(useProjectStore.getState().project.settings.name).toBe("Missed event")
  expect(roll.editor.selectionCount).toBe(1)
})

it("finishes Chop when its revision is mirrored while later recovery continues", async () => {
  const original = roll.backend.documentSnapshot.bind(roll.backend)
  const old = await original()
  await roll.backend.dispatch({
    type: "updateSettings",
    patch: { name: "Missed event" },
  })
  loadSnapshot(old)
  const first = deferred<DocumentSnapshot>()
  const later = deferred<DocumentSnapshot>()
  const snapshot = vi
    .spyOn(roll.backend, "documentSnapshot")
    .mockReturnValueOnce(first.promise)
    .mockReturnValueOnce(later.promise)
  openNoteTools("chop")
  let completed: boolean | undefined
  const work = applyNoteTool(useNoteTools.getState().request!, {
    type: "chop",
    grid: 240,
  }).then((result) => {
    completed = result
    return result
  })
  await settle()
  const chopped = await original()
  await roll.backend.dispatch({
    type: "updateSettings",
    patch: { name: "Later edit" },
  })
  const recovery = refetchSnapshot()
  first.resolve(chopped)
  await settle()
  try {
    expect(snapshot).toHaveBeenCalledTimes(2)
    expect(useProjectStore.getState().revision).toBe(chopped.revision)
    expect(completed).toBe(true)
    expect([...roll.editor.selection].sort()).toEqual(
      notesOf("Lead")
        .map((n) => n.id)
        .sort()
    )
  } finally {
    // The later, unrelated recovery is independent but still must finish.
    later.resolve(await original())
    await recovery
    await work
  }
  expect(useProjectStore.getState().project.settings.name).toBe("Later edit")
})
