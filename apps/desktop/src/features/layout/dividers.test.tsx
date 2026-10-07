import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { useUiStore } from "@/lib/store/ui"
import { settle, startTestApp } from "@/test/harness"

import { dividerMenu } from "./chrome-menus"
import { Workspace } from "./workspace"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

// What the dividers divide is not the point here.
vi.mock("./panels", () => ({
  CENTER_TABS: ["channelRack", "playlist", "pianoRoll"],
  PANELS: Object.fromEntries(
    ["browser", "mixer", "channelRack", "playlist", "pianoRoll"].map((id) => [
      id,
      { title: id, action: `view.${id}`, component: () => null },
    ])
  ),
}))

let stop: () => void

beforeEach(async () => {
  ;({ stop } = await startTestApp())
})
afterEach(() => stop())

describe("the dividers between the docked panels", () => {
  it("open the app's menu on a right-click, with a way back to the size they started at", async () => {
    render(<Workspace />)
    for (const [name, hide] of [
      ["Resize the mixer", /^Mixer/],
      ["Resize the browser", /^Browser/],
    ] as const) {
      const divider = screen.getByRole("separator", { name })
      // Taken by the app, so the webview's own menu stays away.
      expect(
        fireEvent.contextMenu(divider, { clientX: 30, clientY: 30 })
      ).toBe(false)
      await act(settle)
      const menu = screen.getByRole("menu")
      const entries = [...menu.querySelectorAll("[role^=menuitem]")].map(
        (entry) => entry.textContent ?? ""
      )
      expect(entries[0]).toBe("Reset size")
      expect(entries.some((entry) => hide.test(entry))).toBe(true)
      expect(entries.some((entry) => entry.startsWith("Reset layout"))).toBe(
        true
      )
      fireEvent.keyDown(menu, { key: "Escape" })
      await waitFor(() => expect(screen.queryByRole("menu")).toBeNull())
    }
  })

  it("put one divider back and leave the other where it was dragged to", () => {
    const ui = () => useUiStore.getState()
    ui().saveLayout("main:center+mixer", { center: 40, mixer: 60 })
    ui().saveLayout("main:center", { center: 100 })
    ui().saveLayout("workspace:browser+main", { browser: 30, main: 70 })
    const generation = ui().layoutGeneration

    const [reset] = dividerMenu("main", "view.mixer")
    if (typeof reset !== "object" || !("run" in reset)) {
      throw new Error("no Reset size entry")
    }
    void reset.run()
    expect(ui().layouts).toEqual({
      "workspace:browser+main": { browser: 30, main: 70 },
    })
    // The groups are made again, from their default sizes.
    expect(ui().layoutGeneration).toBe(generation + 1)
    // The panels that are showing stay as they are.
    expect(ui().panels).toEqual({ browser: true, mixer: true })
  })
})
