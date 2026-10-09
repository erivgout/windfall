import { act, cleanup, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { TooltipProvider } from "@/components/ui/tooltip"
import { shortcutLabel } from "@/lib/actions"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { useUiStore } from "@/lib/store/ui"
import { settle } from "@/test/harness"

import { auditionOff, auditionOn } from "./audition"
import PianoRollPanel from "./index"
import { usePianoRollStore } from "./store"
import { channel, notesOf, startRoll, undoSteps } from "./test-utils"
import { useTypingKeyboardStore } from "./typing-keyboard"

vi.mock("./audition", () => ({ auditionOn: vi.fn(), auditionOff: vi.fn() }))

let roll: Awaited<ReturnType<typeof startRoll>>
let grid: HTMLElement

beforeEach(async () => {
  vi.clearAllMocks()
  useTypingKeyboardStore.setState({ enabled: false, baseKey: 60 })
  roll = await startRoll()
  render(
    <TooltipProvider>
      <PianoRollPanel />
    </TooltipProvider>
  )
  grid = screen.getByRole("application", { name: "Note grid" })
  act(() => grid.focus())
})

afterEach(() => {
  cleanup()
  roll.stop()
})

function press(key: string, code = `Key${key.toUpperCase()}`, repeat = false) {
  fireEvent.keyDown(grid, { key, code, repeat })
}

function lift(key: string, code = `Key${key.toUpperCase()}`) {
  fireEvent.keyUp(grid, { key, code })
}

function enable() {
  fireEvent.click(screen.getByRole("button", { name: "Typing" }))
}

describe("piano-roll typing keyboard", () => {
  it("auditions A as C on the selected channel, ignores repeat, and releases on keyup without recording", () => {
    const before = undoSteps()
    const original = notesOf("Kick")
    enable()
    press("a")
    press("a", "KeyA", true)
    press("a")
    expect(auditionOn).toHaveBeenCalledTimes(1)
    expect(auditionOn).toHaveBeenCalledWith(
      channel("Kick").id,
      60,
      expect.any(Number)
    )
    lift("a")
    expect(auditionOff).toHaveBeenCalledExactlyOnceWith(channel("Kick").id, 60)
    expect(notesOf("Kick")).toEqual(original)
    expect(undoSteps()).toBe(before)
  })

  it("does not audition A when Typing is off and preserves Z for Zoom", () => {
    press("a")
    lift("a")
    expect(auditionOn).not.toHaveBeenCalled()
    expect(auditionOff).not.toHaveBeenCalled()
    press("z")
    expect(usePianoRollStore.getState().tool).toBe("zoom")
  })

  it("lowers the next A by an octave with Z and keeps tool shortcuts from firing", () => {
    enable()
    press("a")
    press("z")
    press("z", "KeyZ", true)
    lift("a")
    press("a")
    expect(auditionOn).toHaveBeenLastCalledWith(
      channel("Kick").id,
      48,
      expect.any(Number)
    )
    expect(auditionOff).toHaveBeenCalledWith(channel("Kick").id, 60)
    press("b") // Paint's shortcut, but not a piano key.
    expect(usePianoRollStore.getState().tool).toBe("draw")
  })

  it("plays the chromatic letter row through P", () => {
    enable()
    const keys = [
      "a",
      "w",
      "s",
      "e",
      "d",
      "f",
      "t",
      "g",
      "y",
      "h",
      "u",
      "j",
      "k",
      "o",
      "l",
      "p",
    ]
    for (const [offset, key] of keys.entries()) {
      press(key)
      expect(auditionOn).toHaveBeenLastCalledWith(
        channel("Kick").id,
        60 + offset,
        expect.any(Number)
      )
      lift(key)
      expect(auditionOff).toHaveBeenLastCalledWith(
        channel("Kick").id,
        60 + offset
      )
    }
  })

  it("clamps octave controls so A remains between MIDI 24 and 96", () => {
    enable()
    for (let i = 0; i < 10; i++) press("z")
    press("a")
    expect(auditionOn).toHaveBeenLastCalledWith(
      channel("Kick").id,
      24,
      expect.any(Number)
    )
    lift("a")
    for (let i = 0; i < 10; i++) press("x")
    press("a")
    expect(auditionOn).toHaveBeenLastCalledWith(
      channel("Kick").id,
      96,
      expect.any(Number)
    )
  })

  it.each(["toggle", "focus", "window", "project", "unmount"])(
    "releases every held note on %s",
    (reason) => {
      enable()
      press("a")
      press("w")
      if (reason === "toggle") enable()
      if (reason === "focus")
        act(() => screen.getByRole("button", { name: "Typing" }).focus())
      if (reason === "window") fireEvent.blur(window)
      if (reason === "project") act(() => announceProjectReplaced())
      if (reason === "unmount") cleanup()
      expect(auditionOff).toHaveBeenCalledTimes(2)
      expect(auditionOff).toHaveBeenCalledWith(channel("Kick").id, 60)
      expect(auditionOff).toHaveBeenCalledWith(channel("Kick").id, 61)
      lift("a")
      lift("w")
      expect(auditionOff).toHaveBeenCalledTimes(2)
      if (reason === "project")
        expect(useTypingKeyboardStore.getState().enabled).toBe(false)
    }
  )

  it("releases the old channel when the selected channel changes", async () => {
    enable()
    press("a")
    act(() => useUiStore.getState().selectChannel(channel("Clap").id))
    await settle()
    expect(auditionOff).toHaveBeenCalledExactlyOnceWith(channel("Kick").id, 60)
    press("a")
    expect(auditionOn).toHaveBeenLastCalledWith(
      channel("Clap").id,
      60,
      expect.any(Number)
    )
  })

  it("binds Backquote in both keymaps and toggles back off while Typing is on", () => {
    for (const keymap of ["windfall", "fl"] as const) {
      act(() => useUiStore.getState().setKeymap(keymap))
      expect(shortcutLabel("pianoRoll.typing")).toBe("`")
      press("`", "Backquote")
      expect(screen.getByRole("button", { name: "Typing" })).toHaveAttribute(
        "aria-pressed",
        "true"
      )
      press("`", "Backquote", true)
      expect(useTypingKeyboardStore.getState().enabled).toBe(true)
      press("a")
      press("`", "Backquote")
      expect(screen.getByRole("button", { name: "Typing" })).toHaveAttribute(
        "aria-pressed",
        "false"
      )
      expect(auditionOff).toHaveBeenLastCalledWith(channel("Kick").id, 60)
    }
  })

  it("keeps text fields and focus outside the piano roll from auditioning", () => {
    enable()
    expect(fireEvent.keyDown(grid, { key: "Tab", code: "Tab" })).toBe(true)
    const input = document.createElement("input")
    grid.append(input)
    act(() => input.focus())
    fireEvent.keyDown(input, { key: "a", code: "KeyA" })
    const outside = document.createElement("button")
    document.body.append(outside)
    try {
      act(() => outside.focus())
      fireEvent.keyDown(outside, { key: "a", code: "KeyA" })
      expect(auditionOn).not.toHaveBeenCalled()
    } finally {
      outside.remove()
    }
  })
})
