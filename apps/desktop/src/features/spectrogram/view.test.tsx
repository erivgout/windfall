import { Profiler } from "react"
import { act, cleanup, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, expect, it, vi } from "vitest"

import type { RealtimeFrame } from "@/bindings"
import { announceProjectReplaced } from "@/lib/store/replaced"

import { SpectrogramView } from "./view"

const feed = vi.hoisted(() => ({
  receive: null as ((frame: RealtimeFrame) => void) | null,
  stop: vi.fn(),
}))
vi.mock("@/lib/ipc", () => ({
  backend: {
    subscribeRealtime: (receive: (frame: RealtimeFrame) => void) => {
      feed.receive = receive
      return feed.stop
    },
  },
}))

let draw: FrameRequestCallback | undefined
const cancel = vi.fn()

beforeEach(() => {
  feed.receive = null
  feed.stop.mockClear()
  cancel.mockClear()
  vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
    draw = callback
    return 1
  })
  vi.stubGlobal("cancelAnimationFrame", cancel)
})
afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
})

function paint(spectrogram?: number[], spectrum?: number[]) {
  act(() => {
    feed.receive?.({
      spectrogram,
      spectrum,
      playing: false,
      tick: 0,
      meters: [],
      cpu: 0,
      xruns: 0,
      voices: 0,
      audioClips: 0,
      droppedClips: 0,
      gainReductions: [],
      automated: [],
    })
    draw?.(0)
  })
}

function cells() {
  return [...screen.getByRole("img").querySelectorAll("rect")]
}

it("shows unavailable for empty or omitted frames and clears old cells", () => {
  render(<SpectrogramView />)
  expect(screen.getByText("No spectrogram available")).toBeVisible()
  paint(Array(64).fill(1))
  expect(screen.getByText("No spectrogram available")).not.toBeVisible()
  paint([])
  expect(screen.getByText("No spectrogram available")).toBeVisible()
  expect(cells().every((cell) => cell.style.display === "none")).toBe(true)
  paint(Array(64).fill(1))
  paint()
  expect(screen.getByText("No spectrogram available")).toBeVisible()
  expect(cells().every((cell) => cell.style.display === "none")).toBe(true)
})

it("paints a 2×4 row-major power fixture without rendering React per frame", () => {
  const commits = vi.fn()
  render(
    <Profiler id="spectrogram" onRender={commits}>
      <SpectrogramView />
    </Profiler>
  )
  const initialCommits = commits.mock.calls.length
  const powers = [0, 1e-9, 1e-6, 1e-3, 0.01, 0.1, 1, 10]
  const intensities = [0, 0, 1 / 3, 2 / 3, 7 / 9, 8 / 9, 1, 1]
  paint(powers, [0, 0, 0, 0])
  const visible = cells().filter((cell) => cell.style.display !== "none")
  expect(visible).toHaveLength(8)
  visible.forEach((cell, index) => {
    expect(Number(cell.getAttribute("x"))).toBe((index % 4) * 16)
    expect(Number(cell.getAttribute("y"))).toBe(Math.floor(index / 4) * 8)
    expect(Number(cell.getAttribute("width"))).toBe(16)
    expect(Number(cell.getAttribute("height"))).toBe(8)
    expect(Number(cell.getAttribute("fill-opacity"))).toBeCloseTo(
      intensities[index]
    )
  })
  paint(powers, [0, 0, 0, 0])
  expect(commits).toHaveBeenCalledTimes(initialCommits)
})

it("bounds rows, hides invalid power, and rejects incomplete rows", () => {
  render(<SpectrogramView />)
  paint([...Array(64).fill(0), ...Array(16 * 64).fill(1)])
  expect(cells()).toHaveLength(16 * 64)
  expect(
    cells().every((cell) => cell.getAttribute("fill-opacity") === "1")
  ).toBe(true)
  paint([NaN, Infinity, -1, 0], [0, 0, 0, 0])
  expect(cells().filter((cell) => cell.style.display !== "none")).toHaveLength(
    1
  )
  expect(screen.getByText("No spectrogram available")).not.toBeVisible()
  paint([1, 1, 1], [0, 0, 0, 0])
  expect(screen.getByText("No spectrogram available")).toBeVisible()
})

it("draws older valid rows with the default width when spectrum is unavailable", () => {
  render(<SpectrogramView />)
  paint(Array(128).fill(0))
  expect(cells().filter((cell) => cell.style.display !== "none")).toHaveLength(
    128
  )
  expect(screen.getByText("No spectrogram available")).not.toBeVisible()
  expect(cells()[64].getAttribute("y")).toBe("8")
})

it("clears history on project replacement", () => {
  render(<SpectrogramView />)
  paint(Array(64).fill(1))
  act(() => {
    announceProjectReplaced()
    draw?.(0)
  })
  expect(screen.getByText("No spectrogram available")).toBeVisible()
  expect(cells().every((cell) => cell.style.display === "none")).toBe(true)
})

it("drops the subscription and animation on unmount", () => {
  const mounted = render(<SpectrogramView />)
  expect(feed.receive).not.toBeNull()
  mounted.unmount()
  expect(feed.stop).toHaveBeenCalledTimes(1)
  expect(cancel).toHaveBeenCalledWith(1)
})
