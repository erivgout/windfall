import { act, fireEvent, render, screen, within } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { emptyProject } from "@/lib/ipc/sim/project"
import { dispatch, loadSnapshot, redo, undo } from "@/lib/store/project"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { RECT_HLINE, type IndexedBatch, type Viewport } from "@/lib/canvas"
import { settle } from "@/test/harness"

import { clipBox } from "./clip-box"
import { MAX_ROW_HEIGHT, MIN_ROW_HEIGHT, TALL_ROW_HEIGHT } from "./layout"
import { GridMetrics } from "./metrics"
import { playlistYToRow } from "./row-geometry"
import { PlaylistSession } from "./session"
import {
  BAR,
  FakeSurface,
  history,
  project,
  startPlaylist,
  ui,
} from "./test-utils"
import { TrackGridSurface } from "./track-grid-surface"
import { TrackHeaders } from "./track-headers"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

class PixelCanvas extends FakeSurface {
  underlay: IndexedBatch | null = null
  setUnderlay(items: IndexedBatch | null) {
    this.underlay = items
  }
  override setViewport(next: Viewport) {
    const before = this.viewport
    if (
      Object.keys(before).every(
        (key) =>
          before[key as keyof typeof before] === next[key as keyof typeof next]
      )
    )
      return
    super.setViewport(next)
  }
}

