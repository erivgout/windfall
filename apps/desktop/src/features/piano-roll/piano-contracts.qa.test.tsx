import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react"
import { afterEach, beforeEach, expect, it, vi } from "vitest"
import type { Command } from "@/bindings"
import { runAction } from "@/lib/actions"
import { dispatch, redo, undo, useProjectStore } from "@/lib/store/project"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { settle } from "@/test/harness"
import { NoteToolsDialog } from "./note-tools-dialog"
import { closeNoteTools } from "./note-tools"
import { closePatternTimeline, PatternTimelineDialog } from "./pattern-timeline"
import { channel, currentPattern, history, notesOf, project, startRoll, undoSteps } from "./test-utils"

vi.mock("sonner", () => ({ toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }) }))

let roll: Awaited<ReturnType<typeof startRoll>>
beforeEach(async () => {
  roll = await startRoll()
  await dispatch({ type: "addChannel", name: "Lead" })
  await dispatch({ type: "addNotes", pattern: currentPattern().id, channel: channel("Lead").id, notes: [
    { start: 120, length: 601, key: 60, velocity: 0.6, pan: -0.4,
      expression: { release: 0.37, finePitchCents: 42, modulationX: 0.23, modulationY: 0.81,
        articulation: "portamento", glideTicks: 333, colorGroup: 12 } },
    { start: 2000, length: 240, key: 72, velocity: 0.9, pan: 0.2 },
  ] })
  roll.show("Lead")
  roll.editor.setSelection([notesOf("Lead")[0].id])
  render(<><NoteToolsDialog /><PatternTimelineDialog /></>)
})
afterEach(() => {
  cleanup()
  closeNoteTools()
  closePatternTimeline()
  roll.stop()
  vi.restoreAllMocks()
})

function field(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } })
}
async function open(action: string) {
  await act(async () => { await runAction(action); await settle() })
}
async function click(label: string) {
  const button = screen.getAllByRole("button", { name: label })
    .find((item) => label !== "Close" || item.getAttribute("data-slot") !== "dialog-close")!
  await act(async () => { fireEvent.click(button); await settle() })
}
function transformCommands(commands: Command[]) {
  return commands.filter((command) => command.type === "transformNotes")
}

it("mounted randomizer applies chosen seeded offsets once, preserving identity/expression and undo/save", async () => {
  const original = structuredClone(notesOf("Lead"))
  const before = undoSteps()
  const send = vi.spyOn(roll.backend, "dispatch")
  await open("pianoRoll.randomize")
  field("Seed", "4660")
  await click("Next seed")
  expect(screen.getByLabelText("Seed")).toHaveValue(4661)
  field("Pitch variation (± semitones)", "7")
  field("Velocity variation (± percentage points)", "25")
  field("Pan variation (± percentage points)", "50")
  field("Timing variation (± ticks)", "100")
  field("Length variation (± %)", "40")
  expect(send).not.toHaveBeenCalled()
  await click("Apply")
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull())
  expect(transformCommands(send.mock.calls.map(([command]) => command))).toEqual([expect.objectContaining({
    pattern: currentPattern().id, channel: channel("Lead").id, notes: [original[0]],
    transform: { type: "randomize", seed: 4661, pitch: 7, velocity: 0.25, pan: 0.5, timing: 100, length: 0.4 },
  })])
  const changed = structuredClone(notesOf("Lead"))
  expect(changed[0].id).toBe(original[0].id)
  expect(changed[0].expression).toEqual(original[0].expression)
  expect(changed[1]).toEqual(original[1])
  expect(changed[0]).not.toEqual(original[0])
  expect(undoSteps()).toBe(before + 1)
  expect(history().entries.at(-1)?.label).toBe("Randomize notes")
  await act(async () => { await undo(); await settle() })
  expect(notesOf("Lead")).toEqual(original)
  await act(async () => { await redo(); await settle() })
  expect(notesOf("Lead")).toEqual(changed)
  await roll.backend.projectSave("/projects/randomizer-qa.windfall")
  await act(async () => { await undo(); await roll.backend.projectOpen("/projects/randomizer-qa.windfall"); await settle() })
  expect(notesOf("Lead")).toEqual(changed)
})

