import { act, fireEvent, render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { dbToGain } from "@/components/audio"
import { usePlaylistStore } from "@/features/playlist/store"
import { getAppState, isEnabled, registry, runAction } from "@/lib/actions"
import type { Backend } from "@/lib/ipc"
import { dispatch, redo, undo } from "@/lib/store/project"
import { usePromptStore } from "@/lib/store/prompts"
import { useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"
import { MASTER_TRACK } from "@/lib/units"
import { startTestApp } from "@/test/harness"

import MixerPanel from "."
import { useMixerUi } from "./mixer-ui"
import { setOutput } from "./operations"
import {
  channelNamed,
  drag,
  flush,
  history,
  project,
  strip,
  stubCanvas,
  trackNamed,
  tracks,
} from "./test-utils"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

const { toast } = await import("sonner")

let stop: () => void
let backend: Backend

beforeEach(async () => {
  stubCanvas()
  useMixerUi.setState(useMixerUi.getInitialState(), true)
  usePlaylistStore.setState(usePlaylistStore.getInitialState(), true)
  ;({ stop, backend } = await startTestApp())
})
afterEach(() => {
  stop()
  vi.restoreAllMocks()
})

const ui = () => useUiStore.getState()
const fader = (name: string) =>
  within(strip(name)).getByRole("slider", { name: `${name} volume` })
const panKnob = (name: string) =>
  within(strip(name)).getByRole("slider", { name: `${name} pan` })
const button = (name: string, label: string | RegExp) =>
  within(strip(name)).getByRole("button", { name: label })
const nameOf = (name: string) =>
  within(strip(name)).getByText(name, { selector: "[data-slot=strip-name]" })
const focus = (element: HTMLElement) => act(() => element.focus())
const selectTrack = (id: number) => act(() => ui().selectTrack(id))
const enabled = (id: string) => {
  const action = registry.get(id)
  if (!action) throw new Error(`"${id}" is not registered`)
  return isEnabled(action, getAppState())
}

describe("strips", () => {
  it("shows the master first and one strip per insert track, in order", () => {
    render(<MixerPanel />)
    const names = screen
      .getAllByRole("group", { name: /, (master track|track \d+)/ })
      .map((item) => item.getAttribute("aria-label"))
    expect(names).toEqual([
      "Master, master track",
      "Kick, track 1",
      "Clap, track 2",
      "Hat, track 3",
      "Snare, track 4",
    ])
    expect(strip("Master").closest("[data-slot=mixer-master]")).not.toBeNull()
    expect(strip("Kick").closest("[data-slot=mixer-dock-middle]")).not.toBeNull()
  })

  it("reads 0 dB at unity gain and centered pan", () => {
    render(<MixerPanel />)
    expect(fader("Kick")).toHaveAttribute("aria-valuetext", "0.0 dB")
    expect(fader("Kick")).toHaveAttribute("aria-valuenow", "1")
    expect(panKnob("Kick")).toHaveAttribute("aria-valuetext", "C")
    expect(within(strip("Kick")).getByText("0.0 dB")).toBeVisible()
  })
})

describe("fader and pan", () => {
  it("dispatches a fader drag as one undo step", async () => {
    render(<MixerPanel />)
    const before = history().entries.length

    // 0 dB sits at 78% of the travel and -12 dB at 48%.
    drag(fader("Kick"), [-8, -8, -14])
    await flush()

    expect(trackNamed("Kick").volume).toBeCloseTo(dbToGain(-12), 4)
    expect(fader("Kick")).toHaveAttribute("aria-valuetext", "−12.0 dB")
    expect(history().entries).toHaveLength(before + 1)
    expect(history().entries.at(-1)?.label).toBe("Change mixer track volume")

    await undo()
    await flush()
    expect(trackNamed("Kick").volume).toBe(1)
    expect(fader("Kick")).toHaveAttribute("aria-valuetext", "0.0 dB")

    await redo()
    await flush()
    expect(fader("Kick")).toHaveAttribute("aria-valuetext", "−12.0 dB")
  })

  it("starts a new undo step for the next drag", async () => {
    render(<MixerPanel />)
    const before = history().entries.length
    drag(fader("Kick"), [-8, -8])
    await flush()
    drag(fader("Kick"), [-14])
    await flush()
    expect(history().entries).toHaveLength(before + 2)
    expect(trackNamed("Kick").volume).toBeCloseTo(dbToGain(-12), 4)
  })

  it("shows the dragged value at once, before the backend answers", () => {
    render(<MixerPanel />)
    drag(fader("Kick"), [-16])
    expect(fader("Kick")).toHaveAttribute("aria-valuetext", "−6.0 dB")
    expect(trackNamed("Kick").volume).toBe(1)
  })

  it("never sends a value outside the fader's range", async () => {
    render(<MixerPanel />)
    drag(fader("Kick"), [500])
    await flush()
    expect(trackNamed("Kick").volume).toBe(2)
    expect(fader("Kick")).toHaveAttribute("aria-valuetext", "+6.0 dB")
    drag(fader("Kick"), [-900])
    await flush()
    expect(trackNamed("Kick").volume).toBe(0)
    expect(fader("Kick")).toHaveAttribute("aria-valuetext", "−∞ dB")
    expect(toast.error).not.toHaveBeenCalled()
  })

  it("dispatches a pan drag as one undo step", async () => {
    render(<MixerPanel />)
    const before = history().entries.length

    // A knob covers its range in 200 pixels.
    drag(panKnob("Clap"), [20, 30])
    await flush()

    expect(trackNamed("Clap").pan).toBeCloseTo(0.5, 5)
    expect(panKnob("Clap")).toHaveAttribute("aria-valuetext", "R50")
    expect(history().entries).toHaveLength(before + 1)
    expect(history().entries.at(-1)?.label).toBe("Change mixer track pan")

    await undo()
    await flush()
    expect(panKnob("Clap")).toHaveAttribute("aria-valuetext", "C")
  })

  it("moves the fader from the keyboard and resets it on double-click", async () => {
    render(<MixerPanel />)
    fireEvent.keyDown(fader("Hat"), { key: "End" })
    fireEvent.keyUp(fader("Hat"), { key: "End" })
    await flush()
    expect(trackNamed("Hat").volume).toBe(2)

    fireEvent.doubleClick(fader("Hat"))
    await flush()
    expect(trackNamed("Hat").volume).toBe(1)
  })

  it("follows edits that come from somewhere else", async () => {
    render(<MixerPanel />)
    await dispatch({
      type: "updateMixerTrack",
      id: trackNamed("Snare").id,
      patch: { volume: dbToGain(-6), pan: -1 },
    })
    await flush()
    expect(fader("Snare")).toHaveAttribute("aria-valuetext", "−6.0 dB")
    expect(panKnob("Snare")).toHaveAttribute("aria-valuetext", "L100")
  })
})

describe("mute and solo", () => {
  it("toggles mute and solo from the strip", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)

    await user.click(button("Kick", "Mute"))
    await flush()
    expect(trackNamed("Kick").muted).toBe(true)
    expect(button("Kick", "Mute")).toHaveAttribute("aria-pressed", "true")
    expect(strip("Kick")).toHaveAttribute("data-audible", "muted")

    await user.click(button("Kick", "Solo"))
    await flush()
    expect(trackNamed("Kick").solo).toBe(true)
    expect(button("Kick", "Solo")).toHaveAttribute("aria-pressed", "true")

    await user.click(button("Kick", "Mute"))
    await flush()
    expect(trackNamed("Kick").muted).toBe(false)
  })

  it("dims the strips a solo silences and keeps the master lit", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    await user.click(button("Clap", "Solo"))
    await flush()

    expect(strip("Clap")).toHaveAttribute("data-audible", "heard")
    expect(strip("Master")).toHaveAttribute("data-audible", "heard")
    for (const name of ["Kick", "Hat", "Snare"]) {
      expect(strip(name)).toHaveAttribute("data-audible", "silenced")
      expect(strip(name)).toHaveAccessibleName(
        expect.stringContaining("silenced by solo")
      )
    }

    await runAction("mixer.unsoloAll")
    await flush()
    for (const name of ["Kick", "Clap", "Hat", "Snare"]) {
      expect(strip(name)).toHaveAttribute("data-audible", "heard")
    }
  })

  it("keeps a bus lit when a track that plays into it is soloed", async () => {
    render(<MixerPanel />)
    const kick = trackNamed("Kick").id
    const clap = trackNamed("Clap").id
    await dispatch({ type: "setTrackOutput", id: kick, output: clap })
    await dispatch({
      type: "updateMixerTrack",
      id: kick,
      patch: { solo: true },
    })
    await flush()
    expect(strip("Kick")).toHaveAttribute("data-audible", "heard")
    expect(strip("Clap")).toHaveAttribute("data-audible", "heard")
    expect(strip("Hat")).toHaveAttribute("data-audible", "silenced")
  })

  it("unmutes or unsolos every track as one undo step", async () => {
    render(<MixerPanel />)
    for (const name of ["Kick", "Clap", "Hat"]) {
      await dispatch({
        type: "updateMixerTrack",
        id: trackNamed(name).id,
        patch: { muted: true, solo: true },
      })
    }
    await flush()
    const before = history().entries.length

    await runAction("mixer.unmuteAll")
    await flush()
    expect(tracks().some((track) => track.muted)).toBe(false)
    expect(history().entries).toHaveLength(before + 1)

    await runAction("mixer.unsoloAll")
    await flush()
    expect(tracks().some((track) => track.solo)).toBe(false)
    expect(history().entries).toHaveLength(before + 2)

    await undo()
    await flush()
    expect(tracks().filter((track) => track.solo)).toHaveLength(3)
    expect(enabled("mixer.unmuteAll")).toBe(false)
    expect(enabled("mixer.unsoloAll")).toBe(true)
  })
})

