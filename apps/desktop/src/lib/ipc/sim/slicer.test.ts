import { afterEach, describe, expect, it } from "vitest"
import { createMockBackend, type MockBackend } from "../mock"
import type { SliceAudio } from "@/features/slicer/types"
import { sim } from "./wasm"

let backend: MockBackend
afterEach(() => backend?.dispose())
const grid = { mode: "grid", gridTicks: 960 } as const
async function setup(slicerAudio?: Record<number, SliceAudio>) {
  backend = createMockBackend({ storage: null, slicerAudio })
  const result = await backend.addAudioClipFromFile(
    "/factory/Loops/Drum loop 128.wav",
    { start: 120 }
  )
  const id = result.created.at(-1)!
  await backend.dispatch({
    type: "updateClips",
    updates: [{ id, patch: { length: 2880, offset: 240, muted: true } }],
  })
  await backend.dispatch({
    type: "updateAudioClips",
    updates: [{ id, patch: { reverse: true, pitch: 6, gain: 0.7, pan: -0.3 } }],
  })
  return id
}
describe("shared WASM slicer backend", () => {
  it("reviews then creates linked slices as one undo step, saves, and reopens", async () => {
    const id = await setup()
    const before = await backend.documentSnapshot()
    const review = await backend.sliceAnalyze(id, grid)
    expect(review.analysis.markers.map((m) => m.tick)).toEqual([
      840, 1800, 2760,
    ])
    expect(review.analysis.peaks).toHaveLength(128)
    expect(await backend.documentSnapshot()).toEqual(before)
    const result = await backend.sliceApply(review.token, [840, 1800])
    const after = await backend.documentSnapshot()
    expect(after.history.cursor).toBe(before.history.cursor + 1)
    expect(after.history.entries.at(-1)?.label).toBe("Slice audio clip")
    const original = before.project.playlist.clips.find((c) => c.id === id)!
    const slices = after.project.playlist.clips.filter((c) =>
      result.created.includes(c.id)
    )
    expect(slices.map((c) => [c.start, c.length, c.offset])).toEqual([
      [120, 840, 240],
      [960, 960, 1080],
      [1920, 1080, 2040],
    ])
    for (const slice of slices) {
      expect(slice.content).toEqual(original.content)
      expect(slice.muted).toBe(true)
    }
    expect(after.project.samples).toEqual(before.project.samples)
    await expect(backend.sliceApply(review.token, [840])).rejects.toThrow(
      "expired"
    )
    await backend.undo()
    expect((await backend.documentSnapshot()).project.playlist).toEqual(
      before.project.playlist
    )
    await backend.redo()
    expect((await backend.documentSnapshot()).project).toEqual(after.project)
    const path = await backend.projectSave("sliced.windfall")
    await backend.projectNew()
    expect((await backend.projectOpen(path)).project.playlist).toEqual(
      after.project.playlist
    )
  })
  it("rejects edited, undone, superseded, discarded, or replaced reviews and invalid cuts", async () => {
    const id = await setup()
    let review = await backend.sliceAnalyze(id, grid)
    const latest = await backend.sliceAnalyze(id, grid)
    await expect(backend.sliceApply(review.token, [840])).rejects.toThrow(
      "expired"
    )
    const before = await backend.documentSnapshot()
    for (const markers of [[], [0], [1], [840, 840], [1800, 840], [2880]])
      await expect(backend.sliceApply(latest.token, markers)).rejects.toThrow()
    expect(await backend.documentSnapshot()).toEqual(before)
    await backend.sliceDiscard(latest.token)
    await expect(backend.sliceApply(latest.token, [840])).rejects.toThrow(
      "expired"
    )
    review = await backend.sliceAnalyze(id, grid)
    await backend.dispatch({
      type: "updateSettings",
      patch: { name: "Edited" },
    })
    await backend.undo()
    await expect(backend.sliceApply(review.token, [840])).rejects.toThrow(
      "changed"
    )
    review = await backend.sliceAnalyze(id, grid)
    await backend.projectNew()
    await expect(backend.sliceApply(review.token, [840])).rejects.toThrow(
      "changed"
    )
  })
  it("invalidates the review if its immutable decoded source is replaced", async () => {
    const sources: Record<number, SliceAudio> = {}
    const id = await setup(sources)
    const clip = (await backend.documentSnapshot()).project.playlist.clips.find(
      (c) => c.id === id
    )!
    if (clip.content.type !== "audio") throw new Error("expected audio")
    sources[clip.content.sample] = {
      sampleRate: 1000,
      channels: 1,
      samples: Array(4000).fill(0.5),
    }
    const review = await backend.sliceAnalyze(id, grid)
    sources[clip.content.sample] = {
      sampleRate: 1000,
      channels: 1,
      samples: Array(4000).fill(0.2),
    }
    await expect(backend.sliceApply(review.token, [840])).rejects.toThrow(
      "changed"
    )
  })
  it("runs deterministic stereo transient detection through the shared Rust export", async () => {
    const id = await setup()
    const clip = (await backend.documentSnapshot()).project.playlist.clips.find(
      (c) => c.id === id
    )!
    clip.start = 0
    clip.offset = 0
    clip.length = 7680
    if (clip.content.type !== "audio") throw new Error("expected audio")
    clip.content.reverse = false
    clip.content.pitch = 0
    const samples = Array<number>(8000).fill(0)
    for (const [frame, level] of [
      [500, 1],
      [1000, 0.1],
      [2000, 1],
    ]) {
      for (let f = frame; f < frame + 30; f++) {
        samples[f * 2] = level
        samples[f * 2 + 1] = -level
      }
    }
    const request = {
      clip,
      tempo: 120,
      swing: 0,
      sampleRate: 1000,
      channels: 2,
      samples,
      options: { mode: "transients", sensitivity: 1 },
    }
    const result = sim.call<{ markers: { tick: number }[] }>(
      "slice_analyze",
      0,
      request
    )
    expect(result.markers.map((m) => m.tick)).toEqual([960, 1920, 3840])
    expect(sim.call("slice_analyze", 0, request)).toEqual(result)
    expect(() =>
      sim.call("slice_analyze", 0, { ...request, channels: 0 })
    ).toThrow("Invalid")
  })
  it("rejects fades, spectral mode, swing and silent tails before mutation", async () => {
    const id = await setup()
    await backend.dispatch({
      type: "updateAudioClips",
      updates: [{ id, patch: { fadeIn: 240 } }],
    })
    const before = await backend.documentSnapshot()
    await expect(backend.sliceAnalyze(id, grid)).rejects.toThrow("fades")
    expect(await backend.documentSnapshot()).toEqual(before)
    await backend.dispatch({
      type: "updateAudioClips",
      updates: [
        {
          id,
          patch: {
            fadeIn: 0,
            stretch: {
              mode: "spectral",
              ratio: 1,
              quality: "standard",
              formants: false,
            },
          },
        },
      ],
    })
    await expect(backend.sliceAnalyze(id, grid)).rejects.toThrow("spectral")
    await backend.dispatch({
      type: "updateAudioClips",
      updates: [{ id, patch: { stretch: { mode: "tape" } } }],
    })
    await backend.dispatch({ type: "updateSettings", patch: { swing: 0.2 } })
    await expect(backend.sliceAnalyze(id, grid)).rejects.toThrow("swing")
    await backend.dispatch({ type: "updateSettings", patch: { swing: 0 } })
    await backend.dispatch({
      type: "updateClips",
      updates: [{ id, patch: { length: 100_000 } }],
    })
    await expect(backend.sliceAnalyze(id, grid)).rejects.toThrow("Trim")
  })
})
