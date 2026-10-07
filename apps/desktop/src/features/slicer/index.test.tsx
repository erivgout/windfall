import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { afterEach, describe, expect, it, vi } from "vitest"
import { SliceControls } from "."
import { startTestApp } from "@/test/harness"
import { dispatch, loadSnapshot, useProjectStore } from "@/lib/store/project"
import { usePlaylistStore } from "@/features/playlist/store"
import type { Clip } from "@/bindings"

let stop: (() => void) | undefined
afterEach(() => {
  stop?.()
  vi.restoreAllMocks()
})
async function setup() {
  const app = await startTestApp()
  stop = app.stop
  const result = await app.backend.addAudioClipFromFile(
    "/factory/Loops/Drum loop 128.wav",
    { start: 0 }
  )
  const clip = useProjectStore
    .getState()
    .project.playlist.clips.find((c) => c.id === result.created.at(-1))!
  const view = render(<SliceControls clips={[clip]} />)
  fireEvent.click(screen.getByRole("button", { name: "Slice clip" }))
  await screen.findByRole("dialog", { name: "Slice audio clip" })
  return { ...app, clip, view }
}
describe("slicer marker review", () => {
  it.each(["project replacement", "unmount"])(
    "ignores an Apply reply after %s, even with reused revisions",
    async (change) => {
      const { backend, clip, view } = await setup()
      const original = await backend.documentSnapshot()
      const review = await backend.sliceAnalyze(clip.id, {
        mode: "grid",
        gridTicks: 960,
      })
      const applied = await backend.sliceApply(
        review.token,
        review.analysis.markers.map((marker) => marker.tick)
      )
      // Native reply fixture at revision 1, following a clip-bearing snapshot
      // at revision 0. New documents can reuse both revisions and clip IDs.
      const result = { ...applied, patch: { ...applied.patch, revision: 1 } }
      await act(async () => loadSnapshot({ ...original, revision: 0 }))
      vi.spyOn(backend, "sliceAnalyze").mockResolvedValue(review)
      let resolve!: (value: typeof result) => void
      vi.spyOn(backend, "sliceApply").mockImplementation(
        () =>
          new Promise((done) => {
            resolve = done
          })
      )
      fireEvent.click(screen.getByRole("button", { name: "Analyze markers" }))
      await screen.findByRole("img", { name: /Slice marker preview/ })
      fireEvent.click(screen.getByRole("button", { name: "Apply slices" }))
      if (change === "project replacement")
        await act(() => backend.projectNew())
      else view.unmount()
      const before = useProjectStore.getState()
      const selection = new Set(usePlaylistStore.getState().selection)
      expect(before.revision).toBe(0)
      await act(async () => resolve(result))
      expect(useProjectStore.getState()).toBe(before)
      expect(usePlaylistStore.getState().selection).toEqual(selection)
    }
  )
  it("previews markers, deselects a cut and applies once with the resulting slices selected", async () => {
    const { clip } = await setup()
    const before = useProjectStore.getState().history.cursor
    expect(screen.getByText(/Browser demonstration/)).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Apply slices" })).toBeDisabled()
    fireEvent.click(screen.getByRole("button", { name: "Analyze markers" }))
    await screen.findByRole("img", { name: "Slice marker preview: 7 cuts" })
    const cuts = screen.getAllByRole("checkbox")
    expect(cuts).toHaveLength(7)
    fireEvent.click(cuts[0])
    expect(screen.getByRole("status")).toHaveTextContent(
      "7 linked-source slices"
    )
    fireEvent.click(screen.getByRole("button", { name: "Apply slices" }))
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    )
    const state = useProjectStore.getState()
    expect(state.history.cursor).toBe(before + 1)
    expect(state.project.playlist.clips.some((c) => c.id === clip.id)).toBe(
      false
    )
    expect(
      state.project.playlist.clips.filter((c) => c.content.type === "audio")
    ).toHaveLength(7)
  })
  it("requires fresh review after grid, sensitivity, or document changes", async () => {
    await setup()
    fireEvent.click(screen.getByRole("button", { name: "Analyze markers" }))
    await screen.findByRole("img", { name: /Slice marker preview/ })
    fireEvent.click(screen.getByRole("button", { name: "1/8" }))
    expect(
      screen.queryByRole("img", { name: /Slice marker preview/ })
    ).not.toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Apply slices" })).toBeDisabled()
    fireEvent.click(screen.getByRole("button", { name: "Transients" }))
    fireEvent.change(
      screen.getByRole("spinbutton", { name: "Sensitivity (%)" }),
      { target: { value: "101" } }
    )
    expect(
      screen.getByRole("button", { name: "Analyze markers" })
    ).toBeDisabled()
    fireEvent.change(
      screen.getByRole("spinbutton", { name: "Sensitivity (%)" }),
      { target: { value: "80" } }
    )
    fireEvent.click(screen.getByRole("button", { name: "Analyze markers" }))
    await screen.findByRole("img", { name: /Slice marker preview/ })
    await act(() =>
      dispatch({ type: "updateSettings", patch: { name: "Changed" } })
    )
    expect(screen.getByRole("alert")).toHaveTextContent("The project changed")
    expect(screen.getByRole("button", { name: "Apply slices" })).toBeDisabled()
  })
  it("surfaces unsupported fade settings without changing history", async () => {
    const { backend, clip } = await setup()
    await act(() =>
      dispatch({
        type: "updateAudioClips",
        updates: [{ id: clip.id, patch: { fadeIn: 240 } }],
      })
    )
    const before = await backend.documentSnapshot()
    fireEvent.click(screen.getByRole("button", { name: "Analyze markers" }))
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Remove the clip fades"
    )
    expect(await backend.documentSnapshot()).toEqual(before)
  })
  it("discards an analysis that returns after the dialog closes", async () => {
    const { backend, clip } = await setup()
    const result = await backend.sliceAnalyze(clip.id, {
      mode: "grid",
      gridTicks: 960,
    })
    let resolve: (value: typeof result) => void = () => {}
    vi.spyOn(backend, "sliceAnalyze").mockImplementation(
      () =>
        new Promise((done) => {
          resolve = done
        })
    )
    const discard = vi.spyOn(backend, "sliceDiscard")
    fireEvent.click(screen.getByRole("button", { name: "Analyze markers" }))
    fireEvent.click(screen.getByRole("button", { name: "Close" }))
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    )
    await act(async () => {
      resolve(result)
    })
    expect(discard).toHaveBeenCalledWith(result.token)
  })
  it("discards analysis after project replacement even when the revision repeats", async () => {
    const { backend, clip } = await setup()
    const result = await backend.sliceAnalyze(clip.id, {
      mode: "grid",
      gridTicks: 960,
    })
    const revision = useProjectStore.getState().revision
    let resolve!: (value: typeof result) => void
    vi.spyOn(backend, "sliceAnalyze").mockImplementation(
      () =>
        new Promise((done) => {
          resolve = done
        })
    )
    const discard = vi.spyOn(backend, "sliceDiscard")
    fireEvent.click(screen.getByRole("button", { name: "Analyze markers" }))
    await act(async () => {
      const snapshot = await backend.projectNew()
      loadSnapshot({ ...snapshot, revision })
    })
    const before = useProjectStore.getState()
    await act(async () => resolve(result))
    expect(discard).toHaveBeenCalledWith(result.token)
    expect(
      screen.queryByRole("img", { name: /Slice marker preview/ })
    ).toBeNull()
    expect(screen.getByRole("alert")).toHaveTextContent("The project changed")
    expect(useProjectStore.getState()).toBe(before)
  })
  it("requires one selected audio clip", () => {
    const clip: Clip = {
      id: 0,
      track: 0,
      start: 0,
      length: 960,
      offset: 0,
      muted: false,
      content: { type: "pattern", pattern: 1 },
    }
    render(<SliceControls clips={[clip]} />)
    expect(screen.getByRole("button", { name: "Slice clip" })).toBeDisabled()
  })
})
