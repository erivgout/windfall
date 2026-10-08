import {
  act,
  createEvent,
  fireEvent,
  render,
  waitFor,
} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, describe, expect, it, vi } from "vitest"

import type { BrowserRoot, LibraryFileToken, LibraryResults } from "@/bindings"
import { newProject, openProjectPath } from "@/lib/flows/project"
import { useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { SAMPLE_DRAG_TYPE } from "@/lib/dnd"
import ChannelRackPanel from "@/features/channel-rack"
import { dragData } from "@/features/channel-rack/test-utils"
import { settle } from "@/test/harness"

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
  it.each(["New", "Open"])(
    "does not select a reused channel from an actual rack drop reply after %s",
    async (change) => {
      const reply = deferred<void>()
      const started = deferred<number>()
      let populateReplacement = async () => 0
      const app = await startBrowserTest((mock) => {
        const importFile = async () => {
          const result = await mock.addChannelFromFile(path)
          return result.patch.channels!.find((c) =>
            result.created.includes(c.id)
          )!.id
        }
        populateReplacement = importFile
        return {
          addChannelFromFile: async (...args) => {
            const result = await mock.addChannelFromFile(...args)
            started.resolve(
              result.patch.channels!.find((c) => result.created.includes(c.id))!
                .id
            )
            await reply.promise
            return result
          },
        }
      })
      stop = app.stop
      // Begin with the real New template so both documents allocate the same
      // IDs (the harness initially opens a different demo project).
      await newProject()
      render(<ChannelRackPanel />)
      const scroller = document.querySelector('[data-slot="rack-scroll"]')!
      const token = await app.backend.libraryFile(path)
      const dataTransfer = dragData(
        SAMPLE_DRAG_TYPE,
        JSON.stringify({ path, name: "Kick 02", browser: token })
      )
      const drop = createEvent.drop(scroller, { dataTransfer })
      Object.defineProperty(drop, "clientY", { value: 600 })
      let oldId = 0
      await act(async () => {
        fireEvent(scroller, drop)
        oldId = await started.promise
      })
      // Mutation/event happen before the held reply. Use the real flows and
      // deliberately recreate the ID that the obsolete reply would select.
      await act(async () => {
        const saved = await app.backend.projectSave("/saved.windfall")
        if (change === "New") {
          await newProject()
          expect(await populateReplacement()).toBe(oldId)
        } else await openProjectPath(saved)
      })
      expect(
        useProjectStore.getState().project.channels.some((c) => c.id === oldId)
      ).toBe(true)
      const before = structuredClone(useProjectStore.getState().project)
      expect(useUiStore.getState().selectedChannel).toBeNull()
      await act(async () => {
        reply.resolve()
        await settle()
      })
      expect(useProjectStore.getState().project).toEqual(before)
      expect(useUiStore.getState().selectedChannel).toBeNull()
    }
  )

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
  for (const [destination, importFile] of destinations) {
    it.each(["refresh", "remove/re-add"])(
      `holds a canonical outer-root lookup through nested-root %s before ${destination} import`,
      async (change) => {
        const lookup = deferred<LibraryFileToken>()
        const started = deferred<void>()
        let release = async () => {}
        const app = await startBrowserTest((mock) => {
          release = async () => lookup.resolve(await mock.libraryFile(path))
          return {
            librarySearch: () => new Promise(() => {}),
            libraryFile: () => {
              started.resolve()
              return lookup.promise
            },
          }
        })
        stop = app.stop
        await loadRoots()
        const nested = "/factory/Drums/Kicks"
        await applyRoots(await app.backend.browserAddRoot(nested))
        render(<BrowserPanel />)
        const user = userEvent.setup()
        await user.click(await findItem("Kicks"))
        await user.click(await findItem("Kick 02.wav"))
        await started.promise
        const selection = useBrowserStore.getState().selected
        useUiStore
          .getState()
          .selectChannel(useProjectStore.getState().project.channels[0].id)
        const before = structuredClone(useProjectStore.getState().project)
        const pending = importFile(path)
        await act(async () => {
          if (change === "refresh") await refreshLibrary()
          else {
            const root = useBrowserStore
              .getState()
              .roots.find((r) => r.path === nested)!
            await removeRoot(root)
            await applyRoots(await app.backend.browserAddRoot(nested))
          }
          await release()
          await pending
        })
        expect(useProjectStore.getState().project).toEqual(before)
        expect(selection?.library).toBeUndefined()
        expect(useBrowserStore.getState().selected?.library).toBeUndefined()
      }
    )
  }

  for (const [destination, importFile] of destinations) {
    it(`auditions, reads facts and imports a nested-root selection into ${destination} while search is pending`, async () => {
      const app = await startBrowserTest(() => ({
        librarySearch: () => new Promise(() => {}),
      }))
      stop = app.stop
      await loadRoots()
      const nested = "/factory/Drums/Kicks"
      await applyRoots(await app.backend.browserAddRoot(nested))
      const preview = vi.spyOn(app.backend, "previewPlay")
      render(<BrowserPanel />)
      const user = userEvent.setup()
      await user.click(await findItem("Kicks"))
      await user.click(await findItem("Kick 02.wav"))
      expect(
        useBrowserStore.getState().selected?.librarySelection?.rootPath
      ).toBe(nested)
      expect(useLibraryStore.getState().results).toBeNull()
      // The real fixture backend, like native library_file, picks the first
      // containing root, even though this row belongs to the nested root.
      expect((await app.backend.libraryFile(path)).rootPath).toBe("/factory")
      useUiStore
        .getState()
        .selectChannel(useProjectStore.getState().project.channels[0].id)
      const before = structuredClone(useProjectStore.getState().project)
      await act(async () => importFile(path))
      expect(useProjectStore.getState().project).not.toEqual(before)
      await waitFor(() =>
        expect(preview).toHaveBeenCalledWith(
          path,
          expect.objectContaining({ rootPath: "/factory" })
        )
      )
      await waitFor(() =>
        expect(useBrowserStore.getState().info?.status).toBe("ready")
      )
      expect(useBrowserStore.getState().selected?.library).toMatchObject({
        path,
        rootPath: "/factory",
      })
      expect(
        useBrowserStore.getState().selected?.librarySelection?.rootPath
      ).toBe(nested)
    })
  }

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
