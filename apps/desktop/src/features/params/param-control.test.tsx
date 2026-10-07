import { fireEvent, render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { useState } from "react"
import { describe, expect, it, vi } from "vitest"

import type { ParamInfo } from "@/bindings"
import { useHintStore } from "@/lib/store/hint"

import {
  effectDescriptor,
  instrumentDescriptor,
  paramInfo,
} from "./descriptors"
import { ParamControl, type ParamControlProps } from "./param-control"

const synth = instrumentDescriptor("subtractiveSynth")
const compressor = effectDescriptor("compressor")
const delay = effectDescriptor("delay")

type Log = string[]

/** A control that keeps its own value and writes down what it reports. */
function Live({
  info,
  start,
  log = [],
  ...props
}: Partial<ParamControlProps> & {
  info: ParamInfo
  start?: number
  log?: Log
}) {
  const [value, setValue] = useState(start ?? info.default)
  return (
    <ParamControl
      info={info}
      value={value}
      onGestureStart={() => log.push("start")}
      onValueChange={(next) => {
        log.push(`change ${next}`)
        setValue(next)
      }}
      onGestureEnd={() => log.push("end")}
      {...props}
    />
  )
}

const hint = () => useHintStore.getState().text

describe("ParamControl: a number", () => {
  it("is a knob named after the setting, with its range, value and unit", () => {
    render(<Live info={paramInfo(synth, "filter.cutoffHz")} start={1200} />)
    const knob = screen.getByRole("slider", { name: "Cutoff" })
    expect(knob).toHaveAttribute("aria-valuemin", "20")
    expect(knob).toHaveAttribute("aria-valuemax", "20000")
    expect(knob).toHaveAttribute("aria-valuetext", "1.20 kHz")
    const root = knob.closest('[data-slot="param-control"]') as HTMLElement
    expect(root).toHaveAttribute("data-param", "filter.cutoffHz")
    expect(root).toHaveTextContent("Cutoff")
    expect(root).toHaveTextContent("1.20 kHz")
  })

  it("follows the scale of the descriptor", () => {
    render(<Live info={paramInfo(synth, "filter.cutoffHz")} start={20} />)
    const knob = screen.getByRole("slider")
    // Half the travel of a logarithmic 20 Hz to 20 kHz is their geometric mean.
    fireEvent.pointerDown(knob, { pointerId: 1, button: 0, clientY: 200 })
    fireEvent.pointerMove(knob, { pointerId: 1, clientY: 100 })
    fireEvent.pointerUp(knob, { pointerId: 1 })
    expect(Number(knob.getAttribute("aria-valuenow"))).toBeCloseTo(632.5, 0)
  })

  it("steps an integer by whole numbers", () => {
    const log: Log = []
    render(<Live info={paramInfo(synth, "oscillators.0.coarse")} log={log} />)
    const knob = screen.getByRole("slider", { name: "Osc 1 coarse" })
    fireEvent.keyDown(knob, { key: "ArrowUp" })
    fireEvent.keyUp(knob, { key: "ArrowUp" })
    expect(log).toEqual(["start", "change 1", "end"])
    expect(knob).toHaveAttribute("aria-valuetext", "+1 st")
  })

  it("draws an offset around zero from the middle, and pan too", () => {
    const { unmount } = render(
      <Live info={paramInfo(synth, "oscillators.0.coarse")} />
    )
    expect(
      document.querySelector('[data-slot="param-control"]')
    ).toHaveAttribute("data-bipolar")
    unmount()
    render(<Live info={paramInfo(synth, "pan")} />)
    expect(
      document.querySelector('[data-slot="param-control"]')
    ).toHaveAttribute("data-bipolar")
    expect(screen.getByRole("slider")).toHaveAttribute("aria-valuetext", "C")
  })

  it("returns to the default on a double-click, as one gesture", () => {
    const log: Log = []
    const info = paramInfo(synth, "filter.resonance")
    render(<Live info={info} start={0.8} log={log} />)
    const knob = screen.getByRole("slider")
    fireEvent.doubleClick(knob)
    expect(log).toEqual(["start", `change ${info.default}`, "end"])
    expect(knob).toHaveAttribute("aria-valuetext", "10%")
  })

  it("takes typed values in the setting's own unit", () => {
    render(<Live info={paramInfo(synth, "filter.cutoffHz")} />)
    const knob = screen.getByRole("slider")
    fireEvent.keyDown(knob, { key: "Enter" })
    const entry = screen.getByRole("textbox", { name: "Cutoff" })
    fireEvent.change(entry, { target: { value: "1.5k" } })
    fireEvent.keyDown(entry, { key: "Enter" })
    expect(knob).toHaveAttribute("aria-valuenow", "1500")

    fireEvent.keyDown(knob, { key: "Enter" })
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "5" },
    })
    fireEvent.keyDown(screen.getByRole("textbox"), { key: "Enter" })
    expect(knob).toHaveAttribute("aria-valuenow", "20")
  })

  it("leaves room under its label for the tails of g, p and q", () => {
    render(<Live info={paramInfo(synth, "filter.cutoffHz")} start={1200} />)
    const label = document.querySelector("[data-slot=knob-label] > span")
    // The label is cut off at its box, so the box reaches below the line.
    expect(label).toHaveClass("truncate", "py-[0.25em]", "-my-[0.25em]")
  })

  it("shows a short label and keeps the full name for the slider", () => {
    render(
      <Live info={paramInfo(synth, "oscillators.1.level")} label="Level" />
    )
    const knob = screen.getByRole("slider", { name: "Osc 2 level" })
    const root = knob.closest('[data-slot="param-control"]') as HTMLElement
    expect(within(root).getByText("Level")).toBeVisible()
    expect(root).not.toHaveTextContent("Osc 2")
  })

  it("explains itself in the status bar, with the value as it moves", () => {
    render(<Live info={paramInfo(synth, "filter.resonance")} />)
    const knob = screen.getByRole("slider")
    fireEvent.focus(knob)
    expect(hint()).toBe(
      "Resonance: 10%. Drag up or down, Shift for fine, double-click for 10%"
    )
    fireEvent.keyDown(knob, { key: "End" })
    fireEvent.keyUp(knob, { key: "End" })
    expect(hint()).toMatch(/^Resonance: 100%\./)
    fireEvent.blur(knob)
    expect(hint()).toBeNull()
  })

  it("puts a description in the status bar in place of the how-to", () => {
    render(
      <Live
        info={paramInfo(synth, "filter.drive")}
        description="Overdrive before the filter"
      />
    )
    fireEvent.focus(screen.getByRole("slider"))
    expect(hint()).toBe("Drive: 0%. Overdrive before the filter")
  })

  it("does nothing while disabled", () => {
    const log: Log = []
    render(<Live info={paramInfo(synth, "gain")} log={log} disabled />)
    const knob = screen.getByRole("slider", { name: "Volume" })
    expect(knob).toHaveAttribute("aria-disabled", "true")
    fireEvent.doubleClick(knob)
    fireEvent.keyDown(knob, { key: "ArrowUp" })
    expect(log).toEqual([])
  })
})

