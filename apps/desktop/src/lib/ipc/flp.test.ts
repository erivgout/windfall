import { afterEach, expect, it } from "vitest"
import { createMockBackend, type MockBackend } from "./mock"
import { flpFixture } from "@/features/flp-import/fixture"
let backend: MockBackend | undefined
afterEach(() => backend?.dispose())
it("runs real FL bytes through the wasm importer and preserves opaque state across save/reopen", async () => {
  backend = createMockBackend({
    storage: null,
    flpFiles: { "/source.flp": flpFixture() },
  })
  const before = await backend.documentSnapshot()
  const report = await backend.flpPreview("/source.flp", {
    factoryDataDir: null,
    userDataDir: null,
    sampleSearchFolders: [],
  })
  expect(await backend.documentSnapshot()).toEqual(before)
  expect(report.missingSamples).toHaveLength(1)
  expect(report.retainedPlugins).toBe(1)
  const imported = await backend.flpOpen(report.token)
  expect(imported.dirty).toBe(true)
  expect(imported.project.patterns[0]?.lanes[0]?.notes).toHaveLength(1)
  expect(imported.project.retainedPlugins?.[0]?.state).toEqual([0, 255, 17, 90])
  const path = await backend.projectSave("/converted.windfall")
  await backend.projectNew()
  const reloaded = await backend.projectOpen(path)
  expect(reloaded.project.retainedPlugins).toEqual(
    imported.project.retainedPlugins
  )
})
