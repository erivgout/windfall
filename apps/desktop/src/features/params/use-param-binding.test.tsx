import { act, fireEvent, render, screen } from "@testing-library/react"
import { useState } from "react"
import { describe, expect, it, vi } from "vitest"

import type { SynthParams } from "@/bindings"
import { ValueContextMenus } from "@/components/value-context-menu"

import { writeParam } from "./access"
import {
  instrumentDescriptor,
  paramIndex,
  type InstrumentParamsOf,
} from "./descriptors"
import { ParamControl } from "./param-control"
import { ParamEnvelope } from "./param-envelope"
import { useParamBinding, type SetParam } from "./use-param-binding"

const synth = instrumentDescriptor("subtractiveSynth")
type Params = InstrumentParamsOf<"subtractiveSynth">

type Call = { id: string; value: number; gesture: number | undefined }

/** An editor over settings kept in state, as a store would keep them. */
function Editor({
  calls,
  ids,
  initial = synth.defaults,
  apply = true,
  envelope = false,
}: {
  calls: Call[]
  ids: string[]
  initial?: Params | null
  /** Whether a change reaches the stored settings. */
  apply?: boolean
  envelope?: boolean
}) {
  const [params, setParams] = useState(initial)
  const setParam: SetParam = (index, value, gesture) => {
    const info = synth.params[index]
    calls.push({ id: info.id, value, gesture })
    if (apply) {
      setParams((current) => current && writeParam(current, info, value))
    }
  }
  const bind = useParamBinding({ descriptor: synth, params, setParam })
  return (
    <>
      {ids.map((id) => (
        <ParamControl key={id} {...bind(id)} />
      ))}
      {envelope && (
        <ParamEnvelope
          bind={bind}
          prefix="ampEnvelope"
          aria-label="Amp envelope"
        />
      )}
    </>
  )
}

function drag(target: HTMLElement, pixelsUp: number[], end = true) {
  fireEvent.pointerDown(target, { pointerId: 1, button: 0, clientY: 300 })
  for (const up of pixelsUp) {
    fireEvent.pointerMove(target, { pointerId: 1, clientY: 300 - up })
  }
  if (end) fireEvent.pointerUp(target, { pointerId: 1 })
}

const settle = () => act(() => Promise.resolve())

