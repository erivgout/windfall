import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { afterEach, beforeEach, expect, it, vi } from "vitest"

import type { NoteTransform } from "@/bindings"
import { runAction } from "@/lib/actions"
import { SimDocument } from "@/lib/ipc/sim/document"
import { dispatch, redo, undo, useProjectStore } from "@/lib/store/project"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { settle } from "@/test/harness"

import { NoteToolsDialog } from "./note-tools-dialog"
import { closeNoteTools, parseChopSteps, useNoteTools } from "./note-tools"
import { usePianoRollStore } from "./store"
import {
  channel,
  currentPattern,
  notesOf,
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
      { start: 480, length: 720, key: 61, velocity: 0.5, pan: -0.25 },
      { start: 480, length: 720, key: 65, velocity: 0.8, pan: 0.25 },
      { start: 1440, length: 240, key: 72, velocity: 0.6, pan: 0 },
    ],
  })
  roll.show("Lead")
  roll.editor.setSelection(
    notesOf("Lead")
      .slice(0, 2)
      .map((n) => n.id)
  )
  usePianoRollStore.getState().setSnapToScale(true)
})
afterEach(() => {
  closeNoteTools()
  roll.stop()
})

async function open(tool: NoteTransform["type"]) {
  await runAction(`pianoRoll.${tool}`)
  render(<NoteToolsDialog />)
  expect(screen.getByRole("dialog")).toBeInTheDocument()
}
function field(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } })
}
async function apply() {
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: "Apply" }))
    await settle()
  })
  await waitFor(() =>
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
  )
}
async function historyAndPersistence(
  before: ReturnType<typeof notesOf>,
  cursor: number,
  selected: number
) {
  const after = structuredClone(notesOf("Lead"))
  expect(after.find((n) => n.id === before[2].id)).toEqual(before[2])
  expect(undoSteps()).toBe(cursor + 1)
  expect(roll.editor.selectionCount).toBe(selected)
  expect([...roll.editor.selection].sort()).toEqual(
    after
      .filter((n) => n.id !== before[2].id)
      .map((n) => n.id)
      .sort()
  )
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

it("custom Chop accessible controls produce accented partial edges in real Rust WASM", async () => {
  const before = structuredClone(notesOf("Lead"))
  const cursor = undoSteps()
  await open("chopPattern")
  field("Origin (ticks)", "100")
  field("Period (ticks)", "480")
  field("Pattern boundaries", "0, 120:50:150, 360:100:50")
  expect(undoSteps()).toBe(cursor)
  await apply()
  expect(
    notesOf("Lead")
      .filter((n) => n.key === 61)
      .map((n) => [n.start, n.length, n.velocity, n.pan])
  ).toEqual([
    [480, 100, 0.25, -0.25],
    [580, 120, 0.5, -0.25],
    [700, 120, 0.75, -0.25],
    [940, 120, 0.25, -0.25],
    [1060, 120, 0.5, -0.25],
    [1180, 10, 0.75, -0.25],
  ])
  expect(notesOf("Lead").find((n) => n.id === before[0].id)?.start).toBe(480)
  await historyAndPersistence(before, cursor, 12)
})

it("arpeggio controls rewrite repeated octave voices without scale retuning", async () => {
  const before = structuredClone(notesOf("Lead"))
  const cursor = undoSteps()
  await open("arpeggiate")
  field("Grid (ticks)", "120")
  field("Gate (%)", "50")
  field("Octave span", "2")
  fireEvent.click(screen.getByRole("button", { name: "Fixed repetitions" }))
  field("Repetitions", "2")
  await apply()
  const output = notesOf("Lead").filter((n) => n.id !== before[2].id)
  expect(output.map((n) => [n.start, n.key, n.length])).toEqual([
    [480, 61, 60],
    [600, 65, 60],
    [720, 73, 60],
    [840, 77, 60],
    [960, 61, 60],
    [1080, 65, 60],
    [1200, 73, 60],
    [1320, 77, 60],
  ])
  for (const n of output) {
    const source = before[n.key % 12 === 1 ? 0 : 1]
    expect(n.velocity).toBe(source.velocity)
    expect(n.pan).toBe(source.pan)
  }
  await historyAndPersistence(before, cursor, 8)
})

it("Flam keeps originals and selects positioned grace notes", async () => {
  const before = structuredClone(notesOf("Lead"))
  const cursor = undoSteps()
  await open("flam")
  field("Flam interval (ticks)", "30")
  field("Grace velocity (%)", "50")
  await apply()
  expect(
    notesOf("Lead")
      .slice(0, 2)
      .map((n) => [n.start, n.length, n.key])
  ).toEqual([
    [450, 30, 61],
    [450, 30, 65],
  ])
  for (const n of before) expect(notesOf("Lead")).toContainEqual(n)
  await historyAndPersistence(before, cursor, 4)
})

it.each(["remove", "add", "shift"] as const)(
  "Rhythm reshaper %s uses reviewed phase and offset",
  async (mode) => {
    const before = structuredClone(notesOf("Lead"))
    const cursor = undoSteps()
    await open("rhythmReshape")
    field("Grid (ticks)", "240")
    field("Period (steps)", "2")
    field("Phase (step index)", "0")
    fireEvent.click(
      screen.getByRole("button", {
        name: mode[0].toUpperCase() + mode.slice(1),
      })
    )
    if (mode !== "remove")
      field("Offset (ticks)", mode === "add" ? "240" : "60")
    await apply()
    const selected = notesOf("Lead").filter((n) => n.id !== before[2].id)
    expect(selected.map((n) => n.start)).toEqual(
      mode === "remove"
        ? []
        : mode === "add"
          ? [480, 480, 720, 720]
          : [540, 540]
    )
    expect(selected.every((n) => [61, 65].includes(n.key))).toBe(true)
    await historyAndPersistence(
      before,
      cursor,
      mode === "remove" ? 0 : mode === "add" ? 4 : 2
    )
  }
)

it.each(["chopPattern", "arpeggiate", "flam", "rhythmReshape"] as const)(
  "%s review and Cancel are inert, and stale reviews refuse Apply",
  async (tool) => {
    await open(tool)
    const before = useProjectStore.getState()
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }))
    expect(useProjectStore.getState()).toBe(before)
    await act(async () => {
      await runAction(`pianoRoll.${tool}`)
    })
    expect(useNoteTools.getState().request).not.toBeNull()
    await act(async () => {
      roll.show("Kick")
    })
    expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
    expect(useProjectStore.getState()).toBe(before)
    await act(async () => {
      announceProjectReplaced()
    })
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    expect(useNoteTools.getState().request).toBeNull()
  }
)

