import { act, fireEvent, render, screen, within } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { shortcutLabel } from "@/lib/actions"
import type { Backend } from "@/lib/ipc"
import { dispatch } from "@/lib/store/project"
import { useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"
import { settle } from "@/test/harness"

import { setActiveSession } from "./active"
import { addClips } from "./ops"
import {
  at,
  BAR,
  click,
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
  ui().setSnap("step")
})

afterEach(() => stop())

describe("Playback tool", () => {
  it("selects playback and seeks snapped ticks on press and drag without clip commands or mode changes", async () => {
    await addClips(
      [
        {
          row: 0,
          start: BAR,
          length: BAR,
          offset: 480,
          muted: false,
          content: { type: "pattern", pattern: project().patterns[0].id },
        },
      ],
      "Seed"
    )
    await settle()
    const before = project()
    const steps = history().entries.length
    const transport = useTransportStore.getState()
    const send = vi.spyOn(backend, "dispatch")
    const seek = vi.spyOn(backend, "transportSeek")
    const set = vi.spyOn(backend, "transportSet")
    fireEvent.keyDown(document.body, { key: "q", code: "KeyQ" })
    await settle()
    expect(ui().tool).toBe("playback")

    const { session, surface } = started
    expect(session.pointerDown(at(BAR + 140, 0))).toBe(true)
    session.pointerMove(at(BAR + STEP + 140, 0))
    session.pointerMove(at(2 * BAR + 140, 1))
    expect(seek.mock.calls.map(([tick]) => tick)).toEqual([
      BAR + STEP,
      BAR + 2 * STEP,
      2 * BAR + STEP,
    ])
    await session.pointerUp(at(3 * BAR, 1))
    session.pointerMove(at(4 * BAR, 1))
    await settle()
    expect(seek).toHaveBeenCalledTimes(3)
    expect(session.busy).toBe(false)
    expect(send).not.toHaveBeenCalled()
    expect(set).not.toHaveBeenCalled()
    expect(project()).toEqual(before)
    expect(history().entries).toHaveLength(steps)
    expect(surface.drag).toEqual({ ticks: 0, rows: 0 })
    expect(useTransportStore.getState()).toEqual(transport)
  })

  it("Escape ends scrubbing without another seek, including later moves and release", async () => {
    ui().setTool("playback")
    const seek = vi.spyOn(backend, "transportSeek")
    const send = vi.spyOn(backend, "dispatch")
    const { session } = started
    session.pointerDown(at(BAR + 140, 0))
    session.pointerMove(at(2 * BAR + 140, 0))
    expect(session.busy).toBe(true)
    fireEvent.keyDown(document.body, { key: "Escape", code: "Escape" })
    await settle()
    expect(session.busy).toBe(false)
    session.pointerMove(at(3 * BAR, 0))
    await session.pointerUp(at(4 * BAR, 0))
    expect(seek.mock.calls.map(([tick]) => tick)).toEqual([
      BAR + STEP,
      2 * BAR + STEP,
    ])
    expect(send).not.toHaveBeenCalled()
  })

  it("a click seeks once, and Alt or snap off seeks whole ticks", async () => {
    ui().setTool("playback")
    const seek = vi.spyOn(backend, "transportSeek")
    await click(started.session, at(BAR + 140, 0))
    await click(started.session, at(BAR + 140, 0, { alt: true }))
    ui().setSnap("none")
    await click(started.session, at(BAR + 141, 0))
    expect(seek.mock.calls.map(([tick]) => tick)).toEqual([
      BAR + STEP,
      BAR + 140,
      BAR + 141,
    ])
  })

  it("snaps scrubbing to the later meter's grid origin", async () => {
    await dispatch({
      type: "addMeterChange",
      tick: 4001,
      signature: { numerator: 7, denominator: 8 },
    })
    ui().setTool("playback")
    ui().setSnap("bar")
    const seek = vi.spyOn(backend, "transportSeek")
    await click(started.session, at(4001 + 3360 + 100, 0))
    expect(seek).toHaveBeenCalledExactlyOnceWith(7361)
  })

  it.each(["windfall", "fl"] as const)(
    "selects Playback with Q in the %s keymap",
    async (keymap) => {
      useUiStore.getState().setKeymap(keymap)
      fireEvent.keyDown(document.body, { key: "q", code: "KeyQ" })
      await settle()
      expect(ui().tool).toBe("playback")
      expect(shortcutLabel("playlist.toolPlayback")).toBe("Q")
    }
  )

  it("shows and selects Playback in the toolbar", async () => {
    render(<PlaylistToolbar metrics={started.metrics} />)
    const button = within(
      screen.getByRole("group", { name: "Tools" })
    ).getByRole("button", { name: "Playback tool" })
    expect(button).toHaveAttribute("aria-pressed", "false")
    await act(async () => {
      fireEvent.click(button)
      await settle()
    })
    expect(button).toHaveAttribute("aria-pressed", "true")
    expect(ui().tool).toBe("playback")
  })
})
