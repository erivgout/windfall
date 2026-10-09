import { beforeEach, describe, expect, it, vi } from "vitest"

import { MAX_PATTERN_TICKS } from "@/lib/units"

import {
  noteStartScaleUpdates,
  scaledNoteStart,
  setSelectedNoteStartScale,
} from "./note-start-scale"

const mocks = vi.hoisted(() => ({
  currentSession: vi.fn(),
  dispatch: vi.fn(),
}))

vi.mock("./session", () => ({ currentSession: mocks.currentSession }))
vi.mock("@/lib/store/project", () => ({ dispatch: mocks.dispatch }))

beforeEach(() => {
  mocks.currentSession.mockReset()
  mocks.dispatch.mockReset()
})

describe("note start scaling", () => {
  it.each([
    { start: 0, length: 10, half: 0, double: 0 },
    { start: 1, length: 10, half: 0, double: 2 },
    { start: 5, length: 10, half: 2, double: 10 },
    { start: 100, length: 40, half: 50, double: 200 },
    { start: 240, length: 240, half: 120, double: 480 },
    { start: 0, length: MAX_PATTERN_TICKS, half: 0, double: 0 },
    {
      start: MAX_PATTERN_TICKS - 50,
      length: 40,
      half: Math.floor((MAX_PATTERN_TICKS - 50) / 2),
      double: MAX_PATTERN_TICKS - 40,
    },
    {
      start: MAX_PATTERN_TICKS - 40,
      length: 40,
      half: Math.floor((MAX_PATTERN_TICKS - 40) / 2),
      double: MAX_PATTERN_TICKS - 40,
    },
  ])(
    "scales start $start with length $length",
    ({ start, length, half, double }) => {
      const notes = [{ id: 1, start, length }]
      expect(scaledNoteStart(start, length, "half")).toBe(half)
      expect(scaledNoteStart(start, length, "double")).toBe(double)
      expect(noteStartScaleUpdates(notes, "half")).toEqual(
        half === start ? [] : [{ id: 1, start: half }]
      )
      expect(noteStartScaleUpdates(notes, "double")).toEqual(
        double === start ? [] : [{ id: 1, start: double }]
      )
    }
  )

  it("leaves the start unchanged when the length exceeds the pattern limit", () => {
    const notes = [{ id: 1, start: 5, length: MAX_PATTERN_TICKS + 1 }]
    expect(scaledNoteStart(5, MAX_PATTERN_TICKS + 1, "double")).toBe(5)
    expect(scaledNoteStart(0, MAX_PATTERN_TICKS + 1, "double")).toBe(0)
    expect(noteStartScaleUpdates(notes, "double")).toEqual([])
  })

  it("clamps a halved start to tick zero", () => {
    expect(scaledNoteStart(-1, 10, "half")).toBe(0)
  })

  it.each(["half", "double"] as const)(
    "returns no updates for an empty or unchanged selection when scaling by %s",
    (factor) => {
      expect(noteStartScaleUpdates([], factor)).toEqual([])
      expect(
        noteStartScaleUpdates(
          [
            { id: 1, start: 0, length: 10 },
            { id: 2, start: 0, length: MAX_PATTERN_TICKS },
          ],
          factor
        )
      ).toEqual([])
    }
  )

  it("keeps input order while omitting unchanged starts", () => {
    expect(
      noteStartScaleUpdates(
        [
          { id: 9, start: 100, length: 40 },
          { id: 3, start: 0, length: 10 },
          { id: 7, start: 5, length: 10 },
        ],
        "half"
      )
    ).toEqual([
      { id: 9, start: 50 },
      { id: 7, start: 2 },
    ])
  })

  it("returns only ids and starts without changing the input notes", () => {
    const note = Object.freeze({
      id: 9,
      start: 240,
      length: 240,
      key: 60,
      velocity: 0.8,
      expression: Object.freeze({ glideTicks: 480 }),
    })
    const notes = Object.freeze([note])
    const updates = noteStartScaleUpdates(notes, "double")
    expect(updates).toEqual([{ id: 9, start: 480 }])
    expect(updates[0]).not.toBe(note)
    expect(notes).toEqual([
      {
        id: 9,
        start: 240,
        length: 240,
        key: 60,
        velocity: 0.8,
        expression: { glideTicks: 480 },
      },
    ])
  })
})

describe("selected note start scaling", () => {
  it.each([
    { factor: "half", start: 120 },
    { factor: "double", start: 480 },
  ] as const)(
    "dispatches only selected starts for $factor",
    async ({ factor, start }) => {
      const notes = [
        {
          id: 9,
          start: 240,
          length: 240,
          velocity: 0.8,
          expression: { glideTicks: 480 },
        },
        { id: 3, start: 0, length: 10 },
      ]
      const selectedNotes = vi.fn(() => notes)
      mocks.currentSession.mockReturnValue({
        editor: {
          context: { pattern: { id: 12 }, channel: 7 },
          busy: false,
          notes: [...notes, { id: 99, start: 100, length: 40 }],
          selectedNotes,
        },
      })

      await setSelectedNoteStartScale(factor)

      expect(selectedNotes).toHaveBeenCalledOnce()
      expect(mocks.dispatch).toHaveBeenCalledExactlyOnceWith({
        type: "updateCapturedNotes",
        pattern: 12,
        channel: 7,
        expected: notes,
        updates: [{ id: 9, patch: { start } }],
      })
      expect(mocks.dispatch.mock.calls[0][0].expected).toBe(notes)
      expect(notes[0]).toEqual({
        id: 9,
        start: 240,
        length: 240,
        velocity: 0.8,
        expression: { glideTicks: 480 },
      })
    }
  )

  it("does not dispatch without a session", async () => {
    await setSelectedNoteStartScale("double")
    expect(mocks.dispatch).not.toHaveBeenCalled()
  })

  it.each([
    {
      name: "missing context",
      context: null,
      busy: false,
      notes: [{ id: 1, start: 5, length: 10 }],
    },
    {
      name: "busy editor",
      context: { pattern: { id: 12 }, channel: 7 },
      busy: true,
      notes: [{ id: 1, start: 5, length: 10 }],
    },
    {
      name: "empty selection",
      context: { pattern: { id: 12 }, channel: 7 },
      busy: false,
      notes: [],
    },
    {
      name: "unchanged selection",
      context: { pattern: { id: 12 }, channel: 7 },
      busy: false,
      notes: [
        { id: 1, start: 0, length: 10 },
        { id: 2, start: 0, length: MAX_PATTERN_TICKS },
      ],
    },
  ])("does not dispatch for $name", async ({ context, busy, notes }) => {
    mocks.currentSession.mockReturnValue({
      editor: { context, busy, selectedNotes: () => notes },
    })
    await setSelectedNoteStartScale("half")
    await setSelectedNoteStartScale("double")
    expect(mocks.dispatch).not.toHaveBeenCalled()
  })
})
