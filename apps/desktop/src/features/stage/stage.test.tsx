import { Profiler } from "react"
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, expect, it, vi } from "vitest"

import type { RealtimeFrame } from "@/bindings"
import { registry } from "@/lib/actions"
import { backend } from "@/lib/ipc"
import { useProjectStore } from "@/lib/store/project"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { useTransportStore } from "@/lib/store/transport"
import { formatMusicalPosition } from "@/lib/timeline"
import { formatClock, ticksToSeconds } from "@/lib/time"
import { PositionReadout } from "@/features/transport/position-readout"

import { closeStageView, registerStageActions } from "./actions"
import { LargeClock } from "./large-clock"
import { LargeMasterMeter } from "./large-master-meter"
import { StageViews } from "./views"

const fakeBackend = vi.hoisted(() => ({
  subscribeRealtime:
    vi.fn<(listener: (frame: RealtimeFrame) => void) => () => void>(),
}))
vi.mock("@/lib/ipc", () => ({ backend: fakeBackend }))

let receive: ((frame: RealtimeFrame) => void) | undefined
let frames: Map<number, FrameRequestCallback>
let stopFeed: ReturnType<typeof vi.fn<() => void>>
let unregister: (() => void) | undefined

beforeEach(() => {
  receive = undefined
  frames = new Map()
  let nextFrame = 0
  stopFeed = vi.fn(() => {
    receive = undefined
  })
  fakeBackend.subscribeRealtime.mockReset().mockImplementation((listener) => {
    receive = listener
    return stopFeed
  })
  vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
    const id = ++nextFrame
    frames.set(id, callback)
    return id
  })
  vi.spyOn(window, "cancelAnimationFrame").mockImplementation((id) => {
    frames.delete(id)
  })
  useProjectStore.setState(useProjectStore.getInitialState(), true)
  useTransportStore.setState(useTransportStore.getInitialState(), true)
})

afterEach(() => {
  cleanup()
  unregister?.()
  unregister = undefined
  closeStageView()
  vi.restoreAllMocks()
})

function draw(tick: number, meters: number[] = []) {
  act(() => {
    receive?.({
      tick,
      meters,
      playing: true,
      cpu: 0,
      xruns: 0,
      voices: 0,
      gainReductions: [],
      automated: [],
      audioClips: 0,
      droppedClips: 0,
    })
    const callbacks = [...frames.values()]
    frames.clear()
    for (const callback of callbacks) callback(16)
  })
}

it("writes the known musical position and clock time without rendering React each frame", () => {
  const signature = { numerator: 7, denominator: 8 }
  const tempoBpm = 90
  const project = useProjectStore.getState().project
  useProjectStore.setState({
    project: {
      ...project,
      settings: { ...project.settings, tempoBpm, timeSignature: signature },
    },
  })
  const commits = vi.fn()
  render(
    <Profiler id="clock" onRender={commits}>
      <LargeClock />
    </Profiler>
  )
  draw(7400)
  expect(screen.getByTitle("Bar, beat and step")).toHaveTextContent(
    formatMusicalPosition(7400, signature, [])
  )
  expect(screen.getByTitle("Minutes and seconds")).toHaveTextContent(
    formatClock(ticksToSeconds(7400, tempoBpm))
  )
  draw(7640)
  expect(screen.getByTitle("Bar, beat and step")).toHaveTextContent(
    formatMusicalPosition(7640, signature, [])
  )
  expect(commits).toHaveBeenCalledTimes(1)
})

