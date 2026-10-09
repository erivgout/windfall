import { act, cleanup, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { TooltipProvider } from "@/components/ui/tooltip"
import { shortcutLabel } from "@/lib/actions"
import { realtimeFrame } from "@/lib/store/realtime"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { useSnapStore } from "@/lib/store/snap"
import { useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"
import { TICKS_PER_STEP } from "@/lib/units"
import { settle } from "@/test/harness"

import { auditionOff, auditionOn } from "./audition"
import PianoRollPanel from "./index"
import { useStepEntryStore } from "./step-entry"
import { usePianoRollStore } from "./store"
import {
  channel,
  currentPattern,
  notesOf,
  startRoll,
  undoSteps,
} from "./test-utils"
import { useTypingKeyboardStore } from "./typing-keyboard"

vi.mock("./audition", () => ({ auditionOn: vi.fn(), auditionOff: vi.fn() }))
vi.mock("@/lib/store/realtime", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/store/realtime")>()),
  realtimeFrame: vi.fn(),
}))

let roll: Awaited<ReturnType<typeof startRoll>>
let grid: HTMLElement

beforeEach(async () => {
  vi.clearAllMocks()
  vi.mocked(realtimeFrame).mockReturnValue({ tick: 120 } as ReturnType<
    typeof realtimeFrame
  >)
  useStepEntryStore.setState({ enabled: false, cursor: null })
  useTypingKeyboardStore.setState({ enabled: false, baseKey: 60 })
  roll = await startRoll()
  useSnapStore.getState().setSnap("step")
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

function toggle() {
  fireEvent.click(screen.getByRole("button", { name: "Step entry" }))
}

describe("piano-roll step entry", () => {
  it("inserts two keys in order from the transport position with one addNotes command per key", async () => {
    const original = notesOf("Kick")
    const before = undoSteps()
    const dispatch = vi.spyOn(roll.backend, "dispatch")
    toggle()
    press("a")
    press("a", "KeyA", true)
    press("a")
    press("s") // No wait for the first command's reply.
    expect(useStepEntryStore.getState().cursor).toBe(120 + 2 * TICKS_PER_STEP)
    lift("a")
    lift("s")
    expect(useStepEntryStore.getState().cursor).toBe(120 + 2 * TICKS_PER_STEP)
    await act(settle)
    const expected = [
      { start: 120, length: TICKS_PER_STEP, key: 60 },
      { start: 120 + TICKS_PER_STEP, length: TICKS_PER_STEP, key: 62 },
    ]
    expect(
      notesOf("Kick").filter(
        (note) => !original.some((old) => old.id === note.id)
      )
    ).toEqual(expected.map((note) => expect.objectContaining(note)))
    expect(dispatch).toHaveBeenCalledTimes(2)
    for (const [index, note] of expected.entries()) {
      expect(dispatch).toHaveBeenNthCalledWith(
        index + 1,
        {
          type: "addNotes",
          pattern: currentPattern().id,
          channel: channel("Kick").id,
          notes: [expect.objectContaining(note)],
        },
        undefined
      )
    }
    expect(undoSteps()).toBe(before + 2)
    expect(auditionOn).toHaveBeenCalledTimes(2)
    expect(auditionOff).toHaveBeenCalledWith(channel("Kick").id, 60)
    expect(auditionOff).toHaveBeenCalledWith(channel("Kick").id, 62)
  })

  it("only auditions during playback and leaves the cursor unchanged", async () => {
    const original = notesOf("Kick")
    const dispatch = vi.spyOn(roll.backend, "dispatch")
    toggle()
    act(() => useTransportStore.setState({ playing: true }))
    press("a")
    lift("a")
    await act(settle)
    expect(auditionOn).toHaveBeenCalledExactlyOnceWith(
      channel("Kick").id,
      60,
      expect.any(Number)
    )
    expect(auditionOff).toHaveBeenCalledExactlyOnceWith(channel("Kick").id, 60)
    expect(dispatch).not.toHaveBeenCalled()
    expect(notesOf("Kick")).toEqual(original)
    expect(useStepEntryStore.getState().cursor).toBe(120)
    act(() => useTransportStore.setState({ playing: false }))
    press("s")
    await act(settle)
    expect(dispatch).toHaveBeenCalledExactlyOnceWith(
      expect.objectContaining({
        type: "addNotes",
        notes: [expect.objectContaining({ start: 120, key: 62 })],
      }),
      undefined
    )
  })

  it("inserts nothing with Step entry off, including with Typing on, and keeps tool shortcuts", async () => {
    const original = notesOf("Kick")
    const dispatch = vi.spyOn(roll.backend, "dispatch")
    press("a")
    lift("a")
    press("z")
    expect(usePianoRollStore.getState().tool).toBe("zoom")
    press("`", "Backquote")
    press("a")
    lift("a")
    await act(settle)
    expect(auditionOn).toHaveBeenCalledTimes(1)
    expect(dispatch).not.toHaveBeenCalled()
    expect(notesOf("Kick")).toEqual(original)
    expect(useStepEntryStore.getState().cursor).toBeNull()
  })

  it.each(["none", "step", "beat", "bar"] as const)(
    "uses shared %s snap despite a local division",
    async (snap) => {
      act(() => {
        useSnapStore.getState().setSnap(snap)
        usePianoRollStore.getState().setSnap("step/6")
      })
      toggle()
      press("a")
      await act(settle)
      const length =
        snap === "bar" ? 3840 : snap === "beat" ? 960 : TICKS_PER_STEP
      expect(notesOf("Kick")).toContainEqual(
        expect.objectContaining({ start: 120, key: 60, length })
      )
      expect(useStepEntryStore.getState().cursor).toBe(120 + length)
    }
  )

  it("binds Backslash in both keymaps, lets Backquote toggle Typing, and consumes tool letters", async () => {
    const dispatch = vi.spyOn(roll.backend, "dispatch")
    for (const preset of ["windfall", "fl"] as const) {
      act(() => useUiStore.getState().setKeymap(preset))
      expect(shortcutLabel("pianoRoll.stepEntry")).toBe("\\")
      press("\\", "Backslash")
      expect(
        screen.getByRole("button", { name: "Step entry" })
      ).toHaveAttribute("aria-pressed", "true")
      press("\\", "Backslash", true)
      press("b")
      press("c")
      press("m")
      press("z")
      expect(usePianoRollStore.getState().tool).toBe("draw")
      expect(useTypingKeyboardStore.getState().baseKey).toBe(
        preset === "windfall" ? 48 : 36
      )
      press("`", "Backquote")
      expect(useTypingKeyboardStore.getState().enabled).toBe(true)
      press("`", "Backquote")
      expect(useTypingKeyboardStore.getState().enabled).toBe(false)
      press("\\", "Backslash")
      expect(useStepEntryStore.getState().enabled).toBe(false)
    }
    await act(settle)
    expect(dispatch).not.toHaveBeenCalled()
  })

  it("turns off, clears the cursor, and releases audition on project replacement", async () => {
    toggle()
    press("a")
    await act(settle)
    act(() => announceProjectReplaced())
    expect(useStepEntryStore.getState()).toMatchObject({
      enabled: false,
      cursor: null,
    })
    expect(auditionOff).toHaveBeenCalledExactlyOnceWith(channel("Kick").id, 60)
    lift("a")
    expect(auditionOff).toHaveBeenCalledTimes(1)
  })

  it("auditions and inserts only once when both modes are on, sharing Typing's octave clamps", async () => {
    press("`", "Backquote")
    toggle()
    for (let i = 0; i < 10; i++) press("z")
    press("x", "KeyX", true)
    press("a")
    lift("a")
    for (let i = 0; i < 10; i++) press("x")
    press("a")
    lift("a")
    await act(settle)
    expect(auditionOn).toHaveBeenCalledTimes(2)
    expect(auditionOff).toHaveBeenCalledTimes(2)
    expect(notesOf("Kick")).toContainEqual(
      expect.objectContaining({ start: 120, key: 24 })
    )
    expect(notesOf("Kick")).toContainEqual(
      expect.objectContaining({ start: 360, key: 96 })
    )
  })

  it("keeps text input and Tab working without insertion", async () => {
    const dispatch = vi.spyOn(roll.backend, "dispatch")
    toggle()
    expect(fireEvent.keyDown(grid, { key: "Tab", code: "Tab" })).toBe(true)
    const input = document.createElement("input")
    grid.append(input)
    act(() => input.focus())
    fireEvent.keyDown(input, { key: "a", code: "KeyA" })
    fireEvent.keyDown(input, { key: "\\", code: "Backslash" })
    await act(settle)
    expect(dispatch).not.toHaveBeenCalled()
    expect(auditionOn).not.toHaveBeenCalled()
    expect(useStepEntryStore.getState().enabled).toBe(true)
    expect(useStepEntryStore.getState().cursor).toBe(120)
  })

  it("releases the old channel and inserts subsequent keys on the newly open channel", async () => {
    toggle()
    press("a")
    await act(settle)
    act(() => useUiStore.getState().selectChannel(channel("Clap").id))
    await act(settle)
    expect(auditionOff).toHaveBeenCalledExactlyOnceWith(channel("Kick").id, 60)
    press("s")
    lift("s")
    await act(settle)
    expect(notesOf("Clap")).toContainEqual(
      expect.objectContaining({ start: 360, key: 62 })
    )
  })
})
