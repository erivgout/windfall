import { act, render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { emptyProject } from "@/lib/ipc/sim/project"
import { useProjectStore } from "@/lib/store/project"

import { emptyArrangementBook } from "./arrangement/model"
import { GridMetrics } from "./metrics"
import { startPlaylist } from "./test-utils"
import { TrackHeaders } from "./track-headers"

let app: Awaited<ReturnType<typeof startPlaylist>>

beforeEach(async () => {
  const project = emptyProject()
  project.nextId = 103
  project.playlist.tracks = [
    { id: 100, name: "Kick", muted: false },
    { id: 101, name: "Snare", muted: true, solo: false },
  ]
  project.playlist.arrangementBook = {
    ...emptyArrangementBook(),
    trackGroups: [{ id: 102, name: "Drums" }],
    trackParents: { 100: 102, 101: 102 },
  }
  app = await startPlaylist({ project })
})

afterEach(() => {
  app.stop()
  vi.restoreAllMocks()
})

/** Rust tests apply the command; this response exercises the desktop mirror. */
function replyWithSolo(solo: boolean) {
  const state = useProjectStore.getState()
  return {
    created: [],
    patch: {
      revision: state.revision + 1,
      playlist: {
        ...state.project.playlist,
        tracks: state.project.playlist.tracks.map((track) =>
          track.id === 100 ? { ...track, solo } : track
        ),
      },
      history: state.history,
      dirty: true,
    },
  }
}

describe("playlist track solo", () => {
  it("sends one update per solo toggle and shows the returned solo state", async () => {
    const send = vi.spyOn(app.backend, "dispatch")
    send.mockResolvedValueOnce(replyWithSolo(true))
    render(<TrackHeaders metrics={new GridMetrics()} />)
    const user = userEvent.setup()
    const header = screen.getByRole("group", { name: "Kick" })
    const solo = within(header).getByRole("button", { name: "Solo Kick" })
    expect(solo).toHaveAttribute("aria-pressed", "false")
    expect(header).not.toHaveAttribute("data-solo")
    await user.click(solo)
    expect(send).toHaveBeenCalledExactlyOnceWith(
      { type: "updatePlaylistTrack", id: 100, patch: { solo: true } },
      undefined
    )
    expect(solo).toHaveAttribute("aria-pressed", "true")
    expect(header).toHaveAttribute("data-solo")
    expect(header).toHaveTextContent("Kick · Solo")
    expect(
      within(header).getByRole("button", { name: "Kick on" })
    ).toHaveAttribute("aria-pressed", "true")
    expect(
      useProjectStore
        .getState()
        .project.playlist.tracks.map((track) => track.muted)
    ).toEqual([false, true])

    send.mockClear()
    send.mockResolvedValueOnce(replyWithSolo(false))
    await user.click(solo)
    expect(send).toHaveBeenCalledExactlyOnceWith(
      { type: "updatePlaylistTrack", id: 100, patch: { solo: false } },
      undefined
    )
    expect(solo).toHaveAttribute("aria-pressed", "false")
    expect(header).not.toHaveAttribute("data-solo")
    expect(header).not.toHaveTextContent("· Solo")
  })

  it("shows solo on a muted track and includes the group solo control", () => {
    act(() => {
      useProjectStore.setState(({ project }) => ({
        project: {
          ...project,
          playlist: {
            ...project.playlist,
            tracks: project.playlist.tracks.map((track) =>
              track.id === 101 ? { ...track, solo: true } : track
            ),
          },
        },
      }))
    })
    render(<TrackHeaders metrics={new GridMetrics()} />)
    const header = screen.getByRole("group", { name: "Snare" })
    expect(header).toHaveTextContent("Snare · Solo")
    expect(
      within(header).getByRole("button", { name: "Solo Snare" })
    ).toHaveAttribute("aria-pressed", "true")
    expect(
      within(header).getByRole("button", { name: "Snare on" })
    ).toHaveAttribute("aria-pressed", "false")
    expect(
      within(screen.getByRole("group", { name: "Drums" })).getByRole(
        "button",
        { name: "Solo Drums" }
      )
    ).toHaveAttribute("aria-pressed", "false")
  })
})
