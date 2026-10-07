import { act, render, screen } from "@testing-library/react"
import { Profiler } from "react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { AutomationTarget, RealtimeFrame } from "@/bindings"
import { Fader, faderTaper, gainUnit, Knob } from "@/components/audio"
import { usePlaylistStore } from "@/features/playlist/store"
import { PositionReadout } from "@/features/transport/position-readout"
import { TempoField } from "@/features/transport/tempo-field"
import type { MockBackend } from "@/lib/ipc/mock"
import { useHintStore } from "@/lib/store/hint"
import { dispatch, receivePatch, useProjectStore } from "@/lib/store/project"
import { automatedValue } from "@/lib/store/realtime"
import { setPlayMode } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"
import { settle, startTestApp } from "@/test/harness"

import {
  automatedNow,
  automationFeed,
  useAutomation,
  useAutomationMarker,
} from "./live"
import { automationEntries } from "./menu"
import { NOTICE_MS, watchAutomatedEdits } from "./notice"

const FAKED: ("setTimeout" | "clearTimeout" | "performance")[] = [
  "setTimeout",
  "clearTimeout",
  "performance",
]

const QUIET: RealtimeFrame = {
  playing: true,
  tick: 0,
  meters: [],
  cpu: 0,
  xruns: 0,
  voices: 0,
  gainReductions: [],
  automated: [],
}

let stop: () => void
let backend: MockBackend
/** Hands a frame to the app as the engine would. */
let push: (frame: RealtimeFrame) => void
let pending: FrameRequestCallback[]

/** Runs one animation frame. */
function frame() {
  const due = pending
  pending = []
  act(() => {
    for (const callback of due) callback(0)
  })
}

const playing = (automation: number, value: number, tick = 0) =>
  push({ ...QUIET, tick, automated: [{ automation, value }] })

