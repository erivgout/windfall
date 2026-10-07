import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { toast } from "sonner"
import { backend as currentBackend, setBackend, type Backend } from "@/lib/ipc"
import { runAction, getAppState } from "@/lib/actions"
import { registry, isEnabled } from "@/lib/actions/registry"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { useRecordingStore } from "@/features/transport/recording-store"
import { startTestApp, settle } from "@/test/harness"
import { openProject, openProjectPath } from "./project"
import {
  saveArchive,
  saveNewVersion,
  cancelArchive,
  useArchiveStore,
} from "./portable"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))
let stop: () => void
let mock: Backend
beforeEach(async () => {
  const app = await startTestApp()
  stop = app.stop
  mock = app.backend
  useArchiveStore.setState(useArchiveStore.getInitialState(), true)
  useRecordingStore.setState(useRecordingStore.getInitialState(), true)
  vi.clearAllMocks()
})
afterEach(() => stop())
function native(overrides: Partial<Backend>): Backend {
  const bridge: Backend = { ...mock, kind: "tauri", ...overrides }
  setBackend(bridge)
  return bridge
}
describe("portable file actions", () => {
  it("explains native capability and never simulates filesystem success", async () => {
    for (const id of ["file.archive", "file.saveNewVersion"]) {
      const action = registry.get(id)!
      expect(isEnabled(action, getAppState())).toBe(false)
      expect(action.whyDisabled?.(getAppState())).toBe("Desktop app required")
    }
    expect(await saveNewVersion()).toBe(false)
    await saveArchive()
    expect(toast.error).toHaveBeenCalledWith("Desktop app required")
    await expect(mock.projectArchiveSave("/song.zip")).rejects.toThrow(
      "desktop"
    )
    await expect(mock.projectSaveNewVersion()).rejects.toThrow("desktop")
    await expect(mock.projectOpen("/song.zip")).rejects.toThrow("desktop")
  })
  it("picker cancellation and failures show no success and preserve document", async () => {
    const saved = vi.fn()
    const version = vi.fn()
    native({
      pickProjectArchivePath: async () => null,
      projectArchiveSave: saved,
      pickProjectSavePath: async () => null,
      projectSaveNewVersion: version,
    })
    const before = useProjectStore.getState()
    await saveArchive()
    expect(await saveNewVersion()).toBe(false)
    expect(saved).not.toHaveBeenCalled()
    expect(version).not.toHaveBeenCalled()
    expect(useProjectStore.getState()).toEqual(before)
    expect(toast.success).not.toHaveBeenCalled()
    native({
      pickProjectArchivePath: async () => {
        throw new Error("Picker unavailable")
      },
    })
    await saveArchive()
    expect(toast.error).toHaveBeenCalledWith(
      "Could not create a portable project archive",
      expect.objectContaining({ description: "Picker unavailable" })
    )
    expect(useArchiveStore.getState().busy).toBe(false)
  })
  it("exports the captured archive without marking clean or changing the current file", async () => {
    await dispatch({ type: "updateSettings", patch: { name: "Archive me" } })
    useProjectStore.setState({ path: "/song.windfall" })
    const before = useProjectStore.getState()
    const exportArchive = vi.fn(async () => "/portable.zip")
    native({
      pickProjectArchivePath: async () => "/portable.zip",
      projectArchiveSave: exportArchive,
    })
    await runAction("file.archive")
    expect(exportArchive).toHaveBeenCalledWith("/portable.zip")
    expect(useProjectStore.getState()).toEqual(before)
    expect(toast.success).toHaveBeenCalledWith(
      "Portable project archive created",
      { description: "portable.zip" }
    )
    expect(useArchiveStore.getState().busy).toBe(false)
  })
  it("shows missing audio errors and supports cancellation while native work is pending", async () => {
    let finish!: (error: Error) => void
    const cancel = vi.fn(async () => {
      finish(new Error("Project archive cancelled."))
    })
    native({
      pickProjectArchivePath: async () => "/portable.zip",
      projectArchiveSave: () =>
        new Promise((_, reject) => {
          finish = reject
        }),
      projectArchiveCancel: cancel,
    })
    const pending = saveArchive()
    await settle()
    expect(useArchiveStore.getState().busy).toBe(true)
    await cancelArchive()
    await pending
    expect(cancel).toHaveBeenCalledOnce()
    expect(useArchiveStore.getState().busy).toBe(false)
    expect(toast.success).not.toHaveBeenCalled()
    native({
      pickProjectArchivePath: async () => "/missing.zip",
      projectArchiveSave: async () => {
        throw new Error("Missing audio: take.wav, edited.wav")
      },
    })
    await saveArchive()
    expect(useArchiveStore.getState().report).toBe(
      "Missing audio: take.wav, edited.wav"
    )
    expect(toast.error).toHaveBeenCalledWith(
      "Could not create a portable project archive",
      expect.objectContaining({
        description: "Missing audio: take.wav, edited.wav",
      })
    )
  })
  it("new versions use a base picker only for unsaved projects and leave dirty state to Rust", async () => {
    const picker = vi.fn(async () => "/song.windfall")
    const save = vi.fn(async () => "/song (001).windfall")
    native({
      pickProjectSavePath: picker,
      projectSaveNewVersion: save,
      documentSnapshot: async () => ({
        ...(await mock.documentSnapshot()),
        path: "/song (001).windfall",
        dirty: true,
      }),
    })
    expect(await saveNewVersion()).toBe(true)
    expect(picker).toHaveBeenCalledOnce()
    expect(save).toHaveBeenCalledWith("/song.windfall")
    expect(useProjectStore.getState().path).toBe("/song (001).windfall")
    expect(useProjectStore.getState().dirty).toBe(true)
    expect(await saveNewVersion()).toBe(true)
    expect(picker).toHaveBeenCalledOnce()
    expect(save).toHaveBeenLastCalledWith(undefined)
    native({
      projectSaveNewVersion: async () => {
        throw new Error("Write failed")
      },
    })
    expect(await saveNewVersion()).toBe(false)
    expect(useProjectStore.getState().path).toBe("/song (001).windfall")
  })
  it("a late numbered save cannot move the UI back to a replaced project", async () => {
    native({
      projectSaveNewVersion: async () => "/old (001).windfall",
      pickProjectSavePath: async () => "/old.windfall",
      documentSnapshot: async () => ({
        ...(await mock.documentSnapshot()),
        path: "/replacement.windfall",
        dirty: true,
      }),
    })
    await saveNewVersion()
    expect(useProjectStore.getState().path).toBe("/replacement.windfall")
    expect(useProjectStore.getState().dirty).toBe(true)
  })
  it("archive open remains cancellable and never installs a cancelled response", async () => {
    let rejectOpen!: (error: Error) => void
    native({
      projectOpen: () =>
        new Promise((_, reject) => {
          rejectOpen = reject
        }),
      projectArchiveCancel: async () => {
        rejectOpen(new Error("Project archive cancelled."))
      },
    })
    const before = useProjectStore.getState()
    const pending = openProjectPath("/Song.zip")
    await settle()
    expect(useArchiveStore.getState()).toMatchObject({
      busy: true,
      cancellable: true,
    })
    await runAction("file.cancelArchive")
    await pending
    expect(useProjectStore.getState()).toEqual(before)
    expect(useArchiveStore.getState().busy).toBe(false)
    expect(toast.error).toHaveBeenCalledWith(
      "Could not open the project",
      expect.objectContaining({ description: "Project archive cancelled." })
    )
  })
  it("ordinary reviewed Open handles archive picker cancellation/errors and recording exclusion", async () => {
    const open = vi.fn(async () => ({
      ...(await mock.documentSnapshot()),
      path: null,
    }))
    native({ pickProjectToOpen: async () => null, projectOpen: open })
    await openProject()
    expect(open).not.toHaveBeenCalled()
    native({ pickProjectToOpen: async () => "/Song.zip", projectOpen: open })
    await openProject()
    expect(open).toHaveBeenCalledWith("/Song.zip")
    expect(useProjectStore.getState().path).toBeNull()
    native({
      projectOpen: async () => {
        throw new Error("Unsafe ZIP member")
      },
    })
    await openProjectPath("/bad.zip")
    expect(toast.error).toHaveBeenCalledWith(
      "Could not open the project",
      expect.objectContaining({ description: "Unsafe ZIP member" })
    )
    useRecordingStore.setState({
      state: { ...useRecordingStore.getState().state, active: true },
    })
    expect(await saveNewVersion()).toBe(false)
    await openProjectPath("/bad.zip")
    expect(toast.error).toHaveBeenCalledWith("Stop or cancel recording first")
    expect(currentBackend.kind).toBe("tauri")
  })
})
