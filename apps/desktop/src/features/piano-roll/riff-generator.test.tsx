import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { getAppState, isEnabled, registry, runAction } from "@/lib/actions"
import { dispatch, redo, undo } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { MAX_PATTERN_TICKS } from "@/lib/units"

import {
  applyRiff,
  closeRiffGenerator,
  generateRiffNotes,
  RIFF_SCALES,
  useRiffGenerator,
  type RiffSettings,
} from "./riff-generator"
import { RiffGeneratorDialog } from "./riff-generator-dialog"
import { usePianoRollStore } from "./store"
import {
  channel,
  currentPattern,
  notesOf,
  startRoll,
  undoSteps,
} from "./test-utils"

const settings: RiffSettings = {
  root: 0,
  scale: "major",
  bars: 1,
  density: "medium",
  seed: 123,
}
const timing = { step: 240, barTicks: 3840, velocity: 0.63 }

describe("seeded riff notes", () => {
  it("pins a stable seed and allows a different seed to differ", () => {
    const notes = generateRiffNotes([], settings, timing)
    expect(notes.map((note) => [note.start, note.key, note.length])).toEqual([
      [0, 60, 240],
      [240, 57, 240],
      [480, 62, 240],
      [720, 59, 240],
      [960, 62, 240],
      [1680, 64, 240],
      [2160, 64, 240],
      [2880, 62, 240],
      [3360, 55, 240],
    ])
    expect(generateRiffNotes([], settings, timing)).toEqual(notes)
    expect(
      generateRiffNotes([], { ...settings, seed: 124 }, timing)
    ).not.toEqual(notes)
    expect(
      notes.every((note) => note.velocity === 0.63 && note.pan === 0)
    ).toBe(true)
  })

  it.each(RIFF_SCALES)(
    "keeps every root in $label inside one octave around C4",
    ({ value }) => {
      const intervals =
        value === "major"
          ? [0, 2, 4, 5, 7, 9, 11]
          : value === "minor"
            ? [0, 2, 3, 5, 7, 8, 10]
            : [0, 2, 4, 7, 9]
      for (let root = 0; root < 12; root++) {
        const notes = generateRiffNotes(
          [],
          { ...settings, scale: value, root, bars: 4, density: "high" },
          timing
        )
        expect(notes.length).toBeGreaterThan(0)
        for (const note of notes) {
          expect(intervals).toContain((note.key - root + 12) % 12)
          expect(note.key).toBeGreaterThanOrEqual(54)
          expect(note.key).toBeLessThanOrEqual(65)
          expect(note.start + note.length).toBeLessThanOrEqual(4 * 3840)
        }
      }
    }
  )

  it("uses the latest end, even when an earlier note lasts longer, and centers on the pitch range", () => {
    const source = [
      { key: 72, start: 0, length: 1300 },
      { key: 84, start: 1000, length: 120 },
    ]
    const before = structuredClone(source)
    const notes = generateRiffNotes(source, settings, { ...timing, step: 120 })
    expect(notes[0].start).toBe(1300)
    expect(
      notes.every(
        (note) =>
          note.start >= 1300 &&
          note.length === 120 &&
          note.key >= 72 &&
          note.key <= 83
      )
    ).toBe(true)
    expect(source).toEqual(before)
  })

  it("keeps the octave inside MIDI bounds and rejects an overflowing append", () => {
    for (const key of [0, 127]) {
      const notes = generateRiffNotes(
        [{ key, start: 0, length: 240 }],
        settings,
        timing
      )
      expect(notes.every((note) => note.key >= 0 && note.key <= 127)).toBe(true)
      expect(
        Math.max(...notes.map((note) => note.key)) -
          Math.min(...notes.map((note) => note.key))
      ).toBeLessThan(12)
    }
    expect(() =>
      generateRiffNotes(
        [{ key: 60, start: MAX_PATTERN_TICKS - 240, length: 240 }],
        settings,
        timing
      )
    ).toThrow("not enough room")
  })
})

