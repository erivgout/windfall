import { act, fireEvent, render, screen, within } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { Backend } from "@/lib/ipc"
import { useHintStore } from "@/lib/store/hint"
import { dispatch } from "@/lib/store/project"
import { setPlayMode, useTransportStore } from "@/lib/store/transport"
import { settle } from "@/test/harness"

import { activeMetrics, zoomToFit } from "./active"
import PlaylistPanel from "./index"
import { HEADER_WIDTH, MIN_ROWS } from "./layout"
import { addClips } from "./ops"
import {
  answerText,
  BAR,
  clips,
  labels,
  project,
  startPlaylist,
  tracks,
  ui,
} from "./test-utils"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

// jsdom has no canvas to draw on. The grid's pointer handling is tested on
// a stand-in surface in session.test.ts; here it is everything around it.
vi.mock("@/lib/canvas/TimeGridCanvas", () => ({
  TimeGridCanvas: () => null,
}))

let backend: Backend
let stop: () => void

beforeEach(async () => {
  ;({ backend, stop } = await startPlaylist())
  // The ruler draws on a canvas, which jsdom reports as missing each time.
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null)
})
afterEach(() => stop())

async function flush() {
  await act(async () => {
    await settle()
  })
}

async function seed(...spots: [row: number, start: number, length?: number][]) {
  await act(async () => {
    await addClips(
      spots.map(([row, start, length = BAR]) => ({
        row,
        start,
        length,
        offset: 0,
        muted: false,
        pattern: project().patterns[0].id,
      })),
      "Seed"
    )
    await settle()
  })
}

/** The default view: a bar is 72 pixels wide. */
const xOfBar = (bar: number) => bar * 72

describe("an empty song", () => {
  it("says what to do first, without getting in the way", () => {
    render(<PlaylistPanel />)
    const invitation = screen.getByText(/Pick a pattern on the left/)
    expect(invitation.closest(".pointer-events-none")).not.toBeNull()
  })

  it("shows rows to place clips on although there are no tracks", () => {
    render(<PlaylistPanel />)
    const spare = document.querySelectorAll("[data-spare-row]")
    expect(spare.length).toBeGreaterThanOrEqual(MIN_ROWS - 2)
    expect(spare[0]).toHaveTextContent("1")
  })

  it("drops the invitation once there is a clip", async () => {
    render(<PlaylistPanel />)
    await seed([0, 0])
    expect(screen.queryByText(/Pick a pattern on the left/)).toBeNull()
  })
})

describe("pattern picker", () => {
  it("lists the patterns and marks the one that will be placed", async () => {
    await dispatch({ type: "addPattern", name: "Bass" })
    render(<PlaylistPanel />)
    const list = screen.getByRole("group", { name: "Pattern to place" })
    const rows = within(list).getAllByRole("button")
    expect(rows.map((row) => row.textContent)).toEqual([
      "Pattern 11 bar",
      "Bass1 bar",
    ])
    expect(rows[0]).toHaveAttribute("aria-pressed", "true")
    expect(rows[1]).toHaveAttribute("aria-pressed", "false")
  })

  it("selects the pattern app-wide when one is clicked", async () => {
    const added = await dispatch({ type: "addPattern", name: "Bass" })
    render(<PlaylistPanel />)
    fireEvent.click(screen.getByRole("button", { name: /^Bass/ }))
    await flush()
    expect(useTransportStore.getState().pattern).toBe(added!.created[0])
    expect(screen.getByRole("button", { name: /^Bass/ })).toHaveAttribute(
      "aria-pressed",
      "true"
    )
  })

  it("adds a pattern and renames one through the pattern actions", async () => {
    render(<PlaylistPanel />)
    fireEvent.click(screen.getByRole("button", { name: "Add pattern" }))
    await flush()
    expect(project().patterns).toHaveLength(2)
    fireEvent.doubleClick(screen.getByRole("button", { name: /^Pattern 2/ }))
    await act(() => answerText("Hook"))
    expect(project().patterns[1].name).toBe("Hook")
    expect(screen.getByRole("button", { name: /^Hook/ })).toBeInTheDocument()
  })

  it("hides and shows with its toolbar button", async () => {
    render(<PlaylistPanel />)
    fireEvent.click(screen.getByRole("button", { name: "Pattern list" }))
    await flush()
    expect(screen.queryByRole("group", { name: "Pattern to place" })).toBeNull()
    fireEvent.click(screen.getByRole("button", { name: "Pattern list" }))
    await flush()
    expect(
      screen.getByRole("group", { name: "Pattern to place" })
    ).toBeInTheDocument()
  })
})

