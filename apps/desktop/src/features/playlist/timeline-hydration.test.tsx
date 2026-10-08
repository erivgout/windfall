import userEvent from "@testing-library/user-event"
import { act, render, screen, waitFor } from "@testing-library/react"
import { afterEach, expect, it, vi } from "vitest"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { settle } from "@/test/harness"
import { startPlaylist } from "./test-utils"
import PlaylistPanel from "./index"
import { useTimelineStore } from "./timeline-store"
vi.mock("@/lib/canvas/react", () => ({ TimeGridCanvas: () => null }))
let rig: Awaited<ReturnType<typeof startPlaylist>>
afterEach(() => rig.stop())
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
  const clear = screen.getByRole("menuitem", { name: "Clear time selection" })
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