describe("piano-roll riff action and dialog", () => {
  let roll: Awaited<ReturnType<typeof startRoll>>
  beforeEach(async () => {
    closeRiffGenerator()
    roll = await startRoll()
    await dispatch({ type: "addChannel", name: "Lead" })
    roll.show("Lead")
    render(<RiffGeneratorDialog />)
  })
  afterEach(() => {
    cleanup()
    closeRiffGenerator()
    vi.restoreAllMocks()
    roll.stop()
  })

  async function open() {
    await act(() => runAction("pianoRoll.generateRiff"))
    expect(
      screen.getByRole("dialog", { name: "Generate riff" })
    ).toBeInTheDocument()
  }

  it("enables Generate riff only in a piano roll with an open pattern channel", async () => {
    const action = registry.get("pianoRoll.generateRiff")!
    expect(action.title).toBe("Generate riff…")
    expect(isEnabled(action, getAppState())).toBe(true)
    useUiStore.getState().showCenterTab("playlist")
    expect(isEnabled(action, getAppState())).toBe(false)
    useUiStore.getState().showCenterTab("pianoRoll")
    roll.editor.setContext(null)
    expect(isEnabled(action, getAppState())).toBe(false)
    await act(() => runAction(action.id))
    expect(useRiffGenerator.getState().request).toBeNull()
  })

  it("offers all settings, appends after existing notes via one addNotes, and undoes in one step", async () => {
    await dispatch({
      type: "addNotes",
      pattern: currentPattern().id,
      channel: channel("Lead").id,
      notes: [{ key: 72, start: 100, length: 550, velocity: 0.4 }],
    })
    const original = structuredClone(notesOf("Lead"))
    const patternLength = currentPattern().lengthSteps
    const before = undoSteps()
    usePianoRollStore.getState().setSnap("step/2")
    usePianoRollStore.getState().rememberNote(960, 0.63)
    const send = vi.spyOn(roll.backend, "dispatch")
    await open()
    expect(screen.getByLabelText("Length (bars)")).toHaveValue(1)
    expect(screen.getByLabelText("Seed")).toHaveValue(0)
    const user = userEvent.setup()
    fireEvent.keyDown(screen.getByRole("combobox", { name: "Root" }), {
      key: "ArrowDown",
    })
    expect(await screen.findAllByRole("option")).toHaveLength(12)
    await user.click(screen.getByRole("option", { name: "D" }))
    fireEvent.click(screen.getByRole("button", { name: "Natural minor" }))
    expect(
      screen.getByRole("button", { name: "Pentatonic major" })
    ).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Low" })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Medium" })).toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", { name: "High" }))
    fireEvent.change(screen.getByLabelText("Length (bars)"), {
      target: { value: "2" },
    })
    fireEvent.change(screen.getByLabelText("Seed"), {
      target: { value: "123" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Apply" }))
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    )
    expect(send).toHaveBeenCalledTimes(1)
    expect(send.mock.calls[0][0]).toMatchObject({
      type: "addNotes",
      pattern: currentPattern().id,
      channel: channel("Lead").id,
    })
    expect(notesOf("Lead")[0]).toEqual(original[0])
    const added = notesOf("Lead").slice(1)
    expect(added[0].start).toBe(650)
    expect(
      added.every(
        (note) =>
          note.start >= 650 &&
          note.start + note.length <= 650 + 7680 &&
          note.length === 120 &&
          note.velocity === 0.63
      )
    ).toBe(true)
    expect(
      added.every(
        (note) =>
          [0, 2, 3, 5, 7, 8, 10].includes((note.key - 2 + 12) % 12) &&
          note.key >= 66 &&
          note.key <= 77
      )
    ).toBe(true)
    expect(currentPattern().lengthSteps).toBe(patternLength)
    expect(undoSteps()).toBe(before + 1)
    await act(() => undo())
    expect(notesOf("Lead")).toEqual(original)
    await act(() => redo())
    expect(notesOf("Lead").slice(1)).toEqual(added)
  })

  it("starts an empty channel at zero, uses one step with snap off, and repeats a seed after undo", async () => {
    usePianoRollStore.getState().setSnap("none")
    usePianoRollStore.getState().rememberNote(960, 0.63)
    await open()
    fireEvent.change(screen.getByLabelText("Seed"), {
      target: { value: "123" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Apply" }))
    await waitFor(() => expect(notesOf("Lead")).toHaveLength(9))
    const first = notesOf("Lead").map(({ start, key, length, velocity }) => ({
      start,
      key,
      length,
      velocity,
    }))
    expect(first[0]).toEqual({ start: 0, key: 60, length: 240, velocity: 0.63 })
    await act(() => undo())
    await open()
    fireEvent.change(screen.getByLabelText("Seed"), {
      target: { value: "123" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Apply" }))
    await waitFor(() => expect(notesOf("Lead")).toHaveLength(9))
    expect(
      notesOf("Lead").map(({ start, key, length, velocity }) => ({
        start,
        key,
        length,
        velocity,
      }))
    ).toEqual(first)
  })

  it("cancel adds nothing and invalidates the captured request", async () => {
    const before = undoSteps()
    const send = vi.spyOn(roll.backend, "dispatch")
    await open()
    const request = useRiffGenerator.getState().request!
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }))
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    expect(await applyRiff(request, settings)).toBe(false)
    expect(send).not.toHaveBeenCalled()
    expect(notesOf("Lead")).toEqual([])
    expect(undoSteps()).toBe(before)
  })

  it("rejects invalid settings and refuses a changed channel", async () => {
    await open()
    fireEvent.change(screen.getByLabelText("Seed"), { target: { value: "" } })
    expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
    fireEvent.change(screen.getByLabelText("Seed"), { target: { value: "12" } })
    fireEvent.change(screen.getByLabelText("Length (bars)"), {
      target: { value: "5" },
    })
    expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
    fireEvent.change(screen.getByLabelText("Length (bars)"), {
      target: { value: "4" },
    })
    expect(screen.getByRole("button", { name: "Apply" })).toBeEnabled()
    const request = useRiffGenerator.getState().request!
    act(() => roll.show("Kick"))
    expect(await applyRiff(request, settings)).toBe(false)
    expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
    expect(notesOf("Lead")).toEqual([])
  })
})