beforeEach(async () => {
  ;({ stop, backend } = await startTestApp())
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
})
afterEach(() => {
  stop()
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

const project = () => useProjectStore.getState().project
const kickTrack = () => project().mixer.tracks[1].id

/** Makes an automation of a target. Resolves to its id. */
async function automate(target: AutomationTarget): Promise<number> {
  const result = await backend.automate(target)
  await act(async () => {
    receivePatch(result.patch)
    await settle()
  })
  return result.created[0]
}

function TrackFader({ track, onRender }: { track: number; onRender(): void }) {
  const automation = useAutomation({ type: "trackVolume", track })
  return (
    <Profiler id="fader" onRender={onRender}>
      <Fader
        value={1}
        aria-label="Kick volume"
        showValue
        live={automation.live}
        marker={automation.marker}
      />
    </Profiler>
  )
}

describe("what automation is doing right now", () => {
  it("is the value in the target's own unit, with the automation's color", async () => {
    const target: AutomationTarget = { type: "trackVolume", track: kickTrack() }
    const id = await automate(target)
    const seen: (number | null)[] = []
    const colors: (string | undefined)[] = []
    const off = automationFeed(target)((value, color) => {
      seen.push(value)
      colors.push(color)
    })

    // Stopped, nothing has the fader in hand.
    push(QUIET)
    frame()
    expect(automatedNow(target)).toBeNull()
    expect(seen).toEqual([])

    playing(id, 0.5)
    frame()
    expect(automatedValue(id)).toBe(0.5)
    // Half way up the square taper is a quarter of +6 dB: 0.5.
    expect(seen).toEqual([0.5])
    expect(colors[0]).toMatch(/^#[0-9a-f]{6}$/)

    // The same value again says nothing new.
    playing(id, 0.5)
    frame()
    expect(seen).toEqual([0.5])
    playing(id, 1)
    frame()
    expect(seen).toEqual([0.5, 2])

    // The song stops: the fader is let go of, once.
    push({ ...QUIET, playing: false })
    frame()
    push({ ...QUIET, playing: false })
    frame()
    expect(seen).toEqual([0.5, 2, null])
    off()
  })

  it("belongs to the target, not to another one's automation", async () => {
    const pan: AutomationTarget = { type: "trackPan", track: kickTrack() }
    const fader: AutomationTarget = { type: "trackVolume", track: kickTrack() }
    const id = await automate(pan)
    const seen: (number | null)[] = []
    const off = automationFeed(fader)((value) => seen.push(value))
    playing(id, 0.75)
    frame()
    expect(seen).toEqual([])
    expect(automatedNow(pan)).toMatchObject({ value: 0.5 })
    off()
  })
})

describe("a fader that is automated", () => {
  it("moves with the song without React rendering it", async () => {
    const track = kickTrack()
    const id = await automate({ type: "trackVolume", track })
    const onRender = vi.fn()
    render(<TrackFader track={track} onRender={onRender} />)
    const fader = screen.getByRole("slider", { name: "Kick volume" })
    const root = fader.closest("[data-slot=fader]") as HTMLElement
    const cap = () =>
      root.querySelector<HTMLElement>("[data-slot=fader-live]") as HTMLElement
    const renders = onRender.mock.calls.length

    expect(cap().style.display).toBe("none")
    expect(root).not.toHaveAttribute("data-live")

    playing(id, Math.SQRT1_2)
    frame()
    expect(root).toHaveAttribute("data-live")
    expect(cap().style.display).toBe("")
    // 0 dB sits 78% of the way up the fader.
    expect(parseFloat(cap().style.bottom)).toBeCloseTo(78, 3)
    expect(
      root.querySelector("[data-slot=fader-live-value]")
    ).toHaveTextContent("0.0 dB")
    // The automation's color, from the feed.
    expect(cap().style.getPropertyValue("--live-color")).toMatch(/^#/)

    playing(id, 1)
    frame()
    expect(parseFloat(cap().style.bottom)).toBeCloseTo(100, 3)
    expect(
      root.querySelector("[data-slot=fader-live-value]")
    ).toHaveTextContent("+6.0 dB")
    // The stored value is still what the slider holds and edits.
    expect(fader).toHaveAttribute("aria-valuenow", "1")

    push({ ...QUIET, playing: false })
    frame()
    expect(root).not.toHaveAttribute("data-live")
    expect(cap().style.display).toBe("none")
    expect(fader).toHaveAttribute("aria-valuetext", "0.0 dB")

    // Sixty frames a second, and not one render.
    expect(onRender.mock.calls.length).toBe(renders)
  })

  it("tells a screen reader the live value, four times a second and without a render", async () => {
    // Not the animation frames: the test runs those by hand.
    vi.useFakeTimers({ toFake: FAKED })
    const track = kickTrack()
    const id = await automate({ type: "trackVolume", track })
    const onRender = vi.fn()
    render(<TrackFader track={track} onRender={onRender} />)
    const fader = screen.getByRole("slider", { name: "Kick volume" })
    const renders = onRender.mock.calls.length
    expect(fader).toHaveAttribute("aria-valuetext", "0.0 dB")

    playing(id, 1)
    frame()
    // Where the fader is, and what it is set to.
    expect(fader).toHaveAttribute(
      "aria-valuetext",
      "+6.0 dB, automated (set to 0.0 dB)"
    )
    // What the keys change is still the stored value.
    expect(fader).toHaveAttribute("aria-valuenow", "1")

    // Frames 16 ms apart do not each rewrite the text.
    const said = vi.spyOn(fader, "setAttribute")
    for (const value of [0.9, 0.8, Math.SQRT1_2]) {
      vi.advanceTimersByTime(16)
      playing(id, value)
      frame()
    }
    expect(said).not.toHaveBeenCalledWith("aria-valuetext", expect.anything())
    // A quarter of a second on, the newest value is said, once.
    vi.advanceTimersByTime(250)
    expect(
      said.mock.calls.filter(([name]) => name === "aria-valuetext")
    ).toEqual([["aria-valuetext", "0.0 dB, automated (set to 0.0 dB)"]])

    // The song stops: back to the stored readout.
    push({ ...QUIET, playing: false })
    frame()
    expect(fader).toHaveAttribute("aria-valuetext", "0.0 dB")
    expect(onRender.mock.calls.length).toBe(renders)
    vi.useRealTimers()
  })

  it("carries a marker while it has an automation, playing or not", async () => {
    const track = kickTrack()
    render(<TrackFader track={track} onRender={() => undefined} />)
    const marker = () => document.querySelector("[data-slot=fader-marker]")
    expect(marker()).toBeNull()
    const id = await automate({ type: "trackVolume", track })
    expect(marker()).not.toBeNull()
    await act(async () => {
      await dispatch({ type: "removeAutomation", id })
    })
    expect(marker()).toBeNull()
  })
})

describe("editing a control that automation is moving", () => {
  const notice = () => useHintStore.getState().notice
  const titles = (target: AutomationTarget) =>
    automationEntries(target).map((item) =>
      typeof item === "object" && "title" in item ? item.title : ""
    )
  const setVolume = (volume: number) =>
    act(async () => {
      await dispatch({
        type: "updateMixerTrack",
        id: kickTrack(),
        patch: { volume },
      })
      await settle()
    })

  it("says in the status bar what the edit sets, and does not stop it", async () => {
    // Not the animation frames: the test runs those by hand.
    vi.useFakeTimers({ toFake: FAKED })
    const target: AutomationTarget = { type: "trackVolume", track: kickTrack() }
    const id = await automate(target)
    const off = automationFeed(target)(() => undefined)
    const stopWatching = watchAutomatedEdits()

    // While the song is stopped the fader is the user's alone.
    await setVolume(0.5)
    expect(notice()).toBeNull()

    playing(id, 1)
    frame()
    await setVolume(0.25)
    expect(notice()).toBe(
      "Automated by Kick volume — this sets the value used outside its clips"
    )
    // The edit went through all the same.
    expect(project().mixer.tracks[1].volume).toBe(0.25)

    // An edit to something the song is not moving says nothing new.
    useHintStore.setState({ notice: null })
    await act(async () => {
      await dispatch({
        type: "updateMixerTrack",
        id: kickTrack(),
        patch: { pan: 0.5 },
      })
      await settle()
    })
    expect(notice()).toBeNull()

    // The notice goes by itself a while after the last such edit.
    await setVolume(0.75)
    expect(notice()).not.toBeNull()
    vi.advanceTimersByTime(NOTICE_MS - 1)
    expect(notice()).not.toBeNull()
    vi.advanceTimersByTime(1)
    expect(notice()).toBeNull()

    push({ ...QUIET, playing: false })
    frame()
    stopWatching()
    off()
    vi.useRealTimers()
  })

  it("offers the way to the automation first in the control's menu, while it plays", async () => {
    const target: AutomationTarget = { type: "trackVolume", track: kickTrack() }
    const id = await automate(target)
    const off = automationFeed(target)(() => undefined)
    expect(titles(target)).not.toContain("Edit the automation instead")

    playing(id, 0.5)
    frame()
    expect(titles(target)[0]).toBe("Edit the automation instead")
    // Not on a control the song leaves alone.
    expect(titles({ type: "trackPan", track: kickTrack() })).not.toContain(
      "Edit the automation instead"
    )

    // It goes to the clip on the playlist.
    usePlaylistStore.getState().clearSelection()
    const [first] = automationEntries(target)
    if (typeof first !== "object" || !("run" in first)) {
      throw new Error("no entry")
    }
    await first.run()
    const clip = project().playlist.clips.find(
      (item) =>
        item.content.type === "automation" && item.content.automation === id
    )!
    expect([...usePlaylistStore.getState().selection]).toEqual([clip.id])
    expect(useUiStore.getState().centerTab).toBe("playlist")

    push({ ...QUIET, playing: false })
    frame()
    expect(titles(target)).not.toContain("Edit the automation instead")
    off()
  })
})

describe("a knob that is automated", () => {
  function PanKnob({ track }: { track: number }) {
    const target: AutomationTarget = { type: "trackPan", track }
    const live = automationFeed(target)
    const marker = useAutomationMarker(target)
    return (
      <Knob
        value={0}
        min={-1}
        max={1}
        bipolar
        aria-label="Kick pan"
        live={live}
        marker={marker}
      />
    )
  }

  it("turns a second pointer to the automated value", async () => {
    const track = kickTrack()
    const id = await automate({ type: "trackPan", track })
    render(<PanKnob track={track} />)
    const root = document.querySelector("[data-slot=knob]") as HTMLElement
    const live = root.querySelector<SVGGElement>("[data-slot=knob-live]")!
    const needle = live.querySelector("g")!
    const arc = live.querySelector("path")!
    expect(live.style.display).toBe("none")
    expect(root.querySelector("[data-slot=knob-marker]")).not.toBeNull()

    // Hard right is the end of the sweep.
    playing(id, 1)
    frame()
    expect(root).toHaveAttribute("data-live")
    expect(live.style.display).toBe("")
    expect(needle.getAttribute("transform")).toBe("rotate(135.00 18 18)")
    // The arc grows from the center, which is half way along.
    expect(arc.style.strokeDasharray).toBe("0 0.5 0.5 1")

    playing(id, 0.25)
    frame()
    expect(needle.getAttribute("transform")).toBe("rotate(-67.50 18 18)")
    expect(arc.style.strokeDasharray).toBe("0 0.25 0.25 1")

    push({ ...QUIET, playing: false })
    frame()
    expect(root).not.toHaveAttribute("data-live")
    expect(live.style.display).toBe("none")
  })

  it("draws nothing live on a knob that was given no feed", () => {
    render(
      <Knob
        value={1}
        min={0}
        max={2}
        scale={faderTaper}
        aria-label="Plain"
        {...gainUnit}
      />
    )
    expect(document.querySelector("[data-slot=knob-live]")).toBeNull()
    expect(document.querySelector("[data-slot=knob-marker]")).toBeNull()
  })
})

describe("the tempo under automation", () => {
  it("follows the curve in the readout, and goes back to the stored tempo", async () => {
    render(<TempoField />)
    const field = screen.getByRole("spinbutton", {
      name: "Tempo in beats per minute",
    })
    expect(field).toHaveTextContent("128.00")
    expect(field.querySelector("[data-slot=tempo-marker]")).toBeNull()

    const id = await automate({ type: "tempo" })
    expect(field.querySelector("[data-slot=tempo-marker]")).not.toBeNull()

    // 0.25 of the way from 10 to 522 bpm.
    playing(id, 0.25)
    frame()
    expect(field).toHaveAttribute("data-live")
    expect(field.querySelector("[data-slot=tempo-live]")).toHaveTextContent(
      "138.00"
    )
    // What a drag would change is still the stored tempo.
    expect(field).toHaveAttribute("aria-valuenow", "128")

    push({ ...QUIET, playing: false })
    frame()
    expect(field).not.toHaveAttribute("data-live")
  })

  it("counts the song's time along the tempo curve", async () => {
    render(<PositionReadout />)
    const clock = () => screen.getByTitle("Minutes and seconds")
    const id = await automate({ type: "tempo" })
    // 64 bpm for the whole clip: half the stored 128.
    await act(async () => {
      await dispatch({
        type: "setAutomationPoints",
        id,
        points: [{ tick: 0, value: 54 / 512, curve: 0, hold: false }],
      })
      await setPlayMode("song")
    })

    // Four beats in. At the stored tempo that would be 1.87 s.
    playing(id, 54 / 512, 3840)
    frame()
    expect(clock()).toHaveTextContent("0:03.75")

    // A pattern loops at the stored tempo, whatever the song's curve says.
    await act(async () => {
      await setPlayMode("pattern")
    })
    push({ ...QUIET, tick: 3840 })
    frame()
    expect(clock()).toHaveTextContent("0:01.87")
  })
})
