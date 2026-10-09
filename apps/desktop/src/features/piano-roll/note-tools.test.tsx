import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { NoteTransform } from "@/bindings"
import { runAction } from "@/lib/actions"
import { SimDocument } from "@/lib/ipc/sim/document"
import { dispatch, redo, undo, useProjectStore } from "@/lib/store/project"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { settle } from "@/test/harness"

import { NoteToolsDialog } from "./note-tools-dialog"
import {
  applyNoteTool,
  closeNoteTools,
  openNoteTools,
  requestIsCurrent,
  useNoteTools,
} from "./note-tools"
import {
  channel,
  currentPattern,
  history,
  notesOf,
  project,
  startRoll,
  undoSteps,
} from "./test-utils"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let roll: Awaited<ReturnType<typeof startRoll>>
beforeEach(async () => {
  roll = await startRoll()
  await dispatch({ type: "addChannel", name: "Lead" })
  await dispatch({
    type: "addNotes",
    pattern: currentPattern().id,
    channel: channel("Lead").id,
    notes: [
      { start: 250, length: 400, key: 60, velocity: 0.5, pan: -0.25 },
      { start: 250, length: 200, key: 64, velocity: 0.75, pan: 0.25 },
      { start: 900, length: 240, key: 72, velocity: 0.8, pan: 0 },
    ],
  })
  roll.show("Lead")
  roll.editor.setSelection(
    notesOf("Lead")
      .slice(0, 2)
      .map((n) => n.id)
  )
})
afterEach(() => {
  closeNoteTools()
  roll.stop()
})

const TRANSFORMS: NoteTransform[] = [
  { type: "staccato", factor: 0.5 },
  { type: "chop", grid: 120 },
  { type: "strum", spacing: 30, velocityStep: -0.1, descending: true },
  { type: "flipTime" },
  { type: "flipPitch" },
  { type: "keyRange", low: 48, high: 72, transpose: 12, octaves: false },
  { type: "scaleVelocity", factor: 1.5 },
  {
    type: "quantize",
    grid: 240,
    strength: 0.5,
    edge: "start",
    groove: "swing",
  },
]

