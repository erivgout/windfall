import userEvent from "@testing-library/user-event"
import { act, render, screen, within } from "@testing-library/react"
import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { registry, runAction, shortcutLabel } from "@/lib/actions"
import { useProjectStore } from "@/lib/store/project"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { useUiStore } from "@/lib/store/ui"
import { settle } from "@/test/harness"
import PlaylistPanel from "./index"
import { usePlaylistStore } from "./store"
import { startPlaylist } from "./test-utils"
import { selectTimelineRegion, useTimelineStore } from "./timeline-store"

vi.mock("@/lib/canvas/react", () => ({ TimeGridCanvas: () => null }))
let rig: Awaited<ReturnType<typeof startPlaylist>>
beforeEach(async () => {
  rig = await startPlaylist()
  announceProjectReplaced()
})
afterEach(() => rig.stop())

for (const kind of ["meter", "named", "loop", "skip", "pause"] as const) {
  const id =
    kind === "meter" ? "playlist.addMeterChange" : `playlist.add${kind}Marker`
  it(`creates a ${kind} through its registered menu action, shortcut and context`, async () => {
    await selectTimelineRegion({ start: 17, end: 839 })
    usePlaylistStore.getState().setCursorTick(17)
    const action = registry.get(id)!
    const run = vi.spyOn(action, "run")
    const shortcut = action.defaultShortcut
    action.defaultShortcut = "Mod+Alt+A"
    try {
      const user = userEvent.setup()
      render(<PlaylistPanel />)
      await act(settle)
      screen.getByRole("button", { name: "Timeline" }).focus()
      await user.keyboard("{ArrowDown}")
      const title = await screen.findByText(action.title)
      const item = title.closest<HTMLElement>('[role="menuitem"]')!
      expect(within(item).getByText(shortcutLabel(id)!)).toBeVisible()
      act(() => useUiStore.getState().showCenterTab("channelRack"))
      expect(item).toHaveAttribute("aria-disabled", "true")
      expect(within(item).getByText("Open the playlist first")).toBeVisible()
      await runAction(id)
      expect(run).not.toHaveBeenCalled()
      expect(useTimelineStore.getState().edit).toBeNull()
      act(() => useUiStore.getState().showCenterTab("playlist"))
      await user.click(item)
      expect(run).toHaveBeenCalledOnce()
      expect(screen.getByLabelText("Song tick")).toHaveValue(17)
      if (kind === "loop" || kind === "skip")
        expect(screen.getByLabelText("End tick")).toHaveValue(839)
      await user.click(screen.getByRole("button", { name: "Save" }))
      await act(settle)
      const timeline = useProjectStore.getState().project.playlist.timeline!
      if (kind === "meter") expect(timeline.meters[0].tick).toBe(17)
      else
        expect(timeline.markers[0]).toMatchObject({
          tick: 17,
          kind: {
            type: kind,
            ...(kind === "loop" || kind === "skip" ? { end: 839 } : {}),
          },
        })
    } finally {
      if (shortcut === undefined) delete action.defaultShortcut
      else action.defaultShortcut = shortcut
    }
  })
}

for (const replacement of ["New", "Open"] as const) {
  it(`discards a creation editor across ${replacement} and opens a fresh source context`, async () => {
    await selectTimelineRegion({ start: 17, end: 839 })
    usePlaylistStore.getState().setCursorTick(17)
    const saved = await rig.backend.projectSave("/add-timeline-source")
    const user = userEvent.setup()
    render(<PlaylistPanel />)
    await act(settle)
    screen.getByRole("button", { name: "Timeline" }).focus()
    await user.keyboard("{ArrowDown}")
    await user.click(
      await screen.findByText(registry.get("playlist.addloopMarker")!.title)
    )
    expect(
      screen.getByRole("dialog", { name: "Add loop marker" })
    ).toBeVisible()
    await act(async () => {
      if (replacement === "New") await rig.backend.projectNew()
      else await rig.backend.projectOpen(saved)
      await settle()
    })
    expect(screen.queryByRole("dialog")).toBeNull()
    expect(useTimelineStore.getState()).toMatchObject({
      edit: null,
      selection: null,
    })
    expect(useProjectStore.getState().project.playlist.timeline).toBeUndefined()
    await act(async () => {
      await runAction("playlist.addloopMarker")
    })
    expect(
      screen.getByRole("dialog", { name: "Add loop marker" })
    ).toBeVisible()
    expect(screen.getByLabelText("Song tick")).toHaveValue(0)
    expect(screen.getByLabelText("End tick")).toHaveValue(3840)
    await user.click(screen.getByRole("button", { name: "Save" }))
    await act(settle)
    expect(
      useProjectStore.getState().project.playlist.timeline!.markers[0]
    ).toMatchObject({ tick: 0, kind: { type: "loop", end: 3840 } })
  })
}
