// SPDX-License-Identifier: MIT
import * as React from "react"
import { act, fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"

import {
  PianoKeyboard,
  isBlackKey,
  keyAtPoint,
  layoutKeys,
  noteName,
  type PianoKeyboardHandle,
} from "./piano-keyboard"

describe("key helpers", () => {
  it("names keys in both octave conventions", () => {
    expect(noteName(60)).toBe("C5")
    expect(noteName(60, 4)).toBe("C4")
    expect(noteName(61)).toBe("C#5")
    expect(noteName(69, 4)).toBe("A4")
    expect(noteName(0)).toBe("C0")
  })

  it("knows the black keys", () => {
    const octave = Array.from({ length: 12 }, (_, key) => isBlackKey(60 + key))
    expect(octave.filter(Boolean)).toHaveLength(5)
    expect(isBlackKey(61)).toBe(true)
    expect(isBlackKey(64)).toBe(false)
    expect(isBlackKey(65)).toBe(false)
  })

  it("gives white keys equal widths in the classic layout", () => {
    const shapes = layoutKeys(60, 72, "classic")
    expect(shapes).toHaveLength(13)
    const whites = shapes.filter((shape) => !shape.black)
    expect(whites).toHaveLength(8)
    for (const [index, white] of whites.entries()) {
      expect(white.start).toBeCloseTo(index / 8, 10)
      expect(white.end).toBeCloseTo((index + 1) / 8, 10)
    }
    const cSharp = shapes.find((shape) => shape.key === 61)
    expect(cSharp?.start).toBeGreaterThan(0)
    expect(cSharp?.end).toBeLessThan(2 / 8)
    expect((cSharp?.start ?? 0) < 1 / 8 && (cSharp?.end ?? 0) > 1 / 8).toBe(
      true
    )
  })

  it("widens a classic range that ends on a black key", () => {
    const shapes = layoutKeys(61, 66, "classic")
    expect(shapes[0].key).toBe(60)
    expect(shapes[shapes.length - 1].key).toBe(67)
  })

  it("gives every semitone the same row in the uniform layout", () => {
    const shapes = layoutKeys(60, 71, "uniform")
    const blacks = shapes.filter((shape) => shape.black)
    for (const black of blacks) {
      expect(black.end - black.start).toBeCloseTo(1 / 12, 10)
      expect(black.start).toBeCloseTo((black.key - 60) / 12, 10)
    }
    // White keys cover the rest with no gaps.
    const whites = shapes.filter((shape) => !shape.black)
    expect(whites[0].start).toBe(0)
    expect(whites[whites.length - 1].end).toBeCloseTo(1, 10)
    for (let index = 1; index < whites.length; index += 1) {
      expect(whites[index].start).toBeCloseTo(whites[index - 1].end, 10)
    }
  })

  it("finds the key and the velocity at a point", () => {
    const shapes = layoutKeys(60, 72, "classic")
    expect(keyAtPoint(shapes, 0.01, 0.9)?.key).toBe(60)
    expect(keyAtPoint(shapes, 1 / 8, 0.2)?.key).toBe(61)
    expect(keyAtPoint(shapes, 1 / 8 + 0.001, 0.9)?.key).toBe(62)
    expect(keyAtPoint(shapes, 0.99, 0.5)?.key).toBe(72)
    expect(keyAtPoint(shapes, 1.2, 0.5)).toBeNull()
    expect(keyAtPoint(shapes, 0.5, -0.1)).toBeNull()

    const soft = keyAtPoint(shapes, 0.01, 0)?.velocity ?? 0
    const loud = keyAtPoint(shapes, 0.01, 1)?.velocity ?? 0
    expect(soft).toBeCloseTo(0.2, 10)
    expect(loud).toBe(1)
    expect(keyAtPoint(shapes, 1 / 8, 0.62)?.velocity).toBeCloseTo(1, 10)
  })
})

function renderKeyboard(
  props: Partial<React.ComponentProps<typeof PianoKeyboard>> = {}
) {
  const events: string[] = []
  const ref = React.createRef<PianoKeyboardHandle>()
  const view = render(
    <PianoKeyboard
      ref={ref}
      lowKey={60}
      highKey={72}
      onNoteOn={(key, velocity) => events.push(`on ${key} ${velocity}`)}
      onNoteOff={(key) => events.push(`off ${key}`)}
      {...props}
    />
  )
  const key = (name: string) => screen.getByRole("button", { name })
  return { events, key, ref, ...view }
}

describe("PianoKeyboard", () => {
  it("renders a labelled button per key with note names on the Cs", () => {
    renderKeyboard()
    expect(screen.getByRole("group", { name: "Piano keyboard" })).toBeVisible()
    expect(screen.getAllByRole("button")).toHaveLength(13)
    expect(screen.getByRole("button", { name: "C5" })).toHaveTextContent("C5")
    expect(screen.getByRole("button", { name: "D5" })).toHaveTextContent("")
    expect(screen.getByRole("button", { name: "C#5" })).toBeInTheDocument()
  })

  it("marks the active keys", () => {
    const { key } = renderKeyboard({ activeKeys: new Set([64]) })
    expect(key("E5")).toHaveAttribute("aria-pressed", "true")
    expect(key("C5")).toHaveAttribute("aria-pressed", "false")
  })

  it("pairs note-on with note-off on press and release", () => {
    const { events, key } = renderKeyboard()
    fireEvent.pointerDown(key("E5"), { pointerId: 1, button: 0 })
    expect(key("E5")).toHaveAttribute("data-held")
    fireEvent.pointerUp(key("E5"), { pointerId: 1 })
    expect(events).toEqual(["on 64 0.8", "off 64"])
    expect(key("E5")).not.toHaveAttribute("data-held")
  })

  it("plays a glissando while dragging", () => {
    const { events, key } = renderKeyboard()
    fireEvent.pointerDown(key("C5"), { pointerId: 1, button: 0 })
    fireEvent.pointerMove(key("C5"), { pointerId: 1, buttons: 1 })
    fireEvent.pointerMove(key("D5"), { pointerId: 1, buttons: 1 })
    fireEvent.pointerMove(key("E5"), { pointerId: 1, buttons: 1 })
    fireEvent.pointerUp(key("E5"), { pointerId: 1 })
    expect(events.map((event) => event.slice(0, 6).trim())).toEqual([
      "on 60",
      "off 60",
      "on 62",
      "off 62",
      "on 64",
      "off 64",
    ])
  })

  it.each(["pointerCancel", "lostPointerCapture"] as const)(
    "sends note-off on %s",
    (ending) => {
      const { events, key } = renderKeyboard()
      fireEvent.pointerDown(key("G5"), { pointerId: 4, button: 0 })
      fireEvent[ending](key("G5"), { pointerId: 4 })
      expect(events).toEqual(["on 67 0.8", "off 67"])
    }
  )

  it("sends note-off when the window loses focus, and only once", () => {
    const { events, key } = renderKeyboard()
    fireEvent.pointerDown(key("C5"), { pointerId: 1, button: 0 })
    fireEvent.blur(window)
    expect(events).toEqual(["on 60 0.8", "off 60"])
    fireEvent.pointerUp(key("C5"), { pointerId: 1 })
    expect(events).toHaveLength(2)
  })

  it("sends note-off for held keys when it unmounts", () => {
    const { events, key, unmount } = renderKeyboard()
    fireEvent.pointerDown(key("C5"), { pointerId: 1, button: 0 })
    fireEvent.pointerDown(key("E5"), { pointerId: 2, button: 0 })
    unmount()
    expect(events).toEqual(["on 60 0.8", "on 64 0.8", "off 60", "off 64"])
  })

  it("sends one note for two pointers on the same key", () => {
    const { events, key } = renderKeyboard()
    fireEvent.pointerDown(key("C5"), { pointerId: 1, button: 0 })
    fireEvent.pointerDown(key("C5"), { pointerId: 2, button: 0 })
    fireEvent.pointerUp(key("C5"), { pointerId: 1 })
    expect(events).toEqual(["on 60 0.8"])
    fireEvent.pointerUp(key("C5"), { pointerId: 2 })
    expect(events).toEqual(["on 60 0.8", "off 60"])
  })

  it("ignores the right button", () => {
    const { events, key } = renderKeyboard()
    fireEvent.pointerDown(key("C5"), { pointerId: 1, button: 2 })
    expect(events).toEqual([])
  })

  it("plays the focused key with Space and moves with the arrows", () => {
    const { events, key } = renderKeyboard({ keyboardVelocity: 0.5 })
    expect(key("C5")).toHaveAttribute("tabindex", "0")
    act(() => key("C5").focus())
    fireEvent.keyDown(key("C5"), { key: " " })
    fireEvent.keyDown(key("C5"), { key: " ", repeat: true })
    fireEvent.keyUp(key("C5"), { key: " " })
    expect(events).toEqual(["on 60 0.5", "off 60"])
    fireEvent.keyDown(key("C5"), { key: "ArrowRight" })
    expect(key("C#5")).toHaveFocus()
    fireEvent.keyDown(key("C#5"), { key: "PageUp" })
    expect(key("C6")).toHaveFocus()
    fireEvent.keyDown(key("C6"), { key: "Home" })
    expect(key("C5")).toHaveFocus()
  })

  it("releases a key held with Space when focus leaves", () => {
    const { events, key } = renderKeyboard()
    act(() => key("C5").focus())
    fireEvent.keyDown(key("C5"), { key: "Enter" })
    fireEvent.blur(key("C5"))
    expect(events).toEqual(["on 60 0.8", "off 60"])
  })

  it("lights keys through its ref", () => {
    vi.useFakeTimers()
    const { key, ref, events } = renderKeyboard()
    act(() => ref.current?.setLit(62, true))
    expect(key("D5")).toHaveAttribute("data-lit")
    act(() => ref.current?.setLit(62, false))
    expect(key("D5")).not.toHaveAttribute("data-lit")
    act(() => ref.current?.flash(64, 100))
    expect(key("E5")).toHaveAttribute("data-lit")
    act(() => {
      vi.advanceTimersByTime(120)
    })
    expect(key("E5")).not.toHaveAttribute("data-lit")
    expect(events).toEqual([])
    vi.useRealTimers()
  })

  it("releases everything through its ref", () => {
    const { key, ref, events } = renderKeyboard()
    fireEvent.pointerDown(key("C5"), { pointerId: 1, button: 0 })
    act(() => ref.current?.releaseAll())
    expect(events).toEqual(["on 60 0.8", "off 60"])
  })

  it("plays nothing when disabled", () => {
    const { events, key } = renderKeyboard({ disabled: true })
    fireEvent.pointerDown(key("C5"), { pointerId: 1, button: 0 })
    fireEvent.keyDown(key("C5"), { key: " " })
    expect(events).toEqual([])
    expect(key("C5")).toHaveAttribute("aria-disabled", "true")
  })

  it("uses the position along the key for velocity", () => {
    const { events } = renderKeyboard()
    const root = screen.getByRole("group", { name: "Piano keyboard" })
    root.getBoundingClientRect = () =>
      ({ left: 0, top: 0, width: 800, height: 100 }) as DOMRect
    fireEvent.pointerDown(root, {
      pointerId: 1,
      button: 0,
      clientX: 10,
      clientY: 100,
    })
    fireEvent.pointerUp(root, { pointerId: 1 })
    fireEvent.pointerDown(root, {
      pointerId: 1,
      button: 0,
      clientX: 10,
      clientY: 50,
    })
    fireEvent.pointerUp(root, { pointerId: 1 })
    const velocities = events
      .filter((event) => event.startsWith("on 60"))
      .map((event) => Number(event.split(" ")[2]))
    expect(velocities[0]).toBe(1)
    expect(velocities[1]).toBeCloseTo(0.6, 10)
    expect(events.filter((event) => event === "off 60")).toHaveLength(2)
  })
})
