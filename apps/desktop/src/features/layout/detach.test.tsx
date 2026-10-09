import { act, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { TooltipProvider } from "@/components/ui/tooltip"
import type { MockBackend } from "@/lib/ipc/mock"
import { useUiStore } from "@/lib/store/ui"
import { settle, startTestApp } from "@/test/harness"

import { detachPanel } from "./detach-panel"
import { PanelFrame } from "./panel-frame"
import { Workspace } from "./workspace"

vi.mock("@/features/browser", () => ({ default: () => <p>Browser content</p> }))
vi.mock("@/features/mixer", () => ({ default: () => <p>Mixer content</p> }))
vi.mock("@/features/channel-rack", () => ({
  default: () => <p>Rack content</p>,
}))
vi.mock("@/features/playlist", () => ({
  default: () => <p>Playlist content</p>,
}))
vi.mock("@/features/piano-roll", () => ({
  default: () => <p>Piano roll content</p>,
}))
vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let backend: MockBackend
let stop: () => void

beforeEach(async () => {
  ;({ backend, stop } = await startTestApp())
})
afterEach(() => {
  stop()
  vi.restoreAllMocks()
})

function openWorkspace() {
  return render(
    <TooltipProvider>
      <Workspace />
    </TooltipProvider>
  )
}

async function clickDetach(name: string) {
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: `Detach ${name}` }))
    await settle()
  })
}

describe("detached panel windows", () => {
  it("detaches the mixer once, hides it, and leaves saved visibility true", async () => {
    openWorkspace()
    expect(screen.getByText("Mixer content")).toBeVisible()
    await clickDetach("mixer")
    expect(backend.detachCalls).toEqual(["mixer"])
    expect(screen.queryByText("Mixer content")).toBeNull()
    expect(useUiStore.getState().detachedPanels).toEqual(["mixer"])
    expect(useUiStore.getState().panels.mixer).toBe(true)

    await act(() => detachPanel("mixer"))
    expect(backend.detachCalls).toEqual(["mixer"])
    const saved = JSON.parse(localStorage.getItem("windfall.ui") ?? "{}").state
    expect(saved.panels.mixer).toBe(true)
    expect(saved).not.toHaveProperty("detachedPanels")
  })

  it("waits for the backend and shares rapid repeat clicks", async () => {
    let finish = () => {}
    const request = new Promise<void>((resolve) => {
      finish = resolve
    })
    const detach = vi.spyOn(backend, "detachPanel").mockReturnValue(request)
    openWorkspace()
    const button = screen.getByRole("button", { name: "Detach mixer" })
    fireEvent.click(button)
    fireEvent.click(button)
    expect(detach).toHaveBeenCalledTimes(1)
    expect(screen.getByText("Mixer content")).toBeVisible()
    expect(useUiStore.getState().detachedPanels).toEqual([])
    await act(async () => {
      finish()
      await settle()
    })
    expect(screen.queryByText("Mixer content")).toBeNull()
  })

  it("keeps a panel docked when opening its window fails, and allows retry", async () => {
    vi.spyOn(backend, "detachPanel").mockRejectedValueOnce(
      new Error("Window failed")
    )
    openWorkspace()
    await clickDetach("mixer")
    expect(screen.getByText("Mixer content")).toBeVisible()
    expect(useUiStore.getState().detachedPanels).toEqual([])
    await clickDetach("mixer")
    expect(screen.queryByText("Mixer content")).toBeNull()
  })

  it("docks the mixer through its window header and ignores duplicate events", async () => {
    openWorkspace()
    await clickDetach("mixer")
    const child = render(
      <TooltipProvider>
        <PanelFrame title="Mixer" panel="mixer" detached>
          <p>Child mixer</p>
        </PanelFrame>
      </TooltipProvider>
    )
    const dock = vi.spyOn(backend, "dockPanel")
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Dock mixer" }))
      await settle()
    })
    expect(dock).toHaveBeenCalledWith("mixer")
    expect(screen.getByText("Mixer content")).toBeVisible()
    await act(() => backend.dockPanel("mixer"))
    expect(useUiStore.getState().detachedPanels).toEqual([])
    expect(useUiStore.getState().panels.mixer).toBe(true)
    child.unmount()
  })

  it("restores a docked panel under its saved visibility", async () => {
    openWorkspace()
    await clickDetach("mixer")
    act(() => useUiStore.getState().setPanelVisible("mixer", false))
    await act(() => backend.dockPanel("mixer"))
    expect(useUiStore.getState().detachedPanels).toEqual([])
    expect(screen.queryByText("Mixer content")).toBeNull()
    act(() => useUiStore.getState().setPanelVisible("mixer", true))
    expect(screen.getByText("Mixer content")).toBeVisible()
  })

  it("rejects unknown ids without changing the workspace", async () => {
    openWorkspace()
    await expect(detachPanel("unknown")).rejects.toThrow("Unknown panel")
    await expect(detachPanel("toString")).rejects.toThrow("Unknown panel")
    await expect(
      Reflect.apply(backend.detachPanel, backend, ["unknown"])
    ).rejects.toThrow("Unknown panel")
    expect(backend.detachCalls).toEqual([])
    expect(useUiStore.getState().detachedPanels).toEqual([])
    expect(screen.getByText("Mixer content")).toBeVisible()
    expect(screen.getByText("Rack content")).toBeVisible()
  })

  it("uses another center tab without replacing the saved preference", async () => {
    openWorkspace()
    await clickDetach("channel rack")
    expect(screen.queryByRole("tab", { name: "Channel rack" })).toBeNull()
    expect(screen.getByRole("tab", { name: "Playlist" })).toHaveAttribute(
      "aria-selected",
      "true"
    )
    expect(screen.getByText("Playlist content")).toBeVisible()
    expect(useUiStore.getState().centerTab).toBe("channelRack")
    await act(() => backend.dockPanel("channelRack"))
    expect(screen.getByText("Rack content")).toBeVisible()
    expect(screen.getByRole("tab", { name: "Channel rack" })).toHaveAttribute(
      "aria-selected",
      "true"
    )
  })

  it("handles all center panels detached and restores a docked editor", async () => {
    openWorkspace()
    await clickDetach("channel rack")
    await clickDetach("playlist")
    await clickDetach("piano roll")
    expect(
      screen.getByText("The panels are in their own windows.")
    ).toBeVisible()
    expect(screen.queryAllByRole("tab")).toHaveLength(0)
    await act(() => backend.dockPanel("pianoRoll"))
    expect(screen.getByText("Piano roll content")).toBeVisible()
    expect(useUiStore.getState().centerTab).toBe("channelRack")
  })

  it("detaches and docks the browser without changing its visibility", async () => {
    openWorkspace()
    await clickDetach("browser")
    expect(screen.queryByText("Browser content")).toBeNull()
    expect(useUiStore.getState().panels.browser).toBe(true)
    await act(() => backend.dockPanel("browser"))
    expect(screen.getByText("Browser content")).toBeVisible()
  })

  it("keeps detached panels when the project is replaced", async () => {
    openWorkspace()
    await clickDetach("mixer")
    await act(async () => {
      await backend.projectNew()
      await settle()
    })
    expect(useUiStore.getState().detachedPanels).toEqual(["mixer"])
    expect(screen.queryByText("Mixer content")).toBeNull()
  })
})