describe("routing", () => {
  it("routes a track into another one from the output menu", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    expect(button("Kick", "Output: Master")).toBeVisible()
    await runAction("mixer.addTrack")
    await flush()
    const bus = tracks()[5]

    await user.click(button("Kick", "Output: Master"))
    await user.click(
      await screen.findByRole("menuitemradio", { name: new RegExp(bus.name) })
    )
    await flush()

    expect(trackNamed("Kick").output).toBe(bus.id)
    expect(button("Kick", `Output: ${bus.name}`)).toBeVisible()
    expect(within(strip(bus.name)).getByText("1 track in")).toBeVisible()
  })

  it("leaves choices that would loop out of the output menu", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    await dispatch({
      type: "setTrackOutput",
      id: trackNamed("Kick").id,
      output: trackNamed("Clap").id,
    })
    await flush()

    await user.click(button("Clap", "Output: Master"))
    const menu = await screen.findByRole("menu")
    const offered = within(menu)
      .getAllByRole("menuitemradio")
      .map((item) => item.textContent)
    expect(offered).toEqual(["Master", "3Hat", "4Snare", "None (sends only)"])
    expect(within(menu).getByText(/1 track is not listed/)).toBeVisible()
  })

  it("can route a track nowhere, to feed only its sends", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    await user.click(button("Hat", "Output: Master"))
    await user.click(
      await screen.findByRole("menuitemradio", { name: "None (sends only)" })
    )
    await flush()
    expect(trackNamed("Hat").output).toBeNull()
    expect(button("Hat", "Output: none")).toHaveTextContent("No output")

    selectTrack(trackNamed("Hat").id)
    await runAction("mixer.routeToMaster")
    await flush()
    expect(trackNamed("Hat").output).toBe(MASTER_TRACK)
  })

  it("shows the backend's refusal when a loop gets through anyway", async () => {
    render(<MixerPanel />)
    const kick = trackNamed("Kick").id
    const clap = trackNamed("Clap").id
    await setOutput(kick, clap)
    await setOutput(clap, kick)
    await flush()
    expect(trackNamed("Clap").output).toBe(MASTER_TRACK)
    expect(toast.error).toHaveBeenCalledWith(
      expect.stringContaining("loop back on itself")
    )
  })
})