it("mounted chord-map generation produces bounded partial cells with fresh IDs, inherited expression and exact redo", async () => {
  const original = structuredClone(notesOf("Lead"))
  const before = undoSteps()
  const send = vi.spyOn(roll.backend, "dispatch")
  await open("pianoRoll.generateRandom")
  field("Seed", "17")
  field("Grid (ticks)", "240")
  field("Density (%)", "100")
  field("Gate (%)", "50")
  field("Root pitch class", "0")
  field("Lowest MIDI key", "60")
  field("Highest MIDI key", "72")
  await click("Major triad")
  field("Minimum velocity (%)", "30")
  field("Maximum velocity (%)", "80")
  await click("Apply")
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull())
  expect(transformCommands(send.mock.calls.map(([command]) => command))[0]).toMatchObject({ type: "transformNotes", notes: [original[0]],
    transform: { type: "generateRandom", seed: 17, grid: 240, density: 1, gate: 0.5, root: 0,
      pitchClasses: 145, low: 60, high: 72, velocityLow: 0.3, velocityHigh: 0.8 } })
  const changed = structuredClone(notesOf("Lead"))
  const generated = changed.filter((note) => note.id !== original[1].id)
  expect(generated.map((note) => [note.start, note.length])).toEqual([[120, 120], [360, 120], [600, 61]])
  expect(new Set(generated.map((note) => note.id)).size).toBe(3)
  for (const note of generated) {
    expect(original.map((item) => item.id)).not.toContain(note.id)
    expect([60, 64, 67, 72]).toContain(note.key)
    expect(note.velocity).toBeGreaterThanOrEqual(0.3)
    expect(note.velocity).toBeLessThanOrEqual(0.8)
    expect(note.pan).toBe(original[0].pan)
    expect(note.expression).toEqual(original[0].expression)
  }
  expect(changed.find((note) => note.id === original[1].id)).toEqual(original[1])
  expect(undoSteps()).toBe(before + 1)
  await act(async () => { await undo(); await settle() })
  expect(notesOf("Lead")).toEqual(original)
  await act(async () => { await redo(); await settle() })
  expect(notesOf("Lead")).toEqual(changed)
  await roll.backend.projectSave("/projects/generator-qa.windfall")
  await act(async () => { await undo(); await roll.backend.projectOpen("/projects/generator-qa.windfall"); await settle() })
  expect(notesOf("Lead")).toEqual(changed)
})

it.each(["randomize", "generateRandom"])("mounted %s rejects invalid/stale review and cancel without editing", async (tool) => {
  const before = useProjectStore.getState()
  const send = vi.spyOn(roll.backend, "dispatch")
  await open(`pianoRoll.${tool}`)
  field("Seed", "-1")
  expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
  field("Seed", "17")
  await click("Cancel")
  expect(useProjectStore.getState()).toBe(before)
  expect(send).not.toHaveBeenCalled()
  await open(`pianoRoll.${tool}`)
  await act(async () => { await dispatch({ type: "updatePattern", id: currentPattern().id, patch: { name: "Changed while reviewing" } }); await settle() })
  const afterExternal = useProjectStore.getState()
  expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
  expect(screen.getByText(/document or selection changed/i)).toBeVisible()
  await click("Cancel")
  expect(useProjectStore.getState()).toBe(afterExternal)
  expect(transformCommands(send.mock.calls.map(([command]) => command))).toEqual([])
})