describe("ParamControl: a toggle", () => {
  const info = paramInfo(compressor, "autoMakeup")

  it("is a light that reads On or Off, one gesture per click", async () => {
    const user = userEvent.setup()
    const log: Log = []
    render(<Live info={info} log={log} />)
    const light = screen.getByRole("button", { name: "Auto makeup" })
    expect(light).toHaveAttribute("aria-pressed", "false")
    const root = light.closest('[data-slot="param-control"]') as HTMLElement
    expect(root).toHaveTextContent("Auto makeup")
    expect(root).toHaveTextContent("Off")

    await user.click(light)
    expect(log).toEqual(["start", "change 1", "end"])
    expect(light).toHaveAttribute("aria-pressed", "true")
    expect(root).toHaveTextContent("On")
    expect(screen.queryByRole("slider")).toBeNull()
  })

  it("is a switch at the end of the row when inline", async () => {
    const user = userEvent.setup()
    const log: Log = []
    render(<Live info={info} log={log} layout="inline" />)
    const toggle = screen.getByRole("switch", { name: "Auto makeup" })
    expect(toggle).not.toBeChecked()
    await user.click(toggle)
    expect(log).toEqual(["start", "change 1", "end"])
    expect(toggle).toBeChecked()
  })

  it("returns to the default from its label and with Ctrl-click", async () => {
    const user = userEvent.setup()
    const log: Log = []
    render(<Live info={info} start={1} log={log} />)
    await user.dblClick(screen.getByText("Auto makeup"))
    expect(log).toEqual(["start", "change 0", "end"])

    // Already at the default: nothing to report, and no flip either.
    log.length = 0
    fireEvent.click(screen.getByRole("button", { name: "Auto makeup" }), {
      ctrlKey: true,
    })
    expect(log).toEqual([])
    expect(screen.getByRole("button")).toHaveAttribute("aria-pressed", "false")
  })

  it("says what a click will do", () => {
    render(<Live info={info} />)
    fireEvent.focus(screen.getByRole("button"))
    expect(hint()).toBe("Auto makeup: Off. Click to turn it on")
  })
})

