import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { runAction } from "@/lib/actions"
import { SimDocument } from "@/lib/ipc/sim/document"
import { dispatch, redo, undo, useProjectStore } from "@/lib/store/project"
import { settle, startTestApp } from "@/test/harness"

import { registerToolsActions } from "./actions"

let app: Awaited<ReturnType<typeof startTestApp>>
let unregister: () => void
const state = () => useProjectStore.getState()
const project = () => state().project

beforeEach(async () => {
  app = await startTestApp()
  unregister = registerToolsActions()
})
afterEach(() => {
  unregister()
  app.stop()
  vi.restoreAllMocks()
})

async function assertFileRoundTrip() {
  const snapshot = await app.backend.documentSnapshot()
  const saved = SimDocument.create(snapshot.project)
  const reopened = SimDocument.open(saved.fileText())
  try {
    expect(reopened.project()).toEqual(snapshot.project)
  } finally {
    saved.dispose()
    reopened.dispose()
  }
}

async function audioClip(start: number) {
  const added = await app.backend.addAudioClipFromFile(
    "/factory/Drums/Kicks/Kick 02.wav",
    { start }
  )
  await settle()
  return added.created.at(-1)!
}

describe("Tools acceptance through the shared Rust document WASM", () => {
  it("normalizes both boundary levels in one history step, preserves processing and identity, and round trips the file", async () => {
    const low = await audioClip(240)
    const high = await audioClip(3840)
    await dispatch({
      type: "updateAudioClips",
      updates: [
        {
          id: low,
          patch: {
            gain: 0,
            pan: -1,
            fadeIn: 120,
            fadeOut: 240,
            reverse: true,
            pitch: -12,
            stretch: {
              mode: "spectral",
              ratio: 2,
              quality: "high",
              formants: true,
            },
          },
        },
        { id: high, patch: { gain: 4, pan: 1, pitch: 12 } },
      ],
    })
    const before = structuredClone(project())
    const cursor = state().history.cursor
    await runAction("tools.resetAudioClipLevels")
    const expected = structuredClone(before)
    for (const clip of expected.playlist.clips) {
      if (clip.content.type === "audio") {
        clip.content.gain = 1
        clip.content.pan = 0
      }
    }
    expect(project()).toEqual(expected)
    expect(state().history.cursor).toBe(cursor + 1)
    await assertFileRoundTrip()
    await undo()
    expect(project()).toEqual(before)
    await redo()
    expect(project()).toEqual(expected)
    const send = vi.spyOn(app.backend, "dispatch")
    await runAction("tools.resetAudioClipLevels")
    expect(send).not.toHaveBeenCalled()
    expect(state().history.cursor).toBe(cursor + 1)
  })

  it("switches only audio stretch, survives undo/redo, and skips repeated target actions", async () => {
    await audioClip(0)
    await audioClip(3840)
    const before = structuredClone(project())
    for (const [action, stretch] of [
      [
        "tools.setAllAudioClipsToSpectral",
        { mode: "spectral", ratio: 1, quality: "standard", formants: false },
      ],
      ["tools.setAllAudioClipsToTape", { mode: "tape" }],
    ] as const) {
      const prior = structuredClone(project())
      const cursor = state().history.cursor
      await runAction(action)
      const changed = structuredClone(project())
      expect(state().history.cursor).toBe(cursor + 1)
      const expected = structuredClone(prior)
      for (const clip of expected.playlist.clips) {
        if (clip.content.type === "audio") {
          // The native serializer omits the legacy/default tape mode.
          if (stretch.mode === "tape") delete clip.content.stretch
          else clip.content.stretch = stretch
        }
      }
      expect(changed).toEqual(expected)
      await assertFileRoundTrip()
      await undo()
      expect(project()).toEqual(prior)
      await redo()
      expect(project()).toEqual(changed)
      const send = vi.spyOn(app.backend, "dispatch")
      await runAction(action)
      expect(send).not.toHaveBeenCalled()
      send.mockRestore()
    }
    expect(project().channels).toEqual(before.channels)
    expect(project().samples).toEqual(before.samples)
  })

  it("reads the current track occupancy and clears solos without disturbing mute or clips", async () => {
    const id = await audioClip(0)
    const occupied = project().playlist.clips.find(
      (clip) => clip.id === id
    )!.track
    const empty = (await dispatch({
      type: "addPlaylistTrack",
      name: "Empty QA",
    }))!.created[0]
    await dispatch({
      type: "batch",
      commands: [
        {
          type: "updatePlaylistTrack",
          id: occupied,
          patch: { solo: true, muted: true },
        },
        { type: "updatePlaylistTrack", id: empty, patch: { solo: true } },
      ],
    })
    const before = structuredClone(project())
    let cursor = state().history.cursor
    await runAction("tools.muteEmptyPlaylistTracks")
    expect(state().history.cursor).toBe(cursor + 1)
    expect(
      project().playlist.tracks.find((track) => track.id === empty)?.muted
    ).toBe(true)
    expect(project().playlist.clips).toEqual(before.playlist.clips)
    await undo()
    expect(project()).toEqual(before)
    await redo()
    const muted = structuredClone(project())
    cursor = state().history.cursor
    await runAction("tools.unsoloPlaylistTracks")
    expect(state().history.cursor).toBe(cursor + 1)
    expect(project().playlist.tracks.every((track) => !track.solo)).toBe(true)
    expect(project().playlist.tracks.map((track) => track.muted)).toEqual(
      muted.playlist.tracks.map((track) => track.muted)
    )
    expect(project().playlist.clips).toEqual(muted.playlist.clips)
    await assertFileRoundTrip()
    await undo()
    expect(project()).toEqual(muted)
  })
})
