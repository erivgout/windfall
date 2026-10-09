import { beforeEach, describe, expect, it, vi } from "vitest"

import { MAX_PATTERN_TICKS } from "@/lib/units"

import {
  noteFromEndScaleUpdates,
  scaledNoteFromEnd,
  setSelectedNoteFromEndScale,
} from "./note-from-end-scale"

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

describe("note scaling from the end", () => {
  it.each([
    { start: 0, length: 40, half: { start: 20, length: 20 }, double: null },
    {
      start: 100,
      length: 40,
      half: { start: 120, length: 20 },
      double: { start: 60, length: 80 },
    },
    { start: 10, length: 1, half: null, double: { start: 9, length: 2 } },
    {
      start: 50,
      length: 5,
      half: { start: 53, length: 2 },
      double: { start: 45, length: 10 },
    },
    {
      start: 8,
      length: 10,
      half: { start: 13, length: 5 },
      double: { start: 0, length: 18 },
    },
    { start: 0, length: 1, half: null, double: null },
    {
      start: 3,
      length: 7,
      half: { start: 7, length: 3 },
      double: { start: 0, length: 10 },
    },
    {
      start: MAX_PATTERN_TICKS - 40,
      length: 40,
      half: { start: MAX_PATTERN_TICKS - 20, length: 20 },
      double: { start: MAX_PATTERN_TICKS - 80, length: 80 },
    },
    { start: MAX_PATTERN_TICKS - 39, length: 40, half: null, double: null },
  ])(
    "scales start $start with length $length while keeping the end",
    ({ start, length, half, double }) => {
      const notes = [{ id: 1, start, length }]
      for (const [factor, expected] of [
        ["half", half],
        ["double", double],
      ] as const) {
        const next = scaledNoteFromEnd(start, length, factor)
        expect(next).toEqual(expected)
        expect(noteFromEndScaleUpdates(notes, factor)).toEqual(
          expected === null ? [] : [{ id: 1, ...expected }]
        )
        if (next) {
          expect(next.start + next.length).toBe(start + length)
          expect(next.start).toBeGreaterThanOrEqual(0)
          expect(next.length).toBeGreaterThanOrEqual(1)
        }
      }
    }
  )

  it.each(["half", "double"] as const)(
    "returns an empty update list for an empty selection with %s",
    (factor) => {
      expect(noteFromEndScaleUpdates([], factor)).toEqual([])
    }
  )

  it("keeps input order while omitting unchanged and out-of-range notes", () => {
    expect(
      noteFromEndScaleUpdates(
        [
          { id: 9, start: 100, length: 40 },
          { id: 3, start: 0, length: 40 },
          { id: 5, start: MAX_PATTERN_TICKS, length: 1 },
          { id: 7, start: 8, length: 10 },
        ],
        "double"
      )
    ).toEqual([
      { id: 9, start: 60, length: 80 },
      { id: 7, start: 0, length: 18 },
    ])
  })

  it("returns only ids, starts and lengths without changing the input notes", () => {
    const note = Object.freeze({
      id: 9,
      start: 100,
      length: 40,
      key: 60,
      velocity: 0.8,
      expression: Object.freeze({ glideTicks: 480 }),
    })
    const notes = Object.freeze([note])
    const updates = noteFromEndScaleUpdates(notes, "double")
    expect(updates).toEqual([{ id: 9, start: 60, length: 80 }])
    expect(updates[0]).not.toBe(note)
    expect(notes).toEqual([
      {
        id: 9,
        start: 100,
        length: 40,
        key: 60,
        velocity: 0.8,
        expression: { glideTicks: 480 },
      },
    ])
  })
})

describe("selected note scaling from the end", () => {
  it.each([
    { factor: "half", start: 120, length: 20 },
    { factor: "double", start: 60, length: 80 },
  ] as const)(
    "dispatches selected starts and lengths for $factor",
    async ({ factor, start, length }) => {
      const note = Object.freeze({
        id: 9,
        start: 100,
        length: 40,
        key: 60,
        velocity: 0.8,
        expression: Object.freeze({ glideTicks: 480 }),
      })
      const notes = Object.freeze([note, { id: 3, start: 0, length: 1 }])
      const selectedNotes = vi.fn(() => notes)
      mocks.currentSession.mockReturnValue({
        editor: {
          context: { pattern: { id: 12 }, channel: 7 },
          busy: false,
          notes: [...notes, { id: 99, start: 50, length: 5 }],
          selectedNotes,
        },
      })

      await setSelectedNoteFromEndScale(factor)

      expect(selectedNotes).toHaveBeenCalledOnce()
      expect(mocks.dispatch).toHaveBeenCalledExactlyOnceWith({
        type: "updateCapturedNotes",
        pattern: 12,
        channel: 7,
        expected: notes,
        updates: [{ id: 9, patch: { start, length } }],
      })
      expect(mocks.dispatch.mock.calls[0][0].expected).toBe(notes)
      expect(note).toEqual({
        id: 9,
        start: 100,
        length: 40,
        key: 60,
        velocity: 0.8,
        expression: { glideTicks: 480 },
      })
    }
  )

  it("does not dispatch without a session", async () => {
    await setSelectedNoteFromEndScale("half")
    await setSelectedNoteFromEndScale("double")
    expect(mocks.dispatch).not.toHaveBeenCalled()
  })

  it.each([
    {
      name: "missing context",
      context: null,
      busy: false,
      notes: [{ id: 1, start: 100, length: 40 }],
    },
    {
      name: "busy editor",
      context: { pattern: { id: 12 }, channel: 7 },
      busy: true,
      notes: [{ id: 1, start: 100, length: 40 }],
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
      notes: [{ id: 1, start: 0, length: 1 }],
    },
    {
      name: "out-of-range selection",
      context: { pattern: { id: 12 }, channel: 7 },
      busy: false,
      notes: [{ id: 1, start: MAX_PATTERN_TICKS - 39, length: 40 }],
    },
  ])("does not dispatch for $name", async ({ context, busy, notes }) => {
    mocks.currentSession.mockReturnValue({
      editor: { context, busy, selectedNotes: () => notes },
    })
    await setSelectedNoteFromEndScale("half")
    await setSelectedNoteFromEndScale("double")
    expect(mocks.dispatch).not.toHaveBeenCalled()
  })
})
