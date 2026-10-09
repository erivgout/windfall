import { act, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { SimDocument } from "@/lib/ipc/sim/document"
import { dispatch, redo, undo } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { settle } from "@/test/harness"

import { type GraphProperty, GRAPH_LABELS } from "./graph-values"
import { useRackStore } from "./rack-store"
import { StepGraph } from "./step-graph"
import { channel, history, notesOf, project, startRack } from "./test-utils"

let app: Awaited<ReturnType<typeof startRack>>
let pattern: number
beforeEach(async () => {
  app = await startRack()
  pattern = project().patterns[0].id
  const id = channel("Kick").id
  useUiStore.getState().selectChannel(id)
  useRackStore.getState().setGraphOpen(true)
  await dispatch({
    type: "removeNotes",
    pattern,
    channel: id,
    notes: notesOf("Kick").map((note) => note.id),
  })
  await dispatch({
    type: "addNotes",
    pattern,
    channel: id,
    notes: [
      {
        start: 0,
        length: 120,
        key: 60,
        velocity: 0.7,
        pan: 0.2,
        expression: {
          release: 0.3,
          finePitchCents: 37,
          modulationX: 0.2,
          modulationY: 0.8,
          articulation: "portamento",
          glideTicks: 333,
          colorGroup: 7,
        },
      },
      { start: 0, length: 180, key: 64, velocity: 0.6, pan: -0.2 },
      { start: 250, length: 300, key: 67, velocity: 0.5, pan: 0.1 },
      { start: 720, length: 240, key: 72, velocity: 0.4, pan: 0 },
      { start: 4800, length: 240, key: 80, velocity: 0.9, pan: 0 },
    ],
  })
})
afterEach(() => {
  app.stop()
  vi.restoreAllMocks()
})

function mount(property: GraphProperty) {
  useRackStore.getState().setGraphProperty(property)
  render(<StepGraph pattern={pattern} lengthSteps={16} groupSize={4} />)
  return screen.getByRole("group", {
    name: `Kick ${GRAPH_LABELS[property]} step graph`,
  })
}
const flush = () => act(settle)
function pointer(surface: Element, type: string, x: number, y: number) {
  const event = new MouseEvent(type, {
    bubbles: true,
    cancelable: true,
    button: 0,
    clientX: x,
    clientY: y,
  })
  Object.defineProperty(event, "pointerId", { value: 1 })
  fireEvent(surface, event)
}

describe("step graph real-document acceptance", () => {
  it.each<[GraphProperty, number, number]>([
    ["velocity", 0, 1],
    ["pan", -1, 1],
    ["key", 0, 127],
    ["release", 0, 1],
    ["finePitchCents", -1200, 1200],
    ["modulationX", 0, 1],
    ["modulationY", 0, 1],
    ["length", 1, 960],
    ["shift", 0, 239],
  ])(
    "edits %s at both bounds with one-step history and unrelated properties intact",
    async (property, min, max) => {
      mount(property)
      const sliders = screen.getAllByRole("slider")
      expect(sliders).toHaveLength(4) // chord + off-grid starts; out-of-pattern note is preserved.
      const original = structuredClone(notesOf("Kick"))
      const cursor = history().cursor
      fireEvent.keyDown(sliders[0], { key: "End" })
      await flush()
      expect(sliders[0]).toHaveAttribute("aria-valuenow", String(max))
      expect(history().cursor).toBe(cursor + 1)
      expect(
        notesOf("Kick").filter((note) => note.id !== original[0].id)
      ).toEqual(original.slice(1))
      const changed = structuredClone(notesOf("Kick"))
      const field = [
        "release",
        "finePitchCents",
        "modulationX",
        "modulationY",
      ].includes(property)
        ? "expression"
        : property === "shift"
          ? "start"
          : property
      for (const [key, value] of Object.entries(original[0])) {
        if (key !== field)
          expect(
            changed.find((note) => note.id === original[0].id)![
              key as keyof (typeof changed)[0]
            ]
          ).toEqual(value)
      }
      if (field === "expression") {
        const edited = changed.find(
          (note) => note.id === original[0].id
        )!.expression!
        for (const [key, value] of Object.entries(original[0].expression!)) {
          if (key !== property)
            expect(edited[key as keyof typeof edited]).toEqual(value)
        }
      }
      const doc = SimDocument.create(
        (await app.backend.documentSnapshot()).project
      )
      const reopened = SimDocument.open(doc.fileText())
      expect(reopened.project().patterns).toEqual(
        (await app.backend.documentSnapshot()).project.patterns
      )
      doc.dispose()
      reopened.dispose()
      await act(async () => {
        await undo()
      })
      expect(notesOf("Kick")).toEqual(original)
      await act(async () => {
        await redo()
      })
      expect(notesOf("Kick")).toEqual(changed)
      fireEvent.keyDown(sliders[0], { key: "Home" })
      await flush()
      expect(sliders[0]).toHaveAttribute("aria-valuenow", String(min))
    }
  )

  it("interpolates a fast stroke across chords and off-grid notes, creates no empty-step notes, and undoes once", async () => {
    const surface = mount("velocity")
    vi.spyOn(surface, "getBoundingClientRect").mockReturnValue({
      left: 0,
      top: 0,
      width: 160,
      height: 112,
      right: 160,
      bottom: 112,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    })
    const original = structuredClone(notesOf("Kick"))
    const cursor = history().cursor
    pointer(surface, "pointerdown", 5, 0)
    pointer(surface, "pointermove", 35, 112)
    expect(notesOf("Kick")).toEqual(original)
    pointer(surface, "pointerup", 35, 112)
    await flush()
    expect(history().cursor).toBe(cursor + 1)
    expect(notesOf("Kick").map((note) => note.velocity)).toEqual([
      1, 1, 0.67, 0, 0.9,
    ])
    expect(notesOf("Kick").map((note) => note.id)).toEqual(
      original.map((note) => note.id)
    )
    await act(async () => {
      await undo()
    })
    expect(notesOf("Kick")).toEqual(original)
  })

  it("cancels a preview on pointer cancellation and changed graph controls without committing", async () => {
    const surface = mount("velocity")
    vi.spyOn(surface, "getBoundingClientRect").mockReturnValue({
      left: 0,
      top: 0,
      width: 160,
      height: 112,
      right: 160,
      bottom: 112,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    })
    const original = structuredClone(project())
    const cursor = history().cursor
    pointer(surface, "pointerdown", 5, 0)
    pointer(surface, "pointercancel", 5, 0)
    pointer(surface, "pointerup", 5, 0)
    await flush()
    expect(project()).toEqual(original)
    pointer(surface, "pointerdown", 5, 0)
    await act(async () => {
      useRackStore.getState().setGraphProperty("pan")
    })
    pointer(surface, "pointerup", 5, 0)
    await flush()
    expect(project()).toEqual(original)
    expect(history().cursor).toBe(cursor)
  })

  it("abandons painting on lane changes and refuses stale captured notes atomically", async () => {
    const surface = mount("velocity")
    vi.spyOn(surface, "getBoundingClientRect").mockReturnValue({
      left: 0,
      top: 0,
      width: 160,
      height: 112,
      right: 160,
      bottom: 112,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    })
    const captured = structuredClone(notesOf("Kick"))
    pointer(surface, "pointerdown", 5, 0)
    await act(async () => {
      await dispatch({
        type: "updateNotes",
        pattern,
        channel: channel("Kick").id,
        updates: [{ id: captured[0].id, patch: { pan: -0.5 } }],
      })
    })
    const current = structuredClone(project())
    const cursor = history().cursor
    pointer(surface, "pointerup", 5, 0)
    await flush()
    expect(project()).toEqual(current)
    await expect(
      app.backend.dispatch({
        type: "updateCapturedNotes",
        pattern,
        channel: channel("Kick").id,
        expected: captured,
        updates: [{ id: captured[0].id, patch: { velocity: 1 } }],
      })
    ).rejects.toThrow()
    expect(project()).toEqual(current)
    expect(history().cursor).toBe(cursor)
  })
})