describe("useParamBinding", () => {
  it("reads each control's value from the settings", () => {
    const params = writeParam(
      synth.defaults,
      synth.params[paramIndex(synth, "filter.cutoffHz")],
      440
    )
    render(
      <Editor
        calls={[]}
        initial={params}
        ids={["filter.cutoffHz", "filter.mode", "oscillators.0.level"]}
      />
    )
    expect(screen.getByRole("slider", { name: "Cutoff" })).toHaveAttribute(
      "aria-valuetext",
      "440 Hz"
    )
    expect(screen.getByRole("radio", { name: "Low-pass" })).toBeChecked()
    expect(screen.getByRole("slider", { name: "Osc 1 level" })).toHaveAttribute(
      "aria-valuetext",
      "100%"
    )
  })

  it("sends a whole drag under one gesture id, and the next under another", async () => {
    const calls: Call[] = []
    render(<Editor calls={calls} ids={["filter.resonance", "filter.drive"]} />)
    const resonance = screen.getByRole("slider", { name: "Resonance" })

    drag(resonance, [20, 40, 60])
    await settle()
    expect(calls).toHaveLength(3)
    expect(calls.map((call) => call.id)).toEqual(
      Array(3).fill("filter.resonance")
    )
    const first = calls[0].gesture
    expect(first).toEqual(expect.any(Number))
    expect(calls.every((call) => call.gesture === first)).toBe(true)

    drag(resonance, [10])
    drag(screen.getByRole("slider", { name: "Drive" }), [10])
    await settle()
    const gestures = new Set(calls.map((call) => call.gesture))
    expect(gestures.size).toBe(3)
    expect(calls.at(-1)).toMatchObject({ id: "filter.drive" })
  })

  it("addresses a setting by its place in the descriptor", () => {
    const setParam = vi.fn()
    function One() {
      const bind = useParamBinding({
        descriptor: synth,
        params: synth.defaults,
        setParam,
      })
      return <ParamControl {...bind("lfos.1.rateHz")} />
    }
    render(<One />)
    const knob = screen.getByRole("slider", { name: "LFO 2 rate" })
    fireEvent.keyDown(knob, { key: "End" })
    fireEvent.keyUp(knob, { key: "End" })
    expect(setParam).toHaveBeenCalledWith(44, 30, expect.any(Number))
  })

  it("shows the dragged value while the stored one lags behind", async () => {
    const calls: Call[] = []
    // Nothing is applied, as when the backend has not answered yet.
    render(<Editor calls={calls} ids={["filter.resonance"]} apply={false} />)
    const knob = screen.getByRole("slider")
    drag(knob, [100], false)
    expect(Number(knob.getAttribute("aria-valuenow"))).toBeCloseTo(0.6, 5)

    // Once the gesture is over and the send has settled, the stored value
    // is the truth again.
    fireEvent.pointerUp(knob, { pointerId: 1 })
    await settle()
    expect(Number(knob.getAttribute("aria-valuenow"))).toBeCloseTo(0.1, 5)
  })

  it("keeps the dragged value until the send it returned has settled", async () => {
    let finish: () => void = () => undefined
    const pending = new Promise<void>((resolve) => {
      finish = resolve
    })
    function Slow() {
      const bind = useParamBinding({
        descriptor: synth,
        params: synth.defaults,
        setParam: () => pending,
      })
      return <ParamControl {...bind("filter.resonance")} />
    }
    render(<Slow />)
    const knob = screen.getByRole("slider")
    drag(knob, [100])
    await settle()
    expect(Number(knob.getAttribute("aria-valuenow"))).toBeCloseTo(0.6, 5)

    finish()
    await settle()
    expect(Number(knob.getAttribute("aria-valuenow"))).toBeCloseTo(0.1, 5)
  })

  it("sends a click on a toggle or a choice as a gesture of its own", () => {
    const calls: Call[] = []
    render(<Editor calls={calls} ids={["filter.mode", "voiceMode"]} />)
    fireEvent.click(screen.getByRole("radio", { name: "High-pass" }))
    fireEvent.click(screen.getByRole("radio", { name: "Mono" }))
    expect(calls).toEqual([
      { id: "filter.mode", value: 2, gesture: expect.any(Number) },
      { id: "voiceMode", value: 1, gesture: expect.any(Number) },
    ])
    expect(calls[0].gesture).not.toBe(calls[1].gesture)
  })

  it("disables every control when there are no settings", () => {
    const calls: Call[] = []
    render(
      <Editor
        calls={calls}
        initial={null}
        ids={["filter.cutoffHz", "filter.mode"]}
      />
    )
    const knob = screen.getByRole("slider", { name: "Cutoff" })
    expect(knob).toHaveAttribute("aria-disabled", "true")
    // It shows the default, so the panel keeps its shape.
    expect(knob).toHaveAttribute("aria-valuetext", "20.0 kHz")
    for (const option of screen.getAllByRole("radio")) {
      expect(option).toBeDisabled()
    }
    fireEvent.doubleClick(knob)
    expect(calls).toEqual([])
  })

  it("hands each control the menu entries of its setting", async () => {
    const asked: [string, number][] = []
    const makeClip = vi.fn()
    function Bound() {
      const bind = useParamBinding({
        descriptor: synth,
        params: synth.defaults,
        setParam: () => undefined,
        // Where an entry about the setting itself goes, such as automation.
        contextItems: (info, index) => {
          asked.push([info.id, index])
          return info.id === "filter.cutoffHz"
            ? [{ title: "Create automation clip", run: makeClip }]
            : []
        },
      })
      return (
        <ValueContextMenus>
          <ParamControl {...bind("filter.cutoffHz")} />
          <ParamControl {...bind("filter.resonance")} />
        </ValueContextMenus>
      )
    }
    render(<Bound />)
    expect(asked).toContainEqual([
      "filter.cutoffHz",
      paramIndex(synth, "filter.cutoffHz"),
    ])

    const open = async (name: string) => {
      const knob = screen
        .getByRole("slider", { name })
        .closest("[data-slot=param-control]")
      if (!knob) throw new Error(`no knob for ${name}`)
      fireEvent.contextMenu(knob)
      await act(async () => {
        await Promise.resolve()
      })
      return screen.getAllByRole("menuitem").map((item) => item.textContent)
    }
    // The setting's own entry comes first, then what every control has.
    const cutoff = await open("Cutoff")
    expect(cutoff[0]).toBe("Create automation clip")
    expect(cutoff).toContain("Copy value")
    fireEvent.click(screen.getByRole("menuitem", { name: /automation/ }))
    expect(makeClip).toHaveBeenCalledTimes(1)

    const resonance = await open("Resonance")
    expect(resonance).not.toContain("Create automation clip")
    expect(resonance).toContain("Copy value")
  })

  it("has no entries of its own for a setting unless it is given some", () => {
    function Bound() {
      const bind = useParamBinding({
        descriptor: synth,
        params: synth.defaults,
        setParam: () => undefined,
      })
      const first = bind("filter.cutoffHz").contextItems
      // The same empty list every time, so nothing renders again for it.
      expect(bind("filter.resonance").contextItems).toBe(first)
      return <output>{first.length}</output>
    }
    render(<Bound />)
    expect(screen.getByRole("status")).toHaveTextContent("0")
  })

  it("fails on an id the descriptor does not have", () => {
    const error = vi.spyOn(console, "error").mockImplementation(() => undefined)
    expect(() => render(<Editor calls={[]} ids={["filter.cutoff"]} />)).toThrow(
      'Subtractive synth has no parameter "filter.cutoff"'
    )
    error.mockRestore()
  })

  it("hands a control the same handlers on every render", () => {
    const seen: unknown[] = []
    function Probe({ tick }: { tick: number }) {
      const bind = useParamBinding({
        descriptor: synth,
        params: synth.defaults,
        setParam: () => tick,
      })
      const bound = bind("gain")
      seen.push(bound.onValueChange, bound.onGestureStart, bound.onGestureEnd)
      seen.push(bind.group("ampEnvelope"))
      return null
    }
    const { rerender } = render(<Probe tick={1} />)
    rerender(<Probe tick={2} />)
    expect(seen.slice(0, 4)).toEqual(seen.slice(4))
    expect(seen[0]).toBe(seen[4])
  })

  it("calls the newest setParam, not the one of the first render", () => {
    const first = vi.fn()
    const second = vi.fn()
    function Swap({ setParam }: { setParam: SetParam }) {
      const bind = useParamBinding({
        descriptor: synth,
        params: synth.defaults,
        setParam,
      })
      return <ParamControl {...bind("filter.resonance")} />
    }
    const { rerender } = render(<Swap setParam={first} />)
    rerender(<Swap setParam={second} />)
    fireEvent.doubleClick(screen.getByRole("slider"))
    // Already at the default, so move it first.
    fireEvent.keyDown(screen.getByRole("slider"), { key: "End" })
    fireEvent.keyUp(screen.getByRole("slider"), { key: "End" })
    expect(first).not.toHaveBeenCalled()
    expect(second).toHaveBeenCalledTimes(1)
  })
})

