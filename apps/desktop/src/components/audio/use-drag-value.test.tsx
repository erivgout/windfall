// SPDX-License-Identifier: MIT
import * as React from "react"
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, describe, expect, it, vi } from "vitest"

import { Knob, type KnobProps } from "./knob"
import { percentUnit } from "./units"
import {
  clampValue,
  defaultFormat,
  dragTravel,
  linearScale,
  logScale,
  normalizedToValue,
  parseEntry,
  powerScale,
  snapValue,
  stepValue,
  valueToNormalized,
} from "./use-drag-value"

describe("ranges and steps", () => {
  it("clamps", () => {
    expect(clampValue(5, 0, 1)).toBe(1)
    expect(clampValue(-5, 0, 1)).toBe(0)
    expect(clampValue(Number.NaN, 2, 4)).toBe(2)
    expect(clampValue(-Infinity, -60, 6)).toBe(-60)
  })

  it("snaps to the step without float dust", () => {
    expect(snapValue(0.34, 0, 1, 0.1)).toBe(0.3)
    expect(snapValue(0.1 + 0.2, 0, 1, 0.1)).toBe(0.3)
    expect(snapValue(7.6, 0, 10, 1)).toBe(8)
    expect(snapValue(12, 0, 10, 1)).toBe(10)
    expect(snapValue(-3.2, -5, 5, 0.5)).toBe(-3)
    expect(snapValue(0.123456, 0, 1)).toBe(0.123456)
  })

  it("steps a continuous control along its travel", () => {
    const range = { min: 0, max: 200 }
    expect(stepValue(100, 1, "normal", range)).toBeCloseTo(102, 10)
    expect(stepValue(100, -1, "fine", range)).toBeCloseTo(99.8, 10)
    expect(stepValue(100, 1, "coarse", range)).toBeCloseTo(120, 10)
    expect(stepValue(199, 1, "coarse", range)).toBe(200)
    expect(stepValue(0, -1, "normal", range)).toBe(0)
  })

  it("steps a stepped control by whole steps", () => {
    const range = { min: 0, max: 127, step: 1 }
    expect(stepValue(60, 1, "normal", range)).toBe(61)
    expect(stepValue(60, 1, "fine", range)).toBe(61)
    expect(stepValue(60, -1, "coarse", range)).toBe(50)
    expect(stepValue(125, 1, "coarse", range)).toBe(127)
  })

  it("uses keyStep for normal steps and step for fine ones", () => {
    const range = { min: 10, max: 522, step: 0.01, keyStep: 1 }
    expect(stepValue(128, 1, "normal", range)).toBe(129)
    expect(stepValue(128, 1, "fine", range)).toBe(128.01)
    expect(stepValue(128, -1, "coarse", range)).toBe(118)
  })

  it("scales pointer travel, ten times slower in fine mode", () => {
    expect(dragTravel(100, 200, false)).toBe(0.5)
    expect(dragTravel(100, 200, true)).toBeCloseTo(0.05, 10)
    expect(dragTravel(-50, 200, false)).toBe(-0.25)
    expect(dragTravel(100, 0, false)).toBe(0)
  })
})

describe("scales", () => {
  it("maps linearly", () => {
    expect(linearScale.toNormalized(5, 0, 10)).toBe(0.5)
    expect(linearScale.fromNormalized(0.25, -1, 1)).toBe(-0.5)
  })

  it("maps logarithmically", () => {
    expect(logScale.toNormalized(200, 20, 20000)).toBeCloseTo(1 / 3, 10)
    expect(logScale.fromNormalized(2 / 3, 20, 20000)).toBeCloseTo(2000, 6)
    expect(valueToNormalized(20, 20, 20000, "log")).toBe(0)
    expect(valueToNormalized(20000, 20, 20000, "log")).toBeCloseTo(1, 10)
  })

  it("round-trips a custom scale and stays in range", () => {
    const scale = powerScale(3)
    for (const value of [0, 1, 12, 400, 5000]) {
      const position = valueToNormalized(value, 0, 5000, scale)
      expect(position).toBeGreaterThanOrEqual(0)
      expect(position).toBeLessThanOrEqual(1)
      expect(normalizedToValue(position, 0, 5000, scale)).toBeCloseTo(value, 6)
    }
    expect(normalizedToValue(1.5, 0, 5000, scale)).toBe(5000)
    expect(normalizedToValue(-1, 0, 5000, scale)).toBe(0)
  })
})