it("custom boundary validation prevents malformed or duplicate pattern Apply", async () => {
  await open("chopPattern")
  const before = useProjectStore.getState()
  for (const text of [
    "",
    "0,0",
    "1,120",
    "0,960",
    "0:0",
    "0:NaN",
    "0:100:Infinity",
    "0,".repeat(64) + "0",
  ]) {
    field("Pattern boundaries", text)
    expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
    expect(screen.getByLabelText("Pattern boundaries")).toHaveAttribute(
      "aria-invalid",
      "true"
    )
  }
  expect(parseChopSteps("0,120:50:80", 480)).toEqual([
    { tick: 0, gate: 1, velocity: 1 },
    { tick: 120, gate: 0.5, velocity: 0.8 },
  ])
  expect(useProjectStore.getState()).toBe(before)
})

it("original-span arpeggio refuses incomplete rate slots without partial edit", async () => {
  await open("arpeggiate")
  field("Grid (ticks)", "200")
  const before = useProjectStore.getState()
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: "Apply" }))
    await settle()
  })
  expect(screen.getByRole("dialog")).toBeInTheDocument()
  expect(screen.getByText(/Could not apply the tool/)).toBeInTheDocument()
  expect(useProjectStore.getState()).toBe(before)
})

it.each([
  ["flam", "Flam interval (ticks)", "960"],
  ["arpeggiate", "Octave span", "7"],
  ["rhythmReshape", "Offset (ticks)", "-1000"],
] as const)(
  "%s refuses out-of-bounds complete output atomically",
  async (tool, label, value) => {
    await open(tool)
    if (tool === "rhythmReshape")
      fireEvent.click(screen.getByRole("button", { name: "Shift" }))
    field(label, value)
    const before = useProjectStore.getState()
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Apply" }))
      await settle()
    })
    expect(screen.getByText(/Could not apply the tool/)).toBeInTheDocument()
    expect(useProjectStore.getState()).toBe(before)
  }
)

it.each([
  ["arpeggiate", "Gate (%)", "0"],
  ["flam", "Flam interval (ticks)", "0"],
  ["rhythmReshape", "Phase (step index)", "2"],
] as const)(
  "%s disables Apply for invalid parameters",
  async (tool, label, value) => {
    await open(tool)
    const before = useProjectStore.getState()
    field(label, value)
    expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
    expect(screen.getByLabelText(label)).toHaveAttribute("aria-invalid", "true")
    expect(useProjectStore.getState()).toBe(before)
  }
)

it("zero-offset Shift closes review while preserving redo, dirty state and IDs", async () => {
  const beforeNotes = structuredClone(notesOf("Lead"))
  await dispatch({ type: "updateSettings", patch: { name: "Undo me" } })
  await undo()
  await open("rhythmReshape")
  fireEvent.click(screen.getByRole("button", { name: "Shift" }))
  field("Offset (ticks)", "0")
  const before = useProjectStore.getState()
  await apply()
  const after = useProjectStore.getState()
  expect(after.history).toEqual(before.history)
  expect(after.dirty).toBe(before.dirty)
  expect(after.project.nextId).toBe(before.project.nextId)
  expect(notesOf("Lead")).toEqual(beforeNotes)
  await redo()
  expect(useProjectStore.getState().project.settings.name).toBe("Undo me")
})

it("fixed arpeggio duration extends the pattern in the same undo step", async () => {
  const before = structuredClone(notesOf("Lead"))
  const cursor = undoSteps()
  const length = currentPattern().lengthSteps
  await open("arpeggiate")
  field("Grid (ticks)", "120")
  fireEvent.click(screen.getByRole("button", { name: "Fixed repetitions" }))
  field("Repetitions", "16")
  await apply()
  expect(currentPattern().lengthSteps).toBe(18)
  expect(currentPattern().lengthSteps).toBeGreaterThan(length)
  expect(undoSteps()).toBe(cursor + 1)
  expect(notesOf("Lead")).toHaveLength(33)
  const after = structuredClone(notesOf("Lead"))
  await undo()
  expect(currentPattern().lengthSteps).toBe(length)
  expect(notesOf("Lead")).toEqual(before)
  await redo()
  expect(currentPattern().lengthSteps).toBe(18)
  expect(notesOf("Lead")).toEqual(after)
})
