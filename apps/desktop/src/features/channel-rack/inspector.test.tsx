import {
  createEvent,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import type { ReactNode } from "react"
import { toast } from "sonner"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { runAction } from "@/lib/actions"
import { SAMPLE_DRAG_TYPE } from "@/lib/dnd"
import type { Backend } from "@/lib/ipc"
import { dispatch, undo } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { settle } from "@/test/harness"

import ChannelRackPanel from "./index"
import { DEFAULT_ENVELOPE } from "./inspector/envelope-section"
import { parseNoteName } from "./inspector/sound-section"
import { useRackStore } from "./rack-store"
import {
  channel,
  dragData,
  history,
  labels,
  project,
  startRack,
} from "./test-utils"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

// The resize handle finds the pointer by measuring, and jsdom measures
// everything as zero wide at the origin, so it would take every click.
vi.mock("@/components/ui/resizable", () => ({
  ResizablePanelGroup: ({ children }: { children: ReactNode }) => (
    <div>{children}</div>
  ),
  ResizablePanel: ({ children }: { children: ReactNode }) => (
    <div>{children}</div>
  ),
  ResizableHandle: () => null,
}))

let backend: Backend
let stop: () => void

beforeEach(async () => {
  ;({ backend, stop } = await startRack())
  vi.mocked(toast.error).mockClear()
})
afterEach(() => stop())

const sampler = (name: string) => channel(name).source
const settings = () =>
  screen.getByRole("complementary", { name: "Channel settings" })
const slider = (name: string) =>
  within(settings()).getByRole("slider", { name })

/** Renders the rack with a channel selected and its settings open. */
async function openSettings(name: string) {
  useUiStore.getState().selectChannel(channel(name).id)
  useRackStore.getState().setInspectorOpen(true)
  const view = render(<ChannelRackPanel />)
  // The waveform arrives from the backend a moment later.
  await settle()
  return view
}

/** One key press: down and up, which is one gesture for a kit control. */
function tap(target: HTMLElement, key: string, times = 1) {
  for (let count = 0; count < times; count += 1) {
    fireEvent.keyDown(target, { key })
  }
  fireEvent.keyUp(target, { key })
}

describe("the settings panel", () => {
  it("is closed until a channel is opened, and follows the selection", async () => {
    const user = userEvent.setup()
    render(<ChannelRackPanel />)
    expect(screen.queryByRole("complementary")).toBeNull()

    await user.click(screen.getByRole("button", { name: "Kick" }))
    await settle()
    expect(
      within(settings()).getByRole("heading", { name: "Kick" })
    ).toBeVisible()

    await user.click(screen.getByRole("button", { name: "Hat" }))
    await settle()
    expect(
      within(settings()).getByRole("heading", { name: "Hat" })
    ).toBeVisible()
  })

  it("closes from its own button and opens from the action", async () => {
    const user = userEvent.setup()
    await openSettings("Kick")
    const close = within(settings()).getByRole("button", {
      name: "Channel settings",
    })
    await user.click(close)
    expect(screen.queryByRole("complementary")).toBeNull()

    await runAction("channelRack.settings")
    await settle()
    expect(settings()).toBeVisible()
  })

  it("says what to do when no channel is selected", async () => {
    useRackStore.getState().setInspectorOpen(true)
    render(<ChannelRackPanel />)
    expect(settings()).toHaveTextContent("No channel selected")
    expect(settings()).toHaveTextContent("Click a channel's name")
  })
})

describe("the sample", () => {
  it("shows the sample's name and facts", async () => {
    await openSettings("Kick")
    expect(settings()).toHaveTextContent("Kick")
    expect(settings()).toHaveTextContent("420 ms")
    expect(settings()).toHaveTextContent("44.1 kHz")
    expect(settings()).toHaveTextContent("mono")
  })

  it("trims the start and the end, each drag one undo step", async () => {
    await openSettings("Kick")
    const sent = vi.spyOn(backend, "dispatch")
    const start = slider("Region start")
    const end = slider("Region end")

    tap(start, "ArrowRight", 3)
    await settle()
    // An arrow key moves a handle by a hundredth of its travel.
    expect(sampler("Kick").start).toBeCloseTo(0.03, 3)
    expect(sampler("Kick").end).toBe(1)
    expect(sent).toHaveBeenCalledTimes(3)
    expect(new Set(sent.mock.calls.map((call) => call[1])).size).toBe(1)
    expect(labels()).toEqual(["Change sample start"])

    tap(end, "ArrowLeft", 2)
    await settle()
    expect(sampler("Kick").end).toBeCloseTo(0.98, 2)
    expect(labels()).toEqual(["Change sample start", "Change sample end"])

    await undo()
    await undo()
    expect(sampler("Kick").start).toBe(0)
    expect(sampler("Kick").end).toBe(1)
    expect(start).toHaveAttribute("aria-valuenow", "0")
  })

  it("never lets the handles cross", async () => {
    await openSettings("Kick")
    tap(slider("Region start"), "End")
    await settle()
    expect(sampler("Kick").start).toBeLessThan(1)
    expect(sampler("Kick").end).toBeGreaterThan(sampler("Kick").start)
    expect(toast.error).not.toHaveBeenCalled()
  })

  it("fetches each waveform once", async () => {
    const info = vi.spyOn(backend, "sampleInfoById")
    await openSettings("Kick")
    useUiStore.getState().selectChannel(channel("Hat").id)
    await settle()
    useUiStore.getState().selectChannel(channel("Kick").id)
    await settle()
    expect(slider("Region start")).toBeVisible()
    expect(info.mock.calls.map((call) => call[0])).toEqual([
      sampler("Kick").sample,
      sampler("Hat").sample,
    ])
  })

  it("says so when the waveform cannot be read, and keeps the controls", async () => {
    vi.spyOn(backend, "sampleInfoById").mockRejectedValue(
      new Error("The file is missing.")
    )
    await openSettings("Kick")
    expect(within(settings()).getByRole("alert")).toHaveTextContent(
      "The waveform could not be read. The file is missing."
    )
    expect(slider("Tune")).toBeVisible()
    expect(toast.error).not.toHaveBeenCalled()
  })

  it("explains a channel with no sample and lets one be chosen", async () => {
    const user = userEvent.setup()
    await dispatch({ type: "addChannel", name: "Empty" })
    await openSettings("Empty")
    expect(settings()).toHaveTextContent(
      "No sample. Drag one here from the browser."
    )
    expect(
      within(settings()).getByRole("group", { name: "Piano keyboard" })
    ).toHaveAttribute("data-disabled")

    await user.click(
      within(settings()).getByRole("button", { name: /Choose a sample/ })
    )
    await user.click(await screen.findByRole("menuitemradio", { name: "Hat" }))
    await settle()
    expect(sampler("Empty").sample).toBe(sampler("Hat").sample)
    expect(labels().at(-1)).toBe("Change channel sample")
    expect(settings()).not.toHaveTextContent("No sample.")
  })

  it("takes a sample dropped on the waveform", async () => {
    await openSettings("Clap")
    const zone = settings().querySelector('[data-slot="sample-drop"]')!
    const data = dragData(
      SAMPLE_DRAG_TYPE,
      JSON.stringify({
        path: "/factory/Drums/Percussion/Tom 01.wav",
        name: "Tom 01",
      })
    )
    const over = createEvent.dragOver(zone, { dataTransfer: data })
    fireEvent(zone, over)
    expect(over.defaultPrevented).toBe(true)
    expect(zone).toHaveTextContent("Drop to use this sample")

    fireEvent.drop(zone, { dataTransfer: data })
    await settle()
    const used = project().samples.find(
      (item) => item.id === sampler("Clap").sample
    )
    expect(used?.name).toBe("Tom 01")
    expect(labels()).toEqual(["Change channel sample"])
    expect(zone).not.toHaveTextContent("Drop to use this sample")
  })
})

describe("the sound", () => {
  it("tunes in semitones and in cents", async () => {
    await openSettings("Kick")
    tap(slider("Tune"), "ArrowUp", 2)
    await settle()
    expect(sampler("Kick").tune).toBe(2)
    expect(slider("Tune")).toHaveAttribute("aria-valuetext", "+2 st")

    tap(slider("Fine"), "ArrowDown")
    await settle()
    expect(sampler("Kick").tune).toBeCloseTo(1.99, 5)
    // 1.99 is shown as two semitones less one cent.
    expect(slider("Tune")).toHaveAttribute("aria-valuenow", "2")
    expect(slider("Fine")).toHaveAttribute("aria-valuetext", "−1 ct")
    expect(labels()).toEqual(["Change tuning", "Change tuning"])
  })

  it("keeps the tuning inside its range", async () => {
    await openSettings("Kick")
    tap(slider("Tune"), "End")
    await settle()
    tap(slider("Fine"), "End")
    await settle()
    expect(sampler("Kick").tune).toBe(48)
    expect(toast.error).not.toHaveBeenCalled()
  })

  it("sets the root key by note name", async () => {
    await openSettings("Kick")
    expect(slider("Root")).toHaveAttribute("aria-valuetext", "C5")
    tap(slider("Root"), "ArrowDown")
    await settle()
    expect(sampler("Kick").rootKey).toBe(59)
    expect(slider("Root")).toHaveAttribute("aria-valuetext", "B4")
    expect(labels()).toEqual(["Change root key"])
  })

  it("reads note names the way it prints them", () => {
    expect(parseNoteName("C5")).toBe(60)
    expect(parseNoteName("c#5")).toBe(61)
    expect(parseNoteName("Bb4")).toBe(58)
    expect(parseNoteName(" a 4 ")).toBe(57)
    expect(parseNoteName("64")).toBe(64)
    expect(parseNoteName("loud")).toBeNull()
  })

  it("changes gain, reverse, cut itself and the cut group", async () => {
    const user = userEvent.setup()
    await openSettings("Hat")
    tap(slider("Gain"), "End")
    await settle()
    expect(sampler("Hat").gain).toBe(2)

    await user.click(
      within(settings()).getByRole("switch", { name: "Reverse" })
    )
    await user.click(
      within(settings()).getByRole("switch", { name: "Cut itself" })
    )
    await settle()
    expect(sampler("Hat").reverse).toBe(true)
    expect(sampler("Hat").cutSelf).toBe(true)

    expect(settings()).toHaveTextContent("none")
    tap(slider("Cut group"), "ArrowUp", 2)
    await settle()
    expect(sampler("Hat").cutGroup).toBe(2)
    expect(labels()).toEqual([
      "Change sample gain",
      "Reverse sample",
      "Change cut mode",
      "Change cut group",
    ])
  })

  it("puts a control back with a double-click", async () => {
    await openSettings("Kick")
    tap(slider("Tune"), "ArrowUp", 5)
    await settle()
    fireEvent.doubleClick(slider("Tune"))
    await settle()
    expect(sampler("Kick").tune).toBe(0)
    expect(history().cursor).toBe(2)
  })
})

describe("the envelope", () => {
  const toggle = () =>
    within(settings()).getByRole("switch", { name: "Volume envelope" })
  const editor = () =>
    within(settings()).getByRole("group", { name: "Envelope shape" })

  it("turns on with sensible values and off again", async () => {
    const user = userEvent.setup()
    await openSettings("Kick")
    expect(sampler("Kick").envelope).toBeNull()
    expect(settings()).toHaveTextContent(
      "Every hit plays the sample to its end"
    )

    await user.click(toggle())
    await settle()
    expect(sampler("Kick").envelope).toEqual(DEFAULT_ENVELOPE)
    expect(editor()).toBeVisible()

    await user.click(toggle())
    await settle()
    expect(sampler("Kick").envelope).toBeNull()
    expect(labels()).toEqual(["Turn envelope on", "Turn envelope off"])
  })

  it("edits one shape from the knobs and from the nodes", async () => {
    const user = userEvent.setup()
    await openSettings("Kick")
    await user.click(toggle())
    await settle()

    // The knob, not the node of the same name inside the editor.
    const releaseKnob = within(settings())
      .getAllByRole("slider", { name: "Release" })
      .find((element) => !editor().contains(element))!
    tap(releaseKnob, "End")
    await settle()
    expect(sampler("Kick").envelope?.releaseMs).toBe(10_000)

    const decayNode = within(editor()).getByRole("slider", {
      name: "Decay and sustain",
    })
    tap(decayNode, "ArrowDown", 4)
    await settle()
    const envelope = sampler("Kick").envelope
    expect(envelope?.sustain).toBeCloseTo(0.96, 5)
    expect(envelope?.releaseMs).toBe(10_000)
    expect(envelope?.attackMs).toBe(DEFAULT_ENVELOPE.attackMs)
    // Turning it on, the knob and the held key are three steps.
    expect(labels()).toEqual([
      "Turn envelope on",
      "Change envelope",
      "Change envelope",
    ])
  })

  it("remembers the shape while it is off", async () => {
    const user = userEvent.setup()
    await openSettings("Kick")
    await user.click(toggle())
    await settle()
    const sustain = within(settings())
      .getAllByRole("slider", { name: "Sustain" })
      .find((element) => !editor().contains(element))!
    tap(sustain, "Home")
    await settle()
    expect(sampler("Kick").envelope?.sustain).toBe(0)

    await user.click(toggle())
    await user.click(toggle())
    await settle()
    expect(sampler("Kick").envelope?.sustain).toBe(0)
  })
})

describe("playing and routing", () => {
  it("plays the channel from the keyboard and always lets go", async () => {
    await openSettings("Kick")
    const on = vi.spyOn(backend, "auditionNoteOn")
    const off = vi.spyOn(backend, "auditionNoteOff")
    const kick = channel("Kick").id
    const keys = within(settings()).getByRole("group", {
      name: "Piano keyboard",
    })
    const key = within(keys).getByRole("button", { name: "D5" })
    key.focus()
    fireEvent.keyDown(key, { key: "Enter" })
    expect(on).toHaveBeenCalledWith(kick, 62, 0.8)
    fireEvent.keyUp(key, { key: "Enter" })
    expect(off).toHaveBeenCalledWith(kick, 62)
    expect(off).toHaveBeenCalledTimes(1)
  })

  it("lights the root key", async () => {
    await openSettings("Kick")
    const keys = within(settings()).getByRole("group", {
      name: "Piano keyboard",
    })
    expect(within(keys).getByRole("button", { name: "C5" })).toHaveAttribute(
      "aria-pressed",
      "true"
    )
  })

  it("shows the mixer track and jumps to it", async () => {
    const user = userEvent.setup()
    await openSettings("Snare")
    useUiStore.getState().setPanelVisible("mixer", false)
    expect(
      within(settings()).getByRole("button", { name: "Mixer track: Snare" })
    ).toHaveTextContent("4")

    await user.click(
      within(settings()).getByRole("button", {
        name: "Show the channel's mixer track",
      })
    )
    expect(useUiStore.getState().selectedTrack).toBe(
      channel("Snare").mixerTrack
    )
    expect(useUiStore.getState().panels.mixer).toBe(true)
  })

  it("routes from the settings to another track", async () => {
    const user = userEvent.setup()
    await openSettings("Snare")
    await user.click(
      within(settings()).getByRole("button", { name: "Mixer track: Snare" })
    )
    await user.click(await screen.findByRole("menuitemradio", { name: /Kick/ }))
    await settle()
    expect(channel("Snare").mixerTrack).toBe(channel("Kick").mixerTrack)
    expect(
      within(settings()).getByRole("button", { name: "Mixer track: Kick" })
    ).toBeVisible()
  })
})