describe("sends", () => {
  it("adds a send, changes its level and removes it", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    const kick = trackNamed("Kick").id
    const snare = trackNamed("Snare").id

    await user.click(button("Kick", "Add send"))
    await user.click(await screen.findByRole("menuitem", { name: /Snare/ }))
    await flush()
    expect(trackNamed("Kick").sends).toEqual([{ target: snare, gain: 1 }])

    const level = within(strip("Kick")).getByRole("slider", {
      name: "Send to Snare",
    })
    expect(level).toHaveAttribute("aria-valuetext", "0.0 dB")
    const before = history().entries.length
    drag(level, [-10, -22])
    await flush()
    // A knob covers its range in 200 pixels: 16% down from 0 dB is -6 dB.
    expect(trackNamed("Kick").sends[0].gain).toBeCloseTo(dbToGain(-6), 4)
    expect(level).toHaveAttribute("aria-valuetext", "−6.0 dB")
    expect(history().entries).toHaveLength(before + 1)
    expect(history().entries.at(-1)?.label).toBe("Change send level")

    await user.click(button("Kick", "Remove the send to Snare"))
    await flush()
    expect(trackNamed("Kick").sends).toEqual([])
    expect(
      within(strip("Kick")).queryByRole("slider", { name: "Send to Snare" })
    ).toBeNull()
    expect(kick).not.toBe(snare)
  })

  it("offers only targets that are free and would not loop", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    const [kick, clap, hat] = ["Kick", "Clap", "Hat"].map(
      (name) => trackNamed(name).id
    )
    // Kick plays into Clap, and Clap already sends to Hat.
    await dispatch({ type: "setTrackOutput", id: kick, output: clap })
    await dispatch({ type: "setSend", from: clap, to: hat, gain: 0.5 })
    await flush()

    await user.click(button("Clap", "Add send"))
    const menu = await screen.findByRole("menu")
    const offered = within(menu)
      .getAllByRole("menuitem")
      .map((item) => item.textContent)
    expect(offered).toEqual(["Master", "4Snare"])
  })

  it("keeps a row free in every strip when any track has a send", async () => {
    render(<MixerPanel />)
    await dispatch({
      type: "setSend",
      from: trackNamed("Kick").id,
      to: trackNamed("Hat").id,
      gain: 1,
    })
    await flush()
    const rows = (name: string) =>
      strip(name).querySelector<HTMLElement>(
        "[data-slot=track-routing-inline] > div"
      )?.style.height
    expect(rows("Kick")).toBe("26px")
    expect(rows("Snare")).toBe("26px")
    expect(rows("Master")).toBe("26px")
  })
})

