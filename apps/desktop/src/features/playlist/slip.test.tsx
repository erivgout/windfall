import { act, fireEvent, render, screen, within } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { ClipContent } from "@/bindings"
import { shortcutLabel } from "@/lib/actions"
import type { Backend } from "@/lib/ipc"
import { dispatch, redo, undo } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { MAX_SONG_TICKS } from "@/lib/units"
import { settle } from "@/test/harness"

import { setActiveSession } from "./active"
import { addClips } from "./ops"
import {
  at,
  BAR,
  click,
  drag,
  history,
  project,
  startPlaylist,
  startSession,
  STEP,
  ui,
} from "./test-utils"
import { PlaylistToolbar } from "./toolbar"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let backend: Backend
let started: ReturnType<typeof startSession>
let stop: () => void

beforeEach(async () => {
  const app = await startPlaylist()
  backend = app.backend
  started = startSession()
  const deactivate = setActiveSession(started.session)
  stop = () => {
    deactivate()
    started.stop()
    app.stop()
  }
  ui().setTool("slip")
  ui().setSnap("step")
})

afterEach(() => stop())

const clip = () => project().playlist.clips[0]
const origin = BAR + BAR / 2

async function seed(kind: ClipContent["type"] = "pattern", offset = 480) {
  let content: ClipContent
  switch (kind) {
    case "pattern":
      content = { type: "pattern", pattern: project().patterns[0].id }
      break
    case "audio":
      content = {
        type: "audio",
        sample: project().samples[0].id,
        mixerTrack: project().mixer.tracks[0].id,
        gain: 0.7,
        pan: 0.2,
        fadeIn: 60,
        fadeOut: 120,
        reverse: true,
        pitch: 2,
      }
      break
    case "automation": {
      const made = await dispatch({
        type: "addAutomation",
        target: { type: "trackVolume", track: project().mixer.tracks[0].id },
        points: [
          { tick: 0, value: 0.3, curve: 0, hold: false },
          { tick: 2 * BAR, value: 0.8, curve: 0, hold: false },
        ],
      })
      content = { type: "automation", automation: made!.created[0] }
      break
    }
  }
  await addClips(
    [{ row: 0, start: BAR + 37, length: BAR, offset, muted: true, content }],
    "Seed"
  )
  await settle()
  return clip()
}

