import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { Command, Project } from "@/bindings"
import { emptyProject } from "@/lib/ipc/sim/project"
import { dispatch, undo } from "@/lib/store/project"
import { MAX_SONG_TICKS } from "@/lib/units"

import { emptyArrangementBook } from "./arrangement/model"
import { expandClipSelection } from "./clip-groups"
import {
  at,
  BAR,
  click,
  clips,
  drag,
  history,
  project,
  startPlaylist,
  startSession,
  ui,
} from "./test-utils"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

function groupedProject(): Project {
  const value = emptyProject()
  value.nextId = 121
  value.playlist = {
    tracks: [100, 101, 102].map((id, index) => ({
      id,
      name: `Track ${index + 1}`,
      muted: false,
    })),
    clips: [BAR, 2 * BAR, 4 * BAR].map((start, index) => ({
      id: 110 + index,
      track: 100 + index,
      start,
      length: BAR,
      offset: 0,
      muted: false,
      content: { type: "pattern", pattern: value.patterns[0].id },
    })),
    arrangementBook: {
      ...emptyArrangementBook(),
      clipGroups: [{ id: 120, clips: [110, 111] }],
    },
  }
  return value
}

let app: Awaited<ReturnType<typeof startPlaylist>>
let started: ReturnType<typeof startSession>

beforeEach(async () => {
  app = await startPlaylist({ project: groupedProject() })
  started = startSession()
})

afterEach(() => {
  started.stop()
  app.stop()
})

function commands(command: Command): Command[] {
  return command.type === "batch"
    ? command.commands.flatMap(commands)
    : [command]
}

