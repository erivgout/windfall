import { describe, expect, it } from "vitest"

import type { ProjectPatch } from "@/bindings"
import { SimDocument } from "@/lib/ipc/sim/document"
import { demoProject } from "@/lib/ipc/sim/project"

import { applyPatch, shareStructure, type DocumentState } from "./patch"

/** A document and a UI copy of it that only ever sees JSON, like over IPC. */
function setup() {
  const document = new SimDocument(demoProject())
  const overTheWire = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T
  const state: DocumentState = overTheWire(document.snapshot(null))
  const edit = (...args: Parameters<SimDocument["dispatch"]>): ProjectPatch => {
    const applied = document.dispatch(...args)
    return overTheWire(document.patch(applied.touched))
  }
  return { document, state, edit, overTheWire }
}

describe("applyPatch", () => {
  it("applies the next revision and ends up equal to the document", () => {
    const { document, state, edit } = setup()
    const channel = state.project.channels[0].id
    const pattern = state.project.patterns[0].id

    let current = state
    for (const patch of [
      edit({ type: "updateSettings", patch: { tempoBpm: 99 } }),
      edit({ type: "toggleStep", pattern, channel, step: 1 }),
      edit({ type: "addPattern" }),
      edit({ type: "removeChannel", id: channel }),
    ]) {
      const outcome = applyPatch(current, patch)
      expect(outcome.status).toBe("applied")
      current = outcome.state
    }

    expect(current.revision).toBe(4)
    expect(current.dirty).toBe(true)
    expect(current.history.entries).toHaveLength(4)
    expect({ ...current.project, nextId: 0 }).toEqual({
      ...document.project(),
      nextId: 0,
    })
  })

  it("ignores a patch it has already applied", () => {
    const { state, edit } = setup()
    const patch = edit({ type: "addPattern" })
    const once = applyPatch(state, patch)
    const twice = applyPatch(once.state, patch)
    expect(twice.status).toBe("ignored")
    expect(twice.state).toBe(once.state)
  })

  it("reports a gap and changes nothing when a patch was missed", () => {
    const { state, edit } = setup()
    edit({ type: "addPattern" })
    const second = edit({ type: "addPattern" })
    const outcome = applyPatch(state, second)
    expect(outcome.status).toBe("gap")
    expect(outcome.state).toBe(state)
  })

  it("keeps the reference of everything a patch did not change", () => {
    const { state, edit } = setup()
    const [first, second] = state.project.channels
    const patch = edit({
      type: "updateChannel",
      id: first.id,
      patch: { volume: 0.5 },
    })
    const { state: next } = applyPatch(state, patch)

    expect(next.project.channels[0]).not.toBe(first)
    expect(next.project.channels[0].volume).toBe(0.5)
    expect(next.project.channels[1]).toBe(second)
    expect(next.project.mixer).toBe(state.project.mixer)
    expect(next.project.patterns).toBe(state.project.patterns)
    expect(next.project.settings).toBe(state.project.settings)
  })

  it("keeps untouched lanes when one lane of a pattern changes", () => {
    const { state, edit } = setup()
    const pattern = state.project.patterns[0]
    const patch = edit({
      type: "toggleStep",
      pattern: pattern.id,
      channel: pattern.lanes[0].channel,
      step: 1,
    })
    const { state: next } = applyPatch(state, patch)

    expect(next.project.patterns[0].lanes[0]).not.toBe(pattern.lanes[0])
    expect(next.project.patterns[0].lanes[0].notes).toHaveLength(5)
    expect(next.project.patterns[0].lanes[1]).toBe(pattern.lanes[1])
    expect(next.project.patterns[0].lanes[0].notes[0]).toBe(
      pattern.lanes[0].notes[0]
    )
  })

  it("follows the pattern order and drops patterns missing from it", () => {
    const { state, edit } = setup()
    const first = state.project.patterns[0]
    const added = applyPatch(state, edit({ type: "addPattern" })).state
    expect(added.project.patterns).toHaveLength(2)
    expect(added.project.patterns[0]).toBe(first)

    const second = added.project.patterns[1]
    const moved = applyPatch(
      added,
      edit({ type: "movePattern", id: second.id, index: 0 })
    ).state
    expect(moved.project.patterns.map((item) => item.id)).toEqual([
      second.id,
      first.id,
    ])
    expect(moved.project.patterns[1]).toBe(first)

    const removed = applyPatch(
      moved,
      edit({ type: "removePattern", id: second.id })
    ).state
    expect(removed.project.patterns).toEqual([first])
  })
})

describe("shareStructure", () => {
  it("returns the previous value when nothing differs", () => {
    const prev = { list: [{ id: 1, name: "a" }], count: 1 }
    const next = { list: [{ id: 1, name: "a" }], count: 1 }
    expect(shareStructure(prev, next)).toBe(prev)
  })

  it("matches list items by id, so a reorder keeps every item", () => {
    const a = { id: 1, name: "a" }
    const b = { id: 2, name: "b" }
    const shared = shareStructure(
      [a, b],
      [
        { id: 2, name: "b" },
        { id: 1, name: "a" },
      ]
    )
    expect(shared[0]).toBe(b)
    expect(shared[1]).toBe(a)
  })

  it("notices added and removed keys", () => {
    const prev: Record<string, number> = { a: 1 }
    expect(shareStructure(prev, { a: 1, b: 2 })).toEqual({ a: 1, b: 2 })
    expect(shareStructure({ a: 1, b: 2 }, prev)).toEqual({ a: 1 })
  })
})