describe("channel chips", () => {
  it("shows the channels that play into each track", () => {
    render(<MixerPanel />)
    for (const name of ["Kick", "Clap", "Hat", "Snare"]) {
      expect(button(name, `Channel ${name}`)).toBeVisible()
    }
    expect(
      within(strip("Master")).queryByRole("button", { name: /^Channel / })
    ).toBeNull()
  })

  it("follows a channel that is routed to another track", async () => {
    render(<MixerPanel />)
    await dispatch({
      type: "updateChannel",
      id: channelNamed("Kick").id,
      patch: { mixerTrack: trackNamed("Clap").id },
    })
    await flush()

    expect(within(strip("Kick")).getByText("unused")).toBeVisible()
    expect(
      within(strip("Kick")).queryByRole("button", { name: "Channel Kick" })
    ).toBeNull()
    // The first channel shows as a chip, the rest behind a count.
    expect(button("Clap", "Channel Kick")).toBeVisible()
    expect(button("Clap", "All 2 channels of this track")).toHaveTextContent(
      "+1"
    )
  })

  it("lists every channel of a track behind the count", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    await dispatch({
      type: "updateChannel",
      id: channelNamed("Kick").id,
      patch: { mixerTrack: MASTER_TRACK },
    })
    await dispatch({
      type: "updateChannel",
      id: channelNamed("Hat").id,
      patch: { mixerTrack: MASTER_TRACK },
    })
    await flush()

    await user.click(button("Master", "All 2 channels of this track"))
    const popover = await screen.findByRole("dialog")
    const list = within(popover).getByRole("list")
    expect(
      within(list)
        .getAllByRole("button")
        .map((item) => item.textContent)
    ).toEqual(["Kick", "Hat"])
  })

  it("selects the channel when its chip is clicked", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    await user.click(button("Hat", "Channel Hat"))
    expect(ui().selectedChannel).toBe(channelNamed("Hat").id)
    expect(button("Hat", "Channel Hat")).toHaveAttribute("aria-pressed", "true")
    expect(button("Kick", "Channel Kick")).toHaveAttribute(
      "aria-pressed",
      "false"
    )
  })
})

