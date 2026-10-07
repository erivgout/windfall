import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import type { DocumentSnapshot } from "@/bindings"
import { useProjectStore } from "@/lib/store/project"
import { startTestApp } from "@/test/harness"
import { newProject, openProject, openProjectPath } from "./project"

let app: Awaited<ReturnType<typeof startTestApp>>
beforeEach(async () => {
  app = await startTestApp()
})
afterEach(() => {
  app.stop()
  vi.restoreAllMocks()
})
function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

describe("replacement events preceding New/Open replies", () => {
  it.each(["new", "open", "path"] as const)(
    "%s never rolls back edits made after the loaded event",
    async (operation) => {
      await checkDelayedReply(operation, false)
    }
  )
  it.each(["new", "open", "path"] as const)(
    "%s never replaces a later document with reused revisions",
    async (operation) => {
      await checkDelayedReply(operation, true)
    }
  )
})

async function checkDelayedReply(
  operation: "new" | "open" | "path",
  replaceAgain: boolean
) {
  const backend = app.backend
  const path = "/saved/old.windfall"
  await backend.projectSave(path)
  const create = backend.projectNew.bind(backend)
  const open = backend.projectOpen.bind(backend)
  const reply = deferred<DocumentSnapshot>()
  const published = deferred<DocumentSnapshot>()
  if (operation === "new") {
    vi.spyOn(backend, "projectNew").mockImplementationOnce(async () => {
      published.resolve(await create())
      return reply.promise
    })
  } else {
    vi.spyOn(backend, "projectOpen").mockImplementationOnce(async (path) => {
      published.resolve(await open(path))
      return reply.promise
    })
    vi.spyOn(backend, "pickProjectToOpen").mockResolvedValue(path)
  }
  const work =
    operation === "new"
      ? newProject()
      : operation === "open"
        ? openProject()
        : openProjectPath(path)
  const stale = await published.promise
  if (replaceAgain) {
    await create()
    expect(useProjectStore.getState().revision).toBe(stale.revision)
  }
  await backend.dispatch({
    type: "updateSettings",
    patch: { name: "Keep this edit", tempoBpm: 91 },
  })
  const current = useProjectStore.getState()
  reply.resolve(stale)
  await work
  expect(useProjectStore.getState()).toBe(current)
}
