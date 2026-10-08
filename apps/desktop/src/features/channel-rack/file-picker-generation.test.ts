import { afterEach, describe, expect, it, vi } from "vitest"

import { deferred, startBrowserTest } from "@/features/browser/testing"
import { newProject, openProjectPath } from "@/lib/flows/project"
import { useProjectStore } from "@/lib/store/project"
import { getProjectGeneration } from "@/lib/store/replaced"
import { useUiStore } from "@/lib/store/ui"

import {
  addChannelFromPickedFile,
  replaceSampleFromPickedFile,
} from "./channel-ops"

let stop = () => {}
afterEach(() => stop())
const path = "/factory/Drums/Kicks/Kick 02.wav"

describe("picked-file project identity", () => {
  for (const destination of ["rack", "replacement"] as const) {
    it.each(["New", "Open"])(
      `refuses the actual ${destination} picker reply after %s`,
      async (change) => {
        const picker = deferred<string | null>()
        const app = await startBrowserTest(() => ({
          pickAudioFile: () => picker.promise,
        }))
        stop = app.stop
        // Use the real New template so replacement reuses the selected ID.
        await newProject()
        const saved = await app.backend.projectSave("/picked-file.windfall")
        const channel = useProjectStore.getState().project.channels[0].id
        useUiStore.getState().selectChannel(channel)
        useUiStore.getState().showCenterTab("playlist")
        const generation = getProjectGeneration()
        const add = vi.spyOn(app.backend, "addChannelFromFile")
        const replace = vi.spyOn(app.backend, "setChannelSampleFromFile")
        const pending =
          destination === "rack"
            ? addChannelFromPickedFile()
            : replaceSampleFromPickedFile(channel)
        if (change === "New") await newProject()
        else await openProjectPath(saved)
        expect(getProjectGeneration()).not.toBe(generation)
        expect(useProjectStore.getState().project.channels[0].id).toBe(channel)
        useUiStore.getState().showCenterTab("playlist")
        const before = structuredClone(useProjectStore.getState().project)
        const history = structuredClone(useProjectStore.getState().history)
        picker.resolve(path)
        await pending
        expect(add).not.toHaveBeenCalled()
        expect(replace).not.toHaveBeenCalled()
        expect(useProjectStore.getState().project).toEqual(before)
        expect(useProjectStore.getState().history).toEqual(history)
        expect(useUiStore.getState().selectedChannel).toBeNull()
        expect(useUiStore.getState().centerTab).toBe("playlist")
      }
    )

    it.each(["New", "Open"])(
      `ignores the actual ${destination} import reply after %s`,
      async (change) => {
        const reply = deferred<void>()
        const started = deferred<void>()
        const app = await startBrowserTest((mock) => ({
          pickAudioFile: async () => path,
          addChannelFromFile: async (...args) => {
            const result = await mock.addChannelFromFile(...args)
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
        await newProject()
        const channel = useProjectStore.getState().project.channels[0].id
        const pending =
          destination === "rack"
            ? addChannelFromPickedFile()
            : replaceSampleFromPickedFile(channel)
        // Actual mutation/event before its delayed reply, then actual flows.
        await started.promise
        const saved = await app.backend.projectSave("/picked-reply.windfall")
        if (change === "New") await newProject()
        else await openProjectPath(saved)
        expect(useProjectStore.getState().project.channels[0].id).toBe(channel)
        useUiStore.getState().showCenterTab("playlist")
        const before = useProjectStore.getState()
        reply.resolve()
        await pending
        expect(useUiStore.getState().centerTab).toBe("playlist")
        expect(useUiStore.getState().selectedChannel).toBeNull()
        expect(useProjectStore.getState().project).toEqual(before.project)
        expect(useProjectStore.getState().history).toEqual(before.history)
        expect(useProjectStore.getState().revision).toBe(before.revision)
        expect(useProjectStore.getState().dirty).toBe(before.dirty)
      }
    )

    it(`preserves ${destination} picker cancellation`, async () => {
      const app = await startBrowserTest(() => ({
        pickAudioFile: async () => null,
      }))
      stop = app.stop
      useUiStore.getState().showCenterTab("playlist")
      const before = useProjectStore.getState()
      const add = vi.spyOn(app.backend, "addChannelFromFile")
      const replace = vi.spyOn(app.backend, "setChannelSampleFromFile")
      if (destination === "rack") await addChannelFromPickedFile()
      else await replaceSampleFromPickedFile(before.project.channels[0].id)
      expect(add).not.toHaveBeenCalled()
      expect(replace).not.toHaveBeenCalled()
      expect(useProjectStore.getState()).toBe(before)
      expect(useUiStore.getState().centerTab).toBe("playlist")
    })

    it(`preserves successful ${destination} picker import`, async () => {
      const app = await startBrowserTest(() => ({
        pickAudioFile: async () => path,
      }))
      stop = app.stop
      useUiStore.getState().showCenterTab("playlist")
      const before = useProjectStore.getState()
      if (destination === "rack") {
        await addChannelFromPickedFile()
        expect(useProjectStore.getState().project.channels.length).toBe(
          before.project.channels.length + 1
        )
        expect(useUiStore.getState().selectedChannel).not.toBeNull()
        expect(useUiStore.getState().centerTab).toBe("channelRack")
      } else {
        const channel = before.project.channels[0].id
        await replaceSampleFromPickedFile(channel)
        const source = useProjectStore
          .getState()
          .project.channels.find((c) => c.id === channel)!.source
        expect(source).not.toEqual(before.project.channels[0].source)
        expect(useUiStore.getState().centerTab).toBe("playlist")
      }
      expect(useProjectStore.getState().history.cursor).toBe(
        before.history.cursor + 1
      )
    })
  }
})