describe("playlist clip group gestures", () => {
  it("selects both clips on press and release, with the pressed clip first", async () => {
    const send = vi.spyOn(app.backend, "dispatch")
    const point = at(2.5 * BAR, 1)
    started.session.pointerDown(point)
    expect([...ui().selection]).toEqual([111, 110])
    await started.session.pointerUp(point)
    expect([...ui().selection]).toEqual([111, 110])
    expect(send).not.toHaveBeenCalled()
  })

  it("keeps existing members and unrelated clips during additive selection", async () => {
    ui().select([112, 111])
    await click(started.session, at(2.5 * BAR, 1, { shift: true }))
    expect([...ui().selection]).toEqual([112, 111, 110])
  })

  it("moves both by the same snapped tick and track delta in one updateClips", async () => {
    const send = vi.spyOn(app.backend, "dispatch")
    await drag(started.session, at(1.5 * BAR, 0), at(3.5 * BAR + 300, 1))
    expect(send).toHaveBeenCalledTimes(1)
    expect(commands(send.mock.calls[0][0])).toEqual([
      {
        type: "updateClips",
        updates: [
          { id: 110, patch: { start: 3 * BAR, track: 101 } },
          { id: 111, patch: { start: 4 * BAR, track: 102 } },
        ],
      },
    ])
    expect(clips()).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: 110, start: 3 * BAR, row: 1 }),
        expect.objectContaining({ id: 111, start: 4 * BAR, row: 2 }),
        expect.objectContaining({ id: 112, start: 4 * BAR, row: 2 }),
      ])
    )
    expect(history().entries).toHaveLength(1)
  })

  it.each([
    ["before tick zero", at(2.5 * BAR, 1), at(0.5 * BAR, 1)],
    ["above the first track", at(2.5 * BAR, 1), at(2.5 * BAR, 0)],
    ["below the last track", at(1.5 * BAR, 0), at(1.5 * BAR, 2)],
  ])(
    "dispatches nothing when another member would move %s",
    async (_, from, to) => {
      const send = vi.spyOn(app.backend, "dispatch")
      const before = project()
      await drag(started.session, from, to)
      expect(send).not.toHaveBeenCalled()
      expect(project()).toBe(before)
      expect(history().entries).toHaveLength(0)
    }
  )

  it("dispatches nothing when another member would end past the song limit", async () => {
    await dispatch({
      type: "updateClips",
      updates: [{ id: 111, patch: { start: MAX_SONG_TICKS - BAR } }],
    })
    const send = vi.spyOn(app.backend, "dispatch")
    const before = project()
    await drag(started.session, at(1.5 * BAR, 0), at(2.5 * BAR, 0))
    expect(send).not.toHaveBeenCalled()
    expect(project()).toBe(before)
  })

  it("discards an earlier valid move when the final destination is invalid", async () => {
    const send = vi.spyOn(app.backend, "dispatch")
    started.session.pointerDown(at(1.5 * BAR, 0))
    started.session.pointerMove(at(2.5 * BAR, 1))
    expect(started.surface.drag).toEqual({ ticks: BAR, rows: 1 })
    started.session.pointerMove(at(2.5 * BAR, 2))
    expect(started.surface.drag).toEqual({ ticks: 0, rows: 0 })
    await started.session.pointerUp(at(2.5 * BAR, 2))
    expect(send).not.toHaveBeenCalled()
  })

  it("erases both members in one existing removeClips command", async () => {
    ui().setTool("erase")
    const send = vi.spyOn(app.backend, "dispatch")
    const point = at(2.5 * BAR, 1)
    started.session.pointerDown(point)
    expect([...started.session.marked]).toEqual([111, 110])
    await started.session.pointerUp(point)
    expect(send).toHaveBeenCalledTimes(1)
    expect(commands(send.mock.calls[0][0])).toEqual([
      { type: "removeClipGroup", id: 120 },
      { type: "removeClips", clips: [111, 110] },
    ])
    expect(clips().map((clip) => clip.id)).toEqual([112])
    expect(history().entries).toHaveLength(1)
    expect(project().playlist.arrangementBook?.clipGroups ?? []).toEqual([])
    await undo()
    expect(clips().map((clip) => clip.id)).toEqual([110, 111, 112])
    expect(project().playlist.arrangementBook?.clipGroups).toEqual([
      { id: 120, clips: [110, 111] },
    ])
  })

  it("expands groups encountered later during an erase stroke", async () => {
    ui().setTool("erase")
    const send = vi.spyOn(app.backend, "dispatch")
    started.session.pointerDown(at(0.5 * BAR, 1))
    started.session.pointerMove(at(2.5 * BAR, 1))
    expect([...started.session.marked]).toEqual([111, 110])
    await started.session.pointerUp(at(2.5 * BAR, 1))
    expect(send).toHaveBeenCalledTimes(1)
    expect(clips().map((clip) => clip.id)).toEqual([112])
  })

  it("removes deleted group members from saved arrangement references in the same undo step", async () => {
    await dispatch({
      type: "addArrangement",
      name: "Verse",
      clips: [110, 111, 112],
      tracks: [100, 101],
    })
    const arrangement = project().playlist.arrangementBook!.arrangements[0]
    const send = vi.spyOn(app.backend, "dispatch")
    ui().setTool("erase")
    await click(started.session, at(1.5 * BAR, 0))
    expect(send).toHaveBeenCalledTimes(1)
    expect(commands(send.mock.calls[0][0])).toEqual([
      { type: "removeClipGroup", id: 120 },
      {
        type: "setArrangementReferences",
        id: arrangement.id,
        clips: [112],
        tracks: [100, 101],
      },
      { type: "removeClips", clips: [110, 111] },
    ])
    expect(clips().map((clip) => clip.id)).toEqual([112])
    expect(project().playlist.arrangementBook!.arrangements[0].clips).toEqual([
      112,
    ])
    await undo()
    expect(project().playlist.arrangementBook!.arrangements[0].clips).toEqual([
      110, 111, 112,
    ])
  })

  it("preserves expansion order in move updates when dragging the second member", async () => {
    const send = vi.spyOn(app.backend, "dispatch")
    await drag(started.session, at(2.5 * BAR, 1), at(3.5 * BAR, 1))
    expect(send).toHaveBeenCalledTimes(1)
    expect(commands(send.mock.calls[0][0])).toEqual([
      {
        type: "updateClips",
        updates: [
          { id: 111, patch: { start: 3 * BAR, track: 101 } },
          { id: 110, patch: { start: 2 * BAR, track: 100 } },
        ],
      },
    ])
  })

  it("selects the group when opening a member's context menu", () => {
    ui().setTool("select")
    started.session.pointerDown(at(2.5 * BAR, 1, { button: 2 }))
    expect([...ui().selection]).toEqual([111, 110])
  })

  it("expands a marquee that touches only one group member", async () => {
    ui().setTool("select")
    await drag(started.session, at(0.5 * BAR, 0), at(1.5 * BAR, 0))
    expect([...ui().selection]).toEqual([110, 111])
  })

  it("preserves clamping and spare-track creation when the project has no clip groups", async () => {
    await dispatch({ type: "removeClipGroup", id: 120 })
    await drag(started.session, at(1.5 * BAR, 0), at(-1.5 * BAR, 0))
    expect(clips().find((clip) => clip.id === 110)?.start).toBe(0)
    started.clock.time += 1000
    await drag(started.session, at(0.5 * BAR, 0), at(1.5 * BAR, 4))
    expect(clips().find((clip) => clip.id === 110)).toMatchObject({
      start: BAR,
      row: 4,
    })
    expect(project().playlist.tracks).toHaveLength(5)
    expect(clips().find((clip) => clip.id === 111)).toMatchObject({
      start: 2 * BAR,
      row: 1,
    })
  })

  it("moves an ungrouped clip alone", async () => {
    const send = vi.spyOn(app.backend, "dispatch")
    await drag(started.session, at(4.5 * BAR, 2), at(5.5 * BAR, 1))
    expect(send).toHaveBeenCalledTimes(1)
    expect(commands(send.mock.calls[0][0])).toEqual([
      {
        type: "updateClips",
        updates: [{ id: 112, patch: { start: 5 * BAR, track: 101 } }],
      },
    ])
    expect([...ui().selection]).toEqual([112])
    expect(
      clips()
        .filter((clip) => clip.id !== 112)
        .map((clip) => [clip.id, clip.start, clip.row])
    ).toEqual([
      [110, BAR, 0],
      [111, 2 * BAR, 1],
    ])
  })

  it("selects and erases an ungrouped clip alone", async () => {
    await click(started.session, at(4.5 * BAR, 2))
    expect([...ui().selection]).toEqual([112])
    ui().setTool("erase")
    const send = vi.spyOn(app.backend, "dispatch")
    await click(started.session, at(4.5 * BAR, 2))
    expect(send).toHaveBeenCalledTimes(1)
    expect(commands(send.mock.calls[0][0])).toEqual([
      { type: "removeClips", clips: [112] },
    ])
    expect(clips().map((clip) => clip.id)).toEqual([110, 111])
  })
})

describe("clip selection expansion order", () => {
  it("retains originals first, appends touched groups in saved order, and removes duplicates", () => {
    expect(
      expandClipSelection(
        [22, 22, 11, 99],
        [
          { id: 1, clips: [10, 11, 12] },
          { id: 2, clips: [20, 21, 22] },
          { id: 3, clips: [30, 31] },
        ]
      )
    ).toEqual([22, 11, 99, 10, 12, 20, 21])
  })

  it("keeps the original selection when a project has no groups", () => {
    expect(expandClipSelection([22, 11, 99], [])).toEqual([22, 11, 99])
  })
})