describe("text entry", () => {
  it("parses numbers with units into the range", () => {
    const range = { min: -60, max: 6 }
    expect(parseEntry("-6", range)).toBe(-6)
    expect(parseEntry("−6 dB", range)).toBe(-6)
    expect(parseEntry("99", range)).toBe(6)
    expect(parseEntry("-inf", range)).toBe(-60)
    expect(parseEntry("loud", range)).toBeNull()
    expect(parseEntry("", range)).toBeNull()
  })

  it("uses the control's own parser and step", () => {
    expect(parseEntry("50%", { min: 0, max: 1 }, percentUnit.parse)).toBe(0.5)
    expect(parseEntry("3.7", { min: 0, max: 10, step: 1 })).toBe(4)
  })

  it("formats to the precision of the step or the range", () => {
    expect(defaultFormat(4, { min: 0, max: 10, step: 1 })).toBe("4")
    expect(defaultFormat(0.5, { min: 0, max: 1 })).toBe("0.50")
    expect(defaultFormat(120.25, { min: 10, max: 522, step: 0.01 })).toBe(
      "120.25"
    )
  })
})

type Calls = {
  values: number[]
  order: string[]
}

function renderKnob(props: Partial<KnobProps> = {}) {
  const calls: Calls = { values: [], order: [] }
  function Harness() {
    const [value, setValue] = React.useState(props.value ?? 0.5)
    return (
      <Knob
        aria-label="Amount"
        defaultValue={0.25}
        {...props}
        value={value}
        onValueChange={(next) => {
          calls.values.push(next)
          calls.order.push("change")
          setValue(next)
        }}
        onGestureStart={() => calls.order.push("start")}
        onGestureEnd={() => calls.order.push("end")}
      />
    )
  }
  render(<Harness />)
  const slider = screen.getByRole("slider", { name: "Amount" })
  return { slider, calls }
}

const valueOf = (slider: HTMLElement) =>
  Number(slider.getAttribute("aria-valuenow"))

function drag(
  slider: HTMLElement,
  fromY: number,
  toY: number,
  init: { shiftKey?: boolean; release?: boolean } = {}
) {
  fireEvent.pointerDown(slider, { pointerId: 1, button: 0, clientY: fromY })
  fireEvent.pointerMove(slider, {
    pointerId: 1,
    clientY: toY,
    shiftKey: init.shiftKey,
  })
  if (init.release !== false) {
    fireEvent.pointerUp(slider, { pointerId: 1, clientY: toY })
  }
}

