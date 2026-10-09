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

import type { ParamInfo } from "@/bindings"
import { registerBrowserActions } from "@/features/browser/actions"
import {
  formatParam,
  instrumentDescriptor,
  paramIndex,
  paramInfo,
  readParam,
  writeParam,
} from "@/features/params"
import { disabledReason, getAppState, registry, runAction } from "@/lib/actions"
import { SAMPLE_DRAG_TYPE } from "@/lib/dnd"
import type { Backend } from "@/lib/ipc"
import { useHintStore } from "@/lib/store/hint"
import { dispatch, redo, undo } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { settle } from "@/test/harness"

import ChannelRackPanel from "./index"
import { useRackStore } from "./rack-store"
import { matchingPreset, SYNTH_PRESETS, synthPreset } from "./synth/presets"
import {
  channel,
  channelNames,
  dragData,
  history,
  instrumentChannel,
  labels,
  notesOf,
  project,
  shownRow,
  startRack,
  stepButtons,
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

const SYNTH = "Subtractive synth"
const descriptor = instrumentDescriptor("subtractiveSynth")

let backend: Backend
let stop: () => void

beforeEach(async () => {
  ;({ backend, stop } = await startRack())
  vi.mocked(toast.error).mockClear()
})
afterEach(() => stop())

const params = () => {
  const value = instrumentChannel(SYNTH).source.params
  if (value.type !== "subtractiveSynth") throw new Error("Expected subtractive synth")
  return value
}
const stored = (id: string) => readParam(params(), paramInfo(descriptor, id))
const settings = () =>
  screen.getByRole("complementary", { name: "Channel settings" })
const knob = (name: string) => within(settings()).getByRole("slider", { name })
const nameButton = (name: string) => screen.getByRole("button", { name })
const control = (id: string) =>
  settings().querySelector<HTMLElement>(`[data-param="${id}"]`)

/** Adds a synth the way the Add menu does and shows the rack. */
async function addSynth() {
  await runAction("channel.addInstrument.subtractiveSynth")
  await settle()
  const view = render(<ChannelRackPanel />)
  await settle()
  return view
}

/** The `setInstrumentParam` commands sent since the spy was made. */
function sentParams(spy: { mock: { calls: unknown[][] } }) {
  return spy.mock.calls.flatMap(([command, gesture]) => {
    const sent = command as { type: string; param?: number; value?: number }
    return sent.type === "setInstrumentParam"
      ? [{ param: sent.param, value: sent.value, gesture }]
      : []
  })
}

/** One key press: down and up, which is one gesture for a kit control. */
function tap(target: HTMLElement, key: string, times = 1) {
  for (let count = 0; count < times; count += 1) {
    fireEvent.keyDown(target, { key })
  }
  fireEvent.keyUp(target, { key })
}

describe("adding a synth", () => {
  it("is offered by the rack's Add menu and adds an instrument channel", async () => {
    const user = userEvent.setup()
    render(<ChannelRackPanel />)
    await user.click(screen.getByRole("button", { name: "Add channel menu" }))
    await user.click(
      await screen.findByRole("menuitem", { name: "Add subtractive synth" })
    )
    await settle()

    expect(channelNames()).toEqual(["Kick", "Clap", "Hat", "Snare", SYNTH])
    const added = instrumentChannel(SYNTH)
    expect(added.source.params).toEqual(descriptor.defaults)
    expect(labels()).toEqual(["Add channel"])
    // It is selected, with its settings open and its own mixer track.
    expect(useUiStore.getState().selectedChannel).toBe(added.id)
    expect(useRackStore.getState().inspectorOpen).toBe(true)
    expect(
      project().mixer.tracks.find((track) => track.id === added.mixerTrack)
        ?.name
    ).toBe(SYNTH)
  })

  it("is in the menu bar's list and the palette's actions", () => {
    const action = registry.get("channel.addInstrument.subtractiveSynth")
    expect(action).toMatchObject({
      title: "Add subtractive synth",
      section: "Channels",
    })
    expect(action?.keywords).toContain("instrument")
  })

  it("draws a wave on the row instead of a sample state", async () => {
    await addSynth()
    const button = nameButton(SYNTH)
    expect(button).not.toHaveTextContent("no sample")
    expect(button.querySelector("[data-sample-missing]")).toBeNull()
    expect(button.querySelector('[data-slot="wave-glyph"]')).toHaveAttribute(
      "data-shape",
      "saw"
    )

    // The wave follows the first oscillator that sounds.
    await dispatch({
      type: "setInstrumentParam",
      channel: channel(SYNTH).id,
      param: paramIndex(descriptor, "oscillators.0.waveform"),
      value: 3,
    })
    await settle()
    expect(
      nameButton(SYNTH).querySelector('[data-slot="wave-glyph"]')
    ).toHaveAttribute("data-shape", "square")
  })

  it("duplicates with its settings", async () => {
    await addSynth()
    await runAction("channel.synthSound.pluck")
    await runAction("channel.duplicate")
    await settle()
    const copy = instrumentChannel(`${SYNTH} #2`)
    expect(copy.source.params).toEqual(synthPreset("pluck")?.params)
    expect(copy.id).not.toBe(channel(SYNTH).id)
    expect(nameButton(`${SYNTH} #2`)).toBeVisible()
  })
})

describe("the synth's settings", () => {
  it("replace the sampler sections and keep the keyboard and the routing", async () => {
    await addSynth()
    const panel = within(settings())
    expect(panel.getByRole("region", { name: SYNTH })).toBeVisible()
    for (const group of [
      "Oscillators",
      "Filter",
      "Amp envelope",
      "Filter envelope",
      "LFO 1",
      "LFO 2",
      "Voice",
      "Output",
    ]) {
      expect(panel.getByRole("region", { name: group })).toBeVisible()
    }
    for (const group of ["Oscillator 1", "Oscillator 2", "Oscillator 3"]) {
      expect(panel.getByRole("group", { name: group })).toBeVisible()
    }
    expect(panel.getByRole("group", { name: "Unison" })).toBeVisible()
    expect(panel.queryByRole("region", { name: "Sample" })).toBeNull()
    expect(panel.queryByRole("region", { name: "Envelope" })).toBeNull()
    expect(panel.getByRole("region", { name: "Play" })).toBeVisible()
    expect(panel.getByRole("region", { name: "Mixer track" })).toBeVisible()

    // Back on a sampler, the sampler's sections return.
    fireEvent.click(nameButton("Kick"))
    await settle()
    expect(panel.getByRole("region", { name: "Sample" })).toBeVisible()
    expect(panel.queryByRole("region", { name: "Oscillators" })).toBeNull()
  })

  it("shows the pulse width only for a pulse wave", async () => {
    const user = userEvent.setup()
    await addSynth()
    expect(control("oscillators.0.pulseWidth")).toBeNull()

    await user.click(
      within(settings()).getByRole("combobox", { name: "Osc 1 waveform" })
    )
    await user.click(await screen.findByRole("option", { name: "Pulse" }))
    await settle()
    expect(stored("oscillators.0.waveform")).toBe(4)
    expect(knob("Osc 1 pulse width")).toHaveAttribute("aria-valuetext", "50%")
    expect(control("oscillators.1.pulseWidth")).toBeNull()
  })

  it("marks an oscillator with no level as off", async () => {
    await addSynth()
    const second = within(settings()).getByRole("group", {
      name: "Oscillator 2",
    })
    expect(second).toHaveAttribute("data-off")
    expect(second).toHaveTextContent("off")
    tap(knob("Osc 2 level"), "End")
    await settle()
    expect(second).not.toHaveAttribute("data-off")
    expect(
      within(settings()).getByRole("group", { name: "Oscillator 1" })
    ).not.toHaveAttribute("data-off")
  })

  it("has a control for every setting, each sending its own index as one undo step", async () => {
    const user = userEvent.setup({ delay: null })
    await addSynth()
    // Every oscillator on a pulse wave, so every pulse width shows.
    let all = descriptor.defaults
    for (const index of [0, 1, 2]) {
      all = writeParam(
        all,
        paramInfo(descriptor, `oscillators.${index}.waveform`),
        4
      )
    }
    await dispatch({
      type: "setInstrumentParams",
      channel: channel(SYNTH).id,
      params: all,
    })
    await settle()
    const shown = [...settings().querySelectorAll<HTMLElement>("[data-param]")]
    expect(shown.map((element) => element.dataset.param).sort()).toEqual(
      descriptor.params.map((info) => info.id).sort()
    )

    const sent = vi.spyOn(backend, "dispatch")
    const synth = channel(SYNTH).id
    // The waveforms go last: leaving the pulse wave hides a pulse width.
    const order = [...descriptor.params.entries()].sort(
      ([, a], [, b]) =>
        Number(a.id.endsWith(".waveform")) - Number(b.id.endsWith(".waveform"))
    )
    expect(order).toHaveLength(55)
    for (const [index, info] of order) {
      const before = history().entries.length
      const calls = sentParams(sent).length
      const expected = await move(user, info)
      await settle()

      const mine = sentParams(sent).slice(calls)
      expect(mine, info.id).toHaveLength(1)
      expect(mine[0].param, info.id).toBe(index)
      expect(mine[0].value, info.id).toBe(expected)
      expect(mine[0].gesture, info.id).toEqual(expect.any(Number))
      expect(sent.mock.lastCall?.[0]).toMatchObject({ channel: synth })
      expect(stored(info.id), info.id).toBe(expected)
      expect(history().entries.length, info.id).toBe(before + 1)
      // The history names the setting that was moved.
      expect(labels().at(-1)).toBe(`Change ${info.name}`)
    }
  }, 30_000)

  /** Moves one control to another value and says which. */
  async function move(
    user: ReturnType<typeof userEvent.setup>,
    info: ParamInfo
  ): Promise<number> {
    const current = stored(info.id)
    if (info.kind === "choice") {
      const next = (current + 1) % info.choices.length
      const label = info.choices[next].label
      const group = within(settings()).queryByRole("radiogroup", {
        name: info.name,
      })
      if (group) {
        await user.click(within(group).getByRole("radio", { name: label }))
      } else {
        await user.click(
          within(settings()).getByRole("combobox", { name: info.name })
        )
        await user.click(await screen.findByRole("option", { name: label }))
      }
      return next
    }
    const end = current === info.max ? "Home" : "End"
    tap(knob(info.name), end)
    return end === "End" ? info.max : info.min
  }

  it("sends a whole drag under one gesture and undoes it in one step", async () => {
    await addSynth()
    const sent = vi.spyOn(backend, "dispatch")
    const cutoff = knob("Cutoff")
    const index = paramIndex(descriptor, "filter.cutoffHz")

    fireEvent.pointerDown(cutoff, { pointerId: 1, button: 0, clientY: 300 })
    for (const y of [320, 350, 380, 400]) {
      fireEvent.pointerMove(cutoff, { pointerId: 1, clientY: y })
    }
    fireEvent.pointerUp(cutoff, { pointerId: 1 })
    await settle()

    const moves = sentParams(sent)
    expect(moves).toHaveLength(4)
    expect(moves.every((move) => move.param === index)).toBe(true)
    expect(new Set(moves.map((move) => move.gesture)).size).toBe(1)
    expect(moves[0].gesture).toEqual(expect.any(Number))
    // Half the travel of 20 Hz to 20 kHz, down from the top.
    expect(stored("filter.cutoffHz")).toBeCloseTo(632.46, 1)
    expect(cutoff).toHaveAttribute("aria-valuetext", "632 Hz")
    expect(labels()).toEqual(["Add channel", "Change Cutoff"])

    await undo()
    await settle()
    expect(stored("filter.cutoffHz")).toBe(20000)
    expect(knob("Cutoff")).toHaveAttribute("aria-valuetext", "20.0 kHz")
    await redo()
    await settle()
    expect(knob("Cutoff")).toHaveAttribute("aria-valuetext", "632 Hz")
  })

  it("returns a control to its default on a double-click, and takes typed values", async () => {
    await addSynth()
    const resonance = knob("Resonance")
    tap(resonance, "End")
    await settle()
    expect(stored("filter.resonance")).toBe(1)
    fireEvent.doubleClick(resonance)
    await settle()
    expect(stored("filter.resonance")).toBe(0.1)
    expect(labels().slice(-2)).toEqual(["Change Resonance", "Change Resonance"])

    const cutoff = knob("Cutoff")
    fireEvent.keyDown(cutoff, { key: "Enter" })
    const entry = screen.getByRole("textbox", { name: "Cutoff" })
    fireEvent.change(entry, { target: { value: "1.2k" } })
    fireEvent.keyDown(entry, { key: "Enter" })
    await settle()
    expect(stored("filter.cutoffHz")).toBe(1200)
  })

  it("explains each control in the status bar, with its value", async () => {
    await addSynth()
    fireEvent.focus(knob("Cutoff"))
    expect(useHintStore.getState().text).toBe(
      "Cutoff: 20.0 kHz. Where the filter starts to cut, before the envelope, keys and LFOs move it"
    )
    fireEvent.blur(knob("Cutoff"))
    fireEvent.focus(knob("LFO 2 to pitch"))
    expect(useHintStore.getState().text).toBe(
      "LFO 2 to pitch: 0.00 st. How far the LFO bends the pitch: vibrato"
    )
    // Every setting has a line of its own.
    for (const info of descriptor.params) {
      const element = control(info.id)
      if (!element) continue
      const target =
        element.querySelector<HTMLElement>("[role=slider], [role=radio]") ??
        element
      fireEvent.focus(target)
      const text = useHintStore.getState().text ?? ""
      expect(text, info.id).toContain(
        `${info.name}: ${formatParam(info, stored(info.id))}. `
      )
      expect(text, info.id).not.toContain("Drag up or down")
      fireEvent.blur(target)
    }
  })
})

describe("the envelopes", () => {
  it("keep the editor and the knobs on the same four settings", async () => {
    await addSynth()
    const editor = within(settings()).getByRole("group", {
      name: "Amp envelope shape",
    })
    const node = (name: string) => within(editor).getByRole("slider", { name })
    expect(node("Attack")).toHaveAttribute("aria-valuenow", "2")
    expect(node("Release")).toHaveAttribute("aria-valuenow", "150")

    // A knob moves the editor.
    tap(knob("Amp release"), "End")
    await settle()
    expect(node("Release")).toHaveAttribute("aria-valuenow", "10000")
    expect(stored("ampEnvelope.releaseMs")).toBe(10000)

    // A node moves two knobs, as one undo step.
    const sent = vi.spyOn(backend, "dispatch")
    const before = history().entries.length
    const decay = node("Decay and sustain")
    fireEvent.keyDown(decay, { key: "ArrowRight" })
    fireEvent.keyDown(decay, { key: "ArrowDown" })
    fireEvent.keyUp(decay, { key: "ArrowDown" })
    fireEvent.keyUp(decay, { key: "ArrowRight" })
    await settle()
    expect(knob("Amp decay")).toHaveAttribute("aria-valuetext", "210 ms")
    expect(knob("Amp sustain")).toHaveAttribute("aria-valuetext", "79%")
    expect(stored("ampEnvelope.decayMs")).toBe(210)
    expect(stored("ampEnvelope.sustain")).toBeCloseTo(0.79, 5)
    expect(sentParams(sent).map((move) => move.param)).toEqual([
      paramIndex(descriptor, "ampEnvelope.decayMs"),
      paramIndex(descriptor, "ampEnvelope.sustain"),
    ])
    expect(new Set(sentParams(sent).map((move) => move.gesture)).size).toBe(1)
    expect(history().entries.length).toBe(before + 1)

    await undo()
    await settle()
    expect(knob("Amp decay")).toHaveAttribute("aria-valuetext", "200 ms")
    expect(knob("Amp sustain")).toHaveAttribute("aria-valuetext", "80%")
    expect(node("Decay and sustain")).toHaveAttribute("aria-valuenow", "200")
  })

  it("are two editors on two sets of settings", async () => {
    await addSynth()
    const filter = within(settings()).getByRole("group", {
      name: "Filter envelope shape",
    })
    tap(within(filter).getByRole("slider", { name: "Attack" }), "End")
    await settle()
    expect(stored("filterEnvelope.attackMs")).toBe(10000)
    expect(stored("ampEnvelope.attackMs")).toBe(2)
    expect(knob("Filter attack")).toHaveAttribute("aria-valuetext", "10.0 s")
  })
})

describe("sounds", () => {
  it.each(SYNTH_PRESETS.map((preset) => [preset.name, preset] as const))(
    "%s reads back from the project exactly as it is written",
    async (_, preset) => {
      await runAction("channel.addInstrument.subtractiveSynth")
      await settle()
      await dispatch({
        type: "setInstrumentParams",
        channel: channel(SYNTH).id,
        params: preset.params,
      })
      expect(params()).toEqual(preset.params)
      expect(matchingPreset(params())?.id).toBe(preset.id)
      // And once more, as a file would be read back after a save.
      const snapshot = await backend.documentSnapshot()
      const source = snapshot.project.channels.at(-1)?.source
      expect(source).toEqual({ type: "instrument", params: preset.params })
    }
  )

  it("are a handful of named sounds, each a small change from the init sound", () => {
    expect(SYNTH_PRESETS.map((preset) => preset.name)).toEqual([
      "Init",
      "Soft pad",
      "Pluck",
      "Bass",
      "Lead",
      "Keys",
      "Brass",
      "Sub",
    ])
    expect(SYNTH_PRESETS[0].params).toEqual(descriptor.defaults)
    for (const preset of SYNTH_PRESETS.slice(1)) {
      const changed = descriptor.params.filter(
        (info) => readParam(preset.params, info) !== info.default
      )
      expect(changed.length, preset.name).toBeGreaterThan(3)
      expect(changed.length, preset.name).toBeLessThan(20)
      expect(preset.description).not.toBe("")
    }
    const ids = SYNTH_PRESETS.map((preset) => preset.id)
    expect(new Set(ids).size).toBe(ids.length)
  })

  it("load from the menu in the settings, as one undo step each", async () => {
    const user = userEvent.setup()
    await addSynth()
    const menu = () =>
      within(settings()).getByRole("button", { name: /^Sound:/ })
    expect(menu()).toHaveTextContent("Init")

    await user.click(menu())
    const items = await screen.findAllByRole("menuitemcheckbox")
    expect(items.map((item) => item.textContent)).toEqual([
      "Soft pad",
      "Pluck",
      "Bass",
      "Lead",
      "Keys",
      "Brass",
      "Sub",
    ])
    // In the palette each is spelled out.
    expect(registry.get("channel.synthSound.softPad")?.title).toBe(
      "Load synth sound: Soft pad"
    )
    await user.click(screen.getByRole("menuitemcheckbox", { name: /Soft pad/ }))
    await settle()

    expect(params()).toEqual(synthPreset("softPad")?.params)
    expect(labels()).toEqual(["Add channel", "Load sound: Soft pad"])
    expect(menu()).toHaveTextContent("Soft pad")
    expect(knob("Unison voices")).toHaveAttribute("aria-valuetext", "5")
    expect(knob("Amp attack")).toHaveAttribute("aria-valuetext", "600 ms")
    expect(
      within(settings()).getByRole("radio", { name: "24 dB/oct" })
    ).toBeChecked()

    // The loaded sound is ticked the next time the list opens.
    await user.click(menu())
    expect(
      await screen.findByRole("menuitemcheckbox", { name: /Soft pad/ })
    ).toHaveAttribute("aria-checked", "true")
    await user.keyboard("{Escape}")

    // A moved knob makes the sound the user's own.
    tap(knob("Drive"), "End")
    await settle()
    expect(menu()).toHaveTextContent("Edited")

    await undo()
    await undo()
    await settle()
    expect(params()).toEqual(descriptor.defaults)
    expect(menu()).toHaveTextContent("Init")
  })

  it("go back to the init sound in one undo step", async () => {
    await addSynth()
    await runAction("channel.synthSound.bass")
    tap(knob("Cutoff"), "Home")
    await settle()
    expect(matchingPreset(params())).toBeNull()

    const sent = vi.spyOn(backend, "dispatch")
    await runAction("channel.initInstrument")
    await settle()
    expect(params()).toEqual(descriptor.defaults)
    expect(labels().at(-1)).toBe("Init subtractive synth")
    expect(sent.mock.lastCall?.[0]).toMatchObject({
      type: "batch",
      commands: [
        {
          type: "setInstrumentParams",
          channel: channel(SYNTH).id,
          params: descriptor.defaults,
        },
      ],
    })

    await undo()
    await settle()
    expect(stored("filter.cutoffHz")).toBe(20)
    expect(stored("voiceMode")).toBe(1)
  })

  it("are for synth channels only, and say so", async () => {
    await addSynth()
    fireEvent.click(nameButton("Kick"))
    await settle()
    const state = getAppState()
    for (const id of ["channel.initInstrument", "channel.synthSound.lead"]) {
      const action = registry.get(id)
      if (!action) throw new Error(`no action ${id}`)
      expect(disabledReason(action, state), id).toMatch(/only$/)
    }
    const before = project()
    await runAction("channel.synthSound.lead")
    await runAction("channel.initInstrument")
    await settle()
    expect(project()).toBe(before)
  })
})

describe("playing a synth", () => {
  it("holds a note while its name is pressed and always lets go", async () => {
    await addSynth()
    const on = vi.spyOn(backend, "auditionNoteOn")
    const off = vi.spyOn(backend, "auditionNoteOff")
    const synth = channel(SYNTH).id
    const button = nameButton(SYNTH)
    const press = () =>
      fireEvent.pointerDown(button, { pointerId: 1, button: 0 })

    press()
    expect(on.mock.calls).toEqual([[synth, 60, 0.8]])
    expect(off).not.toHaveBeenCalled()
    fireEvent.pointerUp(button, { pointerId: 1 })
    expect(off.mock.calls).toEqual([[synth, 60]])

    // An instrument sustains, so every way out of a press ends the note.
    press()
    fireEvent.pointerLeave(button)
    press()
    fireEvent.pointerCancel(button)
    press()
    fireEvent.blur(button)
    press()
    fireEvent.blur(window)
    expect(on).toHaveBeenCalledTimes(5)
    expect(off).toHaveBeenCalledTimes(5)

    // Nothing is held now, so nothing more is sent.
    fireEvent.pointerUp(button, { pointerId: 1 })
    fireEvent.pointerLeave(button)
    expect(off).toHaveBeenCalledTimes(5)
  })

  it("lets go of the note when the row goes away mid-press", async () => {
    const view = await addSynth()
    const on = vi.spyOn(backend, "auditionNoteOn")
    const off = vi.spyOn(backend, "auditionNoteOff")
    const synth = channel(SYNTH).id
    fireEvent.pointerDown(nameButton(SYNTH), { pointerId: 1, button: 0 })
    await dispatch({ type: "removeChannel", id: synth })
    await settle()
    expect(off.mock.calls).toEqual([[synth, 60]])

    // And when the whole rack goes away.
    fireEvent.pointerDown(nameButton("Kick"), { pointerId: 1, button: 0 })
    view.unmount()
    expect(off).toHaveBeenCalledTimes(on.mock.calls.length)
  })

  it("plays from the settings keyboard across six octaves, with velocity", async () => {
    await addSynth()
    const on = vi.spyOn(backend, "auditionNoteOn")
    const off = vi.spyOn(backend, "auditionNoteOff")
    const synth = channel(SYNTH).id
    const keys = within(settings()).getByRole("group", {
      name: "Piano keyboard",
    })
    expect(within(keys).getByRole("button", { name: "C2" })).toBeVisible()
    expect(within(keys).getByRole("button", { name: "C8" })).toBeVisible()
    // No key is lit: an instrument has no root key.
    expect(keys.querySelector('[aria-pressed="true"]')).toBeNull()

    const key = within(keys).getByRole("button", { name: "G6" })
    key.focus()
    fireEvent.keyDown(key, { key: "Enter" })
    expect(on.mock.calls).toEqual([[synth, 79, 0.8]])
    fireEvent.keyUp(key, { key: "Enter" })
    expect(off.mock.calls).toEqual([[synth, 79]])

    // With the pointer, lower on a key is louder.
    keys.getBoundingClientRect = () =>
      ({
        left: 0,
        top: 0,
        width: 783,
        height: 56,
        right: 783,
        bottom: 56,
      }) as DOMRect
    fireEvent.pointerDown(keys, {
      pointerId: 2,
      button: 0,
      clientX: 5,
      clientY: 54,
    })
    fireEvent.pointerUp(keys, { pointerId: 2 })
    expect(on).toHaveBeenCalledTimes(2)
    expect(on.mock.lastCall?.[0]).toBe(synth)
    expect(on.mock.lastCall?.[2]).toBeGreaterThan(0.9)
    expect(off).toHaveBeenCalledTimes(2)
  })

  it("turns steps into notes at C5, which the piano roll edits too", async () => {
    await addSynth()
    const synth = channel(SYNTH).id
    const steps = stepButtons(SYNTH)
    expect(steps).toHaveLength(16)
    fireEvent.pointerDown(steps[0], { pointerId: 1, button: 0 })
    fireEvent.pointerUp(steps[0], { pointerId: 1 })
    fireEvent.pointerDown(steps[6], { pointerId: 1, button: 0 })
    fireEvent.pointerUp(steps[6], { pointerId: 1 })
    await settle()

    expect(shownRow(SYNTH)).toBe("x.....x.........")
    expect(notesOf(SYNTH)).toMatchObject([
      { start: 0, length: 240, key: 60, velocity: 0.8 },
      { start: 1440, length: 240, key: 60, velocity: 0.8 },
    ])

    // The same lane takes the notes a piano roll writes: a held chord.
    const pattern = project().patterns[0].id
    await dispatch({
      type: "addNotes",
      pattern,
      channel: synth,
      notes: [
        { start: 1920, length: 960, key: 64 },
        { start: 1920, length: 960, key: 67 },
      ],
    })
    await settle()
    expect(notesOf(SYNTH)).toHaveLength(4)
    const thumbnail = screen.getByRole("button", {
      name: `${SYNTH} note preview. Open in piano roll`,
    })
    expect(thumbnail).toBeInTheDocument()
    expect(thumbnail.querySelector("[data-note-shapes]")).toBeInTheDocument()
    expect(thumbnail).toHaveTextContent("4 notes in the pattern")
    expect(
      screen.queryByRole("group", { name: `${SYNTH} steps` })
    ).not.toBeInTheDocument()
    await userEvent.click(
      screen.getByRole("button", { name: `${SYNTH} row view` })
    )
    await userEvent.click(
      await screen.findByRole("menuitemcheckbox", { name: /^Show steps/ })
    )
    // Step 9 holds notes the grid cannot show as a plain step.
    expect(document.querySelector('[data-detail-step="8"]')).not.toBeNull()

    await runAction("channel.fill4")
    await settle()
    expect(shownRow(SYNTH)).toBe("x...x...x...x...")
  })
})

describe("what only a sampler can do", () => {
  const rim = "/factory/Drums/Percussion/Rim 01.wav"
  const sample = () =>
    dragData(SAMPLE_DRAG_TYPE, JSON.stringify({ path: rim, name: "Rim 01" }))

  function fireDrag(type: "dragOver" | "drop", target: Element) {
    const dataTransfer = sample()
    const event = createEvent[type](target, { dataTransfer })
    Object.defineProperty(event, "clientY", { value: 40 })
    fireEvent(target, event)
    return { event, dataTransfer }
  }

  it("is greyed out for a synth, with the reason", async () => {
    await addSynth()
    const state = getAppState()
    const replace = registry.get("channel.replaceSample")
    if (!replace) throw new Error("no replace action")
    expect(replace.enabled?.(state)).toBe(false)
    expect(disabledReason(replace, state)).toBe("Samplers only")

    fireEvent.contextMenu(nameButton(SYNTH))
    const menu = await screen.findByRole("menu")
    const item = within(menu).getByRole("menuitem", { name: /Replace sample/ })
    expect(item).toHaveAttribute("aria-disabled", "true")
    expect(item).toHaveTextContent("Samplers only")
    // What a synth can do instead is offered, and the rest still works.
    expect(
      within(menu).getByRole("menuitem", { name: "Init instrument" })
    ).not.toHaveAttribute("aria-disabled", "true")
    expect(
      within(menu).getByRole("menuitem", { name: /Duplicate channel/ })
    ).not.toHaveAttribute("aria-disabled", "true")

    const pick = vi.spyOn(backend, "pickAudioFile")
    await runAction("channel.replaceSample")
    expect(pick).not.toHaveBeenCalled()
  })

  it("is offered again on a sampler", async () => {
    await addSynth()
    fireEvent.click(nameButton("Kick"))
    await settle()
    const replace = registry.get("channel.replaceSample")
    expect(replace?.enabled?.(getAppState())).toBe(true)
    fireEvent.contextMenu(nameButton("Kick"))
    const menu = await screen.findByRole("menu")
    expect(
      within(menu).getByRole("menuitem", { name: /Replace sample/ })
    ).not.toHaveAttribute("aria-disabled", "true")
    expect(
      within(menu).queryByRole("menuitem", { name: "Init instrument" })
    ).toBeNull()
  })

  it("is refused by the browser's replace action too", async () => {
    const unregister = registerBrowserActions()
    await addSynth()
    const action = registry.get("browser.replaceChannelSample")
    if (!action) throw new Error("no browser action")
    expect(action.enabled?.(getAppState())).toBe(false)
    expect(action.whyDisabled?.(getAppState())).toBe("Samplers only")
    unregister()
  })

  it("refuses a sample dropped on a synth's name and changes nothing", async () => {
    await addSynth()
    const replace = vi.spyOn(backend, "setChannelSampleFromFile")
    const add = vi.spyOn(backend, "addChannelFromFile")
    const before = project()

    const over = fireDrag("dragOver", nameButton(SYNTH))
    expect(over.dataTransfer.dropEffect).toBe("none")
    expect(nameButton(SYNTH)).toHaveTextContent("takes no sample")
    expect(nameButton(SYNTH)).not.toHaveTextContent("replace")

    fireDrag("drop", nameButton(SYNTH))
    await settle()
    expect(replace).not.toHaveBeenCalled()
    expect(add).not.toHaveBeenCalled()
    expect(project()).toBe(before)
    expect(toast.error).not.toHaveBeenCalled()
    expect(nameButton(SYNTH)).not.toHaveTextContent("takes no sample")

    // A sampler's name still takes it.
    const onKick = fireDrag("dragOver", nameButton("Kick"))
    expect(onKick.dataTransfer.dropEffect).toBe("copy")
    expect(nameButton("Kick")).toHaveTextContent("replace")
  })

  it("fails in the core as well, which is what the UI guards against", async () => {
    await runAction("channel.addInstrument.subtractiveSynth")
    await settle()
    const synth = channel(SYNTH).id
    await dispatch({ type: "updateSampler", id: synth, patch: { gain: 0.5 } })
    expect(toast.error).toHaveBeenCalledTimes(1)
    await dispatch({
      type: "setInstrumentParam",
      channel: channel("Kick").id,
      param: 0,
      value: 1,
    })
    expect(toast.error).toHaveBeenCalledTimes(2)
    expect(labels()).toEqual(["Add channel"])
  })
})
