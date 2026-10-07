import { act, fireEvent, render, screen, within } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { AutomationTarget } from "@/bindings"
import type { Backend } from "@/lib/ipc"
import { dispatch, receivePatch } from "@/lib/store/project"
import { settle } from "@/test/harness"

import PlaylistPanel from "../index"
import {
  answerConfirm,
  answerText,
  labels,
  project,
  startPlaylist,
  ui,
} from "../test-utils"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

// jsdom has no canvas to draw on. The curve editing is tested on a stand-in
// surface; here it is the list of automations beside the timeline.
vi.mock("@/lib/canvas/react", () => ({
  TimeGridCanvas: () => null,
}))

let backend: Backend
let stop: () => void

beforeEach(async () => {
  ;({ backend, stop } = await startPlaylist())
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null)
})
afterEach(() => stop())

async function flush() {
  await act(async () => {
    await settle()
  })
}

async function automate(target: AutomationTarget) {
  let created: number[] = []
  await act(async () => {
    const result = await backend.automate(target)
    receivePatch(result.patch)
    created = result.created
    await settle()
  })
  return { automation: created[0], clip: created[2] }
}

const list = () => screen.getByRole("group", { name: "Automation to place" })
const rows = () => within(list()).getAllByRole("button")
const automations = () => project().automations

async function menuOf(element: Element) {
  fireEvent.contextMenu(element)
  await flush()
  return screen.getByRole("menu")
}

describe("the automations to place", () => {
  it("says how to make one while there are none", () => {
    render(<PlaylistPanel />)
    expect(
      screen.getByText(/Right-click a knob or fader and choose/)
    ).toBeInTheDocument()
    expect(
      screen.queryByRole("group", { name: "Automation to place" })
    ).toBeNull()
  })

  it("lists each with what it moves", async () => {
    render(<PlaylistPanel />)
    const kick = project().channels[0]
    const track = project().mixer.tracks[1]
    await automate({ type: "channelVolume", channel: kick.id })
    await automate({ type: "trackPan", track: track.id })
    await automate({ type: "tempo" })
    expect(rows().map((row) => row.textContent)).toEqual([
      "Kick volumeKick → volume",
      "Kick track panKick → pan",
      "TempoTempo",
    ])
    // The words follow the thing: rename the channel and the row says so.
    await act(async () => {
      await dispatch({
        type: "updateChannel",
        id: kick.id,
        patch: { name: "Boom" },
      })
    })
    expect(rows()[0]).toHaveTextContent("Kick volumeBoom → volume")
  })

  it("picks one as the brush, to place more clips of its curve", async () => {
    render(<PlaylistPanel />)
    const { automation } = await automate({ type: "tempo" })
    const pattern = screen.getByRole("button", { name: /^Pattern 1/ })
    expect(rows()[0]).toHaveAttribute("aria-pressed", "false")
    fireEvent.click(rows()[0])
    expect(ui().brush).toEqual({ type: "automation", automation })
    expect(rows()[0]).toHaveAttribute("aria-pressed", "true")
    expect(pattern).toHaveAttribute("aria-pressed", "false")
  })

  it("renames one from its menu and with a double-click", async () => {
    render(<PlaylistPanel />)
    await automate({ type: "tempo" })
    await menuOf(rows()[0])
    fireEvent.click(screen.getByRole("menuitem", { name: "Rename…" }))
    await act(() => answerText("  Slow down "))
    expect(automations()[0].name).toBe("Slow down")
    expect(labels().at(-1)).toBe("Rename automation")
    expect(rows()[0]).toHaveTextContent("Slow downTempo")

    fireEvent.doubleClick(rows()[0])
    await act(() => answerText("Ritardando"))
    expect(automations()[0].name).toBe("Ritardando")
  })

  it("gives one another color", async () => {
    render(<PlaylistPanel />)
    await automate({ type: "tempo" })
    const menu = await menuOf(rows()[0])
    fireEvent.click(within(menu).getByRole("menuitem", { name: "Color" }))
    fireEvent.click(
      await screen.findByRole("menuitemcheckbox", { name: "Teal" })
    )
    await flush()
    expect(automations()[0].color).toBe(0x12a594)
    expect(labels().at(-1)).toBe("Change automation color")
  })

  it("duplicates the curve without its clips, and makes the copy the brush", async () => {
    render(<PlaylistPanel />)
    const { automation } = await automate({ type: "tempo" })
    await act(async () => {
      await dispatch({
        type: "setAutomationPoints",
        id: automation,
        points: [
          { tick: 0, value: 0.2, curve: 0, hold: false },
          { tick: 3840, value: 0.4, curve: 0, hold: false },
        ],
      })
    })
    await menuOf(rows()[0])
    fireEvent.click(screen.getByRole("menuitem", { name: "Duplicate" }))
    await flush()

    expect(automations().map((item) => item.name)).toEqual([
      "Tempo",
      "Tempo #2",
    ])
    expect(automations()[1].points).toEqual(automations()[0].points)
    expect(project().playlist.clips).toHaveLength(1)
    expect(ui().brush).toEqual({
      type: "automation",
      automation: automations()[1].id,
    })
  })

  it("deletes one with its clips, asking first", async () => {
    render(<PlaylistPanel />)
    const { automation } = await automate({ type: "tempo" })
    await menuOf(rows()[0])
    fireEvent.click(screen.getByRole("menuitem", { name: "Delete…" }))
    await act(() => answerConfirm(null))
    expect(automations()).toHaveLength(1)

    await menuOf(rows()[0])
    fireEvent.click(screen.getByRole("menuitem", { name: "Delete…" }))
    await act(() => answerConfirm("delete"))
    expect(automations()).toEqual([])
    expect(project().playlist.clips).toEqual([])
    expect(labels().at(-1)).toBe("Delete automation")
    // The brush had nothing to go back to but the pattern.
    await act(async () => ui().setBrush({ type: "automation", automation }))
    expect(screen.getByRole("button", { name: /^Pattern 1/ })).toHaveAttribute(
      "aria-pressed",
      "false"
    )
  })

  it("counts the clips that share a curve", async () => {
    render(<PlaylistPanel />)
    const { automation } = await automate({ type: "tempo" })
    expect(rows()[0]).toHaveTextContent("TempoTempo")
    await act(async () => {
      await dispatch({
        type: "addClips",
        clips: [
          {
            track: project().playlist.tracks[0].id,
            start: 8 * 3840,
            content: { type: "automation", automation },
          },
        ],
      })
    })
    expect(rows()[0]).toHaveTextContent("TempoTempo×2")
  })
})
