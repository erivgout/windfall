import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it } from "vitest"

import { dispatch, redo, undo } from "@/lib/store/project"
import { SessionContext } from "../context"
import {
  channel,
  currentPattern,
  notesOf,
  startRoll,
  undoSteps,
} from "../test-utils"
import { ChordToolsControl } from "./control"

let roll: Awaited<ReturnType<typeof startRoll>>
beforeEach(async () => {
  roll = await startRoll()
  await dispatch({ type: "addChannel", name: "Lead" })
  roll.show("Lead")
  render(
    <SessionContext.Provider value={roll.session}>
      <ChordToolsControl />
    </SessionContext.Provider>
  )
})
afterEach(() => {
  cleanup()
  roll.stop()
})

function open() {
  fireEvent.click(screen.getByRole("button", { name: "Chords" }))
}

describe("chord tools control", () => {
  it("shows a clear empty result and advances the seed preview", async () => {
    open()
    expect(
      screen.getByRole("status", { name: "Selection chord" })
    ).toHaveTextContent("No notes selected")
    expect(screen.getByLabelText("Seeded chord preview")).toHaveTextContent(
      "I · C major · MIDI 60, 64, 67"
    )
    fireEvent.click(screen.getByRole("button", { name: "Next seed" }))
    expect(screen.getByLabelText("Seed integer")).toHaveValue(1)
    expect(screen.getByLabelText("Seeded chord preview")).toHaveTextContent(
      "vi · A minor · MIDI 69, 72, 76"
    )
    fireEvent.change(screen.getByLabelText("Seed integer"), {
      target: { value: "" },
    })
    expect(
      screen.getByRole("button", { name: "Insert seeded triad" })
    ).toBeDisabled()
    expect(screen.getByRole("button", { name: "Insert triad" })).toBeEnabled()
  })

  it("inserts a seeded triad and grows the pattern in one undo, with redo", async () => {
    const before = undoSteps()
    const previousLength = currentPattern().lengthSteps
    open()
    fireEvent.change(screen.getByLabelText("Start (ticks)"), {
      target: { value: "3840" },
    })
    fireEvent.change(screen.getByLabelText("Length (ticks)"), {
      target: { value: "960" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Next seed" }))
    fireEvent.click(screen.getByRole("button", { name: "Insert seeded triad" }))
    await waitFor(() => expect(notesOf("Lead")).toHaveLength(3))
    expect(
      notesOf("Lead").map((note) => [note.key, note.start, note.length])
    ).toEqual([
      [69, 3840, 960],
      [72, 3840, 960],
      [76, 3840, 960],
    ])
    expect(undoSteps()).toBe(before + 1)
    expect(currentPattern().lengthSteps).toBe(32)
    await undo()
    expect(notesOf("Lead")).toEqual([])
    expect(currentPattern().lengthSteps).toBe(previousLength)
    await redo()
    expect(notesOf("Lead")).toHaveLength(3)
    expect(currentPattern().lengthSteps).toBe(32)
  })

  it("detects the selected chord and inserts a manual triad at the chosen root", async () => {
    const added = await dispatch({
      type: "addNotes",
      pattern: currentPattern().id,
      channel: channel("Lead").id,
      notes: [60, 64, 67].map((key) => ({ key, start: 0, length: 240 })),
    })
    roll.editor.setSelection(added!.created)
    open()
    expect(
      screen.getByRole("status", { name: "Selection chord" })
    ).toHaveTextContent("C major")
    fireEvent.change(screen.getByLabelText("Root / major-key tonic (MIDI)"), {
      target: { value: "62" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Minor" }))
    fireEvent.click(screen.getByRole("button", { name: "Insert triad" }))
    await waitFor(() => expect(notesOf("Lead")).toHaveLength(6))
    expect(
      notesOf("Lead")
        .filter((note) => !added!.created.includes(note.id))
        .map((note) => note.key)
    ).toEqual([62, 65, 69])
  })

  it("blocks overflowing roots and blank or out-of-bounds timing", () => {
    open()
    fireEvent.change(screen.getByLabelText("Root / major-key tonic (MIDI)"), {
      target: { value: "127" },
    })
    expect(screen.getByRole("button", { name: "Insert triad" })).toBeDisabled()
    expect(
      screen.getByRole("button", { name: "Insert seeded triad" })
    ).toBeDisabled()
    fireEvent.change(screen.getByLabelText("Root / major-key tonic (MIDI)"), {
      target: { value: "60" },
    })
    fireEvent.change(screen.getByLabelText("Start (ticks)"), {
      target: { value: "" },
    })
    expect(screen.getByRole("button", { name: "Insert triad" })).toBeDisabled()
  })
})