describe("selection", () => {
  it("selects the strip that is clicked", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    await user.click(nameOf("Clap"))
    expect(ui().selectedTrack).toBe(trackNamed("Clap").id)
    expect(strip("Clap")).toHaveAttribute("data-selected")
    expect(strip("Kick")).not.toHaveAttribute("data-selected")

    await user.click(fader("Master"))
    expect(ui().selectedTrack).toBe(MASTER_TRACK)
    expect(strip("Clap")).not.toHaveAttribute("data-selected")
  })

  it("highlights the strip the rack's selected channel plays into", async () => {
    render(<MixerPanel />)
    act(() => ui().selectChannel(channelNamed("Snare").id))
    expect(strip("Snare")).toHaveAttribute("data-linked")
    expect(strip("Kick")).not.toHaveAttribute("data-linked")

    // Rerouting the channel moves the highlight with it.
    await dispatch({
      type: "updateChannel",
      id: channelNamed("Snare").id,
      patch: { mixerTrack: MASTER_TRACK },
    })
    await flush()
    expect(strip("Snare")).not.toHaveAttribute("data-linked")
    expect(strip("Master")).toHaveAttribute("data-linked")
  })

  it("follows a selection made elsewhere", async () => {
    render(<MixerPanel />)
    selectTrack(trackNamed("Hat").id)
    expect(strip("Hat")).toHaveAttribute("data-selected")
  })

  it("moves the selection with the arrow keys, Home and End", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    focus(strip("Kick"))
    expect(ui().selectedTrack).toBe(trackNamed("Kick").id)

    await user.keyboard("{ArrowRight}")
    expect(ui().selectedTrack).toBe(trackNamed("Clap").id)
    expect(strip("Clap")).toHaveFocus()

    await user.keyboard("{ArrowLeft}{ArrowLeft}")
    expect(ui().selectedTrack).toBe(MASTER_TRACK)
    expect(strip("Master")).toHaveFocus()

    await user.keyboard("{ArrowLeft}")
    expect(ui().selectedTrack).toBe(MASTER_TRACK)

    await user.keyboard("{End}")
    expect(ui().selectedTrack).toBe(trackNamed("Snare").id)
    expect(strip("Snare")).toHaveFocus()
    await user.keyboard("{Home}")
    expect(ui().selectedTrack).toBe(MASTER_TRACK)
  })

  it("leaves the arrow keys to a focused fader", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    focus(fader("Kick"))
    await user.keyboard("{ArrowLeft}")
    await flush()
    expect(ui().selectedTrack).toBe(trackNamed("Kick").id)
    expect(trackNamed("Kick").volume).toBeLessThan(1)
  })

  it("selects the strip a control is focused in, and mutes it with M", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    focus(button("Hat", "Solo"))
    expect(ui().selectedTrack).toBe(trackNamed("Hat").id)
    await user.keyboard("m")
    await flush()
    expect(trackNamed("Hat").muted).toBe(true)
    await user.keyboard("s")
    await flush()
    expect(trackNamed("Hat").solo).toBe(true)
  })
})

