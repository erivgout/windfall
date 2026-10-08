import userEvent from "@testing-library/user-event"
import { act, render, screen, waitFor, within } from "@testing-library/react"
import { afterEach, expect, it, vi } from "vitest"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { settle } from "@/test/harness"
import { startPlaylist } from "./test-utils"
import PlaylistPanel from "./index"
import { useTimelineStore } from "./timeline-store"
import { refreshTimelineState } from "./timeline-store"
import { registry, useActionEnabled, shortcutLabel } from "@/lib/actions"
import { useUiStore } from "@/lib/store/ui"
import { selectTimelineRegion } from "./timeline-store"
import { addClips } from "./ops"
import { useProjectStore } from "@/lib/store/project"
vi.mock("@/lib/canvas/react", () => ({ TimeGridCanvas: () => null }))
let rig: Awaited<ReturnType<typeof startPlaylist>>
afterEach(() => rig.stop())

function ClearConsumer() {
  return (
    <button disabled={!useActionEnabled("playlist.clearRegion")}>
      Registry Clear
    </button>
  )
}
for (const region of [null, { start: 17, end: 839 }]) {
  it(`notifies an open registry consumer when reload hydration returns ${JSON.stringify(region)}`, async () => {
    rig = await startPlaylist()
    announceProjectReplaced()
    const source = await rig.backend.timelineState()
    await rig.backend.timelineRegion(
      region,
      source.generation,
      source.revision,
      source.request + 1
    )
    render(<ClearConsumer />)
    expect(screen.getByRole("button", { name: "Registry Clear" })).toBeEnabled()
    const version = registry.stateVersion()
    await act(refreshTimelineState)
    expect(registry.stateVersion()).toBeGreaterThan(version)
    if (region)
      expect(
        screen.getByRole("button", { name: "Registry Clear" })
      ).toBeEnabled()
    else
      expect(
        screen.getByRole("button", { name: "Registry Clear" })
      ).toBeDisabled()
  })
}
it("hydrates an armed native range before disabling the reload menu and clears it through the menu", async () => {
  rig = await startPlaylist()
  announceProjectReplaced()
  const source = await rig.backend.timelineState()
  await rig.backend.timelineRegion(
    { start: 17, end: 839 },
    source.generation,
    source.revision,
    50
  )
  expect(useTimelineStore.getState()).toMatchObject({
    selection: null,
    active: false,
    hydrated: false,
  })
  const user = userEvent.setup()
  render(<PlaylistPanel />)
  await act(async () => {
    await settle()
  })
  expect(useTimelineStore.getState()).toMatchObject({
    selection: { start: 17, end: 839 },
    active: true,
    hydrated: true,
  })
  expect(screen.getByText("Ticks 17–839 · playback region")).toBeVisible()
  await user.click(screen.getByRole("button", { name: "Timeline" }))
  const clear = await screen.findByRole("menuitem", {
    name: "Clear song time selection",
  })
  expect(clear).not.toHaveAttribute("aria-disabled", "true")
  await user.click(clear)
  await act(async () => {
    await settle()
  })
  expect((await rig.backend.timelineState()).region).toBeNull()
  expect(useTimelineStore.getState()).toMatchObject({
    selection: null,
    active: false,
  })
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Timeline" })).toHaveFocus()
  )
})

it("uses registered menu titles, shortcuts, handlers and disabled reasons in the current context", async () => {
  rig = await startPlaylist()
  announceProjectReplaced()
  await addClips(
    [
      {
        row: 0,
        start: 0,
        length: 3840,
        offset: 0,
        muted: false,
        content: {
          type: "pattern",
          pattern: useProjectStore.getState().project.patterns[0].id,
        },
      },
    ],
    "Menu song"
  )
  const action = registry.get("playlist.loopSelection")!
  const run = vi.spyOn(action, "run")
  const shortcut = action.defaultShortcut
  action.defaultShortcut = "Mod+Alt+L"
  try {
    const user = userEvent.setup()
    render(<PlaylistPanel />)
    await act(async () => {
      await settle()
    })
    await user.click(screen.getByRole("button", { name: "Timeline" }))
    await screen.findByRole("menu")
    for (const id of [
      "playlist.playSelection",
      "playlist.loopSelection",
      "playlist.zoomRegion",
      "playlist.exportRegion",
      "playlist.clearRegion",
    ]) {
      expect(screen.getByText(registry.get(id)!.title)).toBeVisible()
    }
    const play = screen.getByRole("menuitem", {
      name: /^Play selected song region/,
    })
    expect(play).toHaveAttribute("aria-disabled", "true")
    expect(
      within(play).getByText("Select time in the playlist ruler first")
    ).toBeVisible()
    const clear = screen.getByRole("menuitem", {
      name: /^Clear song time selection/,
    })
    expect(clear).toHaveAttribute("aria-disabled", "true")
    expect(
      within(clear).getByText("No song time selection to clear")
    ).toBeVisible()
    await act(async () => {
      await selectTimelineRegion({ start: 17, end: 839 })
    })
    const loop = screen.getByRole("menuitem", {
      name: /^Loop selected song region/,
    })
    expect(loop).not.toHaveAttribute("aria-disabled", "true")
    expect(within(loop).getByText(shortcutLabel(action.id)!)).toBeVisible()
    // The open menu consumes the registry's contextual predicate as well.
    act(() => useUiStore.getState().showCenterTab("channelRack"))
    expect(loop).toHaveAttribute("aria-disabled", "true")
    expect(within(loop).getByText("Open the playlist first")).toBeVisible()
    act(() => useUiStore.getState().showCenterTab("playlist"))
    expect(loop).not.toHaveAttribute("aria-disabled", "true")
    await user.click(loop)
    expect(run).toHaveBeenCalledOnce()
    expect((await rig.backend.transportState()).playing).toBe(true)
  } finally {
    if (shortcut === undefined) delete action.defaultShortcut
    else action.defaultShortcut = shortcut
  }
})
