import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { TooltipProvider } from "@/components/ui/tooltip"
import { ExportDialog } from "@/features/export/export-dialog"
import { AppMenuBar } from "@/features/layout/menu-bar"
import * as actions from "@/lib/actions"
import { registry, runAction } from "@/lib/actions"
import type { MockBackend } from "@/lib/ipc/mock"
import { useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { settle, startTestApp } from "@/test/harness"

import { GettingStartedDialog } from "./getting-started-dialog"
import {
  closeGettingStarted,
  useGettingStartedStore,
} from "./getting-started-store"

let backend: MockBackend
let stop: () => void

beforeEach(async () => {
  closeGettingStarted()
  ;({ backend, stop } = await startTestApp())
})

afterEach(() => {
  closeGettingStarted()
  stop()
  vi.restoreAllMocks()
})

function mount() {
  render(
    <TooltipProvider>
      <AppMenuBar />
      <GettingStartedDialog />
      <ExportDialog />
    </TooltipProvider>
  )
}

async function open() {
  mount()
  await act(() => runAction("help.gettingStarted"))
  return screen.getByRole("dialog", { name: "Getting started" })
}

const steps = [
  ["Add a channel", "channel.add", "Add an instrument channel."],
  ["Open the piano roll", "view.pianoRoll", "Draw notes on that channel."],
  ["Open the playlist", "view.playlist", "Place the pattern on the timeline."],
  ["Play", "transport.play", "Start playback from the transport."],
  ["Export", "file.export", "Write the mix to an audio file."],
]

describe("Getting started (wf-help-tutorials)", () => {
  it("opens from the Help menu without dispatching a command", async () => {
    const dispatch = vi.spyOn(backend, "dispatch")
    const before = useProjectStore.getState()
    mount()
    expect(screen.queryByRole("dialog")).toBeNull()
    fireEvent.click(screen.getByRole("menuitem", { name: "Help" }))
    const menu = await screen.findByRole("menu")
    expect(
      within(menu).getByRole("menuitem", { name: "Keyboard shortcuts" })
    ).toBeVisible()
    expect(
      within(menu).getByRole("menuitem", { name: "About Windfall" })
    ).toBeVisible()
    fireEvent.click(
      within(menu).getByRole("menuitem", { name: "Getting started" })
    )

    const dialog = await screen.findByRole("dialog", {
      name: "Getting started",
    })
    expect(registry.get("help.gettingStarted")?.section).toBe("Help")
    const items = within(dialog).getAllByRole("listitem")
    expect(items).toHaveLength(steps.length)
    steps.forEach(([title, , sentence], index) => {
      expect(within(items[index]).getByText(sentence)).toBeVisible()
      expect(
        within(items[index]).getByRole("button", { name: title })
      ).toBeVisible()
    })
    expect(dispatch).not.toHaveBeenCalled()
    expect(useProjectStore.getState()).toBe(before)
  })

  it.each(steps)("%s runs only %s and leaves help open", async (title, id) => {
    const dialog = await open()
    const run = vi.spyOn(actions, "runAction")
    const actionRuns = registry.list().map((action) => ({
      id: action.id,
      run: vi.spyOn(action, "run"),
    }))

    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: title }))
      await settle()
    })

    expect(run.mock.calls).toEqual([[id]])
    expect(
      actionRuns
        .filter((action) => action.run.mock.calls.length > 0)
        .map((action) => [action.id, action.run.mock.calls.length])
    ).toEqual([[id, 1]])
    expect(useGettingStartedStore.getState().open).toBe(true)
    expect(dialog).toBeInTheDocument()
    if (id === "file.export") {
      expect(useUiStore.getState().dialog).toBe("export")
      const exportDialog = screen.getByRole("dialog", { name: "Export audio" })
      expect(exportDialog).toBeVisible()
      fireEvent.click(
        within(exportDialog).getByRole("button", { name: "Close" })
      )
      await waitFor(() =>
        expect(
          screen.queryByRole("dialog", { name: "Export audio" })
        ).toBeNull()
      )
      expect(
        screen.getByRole("dialog", { name: "Getting started" })
      ).toBeVisible()
    }
  })

  it.each(["Close", "Escape"])(
    "closes with %s without dispatching",
    async (method) => {
      const dispatch = vi.spyOn(backend, "dispatch")
      const before = useProjectStore.getState()
      const dialog = await open()
      if (method === "Close") {
        fireEvent.click(within(dialog).getByRole("button", { name: "Close" }))
      } else {
        fireEvent.keyDown(dialog, { key: "Escape", code: "Escape" })
      }
      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull())
      expect(useGettingStartedStore.getState().open).toBe(false)
      expect(dispatch).not.toHaveBeenCalled()
      expect(useProjectStore.getState()).toBe(before)
    }
  )

  it("closes on project replacement and can reopen", async () => {
    const dispatch = vi.spyOn(backend, "dispatch")
    await open()
    await act(() => backend.projectNew())
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull())
    expect(useGettingStartedStore.getState().open).toBe(false)
    expect(dispatch).not.toHaveBeenCalled()

    await act(() => runAction("help.gettingStarted"))
    expect(
      screen.getByRole("dialog", { name: "Getting started" })
    ).toBeVisible()
    await act(() => backend.projectNew())
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull())
    expect(dispatch).not.toHaveBeenCalled()
  })
})
