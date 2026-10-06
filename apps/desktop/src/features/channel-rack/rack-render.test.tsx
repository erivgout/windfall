import { fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { ChannelId, Command } from "@/bindings"
import { applyCommand } from "@/lib/ipc/sim/commands"
import { newProject } from "@/lib/ipc/sim/project"
import { dispatch } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { settle } from "@/test/harness"

import ChannelRackPanel from "./index"
import { channel, project, startRack, stepButtons } from "./test-utils"

/*
 * Only a row calls `useChannel` while the settings are closed, and it calls
 * it once per render, so counting the calls counts the renders of each row.
 */
const renders = vi.hoisted(() => new Map<number, number>())

vi.mock("@/lib/store/selectors", async (original) => {
  const actual = await original<typeof import("@/lib/store/selectors")>()
  return {
    ...actual,
    useChannel(id: ChannelId | null) {
      if (id !== null) renders.set(id, (renders.get(id) ?? 0) + 1)
      return actual.useChannel(id)
    },
  }
})

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

/** A project with `count` channels and a few steps in each. */
function bigProject(count: number, steps: number) {
  let built = newProject("Big")
  const run = (command: Command) => {
    const applied = applyCommand(built, command)
    built = applied.project
    return applied.created
  }
  const pattern = built.patterns[0].id
  run({ type: "updatePattern", id: pattern, patch: { lengthSteps: steps } })
  for (let index = 0; index < count; index += 1) {
    const [id] = run({ type: "addChannel", name: `Channel ${index + 1}` })
    run({ type: "toggleStep", pattern, channel: id, step: index % steps })
  }
  return built
}

let stop: () => void

beforeEach(async () => {
  ;({ stop } = await startRack({ project: bigProject(50, 64) }))
})
afterEach(() => stop())

function rendersSince(mark: Map<number, number>) {
  const changed = new Map<string, number>()
  for (const item of project().channels) {
    const count = (renders.get(item.id) ?? 0) - (mark.get(item.id) ?? 0)
    if (count > 0) changed.set(item.name, count)
  }
  return changed
}

describe("a rack of 50 channels and 64 steps", () => {
  it("renders every row and step", () => {
    render(<ChannelRackPanel />)
    expect(document.querySelectorAll("[data-channel-row]")).toHaveLength(50)
    expect(document.querySelectorAll("[data-step]")).toHaveLength(50 * 64)
  })

  it("renders only the row whose volume is dragged", async () => {
    render(<ChannelRackPanel />)
    await settle()
    // Selecting first, so the drag itself is all that happens below.
    useUiStore.getState().selectChannel(channel("Channel 7").id)
    await settle()
    const mark = new Map(renders)

    const knob = screen.getByRole("slider", {
      name: "Channel 7 channel volume",
    })
    fireEvent.pointerDown(knob, { pointerId: 1, button: 0, clientY: 100 })
    for (let y = 95; y >= 50; y -= 5) {
      fireEvent.pointerMove(knob, { pointerId: 1, clientY: y })
      await settle()
    }
    fireEvent.pointerUp(knob, { pointerId: 1 })
    await settle()

    expect(channel("Channel 7").volume).toBeGreaterThan(0.8)
    const changed = rendersSince(mark)
    expect([...changed.keys()]).toEqual(["Channel 7"])
  })

  it("renders only the row whose step is toggled", async () => {
    render(<ChannelRackPanel />)
    await settle()
    useUiStore.getState().selectChannel(channel("Channel 20").id)
    await settle()
    const mark = new Map(renders)

    const step = stepButtons("Channel 20")[5]
    fireEvent.pointerDown(step, { pointerId: 1, button: 0 })
    fireEvent.pointerUp(step, { pointerId: 1 })
    await settle()

    expect(step).toHaveAttribute("aria-pressed", "true")
    expect([...rendersSince(mark).keys()]).toEqual(["Channel 20"])
  })

  it("renders only the two rows a selection change touches", async () => {
    render(<ChannelRackPanel />)
    useUiStore.getState().selectChannel(channel("Channel 1").id)
    await settle()
    const mark = new Map(renders)
    useUiStore.getState().selectChannel(channel("Channel 2").id)
    await settle()
    expect([...rendersSince(mark).keys()]).toEqual(["Channel 1", "Channel 2"])
  })

  it("renders no row for a mute on another row or a mixer change", async () => {
    render(<ChannelRackPanel />)
    await settle()
    const mark = new Map(renders)
    await dispatch({
      type: "updateChannel",
      id: channel("Channel 3").id,
      patch: { muted: true },
    })
    await dispatch({
      type: "updateMixerTrack",
      id: channel("Channel 9").mixerTrack,
      patch: { volume: 0.5 },
    })
    await settle()
    expect([...rendersSince(mark).keys()]).toEqual(["Channel 3"])
  })
})
