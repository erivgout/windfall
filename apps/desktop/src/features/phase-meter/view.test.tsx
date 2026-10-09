import { Profiler } from "react"
import { act, cleanup, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, expect, it, vi } from "vitest"

import type { RealtimeFrame } from "@/bindings"
import { announceProjectReplaced } from "@/lib/store/replaced"

import { PhaseMeterView } from "./view"

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

function paint(correlation?: number) {
  act(() => {
    feed.receive?.({
      ...(correlation === undefined ? {} : { correlation }),
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

function indicator() {
  return screen.getByRole("meter").querySelectorAll("span")[1]
}

it("shows absent input as unavailable and clears a previous correlation", () => {
  render(<PhaseMeterView />)
  paint()
  expect(screen.getByText("No correlation available")).toBeVisible()
  expect(screen.getByRole("meter")).not.toHaveAttribute("aria-valuenow")
  expect(indicator()).not.toBeVisible()
  paint(1)
  paint()
  expect(screen.getByText("No correlation available")).toBeVisible()
  expect(indicator()).not.toBeVisible()
  expect(screen.getByRole("meter")).not.toHaveAttribute("aria-valuenow")
})

it.each([
  [1, "1.00", "100%"],
  [-1, "-1.00", "0%"],
  [0, "0.00", "50%"],
])(
  "draws correlation %s at its horizontal position",
  (value, text, position) => {
    render(<PhaseMeterView />)
    paint(value)
    expect(screen.getByText("Side · −1")).toBeVisible()
    expect(screen.getByText("Mid · +1")).toBeVisible()
    expect(screen.getByText(text)).toBeVisible()
    expect(screen.getByRole("meter")).toHaveAttribute(
      "aria-valuenow",
      String(value)
    )
    expect(screen.getByRole("meter")).toHaveAttribute("aria-valuetext", text)
    expect(indicator()).toBeVisible()
    expect(indicator()).toHaveStyle({ left: position })
  }
)

it("updates meter and text without React renders", () => {
  const commits = vi.fn()
  render(
    <Profiler id="phase-meter" onRender={commits}>
      <PhaseMeterView />
    </Profiler>
  )
  const initialCommits = commits.mock.calls.length
  for (const value of [1, -1, 0, 0.5, undefined]) paint(value)
  expect(commits).toHaveBeenCalledTimes(initialCommits)
})

it("suppresses nonfinite and out of range input", () => {
  render(<PhaseMeterView />)
  for (const value of [NaN, Infinity, -Infinity, -1.01, 1.01]) {
    paint(1)
    paint(value)
    expect(screen.getByText("No correlation available")).toBeVisible()
    expect(indicator()).not.toBeVisible()
    expect(screen.getByRole("meter")).not.toHaveAttribute("aria-valuenow")
  }
})

it("clears correlation on project replacement", () => {
  render(<PhaseMeterView />)
  paint(1)
  act(() => {
    announceProjectReplaced()
    draw?.(0)
  })
  expect(screen.getByText("No correlation available")).toBeVisible()
  expect(indicator()).not.toBeVisible()
})

it("drops the realtime subscription and animation on unmount", () => {
  const mounted = render(<PhaseMeterView />)
  expect(feed.receive).not.toBeNull()
  paint(1)
  mounted.unmount()
  expect(feed.stop).toHaveBeenCalledTimes(1)
  expect(cancel).toHaveBeenCalledWith(1)
})
