import { act, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { AutomationPoint, Command, RealtimeFrame } from "@/bindings"
import { TransportBar } from "@/features/transport/transport-bar"
import type { MockBackend } from "@/lib/ipc/mock"
import { dispatch, receivePatch, useProjectStore } from "@/lib/store/project"
import { subscribeRealtime } from "@/lib/store/realtime"
import { useTransportStore } from "@/lib/store/transport"
import { MAX_SONG_TICKS } from "@/lib/units"
import { settle, startTestApp } from "@/test/harness"

import { useAutomationRecordStore, watchAutomationRecording } from "./record"

const QUIET: RealtimeFrame = {
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
}
const point = (tick: number, value: number): AutomationPoint => ({
  tick,
  value,
  curve: 0,
  hold: false,
})
const original: AutomationPoint[] = [
  { ...point(0, 0.1), curve: 0.3, hold: true },
  point(200, 0.8),
]

let stop: () => void
let backend: MockBackend
let stopWatching: () => void
let stopRealtime: () => void
let push: (frame: RealtimeFrame) => void
let pending: FrameRequestCallback[]
let automation: number
let clip: number
let sent: Command[]

const project = () => useProjectStore.getState().project
const kickTrack = () => project().mixer.tracks[1].id
const points = () =>
  project().automations.find((item) => item.id === automation)!.points
const writes = () =>
  sent.filter((command) => command.type === "setAutomationPoints")

/** Hands a frame to the app and draws it, as in live.test.tsx. */
function playing(tick = 120.9) {
  push({ ...QUIET, tick, automated: [{ automation, value: 0.9 }] })
  const due = pending
  pending = []
  act(() => {
    for (const callback of due) callback(0)
  })
}

async function editVolume(volume = 0.125) {
  await act(async () => {
    await dispatch({
      type: "updateMixerTrack",
      id: kickTrack(),
      patch: { volume },
    })
    await settle()
  })
}

beforeEach(async () => {
  ;({ stop, backend } = await startTestApp())
  useAutomationRecordStore.setState({ recordArmed: false })
  pending = []
  push = () => {
    throw new Error("Nothing has subscribed to the realtime feed")
  }
  vi.spyOn(backend, "subscribeRealtime").mockImplementation((handler) => {
    push = handler
    return () => {}
  })
  vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
    pending.push(callback)
    return pending.length
  })
  vi.stubGlobal("cancelAnimationFrame", () => {
    pending = []
  })
  stopRealtime = subscribeRealtime(() => undefined)
  const result = await backend.automate({
    type: "trackVolume",
    track: kickTrack(),
  })
  receivePatch(result.patch)
  ;[automation, , clip] = result.created
  await dispatch({
    type: "setAutomationPoints",
    id: automation,
    points: original,
  })
  await dispatch({
    type: "updateClips",
    updates: [{ id: clip, patch: { start: 100, length: 100, offset: 30 } }],
  })
  await settle()
  sent = []
  const send = backend.dispatch.bind(backend)
  vi.spyOn(backend, "dispatch").mockImplementation((command, gesture) => {
    sent.push(command)
    return send(command, gesture)
  })
  useTransportStore.setState({ playing: true, mode: "song" })
  stopWatching = watchAutomationRecording()
})

