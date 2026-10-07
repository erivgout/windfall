import { act, fireEvent, render, screen, within } from "@testing-library/react"
import type { ReactNode } from "react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { TooltipProvider } from "@/components/ui/tooltip"
import { registerBrowserActions } from "@/features/browser/actions"
import { registerChannelRackActions } from "@/features/channel-rack/actions"
import { registerMixerActions } from "@/features/mixer/actions"
import { useEffectsUi } from "@/features/mixer/effects-ui"
import { registerPianoRollActions } from "@/features/piano-roll/actions"
import { registerPlaylistActions } from "@/features/playlist/actions"
import { runAction } from "@/lib/actions"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { settle, startTestApp } from "@/test/harness"

import { Workspace } from "./workspace"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

// jsdom lays nothing out, so the real resize handles would take every click.
vi.mock("@/components/ui/resizable", () => ({
  ResizablePanelGroup: ({ children }: { children: ReactNode }) => (
    <div>{children}</div>
  ),
  ResizablePanel: ({ children }: { children: ReactNode }) => (
    <div>{children}</div>
  ),
  ResizableHandle: () => null,
}))

let stops: (() => void)[] = []

beforeEach(async () => {
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null)
  useEffectsUi.setState(useEffectsUi.getInitialState(), true)
  const app = await startTestApp()
  stops = [
    registerBrowserActions(),
    registerChannelRackActions(),
    registerMixerActions(),
    registerPianoRollActions(),
    registerPlaylistActions(),
    app.stop,
  ]
})
afterEach(() => {
  for (const stop of stops) stop()
  vi.restoreAllMocks()
})

const flush = () => act(settle)
const ui = () => useUiStore.getState()
const tab = (name: string | RegExp) => screen.getByRole("tab", { name })
const centre = () => screen.getByRole("tabpanel")

/** The workspace with a compressor on the master, which is selected. */
async function open() {
  const master = useProjectStore.getState().project.mixer.tracks[0]
  await dispatch({ type: "addEffect", track: master.id, kind: "compressor" })
  act(() => ui().selectTrack(master.id))
  render(
    <TooltipProvider>
      <Workspace />
    </TooltipProvider>
  )
  await flush()
}

describe("the effects, enlarged into the editor area", () => {
  it("take the place of the editor tab, under a tab of their own", async () => {
    await open()
    expect(tab("Channel rack")).toHaveAttribute("aria-selected", "true")
    expect(screen.queryByRole("tab", { name: /Effects/ })).toBeNull()

    await act(() => runAction("mixer.enlargeEffects"))
    await flush()
    expect(tab(/Effects/)).toHaveAttribute("aria-selected", "true")
    expect(tab("Channel rack")).toHaveAttribute("aria-selected", "false")
    // The whole editor area is the effects now, with the editor in it.
    const effects = within(centre()).getByRole("complementary", {
      name: "Effects",
    })
    expect(effects).toHaveAttribute("data-enlarged")
    expect(
      within(effects).getByRole("heading", { name: "Master effects" })
    ).toBeVisible()
    expect(
      effects.querySelector("[data-slot=compressor-editor]")
    ).not.toBeNull()
    // There is one inspector, not a second one left beside the strips.
    expect(
      screen.getAllByRole("complementary", { name: "Effects" })
    ).toHaveLength(1)
  })

  it("take the keyboard, and Escape sends them back to the mixer", async () => {
    await open()
    await act(() => runAction("mixer.enlargeEffects"))
    await flush()
    const effects = within(centre()).getByRole("complementary", {
      name: "Effects",
    })
    expect(effects).toHaveFocus()

    fireEvent.keyDown(effects, { key: "Escape", code: "Escape" })
    await flush()
    expect(ui().centerOverlay).toBeNull()
    expect(tab("Channel rack")).toHaveAttribute("aria-selected", "true")
    // They are back where they were, still open.
    expect(useEffectsUi.getState().inspectorOpen).toBe(true)
    expect(
      screen.getAllByRole("complementary", { name: "Effects" })
    ).toHaveLength(1)
    expect(within(centre()).queryByRole("complementary")).toBeNull()
  })

  it("give way to an editor tab that is clicked, and to their own tab", async () => {
    await open()
    await act(() => runAction("mixer.enlargeEffects"))
    await flush()
    fireEvent.click(tab("Playlist"))
    await flush()
    expect(ui().centerOverlay).toBeNull()
    expect(ui().centerTab).toBe("playlist")

    await act(() => runAction("mixer.enlargeEffects"))
    await flush()
    fireEvent.click(tab(/Effects/))
    await flush()
    expect(ui().centerOverlay).toBeNull()
    expect(tab("Playlist")).toHaveAttribute("aria-selected", "true")
  })
})