describe("track headers", () => {
  it("names the tracks beside their rows, then the spare rows", async () => {
    render(<PlaylistPanel />)
    await seed([0, 0], [1, 0])
    const headers = screen.getByRole("group", { name: "Tracks" })
    expect(within(headers).getByRole("group", { name: "Track 1" })).toHaveStyle(
      { top: "0px", height: "38px" }
    )
    expect(within(headers).getByRole("group", { name: "Track 2" })).toHaveStyle(
      { top: "38px", height: "38px" }
    )
    expect(headers).toHaveStyle({ width: `${HEADER_WIDTH}px` })
    expect(document.querySelector("[data-spare-row='2']")).toHaveStyle({
      top: "76px",
    })
  })

  it("mutes and unmutes a track from its lamp", async () => {
    render(<PlaylistPanel />)
    await seed([0, 0])
    const lamp = screen.getByRole("button", { name: "Track 1 on" })
    expect(lamp).toHaveAttribute("aria-pressed", "true")
    fireEvent.click(lamp)
    await flush()
    expect(tracks()[0].muted).toBe(true)
    expect(screen.getByRole("button", { name: "Track 1 on" })).toHaveAttribute(
      "aria-pressed",
      "false"
    )
    expect(labels().at(-1)).toBe("Mute playlist track")
  })

  it("renames a track in place on a double-click", async () => {
    render(<PlaylistPanel />)
    await seed([0, 0])
    fireEvent.doubleClick(screen.getByText("Track 1"))
    const field = screen.getByRole("textbox", { name: "Name of Track 1" })
    fireEvent.change(field, { target: { value: "Drums" } })
    fireEvent.keyDown(field, { key: "Enter" })
    await flush()
    expect(tracks()[0].name).toBe("Drums")
    expect(screen.getByRole("group", { name: "Drums" })).toBeInTheDocument()
  })

  it("keeps the old name when the rename is escaped", async () => {
    render(<PlaylistPanel />)
    await seed([0, 0])
    fireEvent.doubleClick(screen.getByText("Track 1"))
    const field = screen.getByRole("textbox", { name: "Name of Track 1" })
    fireEvent.change(field, { target: { value: "Nope" } })
    fireEvent.keyDown(field, { key: "Escape" })
    await flush()
    expect(tracks()[0].name).toBe("Track 1")
  })

  it("makes a pressed header the target of the track actions", async () => {
    render(<PlaylistPanel />)
    await seed([0, 0], [1, 0])
    fireEvent.pointerDown(screen.getByRole("group", { name: "Track 2" }))
    await flush()
    expect(ui().targetTrack).toBe(tracks()[1].id)
    expect(screen.getByRole("group", { name: "Track 2" })).toHaveAttribute(
      "data-target"
    )
  })

  it("only renders the rows in view, and scrolls without rendering", async () => {
    for (let index = 0; index < 200; index++) {
      await dispatch({ type: "addPlaylistTrack" })
    }
    render(<PlaylistPanel />)
    const headers = screen.getByRole("group", { name: "Tracks" })
    expect(within(headers).getAllByRole("group").length).toBeLessThan(20)

    const metrics = activeMetrics()!
    act(() => {
      metrics.setLimits({ rowCount: 208 })
      metrics.setViewport({ ...metrics.viewport, scrollRow: 100.5 })
    })
    expect(
      within(headers).getByRole("group", { name: "Track 101" })
    ).toHaveStyle({ top: "3800px" })
    expect(headers.firstElementChild).toHaveStyle({
      transform: "translateY(-3819px)",
    })
  })

  it("adds a track from the corner button", async () => {
    render(<PlaylistPanel />)
    fireEvent.click(screen.getByRole("button", { name: "Add playlist track" }))
    await flush()
    expect(tracks()).toHaveLength(1)
  })
})

