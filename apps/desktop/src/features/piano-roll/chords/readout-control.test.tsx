import { act, cleanup, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { dispatch } from "@/lib/store/project"
import { SessionContext } from "../context"
import { channel, currentPattern, startRoll } from "../test-utils"
import { PianoRollToolbar } from "../toolbar"

let roll: Awaited<ReturnType<typeof startRoll>>
beforeEach(async () => {
  roll = await startRoll()
  await dispatch({ type: "addChannel", name: "Lead" })
  roll.show("Lead")
})
afterEach(() => {
  cleanup()
  roll.stop()
  vi.restoreAllMocks()
})

function mount() {
  render(
    <SessionContext.Provider value={roll.session}>
      <PianoRollToolbar channelId={channel("Lead").id} readoutRef={null} />
    </SessionContext.Provider>
  )
}

async function add(keys: number[], start = 0, length = 240, name = "Lead") {
  return (await dispatch({
    type: "addNotes",
    pattern: currentPattern().id,
    channel: channel(name).id,
    notes: keys.map((key) => ({ key, start, length })),
  }))!.created
}

describe("persistent toolbar chord readout", () => {
  it("updates with selection while the chord tools dialog is closed", async () => {
    const selected = await add([60, 64, 67], 240)
    await add([60, 63, 67])
    roll.session.setPlayhead(10)
    mount()
    const readout = screen.getByLabelText("Chord")
    expect(readout).toHaveTextContent("C minor")
    expect(
      screen.getByRole("button", { name: "Chords" }).nextElementSibling
    ).toBe(readout)
    act(() => roll.editor.setSelection(selected))
    expect(readout).toHaveTextContent("C major")
    act(() => roll.session.setPlayhead(300))
    expect(readout).toHaveTextContent("C major")
    act(() => {
      roll.session.setPlayhead(10)
      roll.editor.setSelection([])
    })
    expect(readout).toHaveTextContent("C minor")
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
  })

  it("updates at the floored playhead and excludes a note at its end", async () => {
    await add([60, 63, 67], 10, 10)
    await add([64], 0, 10)
    await add([71], 11, 10)
    mount()
    act(() => roll.session.setPlayhead(10.9))
    expect(screen.getByLabelText("Chord")).toHaveTextContent("C minor")
    act(() => roll.session.setPlayhead(21))
    expect(screen.getByLabelText("Chord")).toHaveTextContent("—")
  })

  it("excludes other channels and follows changes of the open lane", async () => {
    await dispatch({ type: "addChannel", name: "Other" })
    await add([60, 63, 67])
    await add([64], 0, 240, "Other")
    roll.session.setPlayhead(10)
    mount()
    expect(screen.getByLabelText("Chord")).toHaveTextContent("C minor")
    act(() => roll.show("Other"))
    expect(screen.getByLabelText("Chord")).toHaveTextContent(
      "Unknown chord (E)"
    )
  })

  it("shows an em dash with a null playhead without dispatching", async () => {
    await add([60, 64, 67])
    const dispatchSpy = vi.spyOn(roll.backend, "dispatch")
    mount()
    expect(screen.getByLabelText("Chord")).toHaveTextContent("—")
    act(() => roll.session.setPlayhead(10))
    expect(screen.getByLabelText("Chord")).toHaveTextContent("C major")
    act(() => roll.session.setPlayhead(null))
    expect(screen.getByLabelText("Chord")).toHaveTextContent("—")
    expect(dispatchSpy).not.toHaveBeenCalled()
  })
})
