import { act, fireEvent, render, screen, within } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { Project, TrackKind } from "@/bindings"
import { buildProject, emptyProject } from "@/lib/ipc/sim/project"
import { useProjectStore } from "@/lib/store/project"

import { emptyArrangementBook } from "./arrangement/model"
import { GridMetrics } from "./metrics"
import { startPlaylist } from "./test-utils"
import { TrackHeaders } from "./track-headers"

const TRACK = 100

function headerProject(): Project {
  const project = buildProject(emptyProject(), (run) => {
    const [sample] = run({
      type: "addSample",
      name: "Vocal.wav",
      path: { kind: "project", path: "Vocal.wav" },
    })
    run({ type: "addChannel", name: "Electric Piano", sample })
  })
  project.nextId = TRACK + 1
  project.playlist = {
    tracks: [{ id: TRACK, name: "Track 1", muted: false }],
    clips: [],
    arrangementBook: emptyArrangementBook(),
  }
  return project
}

let app: Awaited<ReturnType<typeof startPlaylist>>
let send: ReturnType<typeof vi.spyOn>

beforeEach(async () => {
  app = await startPlaylist({ project: headerProject() })
  send = vi.spyOn(app.backend, "dispatch")
})

afterEach(() => {
  try {
    expect(send).not.toHaveBeenCalled()
  } finally {
    app.stop()
    vi.restoreAllMocks()
  }
})

function updateProject(update: (project: Project) => Project) {
  act(() => {
    useProjectStore.setState(({ project }) => ({ project: update(project) }))
  })
}

function setLink(link: TrackKind | null) {
  const linkedTracks: Record<number, TrackKind> = {}
  if (link !== null) linkedTracks[TRACK] = link
  updateProject((project) => ({
    ...project,
    playlist: {
      ...project.playlist,
      arrangementBook: {
        ...emptyArrangementBook(),
        ...project.playlist.arrangementBook,
        linkedTracks,
      },
    },
  }))
}

function renderHeader() {
  render(<TrackHeaders metrics={new GridMetrics()} />)
  return screen.getByRole("group", { name: "Track 1" })
}

describe("playlist track link label", () => {
  it.each([false, true])(
    "keeps the original header without a link (legacy project: %s)",
    (legacy) => {
      if (legacy) {
        updateProject((project) => ({
          ...project,
          playlist: { ...project.playlist, arrangementBook: undefined },
        }))
      }
      const header = renderHeader()
      expect(header.textContent).toBe("1STrack 1")
      expect(within(header).queryByTitle("Missing link")).toBeNull()
    }
  )

  it("shows the linked instrument channel's name", () => {
    const channel = useProjectStore.getState().project.channels[0]
    setLink({ type: "instrument", channel: channel.id })
    const header = renderHeader()
    expect(header).toHaveTextContent("Track 1 · Electric Piano")
    expect(within(header).queryByText("Vocal.wav")).toBeNull()
    fireEvent.click(within(header).getByTitle("Electric Piano"))
  })

  it("shows the linked audio sample's name", () => {
    const sample = useProjectStore.getState().project.samples[0]
    setLink({ type: "audio", source: sample.id })
    const header = renderHeader()
    expect(header).toHaveTextContent("Track 1 · Vocal.wav")
    expect(within(header).queryByText("Electric Piano")).toBeNull()
    fireEvent.click(within(header).getByTitle("Vocal.wav"))
  })

  it.each<TrackKind>([
    { type: "instrument", channel: 999 },
    { type: "audio", source: 999 },
  ])("shows a missing link for an absent $type reference", (link) => {
    setLink(link)
    expect(renderHeader()).toHaveTextContent("Track 1 · Missing link")
  })

  it("updates existing headers when links or channel and sample names change", () => {
    const { channels, samples } = useProjectStore.getState().project
    const header = renderHeader()

    setLink({ type: "instrument", channel: channels[0].id })
    expect(header).toHaveTextContent("Track 1 · Electric Piano")
    updateProject((project) => ({
      ...project,
      channels: project.channels.map((channel) => ({
        ...channel,
        name: "Grand Piano",
      })),
    }))
    expect(header).toHaveTextContent("Track 1 · Grand Piano")
    updateProject((project) => ({ ...project, channels: [] }))
    expect(header).toHaveTextContent("Track 1 · Missing link")

    setLink({ type: "audio", source: samples[0].id })
    expect(header).toHaveTextContent("Track 1 · Vocal.wav")
    updateProject((project) => ({
      ...project,
      samples: project.samples.map((sample) => ({
        ...sample,
        name: "Lead Vocal.wav",
      })),
    }))
    expect(header).toHaveTextContent("Track 1 · Lead Vocal.wav")
    updateProject((project) => ({ ...project, samples: [] }))
    expect(header).toHaveTextContent("Track 1 · Missing link")

    setLink(null)
    expect(header.textContent).toBe("1STrack 1")
  })
})
