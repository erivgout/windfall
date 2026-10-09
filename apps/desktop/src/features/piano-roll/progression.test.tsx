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
import { dispatch, redo, undo, useProjectStore } from "@/lib/store/project"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { useUiStore } from "@/lib/store/ui"
import { MAX_PATTERN_TICKS } from "@/lib/units"

import { NOTE_MENU, PANEL_MENU } from "./menu"
import {
  applyProgression,
  closeProgressionGenerator,
  generateProgressionNotes,
  useProgressionGenerator,
  type ProgressionSettings,
} from "./progression"
import { ProgressionGeneratorDialog } from "./progression-dialog"
import { usePianoRollStore } from "./store"
import {
  channel,
  currentPattern,
  notesOf,
  startRoll,
  undoSteps,
} from "./test-utils"

const settings: ProgressionSettings = {
  root: 0,
  mode: "major",
  mood: "bright",
  bars: 4,
  seed: 0,
}
const timing = { barTicks: 3840, velocity: 0.63 }

describe("diatonic progression notes", () => {
  it("repeats the same seed and rotates the fixed cycle by seed modulo four", () => {
    const notes = generateProgressionNotes([], settings, timing)
    expect(notes.map(({ start, key, length }) => [start, key, length])).toEqual(
      [
        [0, 60, 3840],
        [0, 64, 3840],
        [0, 67, 3840],
        [3840, 55, 3840],
        [3840, 59, 3840],
        [3840, 62, 3840],
        [7680, 57, 3840],
        [7680, 60, 3840],
        [7680, 64, 3840],
        [11520, 65, 3840],
        [11520, 69, 3840],
        [11520, 72, 3840],
      ]
    )
    expect(generateProgressionNotes([], settings, timing)).toEqual(notes)
    expect(
      generateProgressionNotes([], { ...settings, seed: 4 }, timing)
    ).toEqual(notes)
    const rotated = generateProgressionNotes(
      [],
      { ...settings, seed: 1 },
      timing
    )
    expect(rotated.slice(0, 3).map((note) => note.key)).toEqual([55, 59, 62])
    expect(
      rotated.filter((_, index) => index % 3 === 0).map((note) => note.start)
    ).toEqual([0, 3840, 7680, 11520])
  })

  it("makes bright and tense majors different, including a diminished leading triad", () => {
    const bright = generateProgressionNotes([], settings, timing)
    const tense = generateProgressionNotes(
      [],
      { ...settings, mood: "tense" },
      timing
    )
    expect(tense).not.toEqual(bright)
    expect(tense.slice(0, 3).map((note) => note.key)).toEqual([59, 62, 65])
  })

  it.each([
    { mode: "major", mood: "bright", roots: [0, 7, 9, 5] },
    { mode: "major", mood: "calm", roots: [0, 9, 5, 7] },
    { mode: "major", mood: "tense", roots: [11, 7, 0, 5] },
    { mode: "minor", mood: "bright", roots: [0, 8, 3, 10] },
    { mode: "minor", mood: "calm", roots: [0, 8, 5, 7] },
    { mode: "minor", mood: "tense", roots: [0, 10, 8, 7] },
  ] as const)(
    "uses the $mode $mood cycle for every root, with bounded simultaneous triads",
    ({ mode, mood, roots }) => {
      const intervals =
        mode === "major" ? [0, 2, 4, 5, 7, 9, 11] : [0, 2, 3, 5, 7, 8, 10]
      for (const center of [0, 60, 78, 127]) {
        for (let root = 0; root < 12; root++) {
          const source = [{ key: center, start: 20, length: 200 }]
          const before = structuredClone(source)
          const notes = generateProgressionNotes(
            source,
            { ...settings, mode, mood, root, bars: 8 },
            timing
          )
          expect(notes).toHaveLength(24)
          for (let bar = 0; bar < 8; bar++) {
            const chord = notes.slice(bar * 3, bar * 3 + 3)
            const degree = intervals.indexOf(roots[bar % 4])
            expect(chord.map((note) => (note.key - root + 12) % 12)).toEqual(
              [degree, degree + 2, degree + 4].map(
                (step) => intervals[step % 7]
              )
            )
            expect(
              chord.every(
                (note) =>
                  note.start === 220 + bar * 3840 &&
                  note.length === 3840 &&
                  note.key >= 24 &&
                  note.key <= 96 &&
                  note.velocity === 0.63 &&
                  note.pan === 0
              )
            ).toBe(true)
            expect(chord[0].key).toBeLessThan(chord[1].key)
            expect(chord[1].key).toBeLessThan(chord[2].key)
            if (center === 60 || center === 78) {
              expect(chord[0].key).toBeGreaterThanOrEqual(center - 6)
              expect(chord[0].key).toBeLessThan(center + 6)
            }
          }
          expect(source).toEqual(before)
        }
      }
    }
  )

  it("appends after the latest end and rejects an overflowing span", () => {
    const source = [
      { key: 72, start: 0, length: 1300 },
      { key: 84, start: 1000, length: 120 },
    ]
    expect(generateProgressionNotes(source, settings, timing)[0].start).toBe(
      1300
    )
    expect(() =>
      generateProgressionNotes(
        [{ key: 60, start: MAX_PATTERN_TICKS - 240, length: 240 }],
        settings,
        timing
      )
    ).toThrow("not enough room")
  })
})

