import { act, fireEvent, render, screen, within } from "@testing-library/react"
import { afterEach, beforeEach, expect, it, vi } from "vitest"

import type { Clip, Project } from "@/bindings"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"

import { ProjectOverviewView } from "./view"

vi.mock("@/lib/store/project", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/store/project")>()),
  dispatch: vi.fn(),
}))

function fixture(): Project {
  return {
    ...useProjectStore.getInitialState().project,
    channels: [
      {
        id: 30,
        name: "Lead",
        color: 0,
        volume: 1,
        pan: 0,
        muted: false,
        solo: false,
        mixerTrack: 0,
        source: {
          type: "sampler",
          sample: null,
          rootKey: 60,
          tune: 0,
          gain: 1,
          start: 0,
          end: 1,
          reverse: false,
          envelope: null,
          cutSelf: false,
          cutGroup: 0,
        },
      },
    ],
    patterns: [
      {
        id: 10,
        name: "Intro",
        color: 0,
        lengthSteps: 16,
        lanes: [{ channel: 30, notes: [] }],
      },
      {
        id: 20,
        name: "Chorus",
        color: 0,
        lengthSteps: 16,
        lanes: [
          {
            channel: 30,
            notes: [
              { id: 40, start: 0, length: 24, key: 60, velocity: 1, pan: 0 },
            ],
          },
        ],
      },
    ],
    playlist: {
      tracks: [{ id: 50, name: "Melody track", muted: false }],
      clips: [clip(60, 96, { type: "pattern", pattern: 20 })],
    },
    samples: [
      { id: 70, name: "Rain", path: { kind: "project", path: "rain.wav" } },
    ],
    automations: [
      {
        id: 80,
        name: "Volume rise",
        color: 0,
        target: { type: "tempo" },
        points: [],
      },
    ],
  }
}

function clip(id: number, start: number, content: Clip["content"]): Clip {
  return { id, track: 50, start, length: 24, offset: 0, muted: false, content }
}

const audio: Clip["content"] = {
  type: "audio",
  sample: 70,
  mixerTrack: 0,
  gain: 1,
  pan: 0,
  fadeIn: 0,
  fadeOut: 0,
  reverse: false,
  pitch: 0,
}

beforeEach(() => {
  useProjectStore.setState(useProjectStore.getInitialState(), true)
  useProjectStore.setState({ project: fixture(), ready: true })
})

afterEach(() => {
  useProjectStore.setState(useProjectStore.getInitialState(), true)
  vi.clearAllMocks()
})

it("marks only the second pattern for a channel with one note there", () => {
  render(<ProjectOverviewView />)
  const table = screen.getByRole("table", { name: "Channels and patterns" })
  expect(
    within(table)
      .getAllByRole("columnheader")
      .map((cell) => cell.textContent)
  ).toEqual(["Channel", "Intro", "Chorus"])
  const cells = within(screen.getByRole("row", { name: /Lead/ })).getAllByRole(
    "cell"
  )
  expect(cells[0]).toHaveAccessibleName("No notes")
  expect(cells[1]).toHaveAccessibleName("Has notes")
  expect(cells[1]).toHaveTextContent("●")
})

it("leaves a channel with no lane unmarked and keeps rack order", () => {
  const project = useProjectStore.getState().project
  useProjectStore.setState({
    project: {
      ...project,
      channels: [
        ...project.channels,
        { ...project.channels[0], id: 5, name: "Bass" },
      ],
    },
  })
  render(<ProjectOverviewView />)
  expect(
    screen.getAllByRole("rowheader").map((cell) => cell.textContent)
  ).toEqual(["Lead", "Bass"])
  const cells = within(screen.getByRole("row", { name: /Bass/ })).getAllByRole(
    "cell"
  )
  expect(cells).toHaveLength(2)
  for (const cell of cells) expect(cell).toHaveAccessibleName("No notes")
})

it("names the second pattern referenced by a playlist clip and its track", () => {
  render(<ProjectOverviewView />)
  const row = screen.getByRole("listitem")
  expect(row).toHaveTextContent("Melody track")
  expect(row).toHaveTextContent("Pattern: Chorus")
  expect(row).not.toHaveTextContent("Intro")
})

it("lists all target kinds by start tick, breaking ties by clip ID", () => {
  const project = useProjectStore.getState().project
  useProjectStore.setState({
    project: {
      ...project,
      playlist: {
        ...project.playlist,
        clips: [
          clip(90, 96, { type: "pattern", pattern: 20 }),
          clip(92, 24, { type: "automation", automation: 80 }),
          clip(91, 24, audio),
        ],
      },
    },
  })
  render(<ProjectOverviewView />)
  const rows = screen.getAllByRole("listitem")
  expect(rows[0]).toHaveTextContent("Sample: Rain")
  expect(rows[1]).toHaveTextContent("Automation: Volume rise")
  expect(rows[2]).toHaveTextContent("Pattern: Chorus")
  expect(useProjectStore.getState().project.playlist.clips[0].id).toBe(90)
})

it.each([
  { type: "pattern", pattern: 999 } satisfies Clip["content"],
  { ...audio, sample: 999 } satisfies Clip["content"],
  { type: "automation", automation: 999 } satisfies Clip["content"],
])("reports a missing $type target", (content) => {
  const project = useProjectStore.getState().project
  useProjectStore.setState({
    project: {
      ...project,
      playlist: { ...project.playlist, clips: [clip(60, 0, content)] },
    },
  })
  render(<ProjectOverviewView />)
  expect(screen.getByRole("listitem")).toHaveTextContent(/Missing .* target/)
})

it("updates cells and clip names when the project store changes", () => {
  render(<ProjectOverviewView />)
  const project = useProjectStore.getState().project
  act(() => {
    useProjectStore.setState({
      project: {
        ...project,
        patterns: project.patterns.map((pattern) => ({
          ...pattern,
          name: `${pattern.name} updated`,
          lanes: [],
        })),
        playlist: {
          ...project.playlist,
          tracks: [{ id: 50, name: "Renamed track", muted: false }],
        },
      },
    })
  })
  expect(
    screen.getByRole("columnheader", { name: "Chorus updated" })
  ).toBeVisible()
  expect(
    screen.queryByRole("cell", { name: "Has notes" })
  ).not.toBeInTheDocument()
  expect(screen.getByRole("listitem")).toHaveTextContent(
    "Pattern: Chorus updated"
  )
  expect(screen.getByRole("listitem")).toHaveTextContent("Renamed track")
})

it("does not dispatch commands or change editor selection when reading entries", () => {
  const ui = useUiStore.getState()
  const project = useProjectStore.getState()
  render(<ProjectOverviewView />)
  fireEvent.click(screen.getByRole("cell", { name: "Has notes" }))
  fireEvent.click(screen.getByRole("listitem"))
  expect(dispatch).not.toHaveBeenCalled()
  expect(useUiStore.getState()).toBe(ui)
  expect(useProjectStore.getState()).toBe(project)
})