function fixture() {
  const value = emptyProject()
  value.nextId = 110
  value.playlist = {
    tracks: [
      { id: 100, name: "Kick", muted: false },
      { id: 101, name: "Snare", muted: false },
      { id: 102, name: "Bass", muted: false },
    ],
    clips: [100, 101, 102].map((track, index) => ({
      id: 103 + index,
      track,
      start: 0,
      length: BAR,
      offset: 0,
      muted: false,
      content: { type: "pattern" as const, pattern: value.patterns[0].id },
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
let canvas: PixelCanvas
let surface: TrackGridSurface
let metrics: GridMetrics
let session: PlaylistSession
let detach: () => void

beforeEach(async () => {
  app = await startPlaylist({ project: fixture() })
  canvas = new PixelCanvas()
  surface = new TrackGridSurface(canvas)
  metrics = new GridMetrics()
  detach = metrics.attach(surface)
  session = new PlaylistSession(surface, metrics)
})
afterEach(() => {
  session.destroy()
  detach()
  surface.destroy()
  app.stop()
})

function header(name: string) {
  return screen.getByRole("group", { name })
}
function previewResize(height: number) {
  const handle = within(header("Kick")).getByLabelText("Resize Kick")
  const initial = Number.parseFloat(header("Kick").style.height)
  fireEvent.pointerDown(handle, { button: 0, pointerId: 1, clientY: 40 })
  fireEvent.pointerMove(handle, {
    pointerId: 1,
    clientY: 40 + height - initial,
  })
  return handle
}
function resize(height: number) {
  const handle = previewResize(height)
  fireEvent.pointerUp(handle, { pointerId: 1 })
}

describe("saved playlist track resize", () => {
  it("previews locally, moves following rows, and saves one patch on release", async () => {
    render(<TrackHeaders metrics={metrics} />)
    const saved = project()
    const send = vi.spyOn(app.backend, "dispatch")
    const handle = previewResize(100)
    expect(ui().trackHeights.get(100)).toBe(100)
    expect(header("Kick")).toHaveStyle({ top: "20px", height: "100px" })
    expect(header("Snare")).toHaveStyle({ top: "120px", height: "20px" })
    expect(header("Bass")).toHaveStyle({ top: "140px", height: "20px" })
    const batch = canvas.items!.batch
    expect(batch.row(batch.indexOfId(103))).toBe(20)
    expect(batch.rowSpan(batch.indexOfId(103))).toBe(100)
    expect(batch.row(batch.indexOfId(104))).toBe(120)
    const grid = canvas.underlay!.batch
    const rowLines = Array.from({ length: grid.count }, (_, index) => index)
      .filter((index) => (grid.flags[index] & RECT_HLINE) !== 0)
      .map((index) => grid.row(index))
    expect(rowLines).toContain(120)
    expect(rowLines).not.toContain(40)
    expect(
      clipBox(surface.viewport, project().playlist.clips[0], 1)
    ).toMatchObject({ top: 21, bottom: 120 })
    expect(project()).toBe(saved)
    expect(history().entries).toHaveLength(0)
    expect(send).not.toHaveBeenCalled()
    fireEvent.pointerMove(handle, { pointerId: 1, clientY: 120 })
    fireEvent.pointerUp(handle, { pointerId: 1 })
    await act(settle)
    expect(send).toHaveBeenCalledTimes(1)
    expect(send).toHaveBeenCalledWith(
      { type: "updatePlaylistTrack", id: 100, patch: { height: 100 } },
      undefined
    )
    expect(project().playlist.tracks[0].height).toBe(100)
    expect(history().entries).toHaveLength(1)
  })

  it("hits and selects the clip in the lower half of a tall track", async () => {
    ui().setTrackHeight(100, 100)
    expect(surface.hitTest(48, 100)).toMatchObject({ id: 103, part: "body" })
    expect(playlistYToRow(surface.viewport, 100)).toBeCloseTo(1.8)
    ui().setTool("select")
    const point = {
      x: 48,
      y: 100,
      button: 0,
      shift: false,
      mod: false,
      alt: false,
    }
    session.pointerDown(point)
    await session.pointerUp(point)
    expect([...ui().selection]).toEqual([103])
    expect(surface.hitTest(48, 125)).toMatchObject({ id: 104 })
  })

  it("keeps unset tracks and group headers at the global height, including tall toggle", () => {
    render(<TrackHeaders metrics={metrics} />)
    act(() => ui().setTrackHeight(100, 100))
    act(() => metrics.toggleTall())
    expect(header("Kick")).toHaveStyle({ height: "100px" })
    expect(header("Snare")).toHaveStyle({ height: `${TALL_ROW_HEIGHT}px` })
    expect(header("Drums")).toHaveStyle({ height: `${TALL_ROW_HEIGHT}px` })
    expect(within(header("Drums")).queryByLabelText(/Resize/)).toBeNull()
    const batch = canvas.items!.batch
    expect(batch.rowSpan(batch.indexOfId(104))).toBe(TALL_ROW_HEIGHT)
  })

  it("keeps heights out of UI preferences and loads replacement document heights", async () => {
    render(<TrackHeaders metrics={metrics} />)
    resize(100)
    await act(settle)
    const persisted = JSON.parse(
      localStorage.getItem("windfall.playlist")!
    ).state
    expect(persisted).not.toHaveProperty("trackHeights")
    act(announceProjectReplaced)
    expect(ui().trackHeights.get(100)).toBe(100)
    expect(header("Kick")).toHaveStyle({ height: "100px" })
    const snapshot = await app.backend.documentSnapshot()
    act(() => {
      loadSnapshot({
        ...snapshot,
        project: {
          ...snapshot.project,
          playlist: {
            ...snapshot.project.playlist,
            tracks: snapshot.project.playlist.tracks.map((track) => ({
              ...track,
              height: track.id === 101 ? 60 : 0,
            })),
          },
        },
      })
      announceProjectReplaced()
    })
    expect(ui().trackHeights.has(100)).toBe(false)
    expect(ui().trackHeights.get(101)).toBe(60)
    expect(header("Kick")).toHaveStyle({ height: "20px" })
    expect(header("Snare")).toHaveStyle({ height: "60px" })
    await act(async () => {
      await app.backend.projectNew()
      await settle()
    })
    expect(ui().trackHeights.size).toBe(0)
  })

  it("restores the displayed saved height after undo, redo, and project patches", async () => {
    render(<TrackHeaders metrics={metrics} />)
    resize(100)
    await act(settle)
    await act(undo)
    expect(header("Kick")).toHaveStyle({ height: "20px" })
    expect(ui().trackHeights.has(100)).toBe(false)
    expect(header("Snare")).toHaveStyle({ top: "40px" })
    expect(canvas.items!.batch.rowSpan(0)).toBe(20)
    await act(redo)
    expect(header("Kick")).toHaveStyle({ height: "100px" })
    await act(async () => {
      await dispatch({
        type: "updatePlaylistTrack",
        id: 100,
        patch: { height: 60 },
      })
    })
    expect(header("Kick")).toHaveStyle({ height: "60px" })
    await act(async () => {
      await dispatch({
        type: "updatePlaylistTrack",
        id: 100,
        patch: { height: 0 },
      })
    })
    expect(header("Kick")).toHaveStyle({ height: "20px" })
    expect(ui().trackHeights.has(100)).toBe(false)
  })

  it("clamps the edge drag to the playlist height limits without reordering", async () => {
    render(<TrackHeaders metrics={metrics} />)
    resize(1000)
    await act(settle)
    expect(ui().trackHeights.get(100)).toBe(MAX_ROW_HEIGHT)
    const handle = within(header("Kick")).getByLabelText("Resize Kick")
    fireEvent.pointerDown(handle, { button: 0, pointerId: 2, clientY: 180 })
    fireEvent.pointerMove(handle, { pointerId: 2, clientY: 20 })
    expect(ui().trackHeights.get(100)).toBe(MIN_ROW_HEIGHT)
    fireEvent.pointerMove(handle, { pointerId: 2, clientY: -1000 })
    fireEvent.pointerUp(handle, { pointerId: 2 })
    await act(settle)
    expect(ui().trackHeights.get(100)).toBe(MIN_ROW_HEIGHT)
    expect(project().playlist.tracks.map((track) => track.id)).toEqual([
      100, 101, 102,
    ])
    expect(history().entries).toHaveLength(2)
  })

  it("cancels to the saved height without dispatching, including an unset height", async () => {
    render(<TrackHeaders metrics={metrics} />)
    const send = vi.spyOn(app.backend, "dispatch")
    let handle = previewResize(100)
    fireEvent.pointerCancel(handle, { pointerId: 1 })
    expect(send).not.toHaveBeenCalled()
    expect(ui().trackHeights.has(100)).toBe(false)
    expect(header("Kick")).toHaveStyle({ height: "20px" })
    resize(80)
    await act(settle)
    send.mockClear()
    handle = previewResize(120)
    fireEvent.pointerCancel(handle, { pointerId: 1 })
    expect(send).not.toHaveBeenCalled()
    expect(header("Kick")).toHaveStyle({ height: "80px" })
    expect(project().playlist.tracks[0].height).toBe(80)
  })

  it("dispatches nothing when release matches the saved height", async () => {
    render(<TrackHeaders metrics={metrics} />)
    const send = vi.spyOn(app.backend, "dispatch")
    const handle = within(header("Kick")).getByLabelText("Resize Kick")
    fireEvent.pointerDown(handle, { button: 0, pointerId: 1, clientY: 40 })
    fireEvent.pointerUp(handle, { pointerId: 1 })
    expect(send).not.toHaveBeenCalled()
    resize(80)
    await act(settle)
    send.mockClear()
    resize(80)
    await act(settle)
    expect(send).not.toHaveBeenCalled()
    expect(history().entries).toHaveLength(1)
  })

  it("preserves an active preview across a project patch and cancels to the new saved height", async () => {
    render(<TrackHeaders metrics={metrics} />)
    const handle = previewResize(100)
    await act(async () => {
      await dispatch({
        type: "updatePlaylistTrack",
        id: 100,
        patch: { height: 60 },
      })
      await dispatch({
        type: "updatePlaylistTrack",
        id: 101,
        patch: { height: 40 },
      })
    })
    expect(header("Kick")).toHaveStyle({ height: "100px" })
    expect(header("Snare")).toHaveStyle({ height: "40px" })
    const send = vi.spyOn(app.backend, "dispatch")
    fireEvent.pointerCancel(handle, { pointerId: 1 })
    expect(send).not.toHaveBeenCalled()
    expect(header("Kick")).toHaveStyle({ height: "60px" })
  })

  it("keeps spans aligned after scrolling and maps a dragged clip to its destination height", () => {
    ui().setTrackHeight(100, 100)
    metrics.setViewport({ ...surface.viewport, height: 200, scrollRow: 2.5 })
    expect(surface.hitTest(48, 50)).toMatchObject({ id: 103 })
    expect(
      clipBox(surface.viewport, project().playlist.clips[0], 1)
    ).toMatchObject({ top: -29, bottom: 70 })
    ui().select([103])
    const point = {
      x: 48,
      y: 50,
      button: 0,
      shift: false,
      mod: false,
      alt: false,
    }
    session.pointerDown(point)
    session.pointerMove({ ...point, y: 80 })
    const batch = canvas.items!.batch
    const moved = batch.indexOfId(103)
    expect(batch.row(moved)).toBe(120)
    expect(batch.rowSpan(moved)).toBe(20)
  })
})
