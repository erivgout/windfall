import { describe, expect, it } from "vitest"

import type { Command } from "@/bindings"

import { SimDocument } from "./document"
import { demoProject, newProject } from "./project"

const tempo = (tempoBpm: number): Command => ({
  type: "updateSettings",
  patch: { tempoBpm },
})

describe("SimDocument", () => {
  it("undoes and redoes back to exactly the same project", () => {
    const document = new SimDocument(demoProject())
    const before = document.project()
    const channel = before.channels[0].id

    document.dispatch({ type: "removeChannel", id: channel })
    const after = document.project()
    expect(after.channels).toHaveLength(3)

    expect(document.undo()).toMatchObject({ channels: true })
    expect(document.project()).toEqual(before)
    document.redo()
    expect(document.project()).toEqual(after)
  })

  it("returns null when there is nothing to undo or redo", () => {
    const document = new SimDocument(newProject())
    expect(document.undo()).toBeNull()
    expect(document.redo()).toBeNull()
  })

  it("collapses dispatches that share a gesture into one undo step", () => {
    const document = new SimDocument(newProject())
    document.dispatch(tempo(100), 7)
    document.dispatch(tempo(110), 7)
    document.dispatch(tempo(120), 7)

    expect(document.history()).toEqual({
      entries: [{ label: "Change tempo" }],
      cursor: 1,
    })
    document.undo()
    expect(document.project().settings.tempoBpm).toBe(140)
    document.redo()
    expect(document.project().settings.tempoBpm).toBe(120)
  })

  it("keeps separate steps for another gesture, no gesture, or after an undo", () => {
    const document = new SimDocument(newProject())
    document.dispatch(tempo(100), 1)
    document.dispatch(tempo(101), 2)
    document.dispatch(tempo(102))
    document.dispatch(tempo(103))
    expect(document.history().entries).toHaveLength(4)

    document.undo()
    document.redo()
    document.dispatch(tempo(104), 2)
    expect(document.history().entries).toHaveLength(5)
  })

  it("drops the redo steps when a new edit follows an undo", () => {
    const document = new SimDocument(newProject())
    document.dispatch(tempo(100))
    document.dispatch(tempo(110))
    document.undo()
    document.dispatch({ type: "addPattern" })

    expect(document.history()).toEqual({
      entries: [{ label: "Change tempo" }, { label: "Add pattern" }],
      cursor: 2,
    })
    expect(document.redo()).toBeNull()
  })

  it("jumps to any point in the history and reports what changed", () => {
    const document = new SimDocument(newProject())
    document.dispatch(tempo(100))
    document.dispatch({ type: "addPattern" })
    document.dispatch({ type: "addMixerTrack" })

    const touched = document.jump(0)
    expect(touched).toMatchObject({
      settings: true,
      patternList: true,
      mixer: true,
    })
    expect(document.project().settings.tempoBpm).toBe(140)
    expect(document.history().cursor).toBe(0)

    document.jump(2)
    expect(document.project().patterns).toHaveLength(2)
    expect(document.project().mixer.tracks).toHaveLength(1)
    expect(() => document.jump(9)).toThrow("History has no step 9")
  })

  it("leaves the document untouched when a command fails", () => {
    const document = new SimDocument(newProject())
    const before = document.project()
    expect(() => document.dispatch(tempo(9999))).toThrow()
    expect(document.project()).toBe(before)
    expect(document.history().entries).toHaveLength(0)
    expect(document.isDirty()).toBe(false)
  })

  it("does not reuse an id after an undo", () => {
    const document = new SimDocument(newProject())
    const first = document.dispatch({ type: "addPattern" }).created[0]
    document.undo()
    const second = document.dispatch({ type: "addPattern" }).created[0]
    expect(second).toBeGreaterThan(first)
  })

  it("tracks unsaved edits against the saved point", () => {
    const document = new SimDocument(newProject())
    expect(document.isDirty()).toBe(false)
    document.dispatch(tempo(100))
    expect(document.isDirty()).toBe(true)
    document.markSaved()
    expect(document.isDirty()).toBe(false)

    document.undo()
    expect(document.isDirty()).toBe(true)
    document.redo()
    expect(document.isDirty()).toBe(false)

    document.undo()
    document.dispatch(tempo(90))
    document.undo()
    expect(document.isDirty()).toBe(true)
  })

  it("numbers patches one after another and sends only what changed", () => {
    const document = new SimDocument(demoProject())
    const applied = document.dispatch(tempo(100))
    const patch = document.patch(applied.touched)

    expect(patch.revision).toBe(1)
    expect(patch.settings?.tempoBpm).toBe(100)
    expect(patch.channels).toBeUndefined()
    expect(patch.patterns).toBeUndefined()
    expect(patch.dirty).toBe(true)
    expect(patch.history.cursor).toBe(1)

    const removed = document.dispatch({ type: "addPattern" })
    const second = document.patch(removed.touched)
    expect(second.revision).toBe(2)
    expect(second.patternOrder).toHaveLength(2)
    expect(second.patterns?.map((pattern) => pattern.name)).toEqual([
      "Pattern 2",
    ])
    expect(document.snapshot("/a.windfall")).toMatchObject({
      revision: 2,
      path: "/a.windfall",
    })
  })

  it("lists a removed pattern in the order but not in the pattern data", () => {
    const document = new SimDocument(newProject())
    const id = document.dispatch({ type: "addPattern" }).created[0]
    const removed = document.dispatch({ type: "removePattern", id })
    const patch = document.patch(removed.touched)
    expect(patch.patternOrder).toEqual([1])
    expect(patch.patterns).toBeUndefined()

    const undone = document.undo()
    const back = undone && document.patch(undone)
    expect(back?.patterns?.map((pattern) => pattern.id)).toEqual([id])
  })
})
