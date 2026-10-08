import { act, render, screen } from "@testing-library/react"
import { afterEach, expect, it, vi } from "vitest"
import type { RealtimeFrame } from "@/bindings"
import { startTestApp } from "@/test/harness"
import { dispatch } from "@/lib/store/project"
import { setTransport } from "@/lib/store/transport"
import { PositionReadout } from "./position-readout"

const feed = vi.hoisted(() => ({
  draw: (frame: Readonly<RealtimeFrame>) => {
    void frame
  },
}))
vi.mock("@/lib/store/realtime", async (original) => ({
  ...(await original<typeof import("@/lib/store/realtime")>()),
  useRealtime: (draw: typeof feed.draw) => {
    feed.draw = draw
  },
}))
let stop: (() => void) | undefined
afterEach(() => stop?.())

it("uses song meter segments and keeps scalar pattern labels at the same tick", async () => {
  ;({ stop } = await startTestApp())
  await dispatch({
    type: "addMeterChange",
    tick: 4001,
    signature: { numerator: 7, denominator: 8 },
  })
  await dispatch({
    type: "addMeterChange",
    tick: 7400,
    signature: { numerator: 3, denominator: 4 },
  })
  await setTransport({ mode: "song" })
  render(<PositionReadout />)
  const frame: RealtimeFrame = {
    playing: false,
    tick: 7400,
    meters: [],
    cpu: 0,
    xruns: 0,
    voices: 0,
    gainReductions: [],
    automated: [],
    audioClips: 0,
    droppedClips: 0,
  }
  act(() => feed.draw(frame))
  expect(
    screen.getByRole("group", { name: "Song position" })
  ).toHaveTextContent("005:01:1")
  await act(async () => {
    await setTransport({ mode: "pattern" })
  })
  act(() => feed.draw(frame))
  expect(
    screen.getByRole("group", { name: "Song position" })
  ).toHaveTextContent("002:04:3")
})
