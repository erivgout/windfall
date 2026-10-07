// SPDX-License-Identifier: MIT
import * as React from "react"
import { act, fireEvent, render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { describe, expect, it } from "vitest"

import { StepButton } from "./step-button"
import {
  StepGrid,
  StepGridGroup,
  type StepGridGroupHandle,
  type StepGridHandle,
} from "./step-grid"

const pattern = (text: string) => [...text].map((mark) => mark === "x")

function renderGrid(
  text: string,
  props: { rightClickClears?: boolean; spaceToggles?: boolean } = {}
) {
  const events: string[] = []
  const ref = React.createRef<StepGridHandle>()
  function Harness() {
    const [steps, setSteps] = React.useState(() => pattern(text))
    return (
      <StepGrid
        ref={ref}
        aria-label="Kick steps"
        steps={steps}
        onToggle={(step, on) => {
          events.push(`${step}:${on ? "on" : "off"}`)
          setSteps((was) => was.map((lit, at) => (at === step ? on : lit)))
        }}
        onGestureStart={() => events.push("start")}
        onGestureEnd={() => events.push("end")}
        {...props}
      />
    )
  }
  render(<Harness />)
  const grid = screen.getByRole("group", { name: "Kick steps" })
  const steps = () => screen.getAllByRole("button")
  const shown = () =>
    steps()
      .map((step) => (step.getAttribute("aria-pressed") === "true" ? "x" : "."))
      .join("")
  return { grid, steps, shown, events, ref }
}

// jsdom has no layout, so the grid falls back to the step under the pointer.
const press = (step: HTMLElement, button = 0) =>
  fireEvent.pointerDown(step, { pointerId: 1, button })
const lift = (step: HTMLElement) => fireEvent.pointerUp(step, { pointerId: 1 })

describe("StepGrid", () => {
  it("renders one pressed-state button per step", () => {
    const { steps, shown } = renderGrid("x...x...")
    expect(steps()).toHaveLength(8)
    expect(shown()).toBe("x...x...")
    expect(steps()[0]).toHaveAccessibleName("Step 1")
    expect(steps()[7]).toHaveAccessibleName("Step 8")
  })

  it("alternates the shade of every other beat", () => {
    const { steps } = renderGrid("................")
    const alt = steps().map((step) => step.hasAttribute("data-alt"))
    expect(alt.slice(0, 8)).toEqual([
      false,
      false,
      false,
      false,
      true,
      true,
      true,
      true,
    ])
    expect(alt[8]).toBe(false)
    expect(alt[12]).toBe(true)
  })

  it("toggles on a press, as one gesture", () => {
    const { steps, shown, events } = renderGrid("x...")
    press(steps()[2])
    lift(steps()[2])
    expect(shown()).toBe("x.x.")
    press(steps()[0])
    lift(steps()[0])
    expect(shown()).toBe("..x.")
    expect(events).toEqual(["start", "2:on", "end", "start", "0:off", "end"])
  })

  it("has one tab stop and moves it with the arrow keys", () => {
    const { steps } = renderGrid("........")
    const tabStops = () => steps().filter((step) => step.tabIndex === 0)
    expect(tabStops()).toEqual([steps()[0]])
    act(() => steps()[0].focus())
    fireEvent.keyDown(steps()[0], { key: "ArrowRight" })
    expect(steps()[1]).toHaveFocus()
    expect(tabStops()).toEqual([steps()[1]])
    fireEvent.keyDown(steps()[1], { key: "End" })
    expect(steps()[7]).toHaveFocus()
    fireEvent.keyDown(steps()[7], { key: "ArrowRight" })
    expect(steps()[7]).toHaveFocus()
    fireEvent.keyDown(steps()[7], { key: "Home" })
    expect(steps()[0]).toHaveFocus()
    fireEvent.keyDown(steps()[0], { key: "ArrowLeft" })
    expect(steps()[0]).toHaveFocus()
  })

  it("toggles with Enter, which arrives as a click with no detail", async () => {
    const user = userEvent.setup()
    const { steps, shown, events } = renderGrid("....")
    fireEvent.click(steps()[1], { detail: 0 })
    expect(shown()).toBe(".x..")
    act(() => steps()[1].focus())
    await user.keyboard("{Enter}")
    expect(shown()).toBe("....")
    expect(events).toEqual(["start", "1:on", "end", "start", "1:off", "end"])
  })

  it("leaves Space to the app", async () => {
    const user = userEvent.setup()
    const { steps, shown, events } = renderGrid("....")
    act(() => steps()[1].focus())
    // The key goes down untouched, for a transport shortcut to take.
    expect(fireEvent.keyDown(steps()[1], { key: " " })).toBe(true)
    fireEvent.keyUp(steps()[1], { key: " " })
    await user.keyboard(" ")
    expect(shown()).toBe("....")
    expect(events).toEqual([])
  })

  it("toggles with Space when asked to", async () => {
    const user = userEvent.setup()
    const { steps, shown } = renderGrid("....", { spaceToggles: true })
    act(() => steps()[2].focus())
    await user.keyboard(" ")
    expect(shown()).toBe("..x.")
    await user.keyboard(" ")
    expect(shown()).toBe("....")
  })

  it("does not toggle twice for a pointer click", () => {
    const { steps, shown } = renderGrid("....")
    press(steps()[1])
    lift(steps()[1])
    fireEvent.click(steps()[1], { detail: 1 })
    expect(shown()).toBe(".x..")
  })

  it("clears with the right button and blocks the context menu", () => {
    const { grid, steps, shown, events } = renderGrid("xx..")
    press(steps()[0], 2)
    lift(steps()[0])
    press(steps()[2], 2)
    lift(steps()[2])
    expect(shown()).toBe(".x..")
    expect(events).toEqual(["start", "0:off", "end"])
    expect(fireEvent.contextMenu(grid)).toBe(false)
  })

  it("leaves the right button alone when asked to", () => {
    const { grid, steps, shown } = renderGrid("xx..", {
      rightClickClears: false,
    })
    press(steps()[0], 2)
    expect(shown()).toBe("xx..")
    expect(fireEvent.contextMenu(grid)).toBe(true)
  })

  it("moves the playhead through its ref without rendering", () => {
    const { steps, ref } = renderGrid("x...")
    act(() => ref.current?.setPlayStep(2))
    expect(steps()[2]).toHaveAttribute("data-playing")
    act(() => ref.current?.setPlayStep(3))
    expect(steps()[2]).not.toHaveAttribute("data-playing")
    expect(steps()[3]).toHaveAttribute("data-playing")
    press(steps()[0])
    lift(steps()[0])
    expect(steps()[3]).toHaveAttribute("data-playing")
    act(() => ref.current?.setPlayStep(null))
    expect(steps()[3]).not.toHaveAttribute("data-playing")
  })

  it("ignores input when disabled", () => {
    const events: string[] = []
    render(
      <StepGrid
        aria-label="Off"
        steps={pattern("x.")}
        disabled
        onToggle={() => events.push("toggle")}
      />
    )
    const [first] = screen.getAllByRole("button")
    expect(first).toBeDisabled()
    press(first)
    fireEvent.click(first, { detail: 0 })
    expect(events).toEqual([])
  })
})

describe("painting", () => {
  function layout(grid: HTMLElement, width: number) {
    grid.getBoundingClientRect = () =>
      ({
        left: 0,
        top: 0,
        width,
        height: 24,
        right: width,
        bottom: 24,
      }) as DOMRect
  }

  it("paints the opposite of the first step across a drag", () => {
    const { grid, shown, events } = renderGrid(".x......")
    layout(grid, 80)
    fireEvent.pointerDown(grid, { pointerId: 1, button: 0, clientX: 5 })
    fireEvent.pointerMove(grid, { pointerId: 1, clientX: 15 })
    fireEvent.pointerMove(grid, { pointerId: 1, clientX: 25 })
    fireEvent.pointerMove(grid, { pointerId: 1, clientX: 26 })
    fireEvent.pointerMove(grid, { pointerId: 1, clientX: 35 })
    fireEvent.pointerUp(grid, { pointerId: 1 })
    expect(shown()).toBe("xxxx....")
    // Step 1 was already on, so it is not reported.
    expect(events).toEqual(["start", "0:on", "2:on", "3:on", "end"])
  })

  it("clears when the stroke starts on a lit step", () => {
    const { grid, shown } = renderGrid("xxxx")
    layout(grid, 40)
    fireEvent.pointerDown(grid, { pointerId: 1, button: 0, clientX: 15 })
    fireEvent.pointerMove(grid, { pointerId: 1, clientX: 35 })
    fireEvent.pointerUp(grid, { pointerId: 1 })
    expect(shown()).toBe("x...")
  })

  it("fills in the steps a fast drag jumps over, in both directions", () => {
    const { grid, shown, events } = renderGrid("........")
    layout(grid, 80)
    fireEvent.pointerDown(grid, { pointerId: 1, button: 0, clientX: 65 })
    fireEvent.pointerMove(grid, { pointerId: 1, clientX: 12 })
    fireEvent.pointerUp(grid, { pointerId: 1 })
    expect(shown()).toBe(".xxxxxx.")
    expect(events).toEqual([
      "start",
      "6:on",
      "5:on",
      "4:on",
      "3:on",
      "2:on",
      "1:on",
      "end",
    ])
  })

  it("does not undo its own work when the pointer turns back", () => {
    const { grid, shown, events } = renderGrid("....")
    layout(grid, 40)
    fireEvent.pointerDown(grid, { pointerId: 1, button: 0, clientX: 5 })
    fireEvent.pointerMove(grid, { pointerId: 1, clientX: 25 })
    fireEvent.pointerMove(grid, { pointerId: 1, clientX: 5 })
    fireEvent.pointerMove(grid, { pointerId: 1, clientX: 500 })
    fireEvent.pointerUp(grid, { pointerId: 1 })
    expect(shown()).toBe("xxxx")
    expect(events.filter((event) => event.endsWith(":on"))).toHaveLength(4)
  })

  it("clears along a right-drag", () => {
    const { grid, shown } = renderGrid("xxxx")
    layout(grid, 40)
    fireEvent.pointerDown(grid, { pointerId: 1, button: 2, clientX: 5 })
    fireEvent.pointerMove(grid, { pointerId: 1, clientX: 25 })
    fireEvent.pointerCancel(grid, { pointerId: 1 })
    expect(shown()).toBe("...x")
  })
})

describe("StepGridGroup", () => {
  function renderGroup() {
    const ref = React.createRef<StepGridGroupHandle>()
    render(
      <StepGridGroup ref={ref}>
        <StepGrid aria-label="Kick" steps={pattern("x...")} />
        <StepGrid aria-label="Snare" steps={pattern("..x.")} />
      </StepGridGroup>
    )
    const row = (name: string) =>
      Array.from(screen.getByRole("group", { name }).children) as HTMLElement[]
    return { ref, row }
  }

  it("moves between rows with the up and down arrows", () => {
    const { row } = renderGroup()
    act(() => row("Kick")[2].focus())
    fireEvent.keyDown(row("Kick")[2], { key: "ArrowDown" })
    expect(row("Snare")[2]).toHaveFocus()
    fireEvent.keyDown(row("Snare")[2], { key: "ArrowDown" })
    expect(row("Snare")[2]).toHaveFocus()
    fireEvent.keyDown(row("Snare")[2], { key: "ArrowUp" })
    expect(row("Kick")[2]).toHaveFocus()
  })

  it("moves the playhead of every row at once", () => {
    const { ref, row } = renderGroup()
    act(() => ref.current?.setPlayStep(1))
    expect(row("Kick")[1]).toHaveAttribute("data-playing")
    expect(row("Snare")[1]).toHaveAttribute("data-playing")
    act(() => ref.current?.setPlayStep(null))
    expect(document.querySelectorAll("[data-playing]")).toHaveLength(0)
  })
})

describe("StepButton", () => {
  it("toggles on its own when given onToggle", () => {
    const calls: boolean[] = []
    render(
      <StepButton
        on={false}
        onToggle={(on) => calls.push(on)}
        aria-label="Step"
      />
    )
    const button = screen.getByRole("button", { name: "Step" })
    expect(button).toHaveAttribute("aria-pressed", "false")
    fireEvent.click(button)
    expect(calls).toEqual([true])
  })

  it("toggles with Enter and leaves Space to the app", async () => {
    const user = userEvent.setup()
    const calls: boolean[] = []
    render(
      <StepButton
        on={false}
        onToggle={(on) => calls.push(on)}
        aria-label="Step"
      />
    )
    const button = screen.getByRole("button", { name: "Step" })
    act(() => button.focus())
    expect(fireEvent.keyDown(button, { key: " " })).toBe(true)
    fireEvent.keyUp(button, { key: " " })
    await user.keyboard(" ")
    expect(calls).toEqual([])
    await user.keyboard("{Enter}")
    expect(calls).toEqual([true])
  })

  it("toggles with Space when asked to, and still runs its own key handler", async () => {
    const user = userEvent.setup()
    const calls: boolean[] = []
    const keys: string[] = []
    render(
      <StepButton
        on={false}
        spaceToggles
        onToggle={(on) => calls.push(on)}
        onKeyUp={(event) => keys.push(event.key)}
        aria-label="Step"
      />
    )
    act(() => screen.getByRole("button", { name: "Step" }).focus())
    await user.keyboard(" ")
    expect(calls).toEqual([true])
    expect(keys).toEqual([" "])
  })

  it("shows the playing and alternate states", () => {
    render(<StepButton on playing alt aria-label="Step" />)
    const button = screen.getByRole("button", { name: "Step" })
    expect(button).toHaveAttribute("data-playing")
    expect(button).toHaveAttribute("data-alt")
    expect(button).toHaveAttribute("aria-pressed", "true")
  })
})