it("matches the small readout when song meters and pattern overrides change", () => {
  const project = useProjectStore.getState().project
  const songSignature = { numerator: 4, denominator: 4 }
  const songMeters = [
    { id: 1, tick: 4001, signature: { numerator: 7, denominator: 8 } },
  ]
  const patternSignature = { numerator: 3, denominator: 4 }
  const patternMeters = [
    { id: 2, tick: 2880, signature: { numerator: 5, denominator: 8 } },
  ]
  useProjectStore.setState({
    project: {
      ...project,
      settings: {
        ...project.settings,
        tempoBpm: 120,
        timeSignature: songSignature,
      },
      playlist: {
        ...project.playlist,
        timeline: { meters: songMeters, markers: [] },
      },
      patterns: [
        {
          id: 1,
          name: "Test",
          color: 0,
          lengthSteps: 64,
          lanes: [],
          timeSignature: patternSignature,
          timeline: { meters: patternMeters, markers: [] },
        },
      ],
    },
  })
  useTransportStore.setState({ mode: "song", pattern: 1 })
  render(
    <>
      <LargeClock />
      <PositionReadout />
    </>
  )
  draw(7400)
  const [largePosition, smallPosition] =
    screen.getAllByTitle("Bar, beat and step")
  const [largeTime, smallTime] = screen.getAllByTitle("Minutes and seconds")
  expect(largePosition.textContent).toBe(
    formatMusicalPosition(7400, songSignature, songMeters)
  )
  expect(largePosition.textContent).toBe(smallPosition.textContent)
  expect(largeTime.textContent).toBe(smallTime.textContent)
  useTransportStore.setState({ mode: "pattern" })
  draw(7400)
  expect(largePosition.textContent).toBe(
    formatMusicalPosition(7400, patternSignature, patternMeters)
  )
  expect(largePosition.textContent).toBe(smallPosition.textContent)
  expect(largeTime.textContent).toBe(smallTime.textContent)
})

it("removes the clock realtime subscription and animation on unmount", () => {
  const { unmount } = render(<LargeClock />)
  expect(backend.subscribeRealtime).toHaveBeenCalledTimes(1)
  unmount()
  expect(stopFeed).toHaveBeenCalledTimes(1)
  expect(frames.size).toBe(0)
})

it("renders distinct left and right master levels and latches clipping until clicked", () => {
  const commits = vi.fn()
  const { container, unmount } = render(
    <Profiler id="meter" onRender={commits}>
      <LargeMasterMeter />
    </Profiler>
  )
  const meter = container.querySelector('[data-slot="level-meter"]')
  expect(meter).toHaveAttribute("data-orientation", "vertical")
  expect(backend.subscribeRealtime).toHaveBeenCalledTimes(1)
  // Other tracks cannot affect the master display.
  draw(0, [0.5, 0.25, 2, 2])
  expect(
    screen.getByRole("group", { name: "Left peak level" })
  ).toHaveTextContent("-6.0 dBFS")
  expect(
    screen.getByRole("group", { name: "Right peak level" })
  ).toHaveTextContent("-12.0 dBFS")
  expect(meter).not.toHaveAttribute("data-clipped")
  draw(0, [0.1, 1.2])
  expect(meter).toHaveAttribute("data-clipped")
  draw(0, [0, 0])
  expect(meter).toHaveAttribute("data-clipped")
  expect(
    screen.getByRole("group", { name: "Left peak level" })
  ).toHaveTextContent("−∞ dBFS")
  fireEvent.click(
    screen.getByRole("button", { name: "Clear master clip indicator" })
  )
  expect(meter).not.toHaveAttribute("data-clipped")
  expect(commits).toHaveBeenCalledTimes(1)
  unmount()
  expect(stopFeed).toHaveBeenCalledTimes(1)
  expect(frames.size).toBe(0)
})

it("resets master clipping and readouts when the project is replaced", () => {
  const { container } = render(<LargeMasterMeter />)
  draw(0, [1.2, 0.5])
  const meter = container.querySelector('[data-slot="level-meter"]')
  expect(meter).toHaveAttribute("data-clipped")
  act(() => announceProjectReplaced())
  expect(meter).not.toHaveAttribute("data-clipped")
  expect(
    screen.getByRole("group", { name: "Left peak level" })
  ).toHaveTextContent("−∞ dBFS")
  expect(
    screen.getByRole("group", { name: "Right peak level" })
  ).toHaveTextContent("−∞ dBFS")
})

it.each([
  ["view.largeClock", "Large clock"],
  ["view.largeMasterMeter", "Large master meter"],
])(
  "opens %s through the action registry and stops its feed on close",
  (id, title) => {
    unregister = registerStageActions()
    render(<StageViews />)
    expect(backend.subscribeRealtime).not.toHaveBeenCalled()
    const action = registry.get(id)
    expect(action?.title).toBe(title)
    act(() => {
      action?.run()
    })
    expect(screen.getByRole("dialog", { name: title })).toBeInTheDocument()
    expect(backend.subscribeRealtime).toHaveBeenCalledTimes(1)
    fireEvent.click(screen.getByRole("button", { name: "Close" }))
    expect(stopFeed).toHaveBeenCalledTimes(1)
  }
)
