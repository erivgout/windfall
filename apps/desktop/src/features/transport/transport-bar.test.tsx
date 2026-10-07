import { fireEvent, render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { Backend } from "@/lib/ipc"
import { useProjectStore } from "@/lib/store/project"
import { useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"
import { settle, startTestApp } from "@/test/harness"

import { TransportBar } from "./transport-bar"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let backend: Backend
let stop: () => void

beforeEach(async () => {
  ;({ backend, stop } = await startTestApp())
})
afterEach(() => stop())

const tempoField = () => screen.getByRole("spinbutton", { name: /tempo/i })
const tempo = () => useProjectStore.getState().project.settings.tempoBpm
const history = () => useProjectStore.getState().history

describe("TransportBar", () => {
  it("shows the tempo, signature, pattern and a resting playhead", () => {
    render(<TransportBar />)
    expect(tempoField()).toHaveTextContent("128.00")
    expect(tempoField()).toHaveAttribute("aria-valuenow", "128")
    expect(
      screen.getByRole("button", { name: /time signature/i })
    ).toHaveTextContent("4/4")
    expect(
      screen.getByRole("button", { name: "Pattern: Pattern 1" })
    ).toBeVisible()
    expect(
      screen.getByRole("group", { name: "Song position" })
    ).toHaveTextContent("001:01:1")
  })

  it("starts and stops playback through the backend", async () => {
    const user = userEvent.setup()
    render(<TransportBar />)
    const play = screen.getByRole("button", { name: "Play or stop" })
    expect(play).toHaveAttribute("aria-pressed", "false")

    await user.click(play)
    expect(play).toHaveAttribute("aria-pressed", "true")
    expect((await backend.transportState()).playing).toBe(true)

    await user.click(screen.getByRole("button", { name: "Stop" }))
    expect(play).toHaveAttribute("aria-pressed", "false")
    expect((await backend.transportState()).playing).toBe(false)
  })

  it("switches between pattern and song", async () => {
    const user = userEvent.setup()
    render(<TransportBar />)
    const song = screen.getByRole("button", { name: "Song" })
    expect(screen.getByRole("button", { name: "Pattern" })).toHaveAttribute(
      "aria-pressed",
      "true"
    )
    await user.click(song)
    expect(song).toHaveAttribute("aria-pressed", "true")
    expect(useTransportStore.getState().mode).toBe("song")
  })

  it("changes the tempo from the keyboard and by typing", async () => {
    const user = userEvent.setup()
    render(<TransportBar />)

    tempoField().focus()
    await user.keyboard("{ArrowUp}")
    await settle()
    await user.keyboard("{ArrowUp}")
    await settle()
    expect(tempo()).toBe(130)
    expect(tempoField()).toHaveTextContent("130.00")

    await user.keyboard("{Enter}")
    const input = screen.getByRole("textbox", { name: /tempo/i })
    expect(input).toHaveValue("130.00")
    await user.clear(input)
    await user.type(input, "95.5{Enter}")
    await settle()
    expect(tempo()).toBe(95.5)
    expect(tempoField()).toHaveTextContent("95.50")
  })

  it("ignores text that is not a tempo and clamps one out of range", async () => {
    const user = userEvent.setup()
    render(<TransportBar />)
    await user.click(tempoField())
    await user.clear(screen.getByRole("textbox", { name: /tempo/i }))
    await user.keyboard("fast{Enter}")
    await settle()
    expect(tempo()).toBe(128)

    await user.click(tempoField())
    await user.clear(screen.getByRole("textbox", { name: /tempo/i }))
    await user.keyboard("9000{Enter}")
    await settle()
    expect(tempo()).toBe(522)
  })

  it("leaves a digit typed with Alt, Ctrl or Cmd to the shortcuts", async () => {
    render(<TransportBar />)
    tempoField().focus()
    expect(useUiStore.getState().centerTab).toBe("channelRack")
    for (const modifier of ["altKey", "ctrlKey", "metaKey"] as const) {
      fireEvent.keyDown(tempoField(), {
        key: "2",
        code: "Digit2",
        [modifier]: true,
      })
      expect(screen.queryByRole("textbox", { name: /tempo/i })).toBeNull()
    }
    // Alt+2 went to the shortcut it is: it showed the playlist.
    expect(useUiStore.getState().centerTab).toBe("playlist")
    // Neither are the stepping keys taken with a modifier held.
    fireEvent.keyDown(tempoField(), { key: "ArrowUp", ctrlKey: true })
    await settle()
    expect(tempo()).toBe(128)
  })

  it("sets no tempo when the entry is left without Enter or Tab", async () => {
    const user = userEvent.setup()
    render(<TransportBar />)
    tempoField().focus()
    // A stray digit opens the entry, and a click elsewhere leaves it.
    await user.keyboard("2")
    expect(screen.getByRole("textbox", { name: /tempo/i })).toHaveValue("2")
    await user.click(screen.getByRole("button", { name: "Stop" }))
    await settle()
    expect(screen.queryByRole("textbox", { name: /tempo/i })).toBeNull()
    expect(tempo()).toBe(128)
    expect(history().entries).toEqual([])

    // A whole tempo that was typed but not confirmed is dropped as well.
    await user.click(tempoField())
    await user.clear(screen.getByRole("textbox", { name: /tempo/i }))
    await user.keyboard("95")
    await user.click(screen.getByRole("button", { name: "Stop" }))
    await settle()
    expect(tempo()).toBe(128)

    // Tab confirms, like Enter, and Escape gives the field the keys back.
    await user.click(tempoField())
    await user.clear(screen.getByRole("textbox", { name: /tempo/i }))
    await user.keyboard("95{Tab}")
    await settle()
    expect(tempo()).toBe(95)
    await user.click(tempoField())
    await user.keyboard("{Escape}")
    expect(tempoField()).toHaveFocus()
  })

  it("makes one undo step of a tempo drag", async () => {
    render(<TransportBar />)
    const field = tempoField()
    fireEvent.pointerDown(field, { button: 0, pointerId: 1, clientY: 200 })
    for (const clientY of [190, 170, 150, 140]) {
      fireEvent.pointerMove(field, { pointerId: 1, clientY })
      await settle()
    }
    expect(tempoField()).toHaveTextContent("148.00")
    fireEvent.pointerUp(field, { pointerId: 1, clientY: 140 })
    await settle()

    expect(tempo()).toBe(148)
    expect(history().entries).toEqual([{ label: "Change tempo" }])
  })

  it("enables undo once there is something to undo", async () => {
    const user = userEvent.setup()
    render(<TransportBar />)
    const undo = screen.getByRole("button", { name: "Undo" })
    const redo = screen.getByRole("button", { name: "Redo" })
    expect(undo).toBeDisabled()

    tempoField().focus()
    await user.keyboard("{ArrowDown}")
    await settle()
    expect(undo).toBeEnabled()
    expect(redo).toBeDisabled()

    await user.click(undo)
    await settle()
    expect(tempo()).toBe(128)
    expect(redo).toBeEnabled()
  })

  it("lists the history and jumps to the step that is clicked", async () => {
    const user = userEvent.setup()
    render(<TransportBar />)
    tempoField().focus()
    await user.keyboard("{ArrowUp}")
    await settle()
    await user.click(screen.getByRole("button", { name: "Add pattern" }))
    await settle()

    await user.click(screen.getByRole("button", { name: "Undo history" }))
    const list = await screen.findByRole("list", { name: "Undo history" })
    const steps = within(list).getAllByRole("button")
    expect(steps.map((step) => step.textContent)).toEqual([
      "Project opened",
      "Change tempo",
      "Add pattern",
    ])
    expect(steps[2]).toHaveAttribute("aria-current", "step")

    await user.click(steps[0])
    await settle()
    expect(tempo()).toBe(128)
    expect(history().cursor).toBe(0)
  })

  it("adds a pattern and selects it", async () => {
    const user = userEvent.setup()
    render(<TransportBar />)
    await user.click(screen.getByRole("button", { name: "Add pattern" }))
    await settle()
    expect(
      await screen.findByRole("button", { name: "Pattern: Pattern 2" })
    ).toBeVisible()
    expect(useTransportStore.getState().pattern).toBe(
      useProjectStore.getState().project.patterns[1].id
    )
  })

  it("chooses a pattern from the menu of the selector", async () => {
    const user = userEvent.setup()
    render(<TransportBar />)
    await user.click(screen.getByRole("button", { name: "Add pattern" }))
    await settle()
    await user.click(screen.getByRole("button", { name: /^Pattern:/ }))
    await user.click(
      await screen.findByRole("menuitemradio", { name: "Pattern 1" })
    )
    await settle()
    expect(useTransportStore.getState().pattern).toBe(
      useProjectStore.getState().project.patterns[0].id
    )
    expect(
      screen.getByRole("button", { name: "Pattern: Pattern 1" })
    ).toBeVisible()
  })
})