it("mounted pattern metadata edits exact captured native state, keeps notes/song map and survives undo/save", async () => {
  const originalNotes = structuredClone(notesOf("Lead"))
  const song = structuredClone(project().playlist.timeline)
  const before = undoSteps()
  const send = vi.spyOn(roll.backend, "dispatch")
  await open("pianoRoll.patternTimeline")
  field("Beats", "3")
  field("Beat unit", "4")
  await click("Set base")
  expect(currentPattern().timeSignature).toEqual({ numerator: 3, denominator: 4 })
  expect(undoSteps()).toBe(before + 1)
  field("Pattern tick", "4001")
  field("Beats", "7")
  field("Beat unit", "8")
  await click("Add meter change")
  expect(currentPattern().timeline?.meters).toEqual([expect.objectContaining({ tick: 4001, signature: { numerator: 7, denominator: 8 } })])
  field("Pattern tick", "4100")
  field("Marker name", "Local phrase")
  await click("Add marker")
  const marker = currentPattern().timeline!.markers[0]
  expect(marker).toMatchObject({ tick: 4100, name: "Local phrase", kind: { type: "named" } })
  const row = screen.getByText("Tick 4100 · Local phrase").parentElement!
  fireEvent.click(within(row).getByRole("button", { name: "Edit" }))
  field("Marker name", "Renamed phrase")
  await click("Update marker")
  expect(currentPattern().timeline!.markers[0]).toEqual({ ...marker, name: "Renamed phrase" })
  const edits = send.mock.calls.map(([command]) => command).filter((command) => command.type === "editPatternTimeline")
  expect(edits).toHaveLength(4)
  expect(edits[0]).toMatchObject({ expected: { meters: [], markers: [] }, expectedSignature: null,
    edit: { type: "setSignature", signature: { numerator: 3, denominator: 4 } } })
  expect(edits[1]).toMatchObject({ expectedSignature: { numerator: 3, denominator: 4 }, edit: { type: "addMeter", tick: 4001 } })
  expect(undoSteps()).toBe(before + 4)
  expect(notesOf("Lead")).toEqual(originalNotes)
  expect(project().playlist.timeline).toEqual(song)
  const saved = structuredClone(currentPattern())
  await click("Close")
  await act(async () => { await undo(); await settle() })
  expect(currentPattern().timeline!.markers[0].name).toBe("Local phrase")
  await act(async () => { await redo(); await settle() })
  expect(currentPattern()).toEqual(saved)
  await roll.backend.projectSave("/projects/pattern-timeline-qa.windfall")
  await act(async () => { await undo(); await roll.backend.projectOpen("/projects/pattern-timeline-qa.windfall"); await settle() })
  expect(currentPattern()).toEqual(saved)
})

it("mounted pattern editor refuses a raced captured map and invalid fields, then replacement/cancel closes safely", async () => {
  await open("pianoRoll.patternTimeline")
  const before = useProjectStore.getState()
  field("Beats", "0")
  await click("Set base")
  expect(screen.getByRole("alert")).toHaveTextContent("Use 1–16 beats")
  expect(useProjectStore.getState()).toBe(before)
  field("Beats", "3")
  const real = roll.backend.dispatch.bind(roll.backend)
  const send = vi.spyOn(roll.backend, "dispatch")
  send.mockImplementationOnce(async (captured) => {
    await real({ type: "editPatternTimeline", pattern: currentPattern().id,
      expected: currentPattern().timeline ?? { meters: [], markers: [] },
      expectedSignature: currentPattern().timeSignature ?? null,
      edit: { type: "addMarker", tick: 100, name: "Concurrent marker" } })
    return real(captured)
  })
  await click("Set base")
  expect(screen.getByRole("alert")).toHaveTextContent("edit could not be applied")
  expect(currentPattern().timeSignature).toBeUndefined()
  expect(currentPattern().timeline?.markers.map((marker) => marker.name)).toEqual(["Concurrent marker"])
  expect(undoSteps()).toBe(before.history.cursor + 1)
  const afterRace = useProjectStore.getState()
  await click("Close")
  expect(useProjectStore.getState()).toBe(afterRace)
  await open("pianoRoll.patternTimeline")
  act(() => announceProjectReplaced())
  expect(screen.queryByRole("dialog")).toBeNull()
  expect(useProjectStore.getState()).toBe(afterRace)
})