describe("Slip tool", () => {
  it.each(["pattern", "audio", "automation"] as const)(
    "slides %s content right and left with one offset-only command per drag",
    async (kind) => {
      const before = await seed(kind)
      const send = vi.spyOn(backend, "dispatch")
      const steps = history().entries.length

      // The vertical travel does not change the track or the clip's window.
      await drag(started.session, at(origin, 0), at(origin + STEP, 2))
      expect(clip()).toEqual({ ...before, offset: 240 })
      expect(send).toHaveBeenCalledTimes(1)
      expect(send.mock.calls[0][0]).toEqual({
        type: "updateClips",
        updates: [{ id: before.id, patch: { offset: 240 } }],
      })
      expect(history().entries).toHaveLength(steps + 1)
      await undo()
      expect(clip()).toEqual(before)
      await redo()
      expect(clip().offset).toBe(240)
      await undo()

      send.mockClear()
      await drag(started.session, at(origin, 0), at(origin - STEP, 2))
      expect(clip()).toEqual({ ...before, offset: 720 })
      expect(send).toHaveBeenCalledTimes(1)
      expect(send.mock.calls[0][0]).toEqual({
        type: "updateClips",
        updates: [{ id: before.id, patch: { offset: 720 } }],
      })
    }
  )

  it("previews an offset while keeping the document and window fixed", async () => {
    const before = await seed()
    const send = vi.spyOn(backend, "dispatch")
    const { session, surface } = started
    session.pointerDown(at(origin, 0))
    session.pointerMove(at(origin + STEP, 0))
    expect(session.badgeText).toBe("Offset: 240 ticks")
    expect(clip()).toEqual(before)
    expect(surface.drag).toEqual({ ticks: 0, rows: 0 })
    expect(surface.resize).toEqual({ start: 0, end: 0, minLength: 0 })
    expect(send).not.toHaveBeenCalled()
    await session.pointerUp(at(origin + STEP, 0))
    expect(session.badgeText).toBeNull()
  })

  it.each(["cancel", "Escape"] as const)(
    "does not dispatch after %s cancels the drag",
    async (how) => {
      const before = await seed()
      const send = vi.spyOn(backend, "dispatch")
      const { session } = started
      session.pointerDown(at(origin, 0))
      session.pointerMove(at(origin + STEP, 0))
      expect(session.busy).toBe(true)
      if (how === "cancel") session.cancel()
      else {
        fireEvent.keyDown(document.body, { key: "Escape", code: "Escape" })
        await settle()
      }
      expect(session.busy).toBe(false)
      expect(session.badgeText).toBeNull()
      await session.pointerUp(at(origin + STEP, 0))
      expect(send).not.toHaveBeenCalled()
      expect(clip()).toEqual(before)
    }
  )

  it.each([
    { offset: 120, delta: STEP, expected: 0 },
    { offset: MAX_SONG_TICKS - 120, delta: -STEP, expected: MAX_SONG_TICKS },
  ])(
    "clamps offset $offset with travel $delta to $expected",
    async ({ offset, delta, expected }) => {
      const before = await seed("pattern", offset)
      await drag(started.session, at(origin, 0), at(origin + delta, 0))
      expect(clip()).toEqual({ ...before, offset: expected })
    }
  )

  it("does not wrap a pattern offset at its pattern length", async () => {
    const before = await seed("pattern", BAR + 480)
    await drag(started.session, at(origin, 0), at(origin - STEP, 0))
    expect(clip()).toEqual({ ...before, offset: BAR + 720 })
  })

  it("snaps travel independently of an off-grid offset", async () => {
    const before = await seed("pattern", 500)
    await drag(started.session, at(origin, 0), at(origin + 300, 0))
    expect(clip()).toEqual({ ...before, offset: 260 })
  })

  it.each(["Alt", "snap off"])(
    "uses whole tick travel with %s",
    async (mode) => {
      const before = await seed()
      if (mode === "snap off") ui().setSnap("none")
      const modifiers = { alt: mode === "Alt" }
      await drag(
        started.session,
        at(origin, 0, modifiers),
        at(origin - 301, 0, modifiers)
      )
      expect(clip()).toEqual({ ...before, offset: 781 })
    }
  )

  it("does not dispatch on a click or a purely vertical drag", async () => {
    const before = await seed()
    const send = vi.spyOn(backend, "dispatch")
    await click(started.session, at(origin, 0))
    await drag(started.session, at(origin, 0), at(origin, 2))
    expect(send).not.toHaveBeenCalled()
    expect(clip()).toEqual(before)
  })

  it("slips only the pressed clip even when other clips are selected", async () => {
    const before = await seed()
    await addClips(
      [
        {
          row: 1,
          start: BAR,
          length: BAR,
          offset: 480,
          muted: false,
          content: before.content,
        },
      ],
      "Seed"
    )
    const other = project().playlist.clips.find(
      (clip) => clip.id !== before.id
    )!
    ui().select([before.id, other.id])
    await drag(
      started.session,
      at(origin, 0, { mod: true }),
      at(origin - STEP, 0, { mod: true })
    )
    expect(project().playlist.clips).toHaveLength(2)
    expect(
      project().playlist.clips.find((clip) => clip.id === before.id)
    ).toEqual({
      ...before,
      offset: 720,
    })
    expect(
      project().playlist.clips.find((clip) => clip.id === other.id)
    ).toEqual(other)
  })

  it.each(["windfall", "fl"] as const)(
    "selects Slip with Y in the %s keymap",
    async (keymap) => {
      useUiStore.getState().setKeymap(keymap)
      ui().setTool("draw")
      fireEvent.keyDown(document.body, { key: "y", code: "KeyY" })
      await settle()
      expect(ui().tool).toBe("slip")
      expect(shortcutLabel("playlist.toolSlip")).toBe("Y")
    }
  )

  it("shows and selects Slip in the playlist toolbar", async () => {
    ui().setTool("draw")
    render(<PlaylistToolbar metrics={started.metrics} />)
    const tools = within(screen.getByRole("group", { name: "Tools" }))
    const button = tools.getByRole("button", { name: "Slip tool" })
    expect(button).toHaveAttribute("aria-pressed", "false")
    await act(async () => {
      fireEvent.click(button)
      await settle()
    })
    expect(button).toHaveAttribute("aria-pressed", "true")
    expect(ui().tool).toBe("slip")
  })
})
