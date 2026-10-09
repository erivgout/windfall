import { act, cleanup, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { TimeGridView } from "@/lib/canvas"
import { dispatch } from "@/lib/store/project"

import { SessionContext } from "../context"
import { usePianoRollStore } from "../store"
import {
  at,
  channel,
  currentPattern,
  notesOf,
  project,
  startRoll,
  undoSteps,
} from "../test-utils"
import { ScaleHighlightControl } from "./control"

let roll: Awaited<ReturnType<typeof startRoll>>
beforeEach(async () => {
  roll = await startRoll()
  await dispatch({ type: "addChannel", name: "Lead" })
  roll.show("Lead")
})
afterEach(() => {
  cleanup()
  roll.stop()
})

function control() {
  return render(
    <SessionContext.Provider value={roll.session}>
      <ScaleHighlightControl />
    </SessionContext.Provider>
  )
}

async function choose(name: string) {
  fireEvent.click(screen.getByRole("button", { name: /^Scale highlight:/ }))
  fireEvent.click(await screen.findByRole("menuitemradio", { name }))
}

function view() {
  const remove = vi.fn()
  const addOverlayPainter = vi.fn(() => remove)
  return {
    surface: { addOverlayPainter } as unknown as TimeGridView,
    addOverlayPainter,
    remove,
  }
}

describe("local scale highlight control", () => {
  it("offers the requested scales and changes its root without persisting or editing notes", async () => {
    await dispatch({
      type: "addNotes",
      pattern: currentPattern().id,
      channel: channel("Lead").id,
      notes: [{ key: 61, start: 0, length: 240 }],
    })
    const before = project()
    const history = undoSteps()
    const preferences = usePianoRollStore.getState()
    const mounted = control()
    fireEvent.click(
      screen.getByRole("button", { name: "Scale highlight: Off" })
    )
    expect(
      (await screen.findAllByRole("menuitemradio")).map(
        (item) => item.textContent
      )
    ).toEqual([
      "Off",
      "Major",
      "Natural minor",
      "Harmonic minor",
      "Pentatonic major",
    ])
    fireEvent.click(
      screen.getByRole("menuitemradio", { name: "Natural minor" })
    )
    fireEvent.click(
      screen.getByRole("button", { name: "Scale highlight: C Natural minor" })
    )
    fireEvent.click(
      await screen.findByRole("menuitem", { name: "Highlight root: C" })
    )
    fireEvent.click(await screen.findByRole("menuitemradio", { name: "A" }))
    expect(
      screen.getByRole("button", { name: "Scale highlight: A Natural minor" })
    ).toBeVisible()
    expect(project()).toBe(before)
    expect(undoSteps()).toBe(history)
    expect(usePianoRollStore.getState()).toBe(preferences)
    mounted.unmount()
    control()
    expect(
      screen.getByRole("button", { name: "Scale highlight: Off" })
    ).toBeVisible()
  })

  it("keeps off-scale notes editable and permits drawing off-scale pitches", async () => {
    await dispatch({
      type: "addNotes",
      pattern: currentPattern().id,
      channel: channel("Lead").id,
      notes: [{ key: 61, start: 0, length: 240 }],
    })
    control()
    await choose("Major")
    expect(roll.editor.items.batch.count).toBe(1)
    await act(async () => {
      await roll.drag(at(100, 61), at(580, 61))
    })
    expect(notesOf("Lead")[0]).toMatchObject({ key: 61, start: 480 })
    await act(async () => {
      await roll.click(at(1200, 63))
    })
    expect(notesOf("Lead").map((note) => note.key)).toEqual([61, 63])
  })

  it("follows a replaced view and removes shading on Off or unmount", async () => {
    const first = view()
    roll.session.view = first.surface
    const mounted = control()
    expect(first.addOverlayPainter).not.toHaveBeenCalled()
    await choose("Major")
    expect(first.addOverlayPainter).toHaveBeenCalledOnce()
    act(() => roll.session.notifyView())
    expect(first.addOverlayPainter).toHaveBeenCalledOnce()
    const next = view()
    act(() => {
      roll.session.view = next.surface
      roll.session.notifyView()
    })
    expect(first.remove).toHaveBeenCalledOnce()
    expect(next.addOverlayPainter).toHaveBeenCalledOnce()
    await choose("Off")
    expect(next.remove).toHaveBeenCalledOnce()
    await choose("Pentatonic major")
    mounted.unmount()
    expect(next.remove).toHaveBeenCalledTimes(2)
    act(() => roll.session.notifyView())
    expect(next.addOverlayPainter).toHaveBeenCalledTimes(2)
    roll.session.view = null
  })
})