describe("track operations", () => {
  it("adds a track with the button after the last strip and selects it", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    await user.click(screen.getByRole("button", { name: "Add mixer track" }))
    await flush()

    expect(tracks()).toHaveLength(6)
    const added = tracks()[5]
    expect(added.output).toBe(MASTER_TRACK)
    expect(ui().selectedTrack).toBe(added.id)
    expect(strip(added.name)).toHaveAttribute("data-selected")
    expect(within(strip(added.name)).getByText("unused")).toBeVisible()
  })

  it("renames in place on double-click", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    await user.dblClick(nameOf("Kick"))
    const field = screen.getByRole("textbox", { name: "Track name" })
    expect(field).toHaveValue("Kick")
    await user.clear(field)
    await user.type(field, "Sub kick{Enter}")
    await flush()

    expect(tracks()[1].name).toBe("Sub kick")
    expect(screen.queryByRole("textbox", { name: "Track name" })).toBeNull()
    expect(strip("Sub kick")).toHaveFocus()
    expect(history().entries.at(-1)?.label).toBe("Rename mixer track")
  })

  it("renames with F2 and gives up on Escape", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    focus(strip("Clap"))
    await user.keyboard("{F2}")
    const field = screen.getByRole("textbox", { name: "Track name" })
    await user.type(field, " two{Escape}")
    await flush()
    expect(tracks()[2].name).toBe("Clap")
    expect(screen.queryByRole("textbox", { name: "Track name" })).toBeNull()
  })

  it("keeps the old name when the new one is empty", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    const before = history().entries.length
    await user.dblClick(nameOf("Hat"))
    await user.clear(screen.getByRole("textbox", { name: "Track name" }))
    await user.keyboard("{Enter}")
    await flush()
    expect(tracks()[3].name).toBe("Hat")
    expect(history().entries).toHaveLength(before)
  })

  it("changes the color from the swatches", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    selectTrack(trackNamed("Kick").id)
    await runAction("mixer.changeColor")
    await user.click(await screen.findByRole("button", { name: "Teal" }))
    await flush()
    expect(trackNamed("Kick").color).toBe(0x12a594)
    expect(screen.queryByRole("button", { name: "Teal" })).toBeNull()
  })

  it("says what an audio clip's track is fed by, and selects the clips on the playlist", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    const loop = "/factory/Loops/Drum loop 128.wav"
    const first = await backend.addAudioClipFromFile(loop, { start: 0 })
    const second = await backend.addAudioClipFromFile(loop, { start: 7680 })
    await flush()
    const clipIds = [first.created.at(-1)!, second.created.at(-1)!]

    // The track the clips play into is not "unused".
    const name = "Drum loop 128"
    expect(within(strip(name)).queryByText("unused")).toBeNull()
    const chip = button(name, "2 audio clips of this track")
    expect(chip).toHaveTextContent("2 audio clips")

    await user.click(chip)
    expect([...usePlaylistStore.getState().selection].sort()).toEqual(
      [...clipIds].sort()
    )
    expect(ui().centerTab).toBe("playlist")

    // Beside a channel the clips are behind the count, with the tracks
    // that feed the track.
    await dispatch({
      type: "updateAudioClips",
      updates: clipIds.map((id) => ({
        id,
        patch: { mixerTrack: trackNamed("Kick").id },
      })),
    })
    await setOutput(trackNamed("Clap").id, trackNamed("Kick").id)
    await flush()
    expect(button("Kick", "Channel Kick")).toBeVisible()
    const more = button(
      "Kick",
      "Everything that plays into this track: 1 channel, 2 audio clips, 1 track"
    )
    expect(more).toHaveTextContent("+2")
    await user.click(more)
    expect(
      await screen.findByRole("button", { name: "2 audio clips of this track" })
    ).toBeVisible()
    expect(screen.getByText("1 track in")).toBeVisible()
    // And the track the clips left is unused again.
    expect(within(strip(name)).getByText("unused")).toBeVisible()
  })

  it("shows the tracks that feed a track with no channel, and audio clips on the master", async () => {
    render(<MixerPanel />)
    await dispatch({ type: "addMixerTrack", name: "Bus" })
    await flush()
    expect(within(strip("Bus")).getByText("unused")).toBeVisible()
    await setOutput(trackNamed("Kick").id, trackNamed("Bus").id)
    await flush()
    expect(within(strip("Bus")).getByText("1 track in")).toBeVisible()
    expect(within(strip("Bus")).queryByText("unused")).toBeNull()

    const added = await backend.addAudioClipFromFile(
      "/factory/Loops/Drum loop 128.wav",
      { start: 0, mixerTrack: MASTER_TRACK }
    )
    await flush()
    expect(added.created).toHaveLength(3)
    expect(button("Master", "1 audio clip of this track")).toBeVisible()
  })

  it("says in the delete question that a track's audio clips go to the master", async () => {
    render(<MixerPanel />)
    const loop = "/factory/Loops/Drum loop 128.wav"
    const added = await backend.addAudioClipFromFile(loop, { start: 0 })
    await backend.addAudioClipFromFile(loop, { start: 7680 })
    await flush()
    const track = trackNamed("Drum loop 128")
    selectTrack(track.id)

    const confirmed = runAction("mixer.deleteTrack")
    await flush()
    const question = usePromptStore.getState().confirm
    expect(question?.description).toBe(
      "2 audio clips on the playlist play into this track. They will play into the master instead."
    )
    question?.resolve("delete")
    await confirmed
    await flush()
    const clip = project().playlist.clips.find(
      (item) => item.id === added.created.at(-1)
    )
    expect(clip?.content).toMatchObject({
      type: "audio",
      mixerTrack: MASTER_TRACK,
    })
  })

  it("asks before deleting a track a channel plays into", async () => {
    render(<MixerPanel />)
    const kick = trackNamed("Kick").id
    selectTrack(kick)

    const cancelled = runAction("mixer.deleteTrack")
    await flush()
    const question = usePromptStore.getState().confirm
    expect(question?.title).toBe('Delete the track "Kick"?')
    expect(question?.description).toContain('The channel "Kick"')
    expect(question?.description).toContain("fall back to the master")
    question?.resolve(null)
    await cancelled
    await flush()
    expect(tracks()).toHaveLength(5)

    const confirmed = runAction("mixer.deleteTrack")
    await flush()
    usePromptStore.getState().confirm?.resolve("delete")
    await confirmed
    await flush()

    expect(tracks().map((track) => track.name)).toEqual([
      "Master",
      "Clap",
      "Hat",
      "Snare",
    ])
    expect(channelNamed("Kick").mixerTrack).toBe(MASTER_TRACK)
    expect(button("Master", "Channel Kick")).toBeVisible()
    // The selection moves to the strip that took its place.
    expect(ui().selectedTrack).toBe(trackNamed("Clap").id)
  })

  it("says what happens to tracks and sends that play into it", async () => {
    render(<MixerPanel />)
    const [kick, clap, hat] = ["Kick", "Clap", "Hat"].map(
      (name) => trackNamed(name).id
    )
    await dispatch({
      type: "updateChannel",
      id: channelNamed("Clap").id,
      patch: { mixerTrack: MASTER_TRACK },
    })
    await dispatch({ type: "setTrackOutput", id: kick, output: clap })
    await dispatch({ type: "setSend", from: hat, to: clap, gain: 1 })
    await flush()

    selectTrack(clap)
    const deleting = runAction("mixer.deleteTrack")
    await flush()
    const description = usePromptStore.getState().confirm?.description
    expect(description).toContain('The track "Kick" is routed into it')
    expect(description).toContain('The send from "Hat" will be removed')
    usePromptStore.getState().confirm?.resolve("delete")
    await deleting
    await flush()

    expect(trackNamed("Kick").output).toBe(MASTER_TRACK)
    expect(trackNamed("Hat").sends).toEqual([])
  })

  it("deletes a track nothing plays into without asking", async () => {
    render(<MixerPanel />)
    await runAction("mixer.addTrack")
    await flush()
    expect(tracks()).toHaveLength(6)

    await runAction("mixer.deleteTrack")
    await flush()
    expect(usePromptStore.getState().confirm).toBeNull()
    expect(tracks()).toHaveLength(5)
    expect(ui().selectedTrack).toBe(trackNamed("Snare").id)
  })

  it("resets the fader and the pan of the selected track", async () => {
    render(<MixerPanel />)
    const hat = trackNamed("Hat").id
    selectTrack(hat)
    expect(enabled("mixer.resetVolume")).toBe(false)
    expect(enabled("mixer.centerPan")).toBe(false)

    await dispatch({
      type: "updateMixerTrack",
      id: hat,
      patch: { volume: 0.3, pan: 0.7 },
    })
    await flush()
    await runAction("mixer.resetVolume")
    await runAction("mixer.centerPan")
    await flush()
    expect(trackNamed("Hat").volume).toBe(1)
    expect(trackNamed("Hat").pan).toBe(0)
  })

  it("offers the track's actions in a menu on right-click", async () => {
    render(<MixerPanel />)
    fireEvent.contextMenu(nameOf("Snare"))
    const menu = await screen.findByRole("menu")
    expect(ui().selectedTrack).toBe(trackNamed("Snare").id)
    // Each entry shows the key it has while the mixer has the keyboard.
    expect(
      [...menu.querySelectorAll("[role^=menuitem]")].map(
        (item) => item.textContent
      )
    ).toEqual([
      "Render selected mixer tracks…",
      "Render armed mixer tracks…Arm a mixer insert or Master",
      "Arm or disarm mixer track for recording",
      "Record armed mixer tracks…",
      "Rename mixer trackF2",
      "Change track color…",
      "Add effect",
      "Show effectsE",
      "Bypass effects",
      "Enable effects",
      "Mute or unmute trackM",
      "Solo or unsolo trackS",
      "Unmute all tracks",
      "Reset levels",
      "Select tracks routed here",
      "Unsolo all tracks",
      "Reset fader to 0 dB",
      "Center pan",
      "Route to master",
      "Add mixer trackAlt+M",
      "Delete mixer trackDel",
    ])
    // Mute and solo are on or off, and say so.
    expect(
      within(menu).getByRole("menuitemcheckbox", {
        name: /^Mute or unmute track/,
      })
    ).toHaveAttribute("aria-checked", "false")
    expect(
      within(menu).getByRole("menuitem", { name: "Route to master" })
    ).toHaveAttribute("aria-disabled", "true")
    expect(
      within(menu).getByRole("menuitem", { name: "Render selected mixer tracks…" })
    ).not.toHaveAttribute("aria-disabled", "true")
    expect(
      within(menu).getByRole("menuitem", { name: /^Render armed mixer tracks/ })
    ).toHaveAttribute("aria-disabled", "true")
  })

  it("registers every operation for the command palette", () => {
    render(<MixerPanel />)
    const ids = registry
      .list()
      .filter((action) => action.section === "Mixer")
      .map((action) => action.id)
    expect(ids).toEqual(
      expect.arrayContaining([
        "mixer.addTrack",
        "mixer.renameTrack",
        "mixer.changeColor",
        "mixer.deleteTrack",
        "mixer.toggleMute",
        "mixer.toggleSolo",
        "mixer.unmuteAll",
        "mixer.unsoloAll",
        "mixer.resetVolume",
        "mixer.centerPan",
        "mixer.routeToMaster",
      ])
    )
  })
})

