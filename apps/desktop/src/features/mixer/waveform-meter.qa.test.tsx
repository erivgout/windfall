import { act, render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, expect, it, vi } from "vitest"
import type { RealtimeFrame } from "@/bindings"
import { useUiStore } from "@/lib/store/ui"
import { useProjectStore } from "@/lib/store/project"
import { startTestApp } from "@/test/harness"
import { LevelSection } from "./level-section"
import { MixerToolbar } from "./toolbar"
import { StripWaveform } from "./waveform-meter"
import { trackNamed } from "./test-utils"

const feed = vi.hoisted(() => new Set<(frame: RealtimeFrame) => void>())
vi.mock("@/lib/store/realtime", async (original) => ({
  ...(await original<typeof import("@/lib/store/realtime")>()),
  subscribeRealtime: (listener: (frame: RealtimeFrame) => void) => {
    feed.add(listener)
    return () => feed.delete(listener)
  },
}))

const context = {
  setTransform: vi.fn(),
  clearRect: vi.fn(),
  beginPath: vi.fn(),
  moveTo: vi.fn(),
  lineTo: vi.fn(),
  stroke: vi.fn(),
  fillRect: vi.fn(),
  lineWidth: 1,
  strokeStyle: "",
  fillStyle: "",
}
let app: Awaited<ReturnType<typeof startTestApp>>

beforeEach(async () => {
  feed.clear()
  Object.values(context).forEach((value) => {
    if (typeof value === "function") value.mockClear()
  })
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(
    context as unknown as CanvasRenderingContext2D
  )
  vi.spyOn(HTMLCanvasElement.prototype, "clientWidth", "get").mockReturnValue(
    64
  )
  vi.spyOn(HTMLCanvasElement.prototype, "clientHeight", "get").mockReturnValue(
    32
  )
  app = await startTestApp()
})
afterEach(() => {
  app.stop()
  vi.restoreAllMocks()
})

it("the mounted toolbar swaps a real level section to waveform and clears native interest on return", async () => {
  const user = userEvent.setup()
  const interest = vi.spyOn(app.backend, "mixerWaveformTracks")
  const track = trackNamed("Kick")
  const view = render(
    <>
      <MixerToolbar filter="" onFilterChange={() => {}} />
      <LevelSection
        id={track.id}
        name={track.name}
        volume={track.volume}
        index={1}
        metering
        layout="tall"
        wide
      />
    </>
  )
  expect(
    screen.queryByRole("button", { name: /^Stereo waveform history/ })
  ).not.toBeInTheDocument()
  await user.click(screen.getByRole("button", { name: "Levels" }))
  expect(screen.getByRole("button", { name: "Waveforms" })).toHaveAttribute(
    "aria-pressed",
    "true"
  )
  expect(
    screen.getByRole("button", { name: /^Stereo waveform history/ })
  ).toBeInTheDocument()
  await waitFor(() =>
    expect(interest).toHaveBeenLastCalledWith(
      [track.id],
      expect.any(Number),
      useProjectStore.getState().revision
    )
  )
  await user.click(screen.getByRole("button", { name: "Waveforms" }))
  expect(screen.getByRole("button", { name: "Levels" })).toHaveAttribute(
    "aria-pressed",
    "false"
  )
  expect(
    screen.queryByRole("button", { name: /^Stereo waveform history/ })
  ).not.toBeInTheDocument()
  await waitFor(() =>
    expect(interest).toHaveBeenLastCalledWith(
      [],
      expect.any(Number),
      useProjectStore.getState().revision
    )
  )
  view.unmount()
})

it("draws stereo histories, suppresses unchanged serials, clears missing frames and releases inactive/unmounted interests", async () => {
  const track = trackNamed("Kick")
  act(() => useUiStore.getState().setMixerMeterMode("waveform"))
  const interest = vi.spyOn(app.backend, "mixerWaveformTracks")
  const view = render(
    <StripWaveform id={track.id} vertical={false} wide active />
  )
  await waitFor(() =>
    expect(interest).toHaveBeenLastCalledWith(
      [track.id],
      expect.any(Number),
      useProjectStore.getState().revision
    )
  )
  const frame: RealtimeFrame = {
    playing: true,
    tick: 0,
    meters: [],
    cpu: 0,
    xruns: 0,
    voices: 0,
    gainReductions: [],
    automated: [],
    audioClips: 0,
    droppedClips: 0,
    waveforms: [
      {
        track: track.id,
        epoch: 1n,
        serial: 1n,
        sampleRate: 48000,
        bucketFrames: 240,
        points: [[-0.5, 0.75, -0.25, 1.25]],
      },
    ],
  }
  context.moveTo.mockClear()
  context.lineTo.mockClear()
  context.clearRect.mockClear()
  act(() => {
    for (const listener of feed) listener(frame)
  })
  expect(context.moveTo).toHaveBeenCalledWith(63.5, 11.75)
  expect(context.lineTo).toHaveBeenCalledWith(63.5, 2.375)
  expect(context.moveTo).toHaveBeenCalledWith(63.5, 25.875)
  expect(context.lineTo).toHaveBeenCalledWith(63.5, 16.5)
  const draws = context.clearRect.mock.calls.length
  act(() => {
    for (const listener of feed) listener(frame)
  })
  expect(context.clearRect).toHaveBeenCalledTimes(draws)
  act(() => {
    for (const listener of feed) listener({ ...frame, waveforms: [] })
  })
  expect(context.clearRect).toHaveBeenCalledTimes(draws + 1)
  view.rerender(<StripWaveform id={track.id} vertical wide active={false} />)
  await waitFor(() =>
    expect(interest).toHaveBeenLastCalledWith(
      [],
      expect.any(Number),
      useProjectStore.getState().revision
    )
  )
  expect(feed.size).toBe(0)
  view.rerender(<StripWaveform id={track.id} vertical wide active />)
  await waitFor(() =>
    expect(interest).toHaveBeenLastCalledWith(
      [track.id],
      expect.any(Number),
      useProjectStore.getState().revision
    )
  )
  view.unmount()
  await waitFor(() =>
    expect(interest).toHaveBeenLastCalledWith(
      [],
      expect.any(Number),
      useProjectStore.getState().revision
    )
  )
  expect(feed.size).toBe(0)
})
