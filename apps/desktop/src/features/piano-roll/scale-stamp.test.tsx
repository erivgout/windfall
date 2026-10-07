import { fireEvent, render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { toast } from "sonner"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { NoteInit } from "@/bindings"
import { TooltipProvider } from "@/components/ui/tooltip"
import { runAction } from "@/lib/actions"
import { dispatch, redo, undo } from "@/lib/store/project"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { settle } from "@/test/harness"

import { SessionContext } from "./context"
import { MAX_PATTERN_TICKS } from "./edit-math"
import { CHORD_STAMPS, SCALE_STAMPS } from "./stamps"
import { usePianoRollStore } from "./store"
import {
  at,
  channel,
  currentPattern,
  notesOf,
  startRoll,
  undoSteps,
} from "./test-utils"
import { PianoRollToolbar } from "./toolbar"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let roll: Awaited<ReturnType<typeof startRoll>>
const lead = () => notesOf("Lead")
const chord = CHORD_STAMPS[0]
const scale = SCALE_STAMPS[0]

beforeEach(async () => {
  roll = await startRoll()
  await dispatch({ type: "addChannel", name: "Lead" })
  roll.show("Lead")
})
afterEach(() => roll.stop())

async function add(notes: NoteInit[]) {
  await dispatch({
    type: "addNotes",
    pattern: currentPattern().id,
    channel: channel("Lead").id,
    notes,
  })
}
function snap(enabled = true) {
  usePianoRollStore.getState().setSnapToScale(enabled)
}
function toolbar() {
  return render(
    <SessionContext.Provider value={roll.session}>
      <TooltipProvider>
        <PianoRollToolbar channelId={channel("Lead").id} readoutRef={null} />
      </TooltipProvider>
    </SessionContext.Provider>
  )
}

describe("scale policy through the native WASM piano editor", () => {
  it("leaves existing off-scale notes alone when preferences change or time alone is edited", async () => {
    await add([{ start: 0, key: 61, length: 240, velocity: 0.4, pan: -0.3 }])
    const before = undoSteps()
    usePianoRollStore.getState().setHighlightScale(true)
    snap()
    expect(lead()[0]).toMatchObject({ key: 61, pan: -0.3 })
    expect(undoSteps()).toBe(before)
    await roll.drag(at(100, 61), at(580, 61))
    expect(lead()[0]).toMatchObject({
      start: 480,
      key: 61,
      length: 240,
      velocity: 0.4,
      pan: -0.3,
    })
    await roll.editor.nudge(1, 0)
    expect(lead()[0]).toMatchObject({ start: 720, key: 61 })
  })

  it("snaps draw and paint keys, with Alt bypass and matching previews", async () => {
    snap()
    await roll.click(at(500, 61))
    await roll.click(at(1000, 61, { alt: true }))
    expect(lead().map((note) => [note.start, note.key])).toEqual([
      [480, 60],
      [1000, 61],
    ])
    roll.editor.pointerDown(at(1500, 63), "left")
    roll.editor.pointerMove(at(1740, 64))
    expect(roll.surface.dragOffset.rows).toBe(-2)
    roll.editor.pointerUp(at(1740, 64))
    await settle()
    expect(lead().at(-1)).toMatchObject({ start: 1680, key: 64 })
    usePianoRollStore.getState().setTool("paint")
    await roll.drag(at(2400, 66), [at(2640, 68), at(2880, 70)])
    expect(
      lead()
        .slice(-3)
        .map((note) => note.key)
    ).toEqual([65, 67, 69])
    await roll.drag(at(3360, 66, { alt: true }), at(3600, 68, { alt: true }))
    expect(
      lead()
        .slice(-2)
        .map((note) => note.key)
    ).toEqual([66, 68])
  })

  it("moves selected chords rigidly using the grabbed anchor and directional keyboard nudges", async () => {
    await add([60, 64, 67].map((key) => ({ start: 0, key, length: 240 })))
    snap()
    roll.editor.selectAll()
    await roll.drag(at(100, 64), at(100, 65))
    expect(lead().map((note) => note.key)).toEqual([61, 65, 68])
    await roll.editor.nudge(0, 1)
    expect(lead().map((note) => note.key)).toEqual([62, 66, 69])
    await roll.editor.nudge(0, -1)
    expect(lead().map((note) => note.key)).toEqual([60, 64, 67])
    await roll.drag(at(100, 64), at(100, 65, { alt: true }))
    expect(lead().map((note) => note.key)).toEqual([61, 65, 68])
    await roll.editor.transpose(12)
    expect(lead().map((note) => note.key)).toEqual([72, 76, 79])
  })

  it("snaps pasted groups once while preserving spacing, intervals, velocity and pan", async () => {
    await add([
      { start: 0, key: 61, length: 240, velocity: 0.4, pan: 0.7 },
      { start: 120, key: 64, length: 480, velocity: 0.6, pan: -0.2 },
      { start: 240, key: 68, length: 120 },
    ])
    roll.editor.selectAll()
    roll.editor.copy()
    snap()
    const before = undoSteps()
    await roll.editor.paste({ at: "playhead", tick: 960 })
    expect(roll.editor.selectedNotes()).toMatchObject([
      { start: 960, key: 60, velocity: 0.4, pan: 0.7, length: 240 },
      { start: 1080, key: 63, velocity: 0.6, pan: -0.2, length: 480 },
      { start: 1200, key: 67, length: 120 },
    ])
    expect(undoSteps()).toBe(before + 1)
    await undo()
    expect(lead()).toHaveLength(3)
    await redo()
    expect(lead()).toHaveLength(6)
  })

  it("keeps a group inside MIDI bounds and refuses paste if no snapped anchor fits", async () => {
    await add([0, 127].map((key) => ({ start: 0, key, length: 240 })))
    roll.editor.selectAll()
    roll.editor.copy()
    usePianoRollStore.getState().setScaleRoot(1)
    usePianoRollStore.getState().setScaleId("whole-tone")
    snap()
    const before = undoSteps()
    await roll.editor.nudge(0, 1)
    await roll.editor.paste({ at: "playhead", tick: 960 })
    expect(lead().map((note) => note.key)).toEqual([0, 127])
    expect(undoSteps()).toBe(before)
    expect(toast.error).toHaveBeenCalled()
  })
})

describe("one-click stamps through native WASM transactions", () => {
  it("can stamp after Strict Mode cleans up and reattaches a session", async () => {
    roll.editor.dispose()
    roll.editor.attach(roll.surface)
    roll.editor.armStamp(chord)
    await roll.click(at(500, 60))
    expect(lead().map((note) => note.key)).toEqual([60, 64, 67])
  })

  it("previews without editing, then places a whole chord, selects it and extends in one undo step", async () => {
    usePianoRollStore.getState().rememberNote(360, 0.35)
    snap()
    const before = undoSteps()
    roll.editor.armStamp(chord)
    roll.editor.pointerHover(at(4100, 61))
    expect(roll.editor.notes.map((note) => note.key)).toEqual([60, 64, 67])
    expect(lead()).toEqual([])
    expect(undoSteps()).toBe(before)
    await roll.click(at(4100, 61))
    expect(lead()).toMatchObject(
      [60, 64, 67].map((key) => ({
        start: 4080,
        key,
        length: 360,
        velocity: 0.35,
        pan: 0,
      }))
    )
    expect(roll.editor.selectionCount).toBe(3)
    expect(roll.editor.stampState).toBeNull()
    expect(currentPattern().lengthSteps).toBe(32)
    expect(undoSteps()).toBe(before + 1)
    await undo()
    expect(lead()).toEqual([])
    expect(currentPattern().lengthSteps).toBe(16)
    await redo()
    expect(lead()).toHaveLength(3)
    expect(currentPattern().lengthSteps).toBe(32)
    await roll.backend.projectSave("/projects/stamps.windfall")
    await roll.backend.projectNew()
    await roll.backend.projectOpen("/projects/stamps.windfall")
    await settle()
    expect(lead()).toHaveLength(3)
    expect(lead()[0]).toMatchObject({ length: 360, velocity: 0.35, pan: 0 })
  })

  it("places a scale using current note lengths as rhythm, then restores the draw tool", async () => {
    usePianoRollStore.getState().rememberNote(120, 0.6)
    roll.editor.armStamp(scale)
    await roll.click(at(500, 60, { alt: true }))
    expect(lead().map((note) => [note.start, note.key])).toEqual(
      [0, 2, 4, 5, 7, 9, 11, 12].map((offset, index) => [
        500 + index * 120,
        60 + offset,
      ])
    )
    expect(roll.editor.selectionCount).toBe(8)
    await roll.click(at(2400, 60))
    expect(lead()).toHaveLength(9)
  })

  it("rejects entire stamps at key and time boundaries and allows a later valid placement", async () => {
    const before = undoSteps()
    roll.editor.armStamp(chord)
    await roll.click(at(0, 125))
    expect(lead()).toEqual([])
    expect(roll.editor.stampState?.error).toContain("MIDI")
    roll.editor.armStamp(scale)
    await roll.click(at(MAX_PATTERN_TICKS - 240, 60))
    expect(lead()).toEqual([])
    expect(undoSteps()).toBe(before)
    expect(roll.editor.stampState?.error).toContain("pattern length")
    await roll.click(at(0, 60))
    expect(lead()).toHaveLength(8)
    expect(undoSteps()).toBe(before + 1)
  })

  it("cancels previews with Escape, right-click and tool changes without changing selection", async () => {
    await add([{ start: 0, key: 50, length: 240 }])
    roll.editor.selectAll()
    const before = undoSteps()
    roll.editor.armStamp(chord)
    roll.editor.pointerHover(at(960, 60))
    await runAction("pianoRoll.deselect")
    expect(roll.editor.stampState).toBeNull()
    expect(roll.editor.selectionCount).toBe(1)
    roll.editor.armStamp(chord)
    await roll.click(at(100, 50), "right")
    expect(lead()).toHaveLength(1)
    expect(roll.editor.stampState).toBeNull()
    roll.editor.armStamp(chord)
    await runAction("pianoRoll.toolPaint")
    expect(roll.editor.stampState).toBeNull()
    expect(undoSteps()).toBe(before)
  })

  it("abandons stale previews after lane edits, lane switches, project replacement and disposal", async () => {
    roll.editor.armStamp(chord)
    roll.editor.pointerHover(at(960, 60))
    await add([{ start: 0, key: 50, length: 240 }])
    expect(roll.editor.stampState).toBeNull()
    roll.editor.armStamp(chord)
    roll.show("Kick")
    expect(roll.editor.stampState).toBeNull()
    roll.show("Lead")
    roll.editor.armStamp(chord)
    announceProjectReplaced()
    expect(roll.editor.stampState).toBeNull()
    roll.editor.armStamp(chord)
    roll.editor.dispose()
    expect(roll.editor.stampState).toBeNull()
    expect(lead()).toHaveLength(1)
  })

  it("cancels a press if the project is replaced before pointer-up", async () => {
    roll.editor.armStamp(chord)
    roll.editor.pointerDown(at(500, 60), "left")
    await roll.backend.projectNew()
    roll.editor.pointerUp(at(500, 60))
    await settle()
    expect(roll.editor.stampState).toBeNull()
    expect(roll.editor.notes.some((note) => note.id < 0)).toBe(false)
    expect(undoSteps()).toBe(0)
  })
})

describe("accessible piano scale and stamp toolbar", () => {
  it("offers independent highlight and pitch snap controls and root/scale choices", async () => {
    toolbar()
    const user = userEvent.setup()
    fireEvent.click(
      screen.getByRole("button", { name: "Scale settings: C Major" })
    )
    await user.click(
      await screen.findByRole("menuitemcheckbox", { name: "Highlight scale" })
    )
    expect(usePianoRollStore.getState().highlightScale).toBe(true)
    expect(usePianoRollStore.getState().snapToScale).toBe(false)
    // Checkbox items leave this settings menu open.
    await user.click(
      screen.getByRole("menuitemcheckbox", { name: "Snap pitches to scale" })
    )
    expect(usePianoRollStore.getState().snapToScale).toBe(true)
    fireEvent.click(screen.getByRole("menuitem", { name: "Root key" }))
    fireEvent.click(await screen.findByRole("menuitemradio", { name: "D" }))
    expect(usePianoRollStore.getState().scaleRoot).toBe(2)
    expect(lead()).toEqual([])
  })

  it("arms a menu chord for grid placement and exposes an accessible cancel control", async () => {
    toolbar()
    const user = userEvent.setup()
    fireEvent.click(
      screen.getByRole("button", { name: "Choose chord or scale stamp" })
    )
    fireEvent.click(await screen.findByRole("menuitem", { name: "Chords" }))
    fireEvent.click(
      await screen.findByRole("menuitem", { name: "Major triad" })
    )
    await screen.findByRole("button", { name: "Cancel stamp placement" })
    expect(roll.editor.stampState?.stamp).toBe(chord)
    expect(
      screen.getByRole("status", { name: "Stamp placement" })
    ).toHaveTextContent("Major triad")
    expect(lead()).toEqual([])
    await user.click(
      screen.getByRole("button", { name: "Cancel stamp placement" })
    )
    expect(roll.editor.stampState).toBeNull()
  })
})
