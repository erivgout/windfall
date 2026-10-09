import { act, fireEvent, render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { TRACK_COLORS } from "@/features/mixer/colors"
import {
  RECT_FLAT,
  RECT_FULL_WIDTH,
  type IndexedBatch,
  type Viewport,
} from "@/lib/canvas"
import { emptyProject } from "@/lib/ipc/sim/project"
import { useProjectStore } from "@/lib/store/project"

import { GridMetrics } from "./metrics"
import { PlaylistScene } from "./scene"
import { FakeSurface, startPlaylist, ui } from "./test-utils"
import { TrackGridSurface } from "./track-grid-surface"
import { TrackHeaders } from "./track-headers"

class PixelCanvas extends FakeSurface {
  underlay: IndexedBatch | null = null
  setUnderlay(items: IndexedBatch | null) {
    this.underlay = items
  }
  override setViewport(next: Viewport) {
    if (
      Object.keys(this.viewport).every(
        (key) =>
          this.viewport[key as keyof Viewport] === next[key as keyof Viewport]
      )
    )
      return
    super.setViewport(next)
  }
}

let app: Awaited<ReturnType<typeof startPlaylist>>
let canvas: PixelCanvas
let surface: TrackGridSurface
let metrics: GridMetrics
let scene: PlaylistScene
let detach: () => void

beforeEach(async () => {
  const project = emptyProject()
  project.nextId = 102
  project.playlist.tracks = [
    { id: 100, name: "Kick", muted: false, color: 0 },
    { id: 101, name: "Snare", muted: false },
  ]
  app = await startPlaylist({ project })
  canvas = new PixelCanvas()
  surface = new TrackGridSurface(canvas)
  metrics = new GridMetrics()
  detach = metrics.attach(surface)
  scene = new PlaylistScene(surface, metrics)
})

afterEach(() => {
  scene.destroy()
  detach()
  surface.destroy()
  app.stop()
  vi.restoreAllMocks()
})

function laneTints() {
  const batch = canvas.underlay!.batch
  return Array.from({ length: batch.count }, (_, index) => index)
    .filter(
      (index) =>
        batch.flags[index] === (RECT_FLAT | RECT_FULL_WIDTH) &&
        batch.colors[index * 4 + 3] === 26
    )
    .map((index) => ({
      row: batch.row(index),
      height: batch.rowSpan(index),
      rgba: Array.from(batch.colors.slice(index * 4, index * 4 + 4)),
    }))
}

async function chooseColor(name: string) {
  const user = userEvent.setup()
  fireEvent.contextMenu(screen.getByRole("group", { name: "Kick" }))
  const submenu = await screen.findByRole("menuitem", { name: "Color" })
  await user.click(submenu)
  const color = await screen.findByRole("menuitemcheckbox", { name })
  act(() => color.focus())
  await user.keyboard("{Enter}")
}

/** The UI consumes this wire response; Rust command behavior is tested separately. */
function replyWithColor(color: number) {
  const state = useProjectStore.getState()
  return {
    created: [],
    patch: {
      revision: state.revision + 1,
      playlist: {
        ...state.project.playlist,
        tracks: state.project.playlist.tracks.map((track) =>
          track.id === 100 ? { ...track, color } : track
        ),
      },
      history: state.history,
      dirty: true,
    },
  }
}

describe("playlist track color", () => {
  it("offers the existing palette and sends one update that tints the header and lane", async () => {
    const send = vi
      .spyOn(app.backend, "dispatch")
      .mockResolvedValueOnce(replyWithColor(0x12a594))
    render(<TrackHeaders metrics={metrics} />)
    const user = userEvent.setup()
    fireEvent.contextMenu(screen.getByRole("group", { name: "Kick" }))
    await user.click(await screen.findByRole("menuitem", { name: "Color" }))
    for (const { name } of TRACK_COLORS) {
      expect(
        await screen.findByRole("menuitemcheckbox", { name })
      ).toBeInTheDocument()
    }
    act(() => screen.getByRole("menuitemcheckbox", { name: "Teal" }).focus())
    await user.keyboard("{Enter}")
    expect(send).toHaveBeenCalledExactlyOnceWith(
      { type: "updatePlaylistTrack", id: 100, patch: { color: 0x12a594 } },
      undefined
    )
    expect(
      screen.getByRole("group", { name: "Kick" }).style.backgroundColor
    ).not.toBe("")
    expect(laneTints()).toEqual([
      { row: 0, height: 20, rgba: [18, 165, 148, 26] },
    ])

    const handle = screen.getByLabelText("Resize Kick")
    fireEvent.pointerDown(handle, { button: 0, pointerId: 1, clientY: 20 })
    fireEvent.pointerMove(handle, { pointerId: 1, clientY: 100 })
    fireEvent.pointerUp(handle, { pointerId: 1 })
    expect(ui().trackHeights.get(100)).toBe(100)
    expect(laneTints()[0]).toMatchObject({ row: 0, height: 100 })
    // Resizing persists the track height as its own edit, keeping its color.
    expect(send).toHaveBeenCalledTimes(2)
    expect(send).toHaveBeenLastCalledWith(
      { type: "updatePlaylistTrack", id: 100, patch: { height: 100 } },
      undefined
    )
  })

  it("leaves both explicit zero and missing color untinted", () => {
    // Missing fields in legacy wire data and the explicit sentinel look alike.
    useProjectStore.setState(({ project }) => ({
      project: {
        ...project,
        playlist: {
          ...project.playlist,
          tracks: project.playlist.tracks.map((track, index) =>
            index === 0 ? { ...track, color: 0 } : track
          ),
        },
      },
    }))
    render(<TrackHeaders metrics={metrics} />)
    expect(
      screen.getByRole("group", { name: "Kick" }).style.backgroundColor
    ).toBe("")
    expect(
      screen.getByRole("group", { name: "Snare" }).style.backgroundColor
    ).toBe("")
    expect(laneTints()).toEqual([])
  })

  it("clears the tint with one zero-color command", async () => {
    const send = vi
      .spyOn(app.backend, "dispatch")
      .mockResolvedValueOnce(replyWithColor(0xe5484d))
    render(<TrackHeaders metrics={metrics} />)
    await chooseColor("Red")
    expect(laneTints()).toHaveLength(1)
    send.mockClear()
    send.mockResolvedValueOnce(replyWithColor(0))
    await chooseColor("No color")
    expect(send).toHaveBeenCalledExactlyOnceWith(
      { type: "updatePlaylistTrack", id: 100, patch: { color: 0 } },
      undefined
    )
    expect(
      screen.getByRole("group", { name: "Kick" }).style.backgroundColor
    ).toBe("")
    expect(laneTints()).toEqual([])
  })
})
