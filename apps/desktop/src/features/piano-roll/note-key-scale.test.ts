import { beforeEach, describe, expect, it, vi } from "vitest"

import { DEFAULT_KEY } from "@/lib/units"

import {
  noteKeyScaleUpdates,
  scaledNoteKey,
  setSelectedNoteKeyScale,
} from "./note-key-scale"

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

describe("note key scaling", () => {
  it.each([
    { key: DEFAULT_KEY, half: DEFAULT_KEY, double: DEFAULT_KEY },
    { key: 72, half: 66, double: 84 },
    { key: 48, half: 54, double: 36 },
    { key: 61, half: DEFAULT_KEY, double: 62 },
    { key: 59, half: DEFAULT_KEY, double: 58 },
    { key: 0, half: 30, double: 0 },
    { key: 127, half: 93, double: 127 },
    { key: 96, half: 78, double: 127 },
    { key: 24, half: 42, double: 0 },
  ])("scales key $key from C5", ({ key, half, double }) => {
    const notes = [{ id: 1, key }]
    expect(scaledNoteKey(key, "half")).toBe(half)
    expect(scaledNoteKey(key, "double")).toBe(double)
    expect(noteKeyScaleUpdates(notes, "half")).toEqual(
      half === key ? [] : [{ id: 1, key: half }]
    )
    expect(noteKeyScaleUpdates(notes, "double")).toEqual(
      double === key ? [] : [{ id: 1, key: double }]
    )
  })

  it.each(["half", "double"] as const)(
    "returns no updates for an empty or unchanged selection for %s",
    (factor) => {
      expect(noteKeyScaleUpdates([], factor)).toEqual([])
      expect(
        noteKeyScaleUpdates(
          [
            { id: 1, key: DEFAULT_KEY },
            { id: 2, key: DEFAULT_KEY },
          ],
          factor
        )
      ).toEqual([])
    }
  )

  it("omits every note when doubling C5 and MIDI boundaries", () => {
    expect(
      noteKeyScaleUpdates(
        [
          { id: 1, key: 0 },
          { id: 2, key: DEFAULT_KEY },
          { id: 3, key: 127 },
        ],
        "double"
      )
    ).toEqual([])
  })

  it.each([
    { factor: "half", key: 66 },
    { factor: "double", key: 84 },
  ] as const)(
    "omits an unchanged note in a mixed selection for $factor",
    ({ factor, key }) => {
      expect(
        noteKeyScaleUpdates(
          [
            { id: 9, key: 72 },
            { id: 3, key: DEFAULT_KEY },
          ],
          factor
        )
      ).toEqual([{ id: 9, key }])
    }
  )

  it("returns only ids and keys without changing the input notes", () => {
    const note = Object.freeze({
      id: 9,
      key: 48,
      start: 240,
      length: 120,
      velocity: 0.8,
      expression: Object.freeze({ glideTicks: 480 }),
    })
    const notes = Object.freeze([note])
    const updates = noteKeyScaleUpdates(notes, "double")
    expect(updates).toEqual([{ id: 9, key: 36 }])
    expect(updates[0]).not.toBe(note)
    expect(notes).toEqual([
      {
        id: 9,
        key: 48,
        start: 240,
        length: 120,
        velocity: 0.8,
        expression: { glideTicks: 480 },
      },
    ])
  })
})

describe("selected note key scaling", () => {
  it.each([
    { factor: "half", key: 66 },
    { factor: "double", key: 84 },
  ] as const)(
    "dispatches only selected keys for $factor",
    async ({ factor, key }) => {
      const notes = [
        {
          id: 9,
          key: 72,
          start: 240,
          length: 120,
          velocity: 0.8,
          expression: { glideTicks: 480 },
        },
        { id: 3, key: DEFAULT_KEY, start: 0, length: 10 },
      ]
      const selectedNotes = vi.fn(() => notes)
      mocks.currentSession.mockReturnValue({
        editor: {
          context: { pattern: { id: 12 }, channel: 7 },
          busy: false,
          notes: [...notes, { id: 99, key: 48, start: 100, length: 40 }],
          selectedNotes,
        },
      })

      await setSelectedNoteKeyScale(factor)

      expect(selectedNotes).toHaveBeenCalledOnce()
      expect(mocks.dispatch).toHaveBeenCalledExactlyOnceWith({
        type: "updateCapturedNotes",
        pattern: 12,
        channel: 7,
        expected: notes,
        updates: [{ id: 9, patch: { key } }],
      })
      expect(mocks.dispatch.mock.calls[0][0].expected).toBe(notes)
      expect(notes[0]).toEqual({
        id: 9,
        key: 72,
        start: 240,
        length: 120,
        velocity: 0.8,
        expression: { glideTicks: 480 },
      })
    }
  )

  it("does not dispatch without a session", async () => {
    await setSelectedNoteKeyScale("half")
    await setSelectedNoteKeyScale("double")
    expect(mocks.dispatch).not.toHaveBeenCalled()
  })

  it.each([
    {
      name: "missing context",
      context: null,
      busy: false,
      notes: [{ id: 1, key: 72 }],
    },
    {
      name: "busy editor",
      context: { pattern: { id: 12 }, channel: 7 },
      busy: true,
      notes: [{ id: 1, key: 72 }],
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
      notes: [{ id: 1, key: DEFAULT_KEY }],
    },
  ])("does not dispatch for $name", async ({ context, busy, notes }) => {
    mocks.currentSession.mockReturnValue({
      editor: { context, busy, selectedNotes: () => notes },
    })
    await setSelectedNoteKeyScale("half")
    await setSelectedNoteKeyScale("double")
    expect(mocks.dispatch).not.toHaveBeenCalled()
  })

  it("does not dispatch when doubled notes all stay at MIDI boundaries", async () => {
    mocks.currentSession.mockReturnValue({
      editor: {
        context: { pattern: { id: 12 }, channel: 7 },
        busy: false,
        selectedNotes: () => [
          { id: 1, key: 0 },
          { id: 2, key: 127 },
        ],
      },
    })
    await setSelectedNoteKeyScale("double")
    expect(mocks.dispatch).not.toHaveBeenCalled()
  })
})