afterEach(() => {
  stopWatching()
  stopRealtime()
  stop()
  useAutomationRecordStore.setState({ recordArmed: false })
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

describe("recording committed automation edits", () => {
  it("writes one normalized stored value at the floored clip curve tick, keeping other points", async () => {
    useAutomationRecordStore.setState({ recordArmed: true })
    playing()
    await editVolume()
    // Song tick 120 - start 100 + offset 30; square gain range 0..2.
    const expected = [original[0], point(50, 0.25), original[1]]
    expect(writes()).toEqual([
      { type: "setAutomationPoints", id: automation, points: expected },
    ])
    expect(points()).toEqual(expected)
    expect(project().mixer.tracks[1].volume).toBe(0.125)
  })

  it("replaces every point at the recorded tick with one straight, non-held point", async () => {
    await dispatch({
      type: "setAutomationPoints",
      id: automation,
      points: [
        original[0],
        { ...point(50, 0.6), curve: 0.4, hold: true },
        point(50, 0.7),
        original[1],
      ],
    })
    sent = []
    useAutomationRecordStore.setState({ recordArmed: true })
    playing()
    await editVolume()
    expect(writes()).toHaveLength(1)
    expect(points()).toEqual([original[0], point(50, 0.25), original[1]])
  })

  it.each(["disarmed", "stopped", "no covering clip"])(
    "sends no curve command when %s",
    async (condition) => {
      useAutomationRecordStore.setState({
        recordArmed: condition !== "disarmed",
      })
      if (condition === "stopped")
        useTransportStore.setState({ playing: false })
      playing(condition === "no covering clip" ? 200 : 120.9)
      await editVolume()
      expect(writes()).toEqual([])
      expect(points()).toEqual(original)
      expect(project().mixer.tracks[1].volume).toBe(0.125)
    }
  )

  it("uses the covering clip with the earliest start, regardless of playlist order", async () => {
    const track = project().playlist.clips.find(
      (item) => item.id === clip
    )!.track
    await dispatch({
      type: "addClips",
      clips: [
        {
          track,
          start: 90,
          length: 100,
          offset: 5,
          content: { type: "automation", automation },
        },
      ],
    })
    sent = []
    useAutomationRecordStore.setState({ recordArmed: true })
    playing()
    await editVolume()
    expect(writes()).toHaveLength(1)
    expect(points()).toEqual([original[0], point(35, 0.25), original[1]])
  })

  it("includes the clip start", async () => {
    useAutomationRecordStore.setState({ recordArmed: true })
    playing(100)
    await editVolume()
    expect(writes()).toHaveLength(1)
    expect(points()).toEqual([original[0], point(30, 0.25), original[1]])
  })

  it("does not write when the curve tick exceeds MAX_SONG_TICKS", async () => {
    await dispatch({
      type: "updateClips",
      updates: [{ id: clip, patch: { offset: MAX_SONG_TICKS } }],
    })
    sent = []
    useAutomationRecordStore.setState({ recordArmed: true })
    playing()
    await editVolume()
    expect(writes()).toEqual([])
  })

  it("does not send an unchanged point list", async () => {
    await dispatch({
      type: "setAutomationPoints",
      id: automation,
      points: [original[0], point(50, 0.25), original[1]],
    })
    sent = []
    useAutomationRecordStore.setState({ recordArmed: true })
    playing()
    await editVolume()
    expect(writes()).toEqual([])
  })

  it("does not record realtime frames without a committed control edit", () => {
    useAutomationRecordStore.setState({ recordArmed: true })
    playing()
    playing(130)
    expect(writes()).toEqual([])
  })

  it("clears the session arm on project replacement", async () => {
    useAutomationRecordStore.setState({ recordArmed: true })
    await act(async () => {
      await backend.projectNew()
      await settle()
    })
    expect(useAutomationRecordStore.getState().recordArmed).toBe(false)
    expect(writes()).toEqual([])
  })

  it("offers a separate transport button whose pressed state follows the arm and starts recording", async () => {
    // The mounted transport starts its own watcher.
    stopWatching()
    render(<TransportBar />)
    const button = screen.getByRole("button", { name: "Record automation" })
    expect(button).toHaveAttribute("aria-pressed", "false")
    expect(screen.getByRole("button", { name: "Record" })).toBeVisible()
    fireEvent.click(button)
    expect(button).toHaveAttribute("aria-pressed", "true")
    expect(useAutomationRecordStore.getState().recordArmed).toBe(true)
    playing()
    await editVolume()
    expect(writes()).toHaveLength(1)
    expect(points()).toEqual([original[0], point(50, 0.25), original[1]])
    fireEvent.click(button)
    expect(button).toHaveAttribute("aria-pressed", "false")
  })
})