describe("value control", () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it("exposes slider semantics", () => {
    const { slider } = renderKnob({ min: 0, max: 1, ...percentUnit })
    expect(slider).toHaveAttribute("aria-valuemin", "0")
    expect(slider).toHaveAttribute("aria-valuemax", "1")
    expect(slider).toHaveAttribute("aria-valuenow", "0.5")
    expect(slider).toHaveAttribute("aria-valuetext", "50%")
    expect(slider).toHaveAttribute("aria-orientation", "vertical")
    expect(slider).toHaveAttribute("tabindex", "0")
  })

  it("is labelled by its visible label", () => {
    render(<Knob label="Cutoff" value={0.5} />)
    expect(screen.getByRole("slider", { name: "Cutoff" })).toBeInTheDocument()
  })

  it("drags up to raise the value, 200 pixels for the full range", () => {
    const { slider, calls } = renderKnob()
    drag(slider, 300, 250)
    expect(valueOf(slider)).toBeCloseTo(0.75, 10)
    expect(calls.order).toEqual(["start", "change", "end"])
  })

  it("moves ten times slower with Shift", () => {
    const { slider } = renderKnob()
    drag(slider, 300, 250, { shiftKey: true })
    expect(valueOf(slider)).toBeCloseTo(0.525, 10)
  })

  it("never leaves the range and never repeats a value", () => {
    const { slider, calls } = renderKnob()
    fireEvent.pointerDown(slider, { pointerId: 1, button: 0, clientY: 300 })
    for (const y of [200, 100, -500, -900, -900]) {
      fireEvent.pointerMove(slider, { pointerId: 1, clientY: y })
    }
    fireEvent.pointerUp(slider, { pointerId: 1 })
    expect(calls.values.every((value) => value >= 0 && value <= 1)).toBe(true)
    expect(calls.values).toEqual([1])
    expect(calls.order).toEqual(["start", "change", "end"])
  })

  it("responds at once when the pointer turns back from past the end", () => {
    const { slider } = renderKnob()
    fireEvent.pointerDown(slider, { pointerId: 1, button: 0, clientY: 300 })
    fireEvent.pointerMove(slider, { pointerId: 1, clientY: -700 })
    fireEvent.pointerMove(slider, { pointerId: 1, clientY: -680 })
    expect(valueOf(slider)).toBeCloseTo(0.9, 10)
  })

  it("opens no gesture for a press that changes nothing", () => {
    const { slider, calls } = renderKnob()
    fireEvent.pointerDown(slider, { pointerId: 1, button: 0, clientY: 300 })
    fireEvent.pointerUp(slider, { pointerId: 1 })
    expect(calls.order).toEqual([])
  })

  it("closes the gesture when the pointer is cancelled", () => {
    const { slider, calls } = renderKnob()
    drag(slider, 300, 280, { release: false })
    fireEvent.pointerCancel(slider, { pointerId: 1 })
    expect(calls.order).toEqual(["start", "change", "end"])
  })

  it("ignores other buttons and other pointers", () => {
    const { slider, calls } = renderKnob()
    fireEvent.pointerDown(slider, { pointerId: 1, button: 2, clientY: 300 })
    fireEvent.pointerMove(slider, { pointerId: 1, clientY: 200 })
    fireEvent.pointerDown(slider, { pointerId: 1, button: 0, clientY: 300 })
    fireEvent.pointerMove(slider, { pointerId: 2, clientY: 200 })
    expect(calls.values).toEqual([])
  })

  it("snaps a drag to the step", () => {
    const { slider, calls } = renderKnob({ min: 0, max: 10, step: 1, value: 5 })
    fireEvent.pointerDown(slider, { pointerId: 1, button: 0, clientY: 300 })
    fireEvent.pointerMove(slider, { pointerId: 1, clientY: 295 })
    fireEvent.pointerMove(slider, { pointerId: 1, clientY: 268 })
    fireEvent.pointerUp(slider, { pointerId: 1 })
    expect(calls.values).toEqual([7])
  })

  it("resets on double-click and on Ctrl-click", () => {
    const { slider, calls } = renderKnob()
    fireEvent.doubleClick(slider)
    expect(valueOf(slider)).toBe(0.25)
    expect(calls.order).toEqual(["start", "change", "end"])
    drag(slider, 300, 200)
    fireEvent.pointerDown(slider, {
      pointerId: 1,
      button: 0,
      clientY: 300,
      ctrlKey: true,
    })
    expect(valueOf(slider)).toBe(0.25)
  })

  it("resets on a double tap with touch", () => {
    const { slider } = renderKnob()
    const tap = () => {
      fireEvent.pointerDown(slider, {
        pointerId: 3,
        pointerType: "touch",
        button: 0,
        clientX: 10,
        clientY: 10,
      })
      fireEvent.pointerUp(slider, { pointerId: 3, pointerType: "touch" })
    }
    tap()
    tap()
    expect(valueOf(slider)).toBe(0.25)
  })

  it("sticks to the center of a bipolar knob", () => {
    const { slider } = renderKnob({
      min: -1,
      max: 1,
      bipolar: true,
      value: -0.5,
    })
    fireEvent.pointerDown(slider, { pointerId: 1, button: 0, clientY: 300 })
    fireEvent.pointerMove(slider, { pointerId: 1, clientY: 252 })
    expect(valueOf(slider)).toBe(0)
    fireEvent.pointerMove(slider, { pointerId: 1, clientY: 230 })
    expect(valueOf(slider)).toBeCloseTo(0.2, 10)
  })

  it("steps with the keyboard", () => {
    const { slider, calls } = renderKnob()
    fireEvent.keyDown(slider, { key: "ArrowUp" })
    expect(valueOf(slider)).toBeCloseTo(0.51, 10)
    fireEvent.keyDown(slider, { key: "ArrowRight" })
    expect(valueOf(slider)).toBeCloseTo(0.52, 10)
    fireEvent.keyDown(slider, { key: "ArrowDown", shiftKey: true })
    expect(valueOf(slider)).toBeCloseTo(0.519, 10)
    fireEvent.keyDown(slider, { key: "ArrowLeft" })
    expect(valueOf(slider)).toBeCloseTo(0.509, 10)
    fireEvent.keyDown(slider, { key: "PageUp" })
    expect(valueOf(slider)).toBeCloseTo(0.609, 10)
    fireEvent.keyDown(slider, { key: "PageDown" })
    expect(valueOf(slider)).toBeCloseTo(0.509, 10)
    fireEvent.keyDown(slider, { key: "Home" })
    expect(valueOf(slider)).toBe(0)
    fireEvent.keyDown(slider, { key: "End" })
    expect(valueOf(slider)).toBe(1)
    const before = calls.values.length
    fireEvent.keyDown(slider, { key: "End" })
    expect(calls.values.length).toBe(before)
  })

  it("groups a held key into one gesture", () => {
    const { slider, calls } = renderKnob()
    fireEvent.keyDown(slider, { key: "ArrowUp" })
    fireEvent.keyDown(slider, { key: "ArrowUp", repeat: true })
    fireEvent.keyDown(slider, { key: "ArrowUp", repeat: true })
    fireEvent.keyUp(slider, { key: "ArrowUp" })
    expect(calls.order).toEqual(["start", "change", "change", "change", "end"])
  })

  it("leaves Delete and Backspace to the app, default or not", () => {
    // In an app these keys delete things. A control that reset itself with
    // them made one key mean two things, a click apart.
    for (const defaultValue of [0.25, undefined]) {
      const { slider, calls } = renderKnob({ defaultValue })
      expect(fireEvent.keyDown(slider, { key: "Delete" })).toBe(true)
      expect(fireEvent.keyDown(slider, { key: "Backspace" })).toBe(true)
      expect(calls.order).toEqual([])
      expect(valueOf(slider)).toBe(0.5)
      cleanup()
    }
  })

  it("goes back to its default on a double-click and on Ctrl+click", () => {
    const { slider, calls } = renderKnob()
    fireEvent.doubleClick(slider)
    expect(valueOf(slider)).toBe(0.25)
    expect(calls.order).toEqual(["start", "change", "end"])
    drag(slider, 300, 200)
    expect(valueOf(slider)).not.toBe(0.25)
    fireEvent.pointerDown(slider, { button: 0, pointerId: 1, ctrlKey: true })
    expect(valueOf(slider)).toBe(0.25)
  })

  it("leaves modified keys to the app", () => {
    const { slider, calls } = renderKnob()
    fireEvent.keyDown(slider, { key: "ArrowUp", ctrlKey: true })
    fireEvent.keyDown(slider, { key: "z", metaKey: true })
    expect(calls.values).toEqual([])
  })

  it("leaves Space to the app", () => {
    const { slider, calls } = renderKnob()
    // Not handled and not prevented, so a transport shortcut can take it.
    expect(fireEvent.keyDown(slider, { key: " " })).toBe(true)
    expect(fireEvent.keyUp(slider, { key: " " })).toBe(true)
    expect(calls.order).toEqual([])
    expect(screen.queryByRole("textbox")).toBeNull()
  })

  it("opens a text entry on Enter and commits the parsed value", () => {
    const { slider, calls } = renderKnob({ ...percentUnit })
    fireEvent.keyDown(slider, { key: "Enter" })
    const input = screen.getByRole("textbox")
    expect(input).toHaveValue("50%")
    fireEvent.change(input, { target: { value: "80%" } })
    fireEvent.keyDown(input, { key: "Enter" })
    expect(valueOf(slider)).toBe(0.8)
    expect(screen.queryByRole("textbox")).toBeNull()
    expect(calls.order).toEqual(["start", "change", "end"])
    expect(slider).toHaveFocus()
  })

  it("starts the entry with a typed digit", () => {
    const { slider } = renderKnob({ min: -60, max: 6, value: 0 })
    fireEvent.keyDown(slider, { key: "-" })
    const input = screen.getByRole("textbox")
    expect(input).toHaveValue("-")
    fireEvent.change(input, { target: { value: "-6 dB" } })
    fireEvent.keyDown(input, { key: "Enter" })
    expect(valueOf(slider)).toBe(-6)
  })

  it("commits the entry on Tab and lets the focus move on", () => {
    const { slider, calls } = renderKnob()
    fireEvent.keyDown(slider, { key: "Enter" })
    const input = screen.getByRole("textbox")
    fireEvent.change(input, { target: { value: "0.8" } })
    // Not prevented, so the browser takes the focus to the next control.
    expect(fireEvent.keyDown(input, { key: "Tab" })).toBe(true)
    expect(valueOf(slider)).toBe(0.8)
    expect(screen.queryByRole("textbox")).toBeNull()
    expect(calls.order).toEqual(["start", "change", "end"])
  })

  it("cancels the entry when the field is left without Enter or Tab", () => {
    const { slider, calls } = renderKnob({ min: 0, max: 100, value: 50 })
    // A stray digit opened the entry, and a click elsewhere left it.
    fireEvent.keyDown(slider, { key: "2" })
    const input = screen.getByRole("textbox")
    expect(input).toHaveValue("2")
    fireEvent.blur(input)
    expect(screen.queryByRole("textbox")).toBeNull()
    expect(calls.values).toEqual([])
    expect(valueOf(slider)).toBe(50)

    // The same for a whole, valid value that was typed but not confirmed.
    fireEvent.keyDown(slider, { key: "Enter" })
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "80" } })
    fireEvent.blur(screen.getByRole("textbox"))
    expect(calls.values).toEqual([])
    // The focus stays where it went.
    expect(slider).not.toHaveFocus()
  })

  it("leaves a digit typed with Ctrl, Alt or Cmd to the app", () => {
    const { slider } = renderKnob()
    for (const modifier of ["altKey", "ctrlKey", "metaKey"] as const) {
      expect(fireEvent.keyDown(slider, { key: "2", [modifier]: true })).toBe(
        true
      )
    }
    expect(screen.queryByRole("textbox")).toBeNull()
  })

  it("cancels the entry on Escape and on unreadable text", () => {
    const { slider, calls } = renderKnob()
    fireEvent.keyDown(slider, { key: "Enter" })
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "0.9" } })
    fireEvent.keyDown(screen.getByRole("textbox"), { key: "Escape" })
    expect(screen.queryByRole("textbox")).toBeNull()
    fireEvent.keyDown(slider, { key: "Enter" })
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "loud" } })
    fireEvent.keyDown(screen.getByRole("textbox"), { key: "Enter" })
    expect(calls.values).toEqual([])
    expect(valueOf(slider)).toBe(0.5)
  })

  it("does not round the value when the entry is left untouched", () => {
    const { slider, calls } = renderKnob({ value: 0.123456, ...percentUnit })
    fireEvent.keyDown(slider, { key: "Enter" })
    fireEvent.blur(screen.getByRole("textbox"))
    expect(calls.values).toEqual([])
  })

  it("clamps typed values to the range", () => {
    const { slider } = renderKnob()
    fireEvent.keyDown(slider, { key: "Enter" })
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "42" } })
    fireEvent.keyDown(screen.getByRole("textbox"), { key: "Enter" })
    expect(valueOf(slider)).toBe(1)
  })

  it("adjusts with the wheel when focused, as one gesture", () => {
    vi.useFakeTimers()
    const { slider, calls } = renderKnob()
    act(() => slider.focus())
    fireEvent.wheel(slider, { deltaY: -100 })
    fireEvent.wheel(slider, { deltaY: -100 })
    expect(valueOf(slider)).toBeCloseTo(0.52, 10)
    fireEvent.wheel(slider, { deltaY: 100, shiftKey: true })
    expect(valueOf(slider)).toBeCloseTo(0.519, 10)
    expect(calls.order).toEqual(["start", "change", "change", "change"])
    act(() => {
      vi.advanceTimersByTime(500)
    })
    expect(calls.order[calls.order.length - 1]).toBe("end")
  })

  it("leaves the wheel to the page until the pointer moves onto it", async () => {
    const { slider, calls } = renderKnob()
    const scroll = new WheelEvent("wheel", {
      deltaY: -100,
      bubbles: true,
      cancelable: true,
    })
    slider.dispatchEvent(scroll)
    expect(scroll.defaultPrevented).toBe(false)

    // The control scrolled under a resting pointer: it enters and "moves"
    // at one position, which does not count as pointing at it.
    fireEvent(
      slider,
      new PointerEvent("pointerenter", { clientX: 5, clientY: 5 })
    )
    fireEvent.pointerMove(slider, { clientX: 5, clientY: 5 })
    await new Promise((resolve) => setTimeout(resolve, 300))
    fireEvent.wheel(slider, { deltaY: -100 })
    expect(calls.values).toEqual([])

    // A real move arms it, but not while the page is still scrolling.
    fireEvent.pointerMove(slider, { clientX: 8, clientY: 6 })
    fireEvent.wheel(slider, { deltaY: -100 })
    expect(calls.values).toEqual([])
    await new Promise((resolve) => setTimeout(resolve, 300))
    fireEvent.wheel(slider, { deltaY: -100 })
    expect(valueOf(slider)).toBeCloseTo(0.51, 10)
  })

  it("does nothing when disabled", () => {
    const { slider, calls } = renderKnob({ disabled: true })
    drag(slider, 300, 200)
    fireEvent.keyDown(slider, { key: "ArrowUp" })
    fireEvent.keyDown(slider, { key: "Enter" })
    fireEvent.doubleClick(slider)
    expect(calls.order).toEqual([])
    expect(slider).toHaveAttribute("aria-disabled", "true")
    expect(slider).toHaveAttribute("tabindex", "-1")
    expect(screen.queryByRole("textbox")).toBeNull()
  })

  it("closes an open gesture when the control goes away", () => {
    const order: string[] = []
    const { unmount } = render(
      <Knob
        aria-label="Amount"
        value={0.5}
        onValueChange={() => order.push("change")}
        onGestureStart={() => order.push("start")}
        onGestureEnd={() => order.push("end")}
      />
    )
    const slider = screen.getByRole("slider")
    drag(slider, 300, 250, { release: false })
    unmount()
    expect(order).toEqual(["start", "change", "end"])
  })
})
