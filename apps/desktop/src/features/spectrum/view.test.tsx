import { Profiler } from "react"
import { act, cleanup, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, expect, it, vi } from "vitest"

import type { RealtimeFrame } from "@/bindings"
import { announceProjectReplaced } from "@/lib/store/replaced"

import { SpectrumView } from "./view"

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

function paint(spectrum?: number[]) {
  act(() => {
    feed.receive?.({
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

function bars() {
  return [...screen.getByRole("img").querySelectorAll("rect")]
}

it("draws a quiet state for empty or omitted spectrum and clears old bars", () => {
  render(<SpectrumView />)
  paint(Array(64).fill(1))
  paint([])
  expect(screen.getByText("No spectrum available")).toBeVisible()
  expect(bars().every((bar) => bar.style.display === "none")).toBe(true)
  paint()
  expect(screen.getByText("No spectrum available")).toBeVisible()
  paint(Array(64).fill(0))
  expect(screen.getByText("Quiet")).toBeVisible()
  expect(bars().every((bar) => bar.getAttribute("height") === "0")).toBe(true)
})

it("paints one bar per bin for a known 64-bin power fixture without React renders", () => {
  const commits = vi.fn()
  render(
    <Profiler id="spectrum" onRender={commits}>
      <SpectrumView />
    </Profiler>
  )
  const initialCommits = commits.mock.calls.length
  paint(Array.from({ length: 64 }, (_, index) => 10 ** ((-90 + index) / 10)))
  expect(bars().filter((bar) => bar.style.display !== "none")).toHaveLength(64)
  bars().forEach((bar, index) => {
    expect(Number(bar.getAttribute("height"))).toBeCloseTo((index / 90) * 160)
    expect(Number(bar.getAttribute("x"))).toBe(index)
  })
  paint(Array(64).fill(0.01))
  expect(commits).toHaveBeenCalledTimes(initialCommits)
})

it("bounds the view and suppresses nonfinite or negative power", () => {
  render(<SpectrumView />)
  paint([NaN, Infinity, -1, ...Array(80).fill(1)])
  expect(bars()).toHaveLength(64)
  expect(
    bars()
      .slice(0, 3)
      .every((bar) => bar.style.display === "none")
  ).toBe(true)
  expect(
    bars()
      .slice(3)
      .every((bar) => bar.getAttribute("height") === "160")
  ).toBe(true)
})

it("clears spectrum on project replacement", () => {
  render(<SpectrumView />)
  paint(Array(64).fill(1))
  act(() => {
    announceProjectReplaced()
    draw?.(0)
  })
  expect(screen.getByText("No spectrum available")).toBeVisible()
})

it("drops the realtime subscription and animation on unmount", () => {
  const mounted = render(<SpectrumView />)
  expect(feed.receive).not.toBeNull()
  paint(Array(64).fill(1))
  mounted.unmount()
  expect(feed.stop).toHaveBeenCalledTimes(1)
  expect(cancel).toHaveBeenCalledWith(1)
})
