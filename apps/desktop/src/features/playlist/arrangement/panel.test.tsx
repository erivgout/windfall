import { useState } from "react"
import { fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it } from "vitest"
import { ArrangementPanel } from "./panel"
import {
  ArrangementEditError,
  editArrangementBook,
  emptyArrangementBook,
  parseReferences,
} from "./model"
import type { ArrangementBook } from "./model"

function Harness({
  initial = emptyArrangementBook(),
}: {
  initial?: ArrangementBook
}) {
  const [value, setValue] = useState(initial)
  return (
    <>
      <ArrangementPanel
        value={value}
        onChange={setValue}
        tracks={[{ id: 20, name: "Keys" }]}
        clipIds={[10, 11, 12]}
        channels={[{ id: 30, name: "Piano" }]}
        sources={[{ id: 40, name: "Vocal" }]}
      />
      <output aria-label="Local book">{JSON.stringify(value)}</output>
    </>
  )
}
function book(): ArrangementBook {
  return JSON.parse(screen.getByLabelText("Local book").textContent!)
}

describe("local arrangement panel", () => {
  it("adds and renames through onChange, validates names and refuses deleting the last", () => {
    render(<Harness />)
    expect(
      screen.getByText(/Persistence and playlist playback are not connected/)
    ).toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", { name: "Add arrangement" }))
    expect(screen.getByText(/Enter a name/)).toBeInTheDocument()
    expect(book().arrangements).toHaveLength(0)
    fireEvent.change(screen.getByLabelText("New arrangement name"), {
      target: { value: "Verse" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Add arrangement" }))
    expect(book().active).toBe(book().arrangements[0].id)
    expect(book().arrangements[0]).toMatchObject({
      name: "Verse",
      clips: [10, 11, 12],
      tracks: [20],
    })
    expect(
      screen.getByRole("button", { name: "Delete arrangement" })
    ).toBeDisabled()
    fireEvent.change(screen.getByLabelText("Arrangement name"), {
      target: { value: "Chorus" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Rename arrangement" }))
    expect(book().arrangements[0].name).toBe("Chorus")
    fireEvent.change(screen.getByLabelText("New arrangement name"), {
      target: { value: "Outro" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Add arrangement" }))
    const removedId = book().active
    fireEvent.click(screen.getByRole("button", { name: "Delete arrangement" }))
    expect(book().arrangements).toHaveLength(1)
    expect(book().active).not.toBe(removedId)
  })
  it("preserves ordered references and refuses unknown IDs", () => {
    const initial = editArrangementBook(emptyArrangementBook(), {
      type: "addArrangement",
      arrangement: { id: 1, name: "A", clips: [10, 11], tracks: [20] },
    })
    render(<Harness initial={initial} />)
    fireEvent.change(screen.getByLabelText("Ordered clip IDs"), {
      target: { value: "11, 10" },
    })
    fireEvent.click(
      screen.getByRole("button", { name: "Update arrangement references" })
    )
    expect(book().arrangements[0].clips).toEqual([11, 10])
    fireEvent.change(screen.getByLabelText("Ordered clip IDs"), {
      target: { value: "999" },
    })
    fireEvent.click(
      screen.getByRole("button", { name: "Update arrangement references" })
    )
    expect(
      screen.getByText("Reference ID 999 does not exist.")
    ).toBeInTheDocument()
    expect(book().arrangements[0].clips).toEqual([11, 10])
  })
  it("groups clips locally and refuses overlapping membership", () => {
    render(<Harness />)
    fireEvent.change(screen.getByLabelText("Clip IDs to group"), {
      target: { value: "10, 11" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Group clips" }))
    const id = book().clipGroups[0].id
    expect(book().clipGroups[0].clips).toEqual([10, 11])
    fireEvent.change(screen.getByLabelText("Clip IDs to group"), {
      target: { value: "11, 12" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Group clips" }))
    expect(
      screen.getByText("Clip 11 already belongs to a group.")
    ).toBeInTheDocument()
    expect(book().clipGroups).toHaveLength(1)
    fireEvent.click(screen.getByRole("button", { name: "Ungroup clips " + id }))
    expect(book().clipGroups).toEqual([])
  })
})
describe("local metadata guard", () => {
  it("rejects cycles atomically and dissolves groups while retaining tracks", () => {
    let value = emptyArrangementBook()
    for (const id of [1, 2, 3])
      value = editArrangementBook(value, {
        type: "addTrackGroup",
        group: { id, name: "Group " + id },
      })
    value = editArrangementBook(value, {
      type: "moveTrackGroup",
      id: 2,
      parent: 1,
    })
    value = editArrangementBook(value, {
      type: "moveTrackGroup",
      id: 3,
      parent: 2,
    })
    value = editArrangementBook(value, { type: "moveTrack", id: 20, parent: 2 })
    const before = structuredClone(value)
    expect(() =>
      editArrangementBook(value, { type: "moveTrackGroup", id: 1, parent: 3 })
    ).toThrow(ArrangementEditError)
    expect(value).toEqual(before)
    const next = editArrangementBook(value, { type: "removeTrackGroup", id: 2 })
    expect(next.trackParents[20]).toBe(1)
    expect(next.groupParents[3]).toBe(1)
  })
  it("validates links against the caller's IDs and checks reference parsing", () => {
    const value = emptyArrangementBook()
    expect(() =>
      editArrangementBook(
        value,
        {
          type: "linkTrack",
          id: 20,
          kind: { type: "instrument", channel: 99 },
        },
        () => false
      )
    ).toThrow("Channel ID 99 does not exist.")
    expect(() =>
      editArrangementBook(
        value,
        { type: "linkTrack", id: 20, kind: { type: "audio", source: 99 } },
        () => false
      )
    ).toThrow("Audio source ID 99 does not exist.")
    const next = editArrangementBook(
      value,
      { type: "linkTrack", id: 20, kind: { type: "audio", source: 40 } },
      (kind) => kind.type === "audio" && kind.source === 40
    )
    expect(next.linkedTracks[20]).toEqual({ type: "audio", source: 40 })
    expect(value.linkedTracks).toEqual({})
    expect(parseReferences("11,10", [10, 11])).toEqual([11, 10])
    expect(() => parseReferences("10,", [10])).toThrow()
    expect(() => parseReferences("10,10", [10])).toThrow()
  })
})