describe("ParamControl: a choice", () => {
  it("is a strip of buttons for up to four options", async () => {
    const user = userEvent.setup()
    const log: Log = []
    render(<Live info={paramInfo(synth, "filter.mode")} log={log} />)
    const group = screen.getByRole("radiogroup", { name: "Filter mode" })
    const options = within(group).getAllByRole("radio")
    expect(options.map((option) => option.textContent)).toEqual([
      "Low-pass",
      "Band-pass",
      "High-pass",
    ])
    expect(options[0]).toBeChecked()

    await user.click(options[2])
    expect(log).toEqual(["start", "change 2", "end"])
    expect(options[2]).toBeChecked()
    expect(options[0]).not.toBeChecked()

    // The chosen one again changes nothing.
    await user.click(options[2])
    expect(log).toHaveLength(3)
  })

  it("moves the choice with the arrow keys", () => {
    const log: Log = []
    render(<Live info={paramInfo(synth, "voiceMode")} log={log} />)
    const options = screen.getAllByRole("radio")
    options[0].focus()
    fireEvent.keyDown(options[0], { key: "ArrowRight" })
    expect(options[1]).toBeChecked()
    expect(options[1]).toHaveFocus()
    fireEvent.keyDown(options[1], { key: "ArrowLeft" })
    fireEvent.keyDown(options[0], { key: "ArrowLeft" })
    expect(options[2]).toBeChecked()
    expect(log.filter((line) => line.startsWith("change"))).toEqual([
      "change 1",
      "change 0",
      "change 2",
    ])
    // One tab stop: only the chosen option is in the tab order.
    expect(options.map((option) => option.tabIndex)).toEqual([-1, -1, 0])
  })

  it("is a list for more than four options", async () => {
    const user = userEvent.setup()
    const log: Log = []
    render(<Live info={paramInfo(synth, "oscillators.0.waveform")} log={log} />)
    expect(screen.queryByRole("radiogroup")).toBeNull()
    const box = screen.getByRole("combobox", { name: "Osc 1 waveform" })
    expect(box).toHaveTextContent("Saw")

    await user.click(box)
    const options = await screen.findAllByRole("option")
    expect(options.map((option) => option.textContent)).toEqual([
      "Sine",
      "Triangle",
      "Saw",
      "Square",
      "Pulse",
      "White noise",
      "Pink noise",
    ])
    await user.click(screen.getByRole("option", { name: "Pulse" }))
    expect(log).toEqual(["start", "change 4", "end"])
    expect(box).toHaveTextContent("Pulse")
  })

  it("can be told which of the two to be", () => {
    render(
      <Live info={paramInfo(delay, "mode")} choiceStyle="select" label={null} />
    )
    expect(screen.getByRole("combobox", { name: "Mode" })).toBeVisible()
    expect(screen.queryByRole("radiogroup")).toBeNull()
  })

  it("draws an icon for each option when given one", () => {
    render(
      <Live
        info={paramInfo(synth, "filter.slope")}
        choiceIcon={(choice) => <i data-icon={choice.value} />}
      />
    )
    expect(document.querySelectorAll("[data-icon]")).toHaveLength(2)
    expect(document.querySelector('[data-icon="db24"]')).not.toBeNull()
  })

  it("returns to the default with Ctrl-click and from its label", async () => {
    const user = userEvent.setup()
    const log: Log = []
    render(<Live info={paramInfo(synth, "filter.mode")} start={2} log={log} />)
    fireEvent.click(screen.getAllByRole("radio")[1], { ctrlKey: true })
    expect(log).toEqual(["start", "change 0", "end"])
    expect(screen.getAllByRole("radio")[0]).toBeChecked()

    await user.click(screen.getAllByRole("radio")[1])
    log.length = 0
    await user.dblClick(screen.getByText("Filter mode"))
    expect(log).toEqual(["start", "change 0", "end"])
  })

  it("names the chosen option in the status bar", () => {
    render(<Live info={paramInfo(synth, "filter.slope")} start={1} />)
    fireEvent.focus(screen.getAllByRole("radio")[1])
    expect(hint()).toBe("Filter slope: 24 dB/oct. Click to choose another")
  })
})

describe("ParamControl", () => {
  it("renders only the control whose value changed", () => {
    const cutoff = paramInfo(synth, "filter.cutoffHz")
    const mode = paramInfo(synth, "filter.mode")
    // The icon is drawn whenever the choice renders, which counts renders.
    const drawn = vi.fn()
    const icon = (choice: { value: string }) => {
      drawn(choice.value)
      return null
    }
    const noop = () => undefined
    function Pair({ value }: { value: number }) {
      return (
        <>
          <ParamControl info={cutoff} value={value} onValueChange={noop} />
          <ParamControl
            info={mode}
            value={0}
            onValueChange={noop}
            choiceIcon={icon}
          />
        </>
      )
    }
    const { rerender } = render(<Pair value={100} />)
    expect(drawn).toHaveBeenCalledTimes(3)
    drawn.mockClear()
    rerender(<Pair value={200} />)
    // The choice kept its props, so it was not rendered again.
    expect(drawn).not.toHaveBeenCalled()
    expect(screen.getByRole("slider", { name: "Cutoff" })).toHaveAttribute(
      "aria-valuenow",
      "200"
    )
  })
})