describe("piano-roll progression action and dialog", () => {
  let roll: Awaited<ReturnType<typeof startRoll>>
  beforeEach(async () => {
    closeProgressionGenerator()
    roll = await startRoll()
    await dispatch({ type: "addChannel", name: "Chords" })
    roll.show("Chords")
    render(<ProgressionGeneratorDialog />)
  })
  afterEach(() => {
    cleanup()
    closeProgressionGenerator()
    vi.restoreAllMocks()
    roll.stop()
  })

  async function open() {
    await act(() => runAction("pianoRoll.generateProgression"))
    expect(
      screen.getByRole("dialog", { name: "Generate progression" })
    ).toBeInTheDocument()
  }
  async function confirm() {
    fireEvent.click(screen.getByRole("button", { name: "Apply" }))
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    )
  }

  it("registers in the action palette and both menus with the riff's enabled rule", async () => {
    const action = registry.get("pianoRoll.generateProgression")!
    expect(action.title).toBe("Generate progression…")
    expect(NOTE_MENU).toContain(action.id)
    expect(PANEL_MENU).toContain(action.id)
    expect(isEnabled(action, getAppState())).toBe(true)
    useUiStore.getState().showCenterTab("playlist")
    expect(isEnabled(action, getAppState())).toBe(false)
    useUiStore.getState().showCenterTab("pianoRoll")
    roll.editor.setContext(null)
    expect(isEnabled(action, getAppState())).toBe(false)
    await act(() => runAction(action.id))
    expect(useProgressionGenerator.getState().request).toBeNull()
  })

  it("offers the requested fields and appends with one addNotes and one undo step", async () => {
    await dispatch({
      type: "addNotes",
      pattern: currentPattern().id,
      channel: channel("Chords").id,
      notes: [{ key: 72, start: 100, length: 550, velocity: 0.4 }],
    })
    const original = structuredClone(notesOf("Chords"))
    const patternLength = currentPattern().lengthSteps
    const before = undoSteps()
    usePianoRollStore.getState().rememberNote(240, 0.63)
    const send = vi.spyOn(roll.backend, "dispatch")
    await open()
    expect(screen.getByLabelText("Length (bars)")).toHaveValue(4)
    expect(screen.getByLabelText("Seed")).toHaveValue(0)
    const user = userEvent.setup()
    fireEvent.keyDown(screen.getByRole("combobox", { name: "Root" }), {
      key: "ArrowDown",
    })
    expect(await screen.findAllByRole("option")).toHaveLength(12)
    await user.click(screen.getByRole("option", { name: "D" }))
    fireEvent.click(screen.getByRole("button", { name: "Natural minor" }))
    expect(screen.getByRole("button", { name: "Bright" })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Calm" })).toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", { name: "Tense" }))
    fireEvent.change(screen.getByLabelText("Length (bars)"), {
      target: { value: "2" },
    })
    fireEvent.change(screen.getByLabelText("Seed"), { target: { value: "1" } })
    await confirm()
    expect(send).toHaveBeenCalledTimes(1)
    expect(send.mock.calls[0][0]).toMatchObject({
      type: "addNotes",
      pattern: currentPattern().id,
      channel: channel("Chords").id,
    })
    const added = notesOf("Chords").slice(1)
    expect(added).toHaveLength(6)
    expect(
      added.map(({ start, key, length, velocity }) => [
        start,
        key,
        length,
        velocity,
      ])
    ).toEqual([
      [650, 72, 3840, 0.63],
      [650, 76, 3840, 0.63],
      [650, 79, 3840, 0.63],
      [4490, 70, 3840, 0.63],
      [4490, 74, 3840, 0.63],
      [4490, 77, 3840, 0.63],
    ])
    expect(notesOf("Chords")[0]).toEqual(original[0])
    expect(currentPattern().lengthSteps).toBe(patternLength)
    expect(undoSteps()).toBe(before + 1)
    await act(() => undo())
    expect(notesOf("Chords")).toEqual(original)
    await act(() => redo())
    expect(notesOf("Chords").slice(1)).toEqual(added)
  })

  it("starts an empty channel at tick zero and preserves a previous run on the next append", async () => {
    const length = currentPattern().lengthSteps
    await open()
    await confirm()
    const first = structuredClone(notesOf("Chords"))
    expect(first).toHaveLength(12)
    expect(first.slice(0, 3).map((note) => note.start)).toEqual([0, 0, 0])
    await open()
    await confirm()
    expect(notesOf("Chords")).toHaveLength(24)
    expect(notesOf("Chords").slice(0, 12)).toEqual(first)
    expect(notesOf("Chords")[12].start).toBe(15360)
    expect(currentPattern().lengthSteps).toBe(length)
  })

  it("dispatches nothing while unconfirmed or cancelled", async () => {
    const before = undoSteps()
    const send = vi.spyOn(roll.backend, "dispatch")
    await open()
    fireEvent.change(screen.getByLabelText("Length (bars)"), {
      target: { value: "8" },
    })
    fireEvent.change(screen.getByLabelText("Seed"), {
      target: { value: "123" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Calm" }))
    expect(send).not.toHaveBeenCalled()
    expect(notesOf("Chords")).toEqual([])
    const request = useProgressionGenerator.getState().request!
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }))
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    expect(await applyProgression(request, settings)).toBe(false)
    expect(send).not.toHaveBeenCalled()
    expect(notesOf("Chords")).toEqual([])
    expect(undoSteps()).toBe(before)
  })

  it("blocks invalid fields and a stale channel without dispatching", async () => {
    const send = vi.spyOn(roll.backend, "dispatch")
    await open()
    for (const value of ["", "-1", "1.5"]) {
      fireEvent.change(screen.getByLabelText("Seed"), { target: { value } })
      expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
    }
    fireEvent.change(screen.getByLabelText("Seed"), { target: { value: "12" } })
    for (const value of ["1", "9", "2.5"]) {
      fireEvent.change(screen.getByLabelText("Length (bars)"), {
        target: { value },
      })
      expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
    }
    fireEvent.change(screen.getByLabelText("Length (bars)"), {
      target: { value: "8" },
    })
    expect(screen.getByRole("button", { name: "Apply" })).toBeEnabled()
    const request = useProgressionGenerator.getState().request!
    act(() => roll.show("Kick"))
    expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
    expect(await applyProgression(request, settings)).toBe(false)
    expect(send).not.toHaveBeenCalled()
  })

  it("closes and invalidates its request on project replacement", async () => {
    const send = vi.spyOn(roll.backend, "dispatch")
    await open()
    const request = useProgressionGenerator.getState().request!
    act(() => announceProjectReplaced())
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    expect(await applyProgression(request, settings)).toBe(false)
    expect(send).not.toHaveBeenCalled()
  })

  it("measures the pattern's meter at the append tick and falls back to 4/4", async () => {
    // Give the editor a non-common inherited meter; an unset pattern still
    // uses the requested 4/4 fallback for this writer.
    act(() =>
      useProjectStore.setState((state) => ({
        project: {
          ...state.project,
          settings: {
            ...state.project.settings,
            timeSignature: { numerator: 7, denominator: 8 },
          },
        },
      }))
    )
    await open()
    expect(useProgressionGenerator.getState().request?.barTicks).toBe(3840)
    act(closeProgressionGenerator)
    act(() =>
      useProjectStore.setState((state) => ({
        project: {
          ...state.project,
          patterns: state.project.patterns.map((pattern) =>
            pattern.id === currentPattern().id
              ? { ...pattern, timeSignature: { numerator: 3, denominator: 4 } }
              : pattern
          ),
        },
      }))
    )
    await open()
    expect(useProgressionGenerator.getState().request?.barTicks).toBe(2880)
    act(closeProgressionGenerator)
    await act(() =>
      dispatch({
        type: "addNotes",
        pattern: currentPattern().id,
        channel: channel("Chords").id,
        notes: [{ key: 60, start: 0, length: 3840 }],
      })
    )
    act(() =>
      useProjectStore.setState((state) => ({
        project: {
          ...state.project,
          patterns: state.project.patterns.map((pattern) =>
            pattern.id === currentPattern().id
              ? {
                  ...pattern,
                  timeSignature: { numerator: 3, denominator: 4 },
                  timeline: {
                    markers: [],
                    meters: [
                      {
                        id: 100,
                        tick: 2880,
                        signature: { numerator: 5, denominator: 8 },
                      },
                    ],
                  },
                }
              : pattern
          ),
        },
      }))
    )
    await open()
    expect(useProgressionGenerator.getState().request?.barTicks).toBe(2400)
    const request = useProgressionGenerator.getState().request!
    const generated = generateProgressionNotes(
      request.context.notes,
      settings,
      request
    )
    expect(generated[0].start).toBe(3840)
    expect(generated[3].start).toBe(6240)
    expect(generated.every((note) => note.length === 2400)).toBe(true)
  })
})