describe("ParamEnvelope", () => {
  const KNOBS = [
    "ampEnvelope.attackMs",
    "ampEnvelope.decayMs",
    "ampEnvelope.sustain",
    "ampEnvelope.releaseMs",
  ]
  const node = (name: string) => screen.getByRole("slider", { name })

  it("shows the four settings with the descriptor's ranges", () => {
    render(<Editor calls={[]} ids={[]} envelope />)
    const group = screen.getByRole("group", { name: "Amp envelope" })
    expect(group).toBeVisible()
    expect(node("Attack")).toHaveAttribute("aria-valuemax", "10000")
    expect(node("Attack")).toHaveAttribute("aria-valuenow", "2")
    expect(node("Release")).toHaveAttribute("aria-valuenow", "150")
  })

  it("moves two settings with one node, as one gesture, and the knobs follow", async () => {
    const calls: Call[] = []
    render(<Editor calls={calls} ids={KNOBS} envelope />)
    const decay = node("Decay and sustain")
    const sustainKnob = screen.getByRole("slider", { name: "Amp sustain" })
    const decayKnob = screen.getByRole("slider", { name: "Amp decay" })

    // One key hold: right moves the decay time, down the sustain level.
    fireEvent.keyDown(decay, { key: "ArrowRight" })
    fireEvent.keyDown(decay, { key: "ArrowDown" })
    fireEvent.keyDown(decay, { key: "ArrowDown" })
    expect(sustainKnob).toHaveAttribute("aria-valuetext", "78%")
    expect(decayKnob).toHaveAttribute("aria-valuetext", "210 ms")
    fireEvent.keyUp(decay, { key: "ArrowDown" })
    fireEvent.keyUp(decay, { key: "ArrowRight" })
    await settle()

    expect(calls.map((call) => call.id)).toEqual([
      "ampEnvelope.decayMs",
      "ampEnvelope.sustain",
      "ampEnvelope.sustain",
    ])
    expect(new Set(calls.map((call) => call.gesture)).size).toBe(1)
    expect(calls[0].gesture).toEqual(expect.any(Number))
  })

  it("follows a knob while the knob is dragged", () => {
    render(<Editor calls={[]} ids={KNOBS} envelope apply={false} />)
    const knob = screen.getByRole("slider", { name: "Amp sustain" })
    drag(knob, [-100], false)
    // The editor's decay node carries the sustain level.
    expect(node("Decay and sustain")).toHaveAttribute(
      "aria-valuetext",
      expect.stringContaining("30%")
    )
    fireEvent.pointerUp(knob, { pointerId: 1 })
  })

  it("keeps what the core will: a decay of at least a millisecond", () => {
    const calls: Call[] = []
    render(<Editor calls={calls} ids={KNOBS} envelope />)
    const decay = node("Decay and sustain")
    fireEvent.keyDown(decay, { key: "Home" })
    fireEvent.keyUp(decay, { key: "Home" })
    expect(calls.at(-1)).toMatchObject({ id: "ampEnvelope.decayMs", value: 1 })
    expect(screen.getByRole("slider", { name: "Amp decay" })).toHaveAttribute(
      "aria-valuetext",
      "1 ms"
    )
  })
})

describe("types", () => {
  it("keeps the settings type of the descriptor", () => {
    const params: SynthParams = synth.defaults
    expect(params.oscillators).toHaveLength(3)
  })
})
