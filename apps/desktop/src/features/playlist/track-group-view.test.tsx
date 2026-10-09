import { act, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { Project } from "@/bindings"
import { emptyProject } from "@/lib/ipc/sim/project"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { settle } from "@/test/harness"

import { GridMetrics } from "./metrics"
import { addClips, changeClips } from "./ops"
import { PlaylistScene } from "./scene"
import {
  at,
  BAR,
  click,
  FakeSurface,
  history,
  project,
  startPlaylist,
  startSession,
  tracks,
  ui,
} from "./test-utils"
import { toggleGroupMute, toggleGroupSolo } from "./track-group-ops"
import { TrackHeaders } from "./track-headers"
import { buildTrackRows } from "./track-rows"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

function groupedProject(): Project {
  const value = emptyProject()
  value.nextId = 110
  value.playlist = {
    tracks: [
      { id: 100, name: "Kick", muted: false },
      { id: 101, name: "Snare", muted: true },
      { id: 102, name: "Bass", muted: false },
    ],
    clips: [100, 101, 102].map((track, index) => ({
      id: 103 + index,
      track,
      start: index * BAR,
      length: BAR,
      offset: 0,
      muted: false,
      content: { type: "pattern", pattern: value.patterns[0].id },
    })),
    arrangementBook: {
      arrangements: [],
      active: null,
      trackGroups: [{ id: 106, name: "Drums" }],
      groupParents: {},
      trackParents: { 100: 106, 101: 106 },
      clipGroups: [],
      linkedTracks: {},
    },
  }
  return value
}

let app: Awaited<ReturnType<typeof startPlaylist>>
let scene: PlaylistScene
let surface: FakeSurface
let metrics: GridMetrics
let detach: () => void

beforeEach(async () => {
  app = await startPlaylist({ project: groupedProject() })
  surface = new FakeSurface()
  metrics = new GridMetrics()
  detach = metrics.attach(surface)
  scene = new PlaylistScene(surface, metrics)
})

afterEach(() => {
  scene.destroy()
  detach()
  app.stop()
})

async function flush() {
  await act(settle)
}

function canvasClips() {
  const batch = surface.items!.batch
  return Array.from({ length: batch.count }, (_, index) => ({
    id: batch.ids[index],
    row: batch.geometry[index * 4 + 2],
  }))
}

/** Mirror the solo response, as in track-solo tests, without rebuilding the sim. */
function replyWithSolo(ids: number[], solo: boolean) {
  const state = useProjectStore.getState()
  return {
    created: [],
    patch: {
      revision: state.revision + 1,
      playlist: {
        ...state.project.playlist,
        tracks: state.project.playlist.tracks.map((track) =>
          ids.includes(track.id) ? { ...track, solo } : track
        ),
      },
      history: state.history,
      dirty: true,
    },
  }
}

describe("playlist track group view", () => {
  it("collapses two member headers and their canvas clips, then expands without a command", () => {
    render(<TrackHeaders metrics={metrics} />)
    const send = vi.spyOn(app.backend, "dispatch")
    const saved = project()
    expect(screen.getByRole("group", { name: "Drums" })).toHaveStyle({
      top: "0px",
    })
    expect(screen.getByRole("group", { name: "Kick" })).toHaveStyle({
      top: "20px",
      paddingLeft: "16px",
    })
    expect(screen.getByRole("group", { name: "Snare" })).toHaveStyle({
      top: "40px",
    })
    expect(canvasClips()).toEqual([
      { id: 103, row: 1 },
      { id: 104, row: 2 },
      { id: 105, row: 3 },
    ])

    fireEvent.click(screen.getByRole("button", { name: "Collapse Drums" }))
    expect(screen.queryByRole("group", { name: "Kick" })).toBeNull()
    expect(screen.queryByRole("group", { name: "Snare" })).toBeNull()
    expect(screen.getByRole("group", { name: "Bass" })).toHaveStyle({
      top: "20px",
    })
    expect(canvasClips()).toEqual([{ id: 105, row: 1 }])
    expect(scene.trackRow(100)).toBeUndefined()
    expect(
      screen.getByRole("button", { name: "Expand Drums" })
    ).toHaveAttribute("aria-expanded", "false")

    fireEvent.click(screen.getByRole("button", { name: "Expand Drums" }))
    expect(screen.getByRole("group", { name: "Kick" })).toBeInTheDocument()
    expect(screen.getByRole("group", { name: "Snare" })).toBeInTheDocument()
    expect(canvasClips()).toEqual([
      { id: 103, row: 1 },
      { id: 104, row: 2 },
      { id: 105, row: 3 },
    ])
    expect(project()).toBe(saved)
    expect(send).not.toHaveBeenCalled()
  })

  it("mutes both members in one batch when one is on, then unmutes both in one batch", async () => {
    render(<TrackHeaders metrics={metrics} />)
    const send = vi.spyOn(app.backend, "dispatch")
    fireEvent.click(screen.getByRole("button", { name: "Collapse Drums" }))
    fireEvent.click(screen.getByRole("button", { name: "Mute Drums tracks" }))
    await flush()
    expect(send).toHaveBeenCalledTimes(1)
    expect(send.mock.calls[0][0]).toEqual({
      type: "batch",
      label: "Mute playlist track group",
      commands: [100, 101].map((id) => ({
        type: "updatePlaylistTrack",
        id,
        patch: { muted: true },
      })),
    })
    expect(tracks().map((track) => track.muted)).toEqual([true, true, false])
    expect(history().entries).toHaveLength(1)

    fireEvent.click(screen.getByRole("button", { name: "Unmute Drums tracks" }))
    await flush()
    expect(send).toHaveBeenCalledTimes(2)
    expect(send.mock.calls[1][0]).toEqual({
      type: "batch",
      label: "Unmute playlist track group",
      commands: [100, 101].map((id) => ({
        type: "updatePlaylistTrack",
        id,
        patch: { muted: false },
      })),
    })
    expect(tracks().map((track) => track.muted)).toEqual([false, false, false])
    expect(history().entries).toHaveLength(2)
  })

  it("solos every descendant in one batch when any member is not solo", async () => {
    const send = vi.spyOn(app.backend, "dispatch")
    send.mockResolvedValueOnce(replyWithSolo([100], true))
    await dispatch({
      type: "updatePlaylistTrack",
      id: 100,
      patch: { solo: true },
    })
    await flush()
    render(<TrackHeaders metrics={metrics} />)
    send.mockClear()
    send.mockResolvedValueOnce(replyWithSolo([100, 101], true))
    const solo = screen.getByRole("button", { name: "Solo Drums" })
    expect(solo).toHaveAttribute("aria-pressed", "false")
    expect(solo).toBeEnabled()
    fireEvent.click(solo)
    await flush()
    expect(send).toHaveBeenCalledExactlyOnceWith(
      {
        type: "batch",
        label: "Solo playlist track group",
        commands: [100, 101].map((id) => ({
          type: "updatePlaylistTrack",
          id,
          patch: { solo: true },
        })),
      },
      undefined
    )
    expect(tracks().map((track) => track.solo ?? false)).toEqual([
      true,
      true,
      false,
    ])
    expect(tracks().map((track) => track.muted)).toEqual([false, true, false])
    expect(solo).toHaveAttribute("aria-pressed", "true")
  })

  it("unsolos every descendant in one batch when all members are solo", async () => {
    const send = vi.spyOn(app.backend, "dispatch")
    send.mockResolvedValueOnce(replyWithSolo([100, 101], true))
    await toggleGroupSolo(106)
    await flush()
    render(<TrackHeaders metrics={metrics} />)
    send.mockClear()
    send.mockResolvedValueOnce(replyWithSolo([100, 101], false))
    const solo = screen.getByRole("button", { name: "Solo Drums" })
    expect(solo).toHaveAttribute("aria-pressed", "true")
    fireEvent.click(solo)
    await flush()
    expect(send).toHaveBeenCalledExactlyOnceWith(
      {
        type: "batch",
        label: "Unsolo playlist track group",
        commands: [100, 101].map((id) => ({
          type: "updatePlaylistTrack",
          id,
          patch: { solo: false },
        })),
      },
      undefined
    )
    expect(tracks().map((track) => track.solo ?? false)).toEqual([
      false,
      false,
      false,
    ])
    expect(tracks().map((track) => track.muted)).toEqual([false, true, false])
    expect(solo).toHaveAttribute("aria-pressed", "false")
  })

  it("includes a nested descendant in the group solo batch", async () => {
    scene.destroy()
    app.stop()
    const value = groupedProject()
    const book = value.playlist.arrangementBook!
    book.trackGroups.push({ id: 107, name: "Percussion" })
    book.groupParents[107] = 106
    book.trackParents[101] = 107
    app = await startPlaylist({ project: value })
    scene = new PlaylistScene(surface, metrics)
    render(<TrackHeaders metrics={metrics} />)
    const send = vi.spyOn(app.backend, "dispatch")
    send.mockResolvedValueOnce(replyWithSolo([100, 101], true))
    fireEvent.click(screen.getByRole("button", { name: "Solo Drums" }))
    await flush()
    expect(send).toHaveBeenCalledExactlyOnceWith(
      {
        type: "batch",
        label: "Solo playlist track group",
        commands: [100, 101].map((id) => ({
          type: "updatePlaylistTrack",
          id,
          patch: { solo: true },
        })),
      },
      undefined
    )
    expect(tracks().map((track) => track.solo ?? false)).toEqual([
      true,
      true,
      false,
    ])
    expect(
      screen.getByRole("button", { name: "Solo Percussion" })
    ).toHaveAttribute("aria-pressed", "true")
  })

  it("keeps hidden members in the group solo batch after collapse", async () => {
    render(<TrackHeaders metrics={metrics} />)
    const send = vi.spyOn(app.backend, "dispatch")
    send.mockResolvedValueOnce(replyWithSolo([100, 101], true))
    fireEvent.click(screen.getByRole("button", { name: "Collapse Drums" }))
    expect(screen.queryByRole("group", { name: "Kick" })).toBeNull()
    expect(screen.queryByRole("group", { name: "Snare" })).toBeNull()
    fireEvent.click(screen.getByRole("button", { name: "Solo Drums" }))
    await flush()
    expect(send).toHaveBeenCalledExactlyOnceWith(
      {
        type: "batch",
        label: "Solo playlist track group",
        commands: [100, 101].map((id) => ({
          type: "updatePlaylistTrack",
          id,
          patch: { solo: true },
        })),
      },
      undefined
    )
    expect(tracks().map((track) => track.solo ?? false)).toEqual([
      true,
      true,
      false,
    ])
    expect(screen.getByRole("button", { name: "Solo Drums" })).toHaveAttribute(
      "aria-pressed",
      "true"
    )
  })

  it("does nothing when soloing an empty group", async () => {
    await dispatch({ type: "moveTrackToGroup", track: 100, parent: null })
    await dispatch({ type: "moveTrackToGroup", track: 101, parent: null })
    await flush()
    render(<TrackHeaders metrics={metrics} />)
    const send = vi.spyOn(app.backend, "dispatch")
    const solo = screen.getByRole("button", { name: "Solo Drums" })
    expect(solo).toBeDisabled()
    expect(solo).toHaveAttribute("aria-pressed", "false")
    fireEvent.click(solo)
    await toggleGroupSolo(106)
    expect(send).not.toHaveBeenCalled()
  })

  it("follows nested parents for indentation, collapse and descendant mute", async () => {
    // Load a second valid fixture into the backend, with Snare owned by a child.
    scene.destroy()
    app.stop()
    const value = groupedProject()
    const book = value.playlist.arrangementBook!
    book.trackGroups.push({ id: 107, name: "Percussion" })
    book.groupParents[107] = 106
    book.trackParents[101] = 107
    app = await startPlaylist({ project: value })
    scene = new PlaylistScene(surface, metrics)
    render(<TrackHeaders metrics={metrics} />)
    expect(screen.getByRole("group", { name: "Percussion" })).toHaveStyle({
      top: "40px",
      paddingLeft: "16px",
    })
    expect(screen.getByRole("group", { name: "Snare" })).toHaveStyle({
      top: "60px",
      paddingLeft: "28px",
    })
    fireEvent.click(screen.getByRole("button", { name: "Collapse Percussion" }))
    expect(canvasClips()).toEqual([
      { id: 103, row: 1 },
      { id: 105, row: 3 },
    ])
    fireEvent.click(screen.getByRole("button", { name: "Collapse Drums" }))
    expect(screen.queryByRole("group", { name: "Percussion" })).toBeNull()
    expect(canvasClips()).toEqual([{ id: 105, row: 1 }])
    fireEvent.click(screen.getByRole("button", { name: "Mute Drums tracks" }))
    await flush()
    expect(tracks().map((track) => track.muted)).toEqual([true, true, false])
    fireEvent.click(screen.getByRole("button", { name: "Expand Drums" }))
    expect(
      screen.getByRole("button", { name: "Expand Percussion" })
    ).toBeInTheDocument()
    expect(screen.queryByRole("group", { name: "Snare" })).toBeNull()
    fireEvent.click(screen.getByRole("button", { name: "Expand Percussion" }))
    expect(canvasClips()).toEqual([
      { id: 103, row: 1 },
      { id: 104, row: 3 },
      { id: 105, row: 4 },
    ])
  })

  it("does not persist collapse and opens a replaced project expanded", () => {
    render(<TrackHeaders metrics={metrics} />)
    const send = vi.spyOn(app.backend, "dispatch")
    fireEvent.click(screen.getByRole("button", { name: "Collapse Drums" }))
    expect(ui().collapsedGroups.has(106)).toBe(true)
    const persisted = JSON.parse(
      localStorage.getItem("windfall.playlist")!
    ).state
    expect(persisted).not.toHaveProperty("collapsedGroups")
    act(announceProjectReplaced)
    expect(ui().collapsedGroups.size).toBe(0)
    expect(screen.getByRole("group", { name: "Kick" })).toBeInTheDocument()
    expect(surface.items!.batch.count).toBe(3)
    expect(send).not.toHaveBeenCalled()
  })

  it("does nothing when muting an empty group", async () => {
    const send = vi.spyOn(app.backend, "dispatch")
    const value = groupedProject()
    value.playlist.arrangementBook!.trackParents = {}
    expect(buildTrackRows(value.playlist, new Set()).rows.at(-1)).toMatchObject(
      { kind: "group", members: [] }
    )
    // A group with no surviving member also requires no command.
    await app.backend.dispatch({
      type: "moveTrackToGroup",
      track: 100,
      parent: null,
    })
    await app.backend.dispatch({
      type: "moveTrackToGroup",
      track: 101,
      parent: null,
    })
    await settle()
    send.mockClear()
    render(<TrackHeaders metrics={metrics} />)
    expect(
      screen.getByRole("button", { name: "Unmute Drums tracks" })
    ).toBeDisabled()
    await toggleGroupMute(106)
    expect(send).not.toHaveBeenCalled()
  })

  it("keeps clip edit destinations aligned and never places a clip on a group header", async () => {
    const content = {
      type: "pattern" as const,
      pattern: project().patterns[0].id,
    }
    const clip = {
      start: 4 * BAR,
      length: BAR,
      offset: 0,
      muted: false,
      content,
    }
    expect(await addClips([{ ...clip, row: 0 }], "Header")).toBeNull()
    const added = await addClips([{ ...clip, row: 2 }], "Member")
    expect(
      project().playlist.clips.find((item) => item.id === added![0])!.track
    ).toBe(101)
    expect(await changeClips([{ id: added![0], row: 0 }], "Header")).toBe(false)
    act(() => ui().toggleGroupCollapse(106))
    await changeClips([{ id: added![0], row: 1 }], "Visible track")
    expect(
      project().playlist.clips.find((item) => item.id === added![0])!.track
    ).toBe(102)
    const spare = await addClips([{ ...clip, row: 2 }], "Spare")
    expect(
      project().playlist.clips.find((item) => item.id === spare![0])!.track
    ).toBe(tracks().at(-1)!.id)
    expect(tracks()).toHaveLength(4)
  })

  it("preserves the existing headers and canvas layout for projects without groups", async () => {
    scene.destroy()
    app.stop()
    const value = groupedProject()
    delete value.playlist.arrangementBook
    app = await startPlaylist({ project: value })
    scene = new PlaylistScene(surface, metrics)
    render(<TrackHeaders metrics={metrics} />)
    expect(screen.queryByRole("group", { name: "Drums" })).toBeNull()
    for (const [index, name] of ["Kick", "Snare", "Bass"].entries()) {
      const header = screen.getByRole("group", { name })
      expect(header).toHaveStyle({ top: `${index * 20}px`, height: "20px" })
      expect(header.style.paddingLeft).toBe("")
    }
    expect(canvasClips()).toEqual([
      { id: 103, row: 0 },
      { id: 104, row: 1 },
      { id: 105, row: 2 },
    ])
  })

  it("reorders the intended member using visible header rows", async () => {
    render(<TrackHeaders metrics={metrics} />)
    const send = vi.spyOn(app.backend, "dispatch")
    const headers = screen.getByRole("group", { name: "Tracks" })
    fireEvent.pointerDown(screen.getByRole("group", { name: "Snare" }), {
      pointerId: 1,
      button: 0,
      clientY: 50,
    })
    fireEvent.pointerMove(headers, { pointerId: 1, clientY: 25 })
    fireEvent.pointerUp(headers, { pointerId: 1, clientY: 25 })
    await flush()
    expect(send.mock.calls[0][0]).toEqual({
      type: "movePlaylistTrack",
      id: 101,
      index: 0,
    })
    expect(tracks().map((track) => track.id)).toEqual([101, 100, 102])
    expect(canvasClips()).toEqual([
      { id: 103, row: 2 },
      { id: 104, row: 1 },
      { id: 105, row: 3 },
    ])
  })

  it("maps audio brush destinations to visible tracks and rejects group headers", async () => {
    const sample = await dispatch({
      type: "addSample",
      name: "Kick",
      path: { kind: "factory", path: "Drums/Kicks/Kick 01.wav" },
    })
    ui().setBrush({ type: "audio", sample: sample!.created[0] })
    const send = vi.spyOn(app.backend, "addAudioClipFromSample")
    const started = startSession()
    try {
      await click(started.session, at(4 * BAR, 0))
      expect(send).not.toHaveBeenCalled()
      await click(started.session, at(4 * BAR, 2))
      expect(send).toHaveBeenCalledWith(
        sample!.created[0],
        expect.objectContaining({ track: 101 })
      )
      act(() => ui().toggleGroupCollapse(106))
      await click(started.session, at(6 * BAR, 1))
      expect(send.mock.calls.at(-1)![1].track).toBe(102)
    } finally {
      started.stop()
    }
  })
})