describe("selected-note tools through the real Rust WASM backend", () => {
  it.each(TRANSFORMS)(
    "$type is atomic, undoable, selects pieces and survives file reopen",
    async (transform) => {
      const before = structuredClone(notesOf("Lead"))
      const cursor = undoSteps()
      openNoteTools(transform.type)
      const request = useNoteTools.getState().request!
      expect(await applyNoteTool(request, transform)).toBe(true)
      const after = structuredClone(notesOf("Lead"))
      expect(after).not.toEqual(before)
      expect(after.find((n) => n.id === before[2].id)).toEqual(before[2])
      expect(undoSteps()).toBe(cursor + 1)
      if (transform.type === "chop")
        expect(roll.editor.selectionCount).toBe(after.length - 1)
      await undo()
      expect(notesOf("Lead")).toEqual(before)
      await redo()
      expect(notesOf("Lead")).toEqual(after)
      const doc = SimDocument.create(
        (await roll.backend.documentSnapshot()).project
      )
      const reopened = SimDocument.open(doc.fileText())
      expect(reopened.project()).toEqual(doc.project())
      reopened.dispose()
      doc.dispose()
    }
  )

  it("legato and glue use only compatible selected notes", async () => {
    const original = structuredClone(notesOf("Lead"))
    roll.editor.selectAll()
    openNoteTools("legato")
    expect(
      await applyNoteTool(useNoteTools.getState().request!, { type: "legato" })
    ).toBe(true)
    expect(notesOf("Lead").map((n) => n.length)).toEqual([650, 650, 240])
    await undo()
    await dispatch({
      type: "addNotes",
      pattern: currentPattern().id,
      channel: channel("Lead").id,
      notes: [{ start: 600, length: 200, key: 60, velocity: 0.5, pan: -0.25 }],
    })
    roll.editor.setSelection(
      notesOf("Lead")
        .filter((n) => n.key === 60)
        .map((n) => n.id)
    )
    openNoteTools("glue")
    expect(
      await applyNoteTool(useNoteTools.getState().request!, { type: "glue" })
    ).toBe(true)
    expect(notesOf("Lead").find((n) => n.key === 60)).toMatchObject({
      id: original[0].id,
      start: 250,
      length: 550,
      velocity: 0.5,
      pan: -0.25,
    })
    expect(notesOf("Lead").filter((n) => n.key !== 60)).toEqual(
      original.slice(1)
    )
  })

  it("matches native quantize rounding, strength and original grooves", async () => {
    for (const [groove, start] of [
      ["straight", 245],
      ["swing", 265],
      ["latePairs", 275],
    ] as const) {
      openNoteTools("quantize")
      expect(
        await applyNoteTool(useNoteTools.getState().request!, {
          type: "quantize",
          grid: 240,
          strength: 0.5,
          edge: "start",
          groove,
        })
      ).toBe(true)
      expect(
        notesOf("Lead")
          .slice(0, 2)
          .map((n) => n.start)
      ).toEqual([start, start])
      await undo()
    }
  })

  it("rejects stale snapshots and wrong lanes in Rust without changing any state", async () => {
    const notes = structuredClone(notesOf("Lead").slice(0, 2))
    const before = project()
    const cursor = history()
    await expect(
      roll.backend.dispatch({
        type: "transformNotes",
        pattern: currentPattern().id,
        channel: channel("Kick").id,
        notes,
        transform: { type: "glue" },
      })
    ).rejects.toThrow()
    notes[0].length++
    await expect(
      roll.backend.dispatch({
        type: "transformNotes",
        pattern: currentPattern().id,
        channel: channel("Lead").id,
        notes,
        transform: { type: "glue" },
      })
    ).rejects.toThrow()
    await expect(
      roll.backend.dispatch({
        type: "transformNotes",
        pattern: currentPattern().id,
        channel: channel("Lead").id,
        notes: [],
        transform: { type: "glue" },
      })
    ).rejects.toThrow()
    expect(project()).toBe(before)
    expect(history()).toEqual(cursor)
  })

  it("never broadens empty, changed, replaced or cancelled selections", async () => {
    roll.editor.setSelection([])
    const before = useProjectStore.getState()
    await runAction("pianoRoll.quantize")
    await runAction("pianoRoll.octaveUp")
    await runAction("pianoRoll.octaveDown")
    await roll.editor.transpose(12)
    expect(useProjectStore.getState()).toBe(before)
    expect(useNoteTools.getState().request).toBeNull()
    roll.editor.selectAll()
    openNoteTools("flipPitch")
    const request = useNoteTools.getState().request!
    roll.editor.setSelection([notesOf("Lead")[0].id])
    expect(await applyNoteTool(request, { type: "flipPitch" })).toBe(false)
    roll.editor.selectAll()
    closeNoteTools()
    expect(requestIsCurrent(request)).toBe(false)
    openNoteTools("flipPitch")
    const replaced = useNoteTools.getState().request!
    announceProjectReplaced()
    expect(await applyNoteTool(replaced, { type: "flipPitch" })).toBe(false)
  })

  it("invalidates a review on document edits, lane changes and same-size selection changes", async () => {
    openNoteTools("chop")
    const request = useNoteTools.getState().request!
    roll.editor.setSelection([notesOf("Lead")[1].id, notesOf("Lead")[2].id])
    expect(requestIsCurrent(request)).toBe(false)
    roll.editor.setSelection(request.notes.map((n) => n.id))
    expect(requestIsCurrent(request)).toBe(true)
    await dispatch({
      type: "updatePattern",
      id: currentPattern().id,
      patch: { name: "Changed" },
    })
    expect(await applyNoteTool(request, { type: "chop", grid: 120 })).toBe(
      false
    )
    openNoteTools("chop")
    const lane = useNoteTools.getState().request!
    roll.show("Kick")
    expect(await applyNoteTool(lane, { type: "chop", grid: 120 })).toBe(false)
  })

  it("dialog changes and cancel leave revision/history untouched; invalid fields disable Apply", async () => {
    await runAction("pianoRoll.quantize")
    render(<NoteToolsDialog />)
    const before = useProjectStore.getState()
    expect(
      screen.getByRole("dialog", { name: "Selected-note tools" })
    ).toBeInTheDocument()
    fireEvent.change(screen.getByLabelText("Strength (%)"), {
      target: { value: "50" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Swing" }))
    fireEvent.change(screen.getByLabelText("Divisions per unit"), {
      target: { value: "0" },
    })
    expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
    expect(screen.getByLabelText("Divisions per unit")).toHaveAttribute("aria-invalid", "true")
    fireEvent.change(screen.getByLabelText("Divisions per unit"), {
      target: { value: "1" },
    })
    expect(screen.getByRole("button", { name: "Apply" })).toBeEnabled()
    fireEvent.click(screen.getByRole("button", { name: "Custom ticks" }))
    fireEvent.change(screen.getByLabelText("Grid (ticks)"), {
      target: { value: "0" },
    })
    expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
    expect(screen.getByLabelText("Grid (ticks)")).toHaveAttribute(
      "aria-invalid",
      "true"
    )
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }))
    expect(useProjectStore.getState()).toBe(before)
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
  })

  it("dialog Apply commits once and cancellation via Escape is safe", async () => {
    openNoteTools("quantize")
    render(<NoteToolsDialog />)
    const before = undoSteps()
    fireEvent.change(screen.getByLabelText("Strength (%)"), {
      target: { value: "50.05" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Swing" }))
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Apply" }))
      await settle()
    })
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    )
    expect(
      notesOf("Lead")
        .slice(0, 2)
        .map((n) => n.start)
    ).toEqual([265, 265])
    expect(undoSteps()).toBe(before + 1)
    expect(history().entries.at(-1)?.label).toBe("Quantize note starts")
    await act(async () => {
      openNoteTools("strum")
    })
    const state = useProjectStore.getState()
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" })
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    )
    expect(useProjectStore.getState()).toBe(state)
  })
})
