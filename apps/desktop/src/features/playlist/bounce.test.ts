import { afterEach, beforeEach, expect, it, vi } from "vitest"

import type { Project } from "@/bindings"
import { getAppState, runAction } from "@/lib/actions"
import { SimDocument } from "@/lib/ipc/sim/document"
import { demoProject } from "@/lib/ipc/sim/project"
import { useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { startTestApp } from "@/test/harness"

import { PLAYLIST_ACTIONS, registerPlaylistActions } from "./actions"
import { bounceSelectedClips } from "./bounce"
import { usePlaylistStore } from "./store"

let app: Awaited<ReturnType<typeof startTestApp>>
let unregister: () => void
let source: Project
const render =
  vi.fn<
    (
      project: Project,
      region: { start: number; end: number }
    ) => Promise<string>
  >()

beforeEach(async () => {
  source = demoProject()
  source.nextId = 1000
  source.playlist.tracks = [
    { id: 900, name: "Top", muted: true, solo: false },
    { id: 901, name: "Bottom", muted: false, solo: true },
    { id: 902, name: "Other", muted: false, solo: true },
  ]
  source.playlist.clips = [
    {
      id: 910,
      track: 901,
      start: 960,
      length: 960,
      offset: 0,
      muted: false,
      content: { type: "pattern", pattern: source.patterns[0].id },
    },
    {
      id: 911,
      track: 900,
      start: 1440,
      length: 960,
      offset: 0,
      muted: false,
      content: {
        type: "audio",
        sample: source.samples[0].id,
        mixerTrack: source.mixer.tracks[0].id,
        gain: 1,
        pan: 0,
        fadeIn: 0,
        fadeOut: 0,
        reverse: false,
        pitch: 0,
      },
    },
  ]
  // A pre-existing fixture file stands in for the native render, without a second renderer.
  render.mockReset().mockResolvedValue("/factory/Drums/Kicks/Kick 02.wav")
  app = await startTestApp({ project: source, renderPlaylistBounce: render })
  unregister = registerPlaylistActions()
  usePlaylistStore.setState(usePlaylistStore.getInitialState(), true)
  useUiStore.getState().showCenterTab("playlist")
  usePlaylistStore.getState().select([910, 911])
})

afterEach(() => {
  unregister()
  app.stop()
  vi.restoreAllMocks()
})

it("dispatches one successful bounce batch, mutes both source types and selects the new audio", async () => {
  const send = vi.spyOn(SimDocument.prototype, "dispatch")
  const tracks = structuredClone(
    useProjectStore.getState().project.playlist.tracks
  )
  await runAction("playlist.bounceSelectedClips")
  expect(render).toHaveBeenCalledOnce()
  const [copy, span] = render.mock.calls[0]
  expect(span).toEqual({ start: 960, end: 2400 })
  expect(
    copy.playlist.tracks.map((track) => [track.solo, track.muted])
  ).toEqual([
    [true, false],
    [true, false],
    [false, false],
  ])
  expect(send).toHaveBeenCalledOnce()
  expect(send.mock.calls[0][0]).toMatchObject({
    type: "batch",
    label: "Bounce selected clips",
    commands: [
      { type: "addSample" },
      {
        type: "addClips",
        clips: [
          {
            track: 900,
            start: 960,
            length: 1440,
            muted: false,
            content: { type: "audio" },
          },
        ],
      },
      {
        type: "updateClips",
        updates: [
          { id: 910, patch: { muted: true } },
          { id: 911, patch: { muted: true } },
        ],
      },
    ],
  })
  const { project, history } = useProjectStore.getState()
  expect(project.playlist.tracks).toEqual(tracks)
  expect(
    project.playlist.clips
      .filter((clip) => [910, 911].includes(clip.id))
      .every((clip) => clip.muted)
  ).toBe(true)
  const bounced = project.playlist.clips.find(
    (clip) => ![910, 911].includes(clip.id)
  )!
  expect(bounced).toMatchObject({
    track: 900,
    start: 960,
    length: 1440,
    muted: false,
  })
  expect(usePlaylistStore.getState().selection).toEqual(new Set([bounced.id]))
  expect(history.entries.at(-1)?.label).toBe("Bounce selected clips")
  await app.backend.undo()
  expect(useProjectStore.getState().project.playlist.clips).toEqual(
    source.playlist.clips
  )
})

it("dispatches nothing and leaves the document unchanged after a failed render", async () => {
  render.mockRejectedValueOnce(new Error("Renderer failed"))
  const before = await app.backend.documentSnapshot()
  const send = vi.spyOn(SimDocument.prototype, "dispatch")
  await bounceSelectedClips()
  expect(send).not.toHaveBeenCalled()
  expect(await app.backend.documentSnapshot()).toEqual(before)
  expect(usePlaylistStore.getState().selection).toEqual(new Set([910, 911]))
})

it("disables the action and does no work for an empty selection", async () => {
  usePlaylistStore.getState().clearSelection()
  const action = PLAYLIST_ACTIONS.find(
    (action) => action.id === "playlist.bounceSelectedClips"
  )!
  expect(action.enabled?.(getAppState())).toBe(false)
  const send = vi.spyOn(SimDocument.prototype, "dispatch")
  await bounceSelectedClips()
  expect(render).not.toHaveBeenCalled()
  expect(send).not.toHaveBeenCalled()
})

it("refuses a render completed after a document edit", async () => {
  render.mockImplementationOnce(async () => {
    await app.backend.dispatch({ type: "addPlaylistTrack" })
    return "/factory/Drums/Kicks/Kick 02.wav"
  })
  const send = vi.spyOn(SimDocument.prototype, "dispatch")
  await bounceSelectedClips()
  expect(send).toHaveBeenCalledExactlyOnceWith(
    { type: "addPlaylistTrack" },
    undefined
  )
  expect(useProjectStore.getState().project.playlist.clips).toEqual(
    source.playlist.clips
  )
})

it("browser preview refuses bounce without a native render fixture", async () => {
  unregister()
  app.stop()
  app = await startTestApp({ project: source })
  unregister = registerPlaylistActions()
  const before = await app.backend.documentSnapshot()
  await expect(app.backend.bounceSelectedClips([910])).rejects.toThrow(
    "offline renderer"
  )
  expect(await app.backend.documentSnapshot()).toEqual(before)
})
