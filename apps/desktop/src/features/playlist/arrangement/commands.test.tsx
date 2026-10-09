import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { afterEach, expect, it, vi } from "vitest"
import type { Command } from "@/bindings"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { arrangementCommand } from "./commands"
import { ArrangementControls } from "./controls"
import { emptyArrangementBook, type ArrangementEdit } from "./model"

vi.mock("@/lib/store/project", async (original) => ({
  ...(await original<typeof import("@/lib/store/project")>()),
  dispatch: vi.fn(async () => ({})),
}))

afterEach(() => {
  vi.clearAllMocks()
  useProjectStore.setState(useProjectStore.getInitialState(), true)
})

const edits: [ArrangementEdit, Command][] = [
  [
    {
      type: "addArrangement",
      arrangement: { id: 999, name: "Verse", clips: [11], tracks: [20] },
    },
    { type: "addArrangement", name: "Verse", clips: [11], tracks: [20] },
  ],
  [
    { type: "renameArrangement", id: 30, name: "Intro" },
    { type: "renameArrangement", id: 30, name: "Intro" },
  ],
  [
    { type: "setReferences", id: 30, clips: [12, 11], tracks: [20] },
    { type: "setArrangementReferences", id: 30, clips: [12, 11], tracks: [20] },
  ],
  [
    { type: "switchArrangement", id: 31 },
    { type: "switchArrangement", id: 31 },
  ],
  [
    { type: "removeArrangement", id: 31 },
    { type: "removeArrangement", id: 31 },
  ],
  [
    { type: "addTrackGroup", group: { id: 999, name: "Band" } },
    { type: "addTrackGroup", name: "Band", parent: null },
  ],
  [
    { type: "renameTrackGroup", id: 40, name: "Keys" },
    { type: "renameTrackGroup", id: 40, name: "Keys" },
  ],
  [
    { type: "moveTrackGroup", id: 40, parent: 41 },
    { type: "moveTrackGroup", id: 40, parent: 41 },
  ],
  [
    { type: "moveTrack", id: 20, parent: 40 },
    { type: "moveTrackToGroup", track: 20, parent: 40 },
  ],
  [
    { type: "removeTrackGroup", id: 40 },
    { type: "removeTrackGroup", id: 40 },
  ],
  [
    { type: "addClipGroup", group: { id: 999, clips: [11, 12] } },
    { type: "addClipGroup", clips: [11, 12] },
  ],
  [
    { type: "removeClipGroup", id: 50 },
    { type: "removeClipGroup", id: 50 },
  ],
  [
    { type: "linkTrack", id: 20, kind: { type: "instrument", channel: 60 } },
    { type: "linkTrack", track: 20, kind: { type: "instrument", channel: 60 } },
  ],
  [
    { type: "linkTrack", id: 20, kind: { type: "audio", source: 70 } },
    { type: "linkTrack", track: 20, kind: { type: "audio", source: 70 } },
  ],
  [
    { type: "linkTrack", id: 20, kind: null },
    { type: "linkTrack", track: 20, kind: null },
  ],
]
it.each(edits)("maps $type to the checked project command", (edit, command) => {
  expect(arrangementCommand(edit)).toEqual(command)
})

it("mounts the saved book and named entities and dispatches panel actions", async () => {
  const initial = useProjectStore.getInitialState()
  useProjectStore.setState({
    project: {
      ...initial.project,
      nextId: 100,
      patterns: [
        { id: 10, name: "Grand piano", color: 0, lengthSteps: 16, lanes: [] },
      ],
      samples: [
        { id: 70, name: "Vocal", path: { kind: "factory", path: "vocal.wav" } },
      ],
      playlist: {
        tracks: [{ id: 20, name: "Keys", muted: false }],
        clips: [11, 12].map((id) => ({
          id,
          track: 20,
          start: 0,
          length: 960,
          offset: 0,
          muted: false,
          content: { type: "pattern" as const, pattern: 10 },
        })),
        arrangementBook: {
          ...emptyArrangementBook(),
          arrangements: [
            { id: 30, name: "Saved verse", clips: [11, 12], tracks: [20] },
          ],
          active: 30,
        },
      },
    },
  })
  render(<ArrangementControls />)
  fireEvent.click(screen.getByRole("button", { name: "Arrangements (1)" }))
  expect(screen.getByLabelText("Arrangement name")).toHaveValue("Saved verse")
  expect(screen.getAllByText(/11 \(Grand piano\)/).length).toBeGreaterThan(0)
  expect(screen.getByText(/20 \(Keys\)/)).toBeInTheDocument()
  fireEvent.change(screen.getByLabelText("Arrangement name"), {
    target: { value: "Renamed" },
  })
  fireEvent.click(screen.getByRole("button", { name: "Rename arrangement" }))
  await waitFor(() =>
    expect(dispatch).toHaveBeenLastCalledWith({
      type: "renameArrangement",
      id: 30,
      name: "Renamed",
    })
  )
  await act(async () => {
    await Promise.resolve()
  })
  fireEvent.change(screen.getByLabelText("New arrangement name"), {
    target: { value: "Chorus" },
  })
  fireEvent.click(screen.getByRole("button", { name: "Add arrangement" }))
  await waitFor(() =>
    expect(dispatch).toHaveBeenLastCalledWith({
      type: "addArrangement",
      name: "Chorus",
      clips: [11, 12],
      tracks: [20],
    })
  )
  await act(async () => {
    await Promise.resolve()
  })
  fireEvent.change(screen.getByLabelText("Clip IDs to group"), {
    target: { value: "11, 12" },
  })
  fireEvent.click(screen.getByRole("button", { name: "Group clips" }))
  await waitFor(() =>
    expect(dispatch).toHaveBeenLastCalledWith({
      type: "addClipGroup",
      clips: [11, 12],
    })
  )
  await act(async () => {
    await Promise.resolve()
  })
  fireEvent.click(
    screen.getAllByRole("button", {
      name: "Make Grand piano (clip 11) unique",
    })[0]
  )
  await waitFor(() =>
    expect(dispatch).toHaveBeenLastCalledWith({ type: "makeUnique", clip: 11 })
  )
  expect(
    useProjectStore.getState().project.playlist.arrangementBook?.arrangements[0]
      .name
  ).toBe("Saved verse")
})