describe("the master", () => {
  it("has a fader, pan, mute and peak readout, and nothing to route", () => {
    render(<MixerPanel />)
    const master = within(strip("Master"))
    expect(master.getByRole("slider", { name: "Master volume" })).toBeVisible()
    expect(master.getByRole("slider", { name: "Master pan" })).toBeVisible()
    expect(master.getByRole("button", { name: "Mute" })).toBeVisible()
    expect(master.getByRole("button", { name: /^Peak/ })).toBeVisible()
    expect(master.queryByRole("button", { name: "Solo" })).toBeNull()
    expect(master.queryByRole("button", { name: "Add send" })).toBeNull()
    expect(master.queryByRole("button", { name: /^Output: Master/ })).toBeNull()
  })

  it("cannot be deleted, soloed or rerouted", async () => {
    render(<MixerPanel />)
    selectTrack(MASTER_TRACK)
    expect(enabled("mixer.deleteTrack")).toBe(false)
    expect(enabled("mixer.toggleSolo")).toBe(false)
    expect(enabled("mixer.routeToMaster")).toBe(false)
    expect(enabled("mixer.toggleMute")).toBe(true)

    await runAction("mixer.deleteTrack")
    await flush()
    expect(tracks()[0].id).toBe(MASTER_TRACK)
    expect(tracks()).toHaveLength(5)

    fireEvent.contextMenu(nameOf("Master"))
    const menu = await screen.findByRole("menu")
    const titles = within(menu)
      .getAllByRole("menuitem")
      .map((item) => item.textContent)
    expect(titles).not.toContain("Delete mixer track")
    expect(titles).not.toContain("Route to master")
    expect(titles).not.toContain("Solo or unsolo track")
  })

  it("mutes and takes fader moves like any track", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    await user.click(button("Master", "Mute"))
    drag(fader("Master"), [-16])
    await flush()
    expect(tracks()[0].muted).toBe(true)
    expect(tracks()[0].volume).toBeCloseTo(dbToGain(-6), 4)
    expect(strip("Master")).toHaveAttribute("data-audible", "muted")
  })
})

