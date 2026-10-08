import { act, fireEvent, render, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, describe, expect, it, vi } from "vitest"

import type { BrowserRoot, LibraryFileToken, LibraryResults } from "@/bindings"
import { newProject, openProjectPath } from "@/lib/flows/project"
import { useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { SAMPLE_DRAG_TYPE } from "@/lib/dnd"

import BrowserPanel from "."
import {
  addFolder,
  addToPlaylist,
  addToRack,
  removeRoot,
  replaceChannelSample,
} from "./commands"
import { refreshLibrary, useLibraryStore } from "./library-store"
import { applyRoots, loadRoots, useBrowserStore } from "./store"
import { deferred, findItem, item, startBrowserTest } from "./testing"

let stop = () => {}
afterEach(() => stop())
const path = "/factory/Drums/Kicks/Kick 02.wav"
const destinations = [
  ["rack", addToRack],
  ["playlist", addToPlaylist],
  ["replacement", replaceChannelSample],
] as const

describe("browser import document identity", () => {
  it("guards the playlist's mixer routing lookup before sending an import", async () => {
    const roots = deferred<BrowserRoot[]>()
    const started = deferred<void>()
    const app = await startBrowserTest(() => ({
      browserRoots: () => {
        started.resolve()
        return roots.promise
      },
    }))
    stop = app.stop
    await app.backend.addAudioClipFromFile(path, { start: 0 })
    await app.backend.projectSave("/saved.windfall")
    const imported = vi.spyOn(app.backend, "addAudioClipFromFile")
    const pending = addToPlaylist(path)
    await started.promise
    await newProject()
    const before = structuredClone(useProjectStore.getState().project)
    roots.resolve([{ path: "/factory", kind: "factory", name: "Factory" }])
    await pending
    expect(imported).not.toHaveBeenCalled()
    expect(useProjectStore.getState().project).toEqual(before)
  })

  for (const [destination, importFile] of destinations) {
    it(`does not apply or reveal a ${destination} reply from the old project`, async () => {
      const reply = deferred<void>()
      const started = deferred<void>()
      const app = await startBrowserTest((mock) => ({
        addChannelFromFile: async (...args) => {
          const result = await mock.addChannelFromFile(...args)
          started.resolve()
          await reply.promise
          return result
        },
        addAudioClipFromFile: async (...args) => {
          const result = await mock.addAudioClipFromFile(...args)
          started.resolve()
          await reply.promise
          return result
        },
        setChannelSampleFromFile: async (...args) => {
          const result = await mock.setChannelSampleFromFile(...args)
          started.resolve()
          await reply.promise
          return result
        },
      }))
      stop = app.stop
      useUiStore
        .getState()
        .selectChannel(useProjectStore.getState().project.channels[0].id)
      const pending = importFile(path)
      await started.promise
      // Real mutation/event first, then save and real New flow, then the delayed reply.
      await app.backend.projectSave("/saved.windfall")
      await newProject()
      const before = structuredClone(useProjectStore.getState().project)
      reply.resolve()
      await pending
      expect(useProjectStore.getState().project).toEqual(before)
      expect(useUiStore.getState().selectedChannel).toBeNull()
    })
  }
  for (const [destination, importFile] of destinations) {
    it.each(["New", "Open"])(
      `holds ${destination} token lookup across %s`,
      async (change) => {
        const lookup = deferred<LibraryFileToken>()
        const app = await startBrowserTest(() => ({
          libraryFile: () => lookup.promise,
        }))
        stop = app.stop
        const saved = await app.backend.projectSave("/saved.windfall")
        const channel = useProjectStore.getState().project.channels[0].id
        useUiStore.getState().selectChannel(channel)
        const pending = importFile(path)
        if (change === "New") await newProject()
        else await openProjectPath(saved)
        expect(useProjectStore.getState().project.channels[0].id).toBe(channel)
        const before = structuredClone(useProjectStore.getState().project)
        // Query the real backend after replacement, as a delayed native IPC job can.
        const token = (
          await app.backend.librarySearch({
            query: "kick 02",
            favoritesOnly: false,
            tags: [],
          })
        ).entries[0].token
        lookup.resolve(token)
        await pending
        expect(useProjectStore.getState().project).toEqual(before)
        expect(useProjectStore.getState().history.cursor).toBe(0)
        expect(useUiStore.getState().selectedChannel).toBeNull()
      }
    )
  }
})

describe("tree selection during pending library search", () => {
  it("pins a drag from preview while search is pending and refuses it after refresh", async () => {
    const search = deferred<LibraryResults>()
    const app = await startBrowserTest(() => ({
      librarySearch: () => search.promise,
    }))
    stop = app.stop
    const user = userEvent.setup()
    render(<BrowserPanel />)
    await user.click(await findItem("Drums"))
    await user.click(await findItem("Kicks"))
    await user.click(await findItem("Kick 02.wav"))
    await waitFor(() =>
      expect(useBrowserStore.getState().selected?.library).toMatchObject({
        path,
      })
    )
    const data = new Map<string, string>()
    const dataTransfer = {
      effectAllowed: "uninitialized",
      setData: (type: string, value: string) => data.set(type, value),
      setDragImage: vi.fn(),
    }
    expect(fireEvent.dragStart(item("Kick 02.wav"), { dataTransfer })).toBe(
      true
    )
    const captured = JSON.parse(data.get(SAMPLE_DRAG_TYPE)!)
      .browser as LibraryFileToken
    await act(async () => refreshLibrary())
    expect(fireEvent.dragStart(item("Kick 02.wav"), { dataTransfer })).toBe(
      false
    )
    const before = structuredClone(useProjectStore.getState().project)
    await act(async () => addToRack(path, captured))
    expect(useProjectStore.getState().project).toEqual(before)
    fireEvent.dragEnd(item("Kick 02.wav"))
  })
  for (const [destination, importFile] of destinations) {
    it(`invalidates pending-search selection before ${destination} import on refresh`, async () => {
      const search = deferred<LibraryResults>()
      const app = await startBrowserTest(() => ({
        librarySearch: () => search.promise,
      }))
      stop = app.stop
      const user = userEvent.setup()
      const preview = vi.spyOn(app.backend, "previewPlay")
      render(<BrowserPanel />)
      await user.click(await findItem("Drums"))
      await user.click(await findItem("Kicks"))
      await user.click(await findItem("Kick 02.wav"))
      await waitFor(() => expect(preview).toHaveBeenCalled())
      expect(useLibraryStore.getState().results).toBeNull()
      expect(useBrowserStore.getState().selected?.path).toBe(path)
      expect(useBrowserStore.getState().selected?.library).toMatchObject({
        path,
      })
      useUiStore
        .getState()
        .selectChannel(useProjectStore.getState().project.channels[0].id)
      const before = structuredClone(useProjectStore.getState().project)
      await act(async () => refreshLibrary())
      await act(async () => importFile(path))
      expect(useProjectStore.getState().project).toEqual(before)
      // Explicit reselection remains usable even while the search is pending.
      await user.click(item("Kick 02.wav"))
      await act(async () => importFile(path))
      expect(useProjectStore.getState().project).not.toEqual(before)
    })
  }

  it.each(["refresh", "remove/re-add"])(
    "does not attach a lookup from before %s",
    async (change) => {
      const lookup = deferred<LibraryFileToken>()
      const search = deferred<LibraryResults>()
      let release = () => {}
      const app = await startBrowserTest((mock) => {
        release = async () => lookup.resolve(await mock.libraryFile(path))
        return {
          librarySearch: () => search.promise,
          libraryFile: () => lookup.promise,
        }
      })
      stop = app.stop
      const user = userEvent.setup()
      render(<BrowserPanel />)
      await user.click(await findItem("Drums"))
      await user.click(await findItem("Kicks"))
      await user.click(await findItem("Kick 02.wav"))
      const selection = useBrowserStore.getState().selected
      const before = structuredClone(useProjectStore.getState().project)
      const pending = addToRack(path)
      if (change === "refresh") await act(async () => refreshLibrary())
      else {
        // The native root owner invalidates every generation when roots change.
        const roots = await app.backend.browserAddRoot("/samples/Mine")
        const root = roots.find((r) => r.kind === "user")!
        await act(async () => applyRoots(roots))
        await act(async () => removeRoot(root))
        await act(async () =>
          applyRoots(await app.backend.browserAddRoot(root.path))
        )
      }
      await act(async () => {
        await release()
        await pending
      })
      expect(useProjectStore.getState().project).toEqual(before)
      expect(selection?.library).toBeUndefined()
      expect(useBrowserStore.getState().selected?.library).toBeUndefined()
    }
  )

  for (const [destination, importFile] of destinations) {
    it(`holds ${destination} lookup through removal and re-addition of its own root`, async () => {
      const lookup = deferred<LibraryFileToken>()
      const lookupStarted = deferred<void>()
      let release = () => {}
      const ownPath = "/samples/Mine/Vocal chop.wav"
      const app = await startBrowserTest((mock) => {
        release = async () => lookup.resolve(await mock.libraryFile(ownPath))
        return {
          libraryFile: () => {
            lookupStarted.resolve()
            return lookup.promise
          },
          pickFolder: async () => "/samples/Mine",
          librarySearch: () => new Promise(() => {}),
        }
      })
      stop = app.stop
      await loadRoots()
      await addFolder()
      const user = userEvent.setup()
      render(<BrowserPanel />)
      await user.click(await findItem("Vocal chop.wav"))
      await lookupStarted.promise
      useUiStore
        .getState()
        .selectChannel(useProjectStore.getState().project.channels[0].id)
      const before = structuredClone(useProjectStore.getState().project)
      const pending = importFile(ownPath)
      const root = useBrowserStore
        .getState()
        .roots.find((r) => r.kind === "user")!
      await act(async () => removeRoot(root))
      await act(async () => addFolder())
      await act(async () => {
        await release()
        await pending
      })
      expect(useProjectStore.getState().project).toEqual(before)
      expect(useBrowserStore.getState().selected?.library).toBeUndefined()
    })
  }
})