describe("toolbar", () => {
  it("switches tools", async () => {
    render(<PlaylistPanel />)
    const tools = screen.getByRole("group", { name: "Tools" })
    const paint = within(tools).getByRole("button", { name: "Paint tool" })
    expect(
      within(tools).getByRole("button", { name: "Draw tool" })
    ).toHaveAttribute("aria-pressed", "true")
    fireEvent.click(paint)
    await flush()
    expect(ui().tool).toBe("paint")
    expect(paint).toHaveAttribute("aria-pressed", "true")
  })

  it("says the transport is looping the pattern and offers to play the song", async () => {
    render(<PlaylistPanel />)
    await seed([0, 0, 8 * BAR])
    expect(screen.getByText("Play loops the pattern")).toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", { name: "Play song" }))
    await flush()
    expect(useTransportStore.getState()).toMatchObject({
      mode: "song",
      playing: true,
    })
    expect(screen.queryByRole("button", { name: "Play song" })).toBeNull()
    expect(screen.getByText("Playing from the playlist")).toBeInTheDocument()
  })

  it("switches the mode with the same control as the transport bar", async () => {
    render(<PlaylistPanel />)
    fireEvent.click(screen.getByRole("button", { name: "Song" }))
    await flush()
    expect(useTransportStore.getState().mode).toBe("song")
  })

  it("toggles looping the song and following the playhead", async () => {
    render(<PlaylistPanel />)
    const loop = screen.getByRole("button", { name: "Loop the song" })
    const wasLooping = useTransportStore.getState().loopSong
    expect(loop).toHaveAttribute("aria-pressed", String(wasLooping))
    fireEvent.click(loop)
    await flush()
    expect(useTransportStore.getState().loopSong).toBe(!wasLooping)
    expect(loop).toHaveAttribute("aria-pressed", String(!wasLooping))

    fireEvent.click(screen.getByRole("button", { name: "Follow the playhead" }))
    await flush()
    expect(ui().follow).toBe(false)
  })

  it("explains the tool under the pointer in the status bar", async () => {
    render(<PlaylistPanel />)
    fireEvent.pointerEnter(screen.getByRole("button", { name: "Mute tool" }))
    expect(useHintStore.getState().text).toMatch(/^Mute tool \(M\): /)
  })
})

describe("ruler", () => {
  const ruler = () => screen.getByRole("slider", { name: "Song position" })

  it("seeks the transport to the snapped place that was clicked", async () => {
    render(<PlaylistPanel />)
    await act(() => setPlayMode("song"))
    const seek = vi.spyOn(backend, "transportSeek")
    fireEvent.pointerDown(ruler(), {
      pointerId: 1,
      button: 0,
      clientX: xOfBar(4) + 20,
    })
    await flush()
    expect(seek).toHaveBeenCalledWith(4 * BAR)
  })

  it("seeks off the grid with Shift", async () => {
    render(<PlaylistPanel />)
    await act(() => setPlayMode("song"))
    const seek = vi.spyOn(backend, "transportSeek")
    fireEvent.pointerDown(ruler(), {
      pointerId: 1,
      button: 0,
      clientX: 100,
      shiftKey: true,
    })
    await flush()
    expect(seek).toHaveBeenCalledWith(Math.round((100 / 72) * BAR))
  })

  it("follows the snap setting", async () => {
    render(<PlaylistPanel />)
    await act(() => setPlayMode("song"))
    act(() => ui().setSnap("beat"))
    const seek = vi.spyOn(backend, "transportSeek")
    fireEvent.pointerDown(ruler(), {
      pointerId: 1,
      button: 0,
      clientX: xOfBar(2) + 20,
    })
    await flush()
    expect(seek).toHaveBeenCalledWith(2 * BAR + 960)
  })

  it("only sets where the song starts while the pattern is looping", async () => {
    render(<PlaylistPanel />)
    const seek = vi.spyOn(backend, "transportSeek")
    fireEvent.pointerDown(ruler(), {
      pointerId: 1,
      button: 0,
      clientX: xOfBar(6),
    })
    await flush()
    expect(seek).not.toHaveBeenCalled()
    expect(ui().cursorTick).toBe(6 * BAR)
    expect(ruler()).toHaveAttribute("aria-valuetext", "Bar 7")

    // Song mode switched on from anywhere starts the song there.
    await act(() => setPlayMode("song"))
    await flush()
    expect(seek).toHaveBeenCalledWith(6 * BAR)
  })

  it("ignores the right button, which opens its menu", async () => {
    render(<PlaylistPanel />)
    await act(() => setPlayMode("song"))
    const seek = vi.spyOn(backend, "transportSeek")
    fireEvent.pointerDown(ruler(), { pointerId: 1, button: 2, clientX: 300 })
    await flush()
    expect(seek).not.toHaveBeenCalled()
  })
})

describe("zoom", () => {
  it("fits the whole song in the view", async () => {
    render(<PlaylistPanel />)
    await seed([0, 0, 40 * BAR])
    const metrics = activeMetrics()!
    act(() => zoomToFit())
    const { pxPerTick, width, scrollTick } = metrics.viewport
    expect(scrollTick).toBe(0)
    expect(pxPerTick * 41 * BAR).toBeCloseTo(width)
    expect(clips()).toHaveLength(1)
  })

  it("has no panel to zoom once it is gone", () => {
    const { unmount } = render(<PlaylistPanel />)
    expect(activeMetrics()).not.toBeNull()
    unmount()
    expect(activeMetrics()).toBeNull()
  })
})