describe("keys and wheel", () => {
  const playing = () => useTransportStore.getState().playing

  it("plays with Space instead of pressing the focused button", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    await user.click(button("Kick", "Mute"))
    await flush()
    expect(trackNamed("Kick").muted).toBe(true)
    expect(button("Kick", "Mute")).toHaveFocus()

    await user.keyboard(" ")
    await flush()
    expect(playing()).toBe(true)
    expect(trackNamed("Kick").muted).toBe(true)

    // Enter still presses the button.
    await user.keyboard("{Enter}")
    await flush()
    expect(trackNamed("Kick").muted).toBe(false)
  })

  it("plays with Space while a fader has the focus", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    focus(fader("Hat"))
    await user.keyboard(" ")
    await flush()
    expect(playing()).toBe(true)
    expect(trackNamed("Hat").volume).toBe(1)
  })

  it("leaves Space to the name field", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    await user.dblClick(nameOf("Kick"))
    const field = screen.getByRole("textbox", { name: "Track name" })
    await user.clear(field)
    await user.type(field, "Big kick{Enter}")
    await flush()
    expect(tracks()[1].name).toBe("Big kick")
    expect(playing()).toBe(false)
  })

  it("moves along the strips with the mouse wheel", () => {
    render(<MixerPanel />)
    const scroller = screen.getByRole("group", { name: "middle mixer dock" })
    if (!scroller) throw new Error("The mixer is not mounted")
    fireEvent.wheel(strip("Kick"), { deltaY: 240 })
    expect(scroller.scrollLeft).toBe(240)
    // A sideways wheel already scrolls sideways by itself.
    fireEvent.wheel(strip("Kick"), { deltaX: 100, deltaY: 10 })
    expect(scroller.scrollLeft).toBe(240)
  })
})

describe("empty mixer", () => {
  it("says how tracks get here when there are none", async () => {
    render(<MixerPanel />)
    for (const track of tracks().slice(1)) {
      await dispatch({ type: "removeMixerTrack", id: track.id })
    }
    await flush()
    expect(screen.getByText(/No insert tracks yet/)).toBeVisible()
    expect(
      screen.getByRole("button", { name: "Add mixer track" })
    ).toBeEnabled()
  })
})
