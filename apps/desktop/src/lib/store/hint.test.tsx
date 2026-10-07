import { fireEvent, render, screen } from "@testing-library/react"
import { beforeEach, describe, expect, it } from "vitest"

import { useHint, useHintStore } from "./hint"

function Knob({ value, label = "Level" }: { value: number; label?: string }) {
  return <button {...useHint(`${label}: ${value}`)}>{label}</button>
}

const shown = () => useHintStore.getState().text

beforeEach(() => useHintStore.setState({ text: null }))

describe("useHint", () => {
  it("shows the text while the control is hovered or focused", () => {
    render(<Knob value={1} />)
    const knob = screen.getByRole("button")
    expect(shown()).toBeNull()
    fireEvent.pointerEnter(knob)
    expect(shown()).toBe("Level: 1")
    fireEvent.pointerLeave(knob)
    expect(shown()).toBeNull()

    fireEvent.focus(knob)
    expect(shown()).toBe("Level: 1")
    fireEvent.blur(knob)
    expect(shown()).toBeNull()
  })

  it("follows the text as it changes under the pointer", () => {
    const view = render(<Knob value={1} />)
    fireEvent.pointerEnter(screen.getByRole("button"))
    view.rerender(<Knob value={2} />)
    expect(shown()).toBe("Level: 2")
    view.rerender(<Knob value={3} />)
    expect(shown()).toBe("Level: 3")
    fireEvent.pointerLeave(screen.getByRole("button"))
    expect(shown()).toBeNull()
  })

  it("stays up while either the pointer or the focus is still on the control", () => {
    render(<Knob value={1} />)
    const knob = screen.getByRole("button")
    fireEvent.pointerEnter(knob)
    fireEvent.focus(knob)
    fireEvent.pointerLeave(knob)
    expect(shown()).toBe("Level: 1")
    fireEvent.blur(knob)
    expect(shown()).toBeNull()
  })

  it("does not change the hint of another control that took over", () => {
    const view = render(
      <>
        <Knob value={1} />
        <Knob value={5} label="Pan" />
      </>
    )
    fireEvent.pointerEnter(screen.getByRole("button", { name: "Level" }))
    fireEvent.pointerEnter(screen.getByRole("button", { name: "Pan" }))
    expect(shown()).toBe("Pan: 5")
    // The first control changes while the second one is showing.
    view.rerender(
      <>
        <Knob value={2} />
        <Knob value={5} label="Pan" />
      </>
    )
    expect(shown()).toBe("Pan: 5")
    fireEvent.pointerLeave(screen.getByRole("button", { name: "Level" }))
    expect(shown()).toBe("Pan: 5")
  })

  it("takes its text away when the control goes while it is showing", () => {
    const view = render(<Knob value={1} />)
    fireEvent.pointerEnter(screen.getByRole("button"))
    view.unmount()
    expect(shown()).toBeNull()
  })
})
