import { afterEach, beforeEach, expect, it, vi } from "vitest"

import type { Clip, ClipStretch, Playlist } from "@/bindings"
import { searchActions } from "@/features/palette/command-palette"
import { registry, runAction } from "@/lib/actions"
import { dispatch, useProjectStore } from "@/lib/store/project"

import { registerToolsActions } from "./actions"

vi.mock("@/lib/store/project", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/store/project")>()),
  dispatch: vi.fn(async () => null),
}))

const initialState = useProjectStore.getState()
const spectral: ClipStretch = {
  mode: "spectral",
  ratio: 1,
  quality: "standard",
  formants: false,
}
const ids = [
  "tools.setAllAudioClipsToTape",
  "tools.setAllAudioClipsToSpectral",
  "tools.muteEmptyPlaylistTracks",
  "tools.unsoloPlaylistTracks",
  "tools.resetAudioClipLevels",
  "tools.unmutePlaylistClips",
]
let unregister: () => void

function audio(id: number, stretch?: ClipStretch, gain = 1, pan = 0): Clip {
  return {
    id,
    track: 100,
    start: 0,
    length: 960,
    offset: 0,
    muted: false,
    content: {
      type: "audio",
      sample: 200,
      mixerTrack: 0,
      gain,
      pan,
      fadeIn: 0,
      fadeOut: 0,
      reverse: false,
      pitch: 0,
      ...(stretch ? { stretch } : {}),
    },
  }
}

const nonAudio: Clip[] = [
  { ...audio(90), track: 101, content: { type: "pattern", pattern: 300 } },
  {
    ...audio(91),
    track: 102,
    content: { type: "automation", automation: 400 },
  },
]

function seed(playlist: Playlist) {
  useProjectStore.setState({
    ready: true,
    project: { ...initialState.project, playlist },
  })
}

beforeEach(() => {
  vi.mocked(dispatch).mockClear()
  seed({ tracks: [], clips: [] })
  unregister = registerToolsActions()
})

afterEach(() => {
  unregister()
  useProjectStore.setState(initialState, true)
})

it("registers all six commands in the palette's Tools section and removes them on cleanup", () => {
  const tools = registry.list().filter((action) => action.section === "Tools")
  expect(tools.map((action) => action.id)).toEqual(ids)
  expect(tools.map((action) => action.title)).toEqual([
    "Set all audio clips to tape",
    "Set all audio clips to spectral",
    "Mute empty playlist tracks",
    "Unsolo playlist tracks",
    "Reset audio clip levels",
    "Unmute playlist clips",
  ])
  expect(
    searchActions(registry.list(), "tools").map((action) => action.id)
  ).toEqual(expect.arrayContaining(ids))
  unregister()
  for (const id of ids) expect(registry.get(id)).toBeUndefined()
})

it("sets changed audio clips to tape in one dispatch, omitting explicit and default tape and non-audio clips", async () => {
  const playlist = {
    tracks: [],
    clips: [
      audio(1, spectral),
      audio(2, { ...spectral, ratio: 2, quality: "high", formants: true }),
      audio(3, { mode: "tape" }),
      audio(4),
      ...nonAudio,
    ],
  }
  const original = structuredClone(playlist)
  seed(playlist)
  await runAction(ids[0])
  expect(dispatch).toHaveBeenCalledExactlyOnceWith({
    type: "updateAudioClips",
    updates: [1, 2].map((id) => ({ id, patch: { stretch: { mode: "tape" } } })),
  })
  expect(useProjectStore.getState().project.playlist).toEqual(original)
})

it("sets audio clips to the exact spectral target in one dispatch, omitting matching and non-audio clips", async () => {
  seed({
    tracks: [],
    clips: [
      audio(1, { mode: "tape" }),
      audio(2),
      audio(3, { ...spectral, ratio: 2 }),
      audio(4, { ...spectral, quality: "high" }),
      audio(5, { ...spectral, formants: true }),
      audio(6, spectral),
      ...nonAudio,
    ],
  })
  await runAction(ids[1])
  expect(dispatch).toHaveBeenCalledExactlyOnceWith({
    type: "updateAudioClips",
    updates: [1, 2, 3, 4, 5].map((id) => ({
      id,
      patch: { stretch: spectral },
    })),
  })
})

it("mutes empty tracks in one labeled batch, omitting muted tracks and tracks with any clip", async () => {
  seed({
    tracks: [100, 101, 102, 103, 104, 105].map((id) => ({
      id,
      name: `Track ${id}`,
      muted: id === 105,
    })),
    clips: [{ ...audio(1), muted: true }, ...nonAudio],
  })
  await runAction(ids[2])
  expect(dispatch).toHaveBeenCalledExactlyOnceWith({
    type: "batch",
    label: "Mute empty playlist tracks",
    commands: [103, 104].map((id) => ({
      type: "updatePlaylistTrack",
      id,
      patch: { muted: true },
    })),
  })
})

it("dispatches nothing when all audio clips already use tape", async () => {
  seed({
    tracks: [],
    clips: [audio(1, { mode: "tape" }), audio(2), ...nonAudio],
  })
  await runAction(ids[0])
  expect(dispatch).not.toHaveBeenCalled()
})

it("dispatches nothing when all audio clips already have the exact spectral stretch", async () => {
  seed({ tracks: [], clips: [audio(1, spectral), ...nonAudio] })
  await runAction(ids[1])
  expect(dispatch).not.toHaveBeenCalled()
})

it("dispatches nothing when each track is occupied or already muted", async () => {
  seed({
    tracks: [
      { id: 100, name: "Occupied", muted: false },
      { id: 101, name: "Empty but muted", muted: true },
    ],
    clips: [audio(1)],
  })
  await runAction(ids[2])
  expect(dispatch).not.toHaveBeenCalled()
})

it("unsolos solo tracks in one labeled batch, omitting non-solo tracks and preserving mute", async () => {
  const playlist: Playlist = {
    tracks: [
      {
        id: 100,
        name: "Solo",
        muted: true,
        solo: true,
        color: 0xff0000,
        height: 80,
      },
      {
        id: 101,
        name: "Not solo",
        muted: false,
        solo: false,
        color: 0x00ff00,
        height: 60,
      },
      {
        id: 102,
        name: "Also solo",
        muted: false,
        solo: true,
        color: 0x0000ff,
        height: 70,
      },
    ],
    clips: [],
  }
  const original = structuredClone(playlist)
  seed(playlist)
  await runAction(ids[3])
  expect(dispatch).toHaveBeenCalledExactlyOnceWith({
    type: "batch",
    label: "Unsolo playlist tracks",
    commands: [100, 102].map((id) => ({
      type: "updatePlaylistTrack",
      id,
      patch: { solo: false },
    })),
  })
  expect(useProjectStore.getState().project.playlist).toEqual(original)
})

it("dispatches nothing when no playlist track is solo", async () => {
  seed({
    tracks: [
      {
        id: 100,
        name: "Not solo",
        muted: false,
        solo: false,
        color: 0xff0000,
        height: 80,
      },
      {
        id: 101,
        name: "Muted",
        muted: true,
        solo: false,
        color: 0x00ff00,
        height: 60,
      },
    ],
    clips: [],
  })
  await runAction(ids[3])
  expect(dispatch).not.toHaveBeenCalled()
})

it("resets changed audio clip gain and pan in one dispatch, omitting unity and non-audio clips", async () => {
  const playlist: Playlist = {
    tracks: [],
    clips: [audio(1, spectral, 0.5, -0.2), audio(2), ...nonAudio],
  }
  const original = structuredClone(playlist)
  seed(playlist)
  await runAction(ids[4])
  expect(dispatch).toHaveBeenCalledExactlyOnceWith({
    type: "updateAudioClips",
    updates: [{ id: 1, patch: { gain: 1, pan: 0 } }],
  })
  expect(useProjectStore.getState().project.playlist).toEqual(original)
})

it.each([
  { gain: 0.5, pan: 0, patch: { gain: 1 } },
  { gain: 1, pan: -0.2, patch: { pan: 0 } },
])(
  "resets only the changed audio clip level: $patch",
  async ({ gain, pan, patch }) => {
    seed({ tracks: [], clips: [audio(1, spectral, gain, pan)] })
    await runAction(ids[4])
    expect(dispatch).toHaveBeenCalledExactlyOnceWith({
      type: "updateAudioClips",
      updates: [{ id: 1, patch }],
    })
  }
)

it("dispatches nothing when every audio clip is already at unity", async () => {
  seed({ tracks: [], clips: [audio(1, spectral), audio(2), ...nonAudio] })
  await runAction(ids[4])
  expect(dispatch).not.toHaveBeenCalled()
})

it("unmutes every muted playlist clip in one dispatch, omitting clips that are not muted", async () => {
  seed({
    tracks: [],
    clips: [
      { ...audio(1), muted: true },
      audio(2),
      ...nonAudio.map((clip) => ({ ...clip, muted: true })),
    ],
  })
  await runAction(ids[5])
  expect(dispatch).toHaveBeenCalledExactlyOnceWith({
    type: "updateClips",
    updates: [1, 90, 91].map((id) => ({ id, patch: { muted: false } })),
  })
})

it("patches only mute without changing clip placement or content", async () => {
  const playlist: Playlist = {
    tracks: [],
    clips: [
      {
        ...audio(1, spectral, 0.5, -0.2),
        track: 103,
        start: 480,
        length: 1920,
        offset: 240,
        muted: true,
      },
    ],
  }
  const original = structuredClone(playlist)
  seed(playlist)
  await runAction(ids[5])
  expect(dispatch).toHaveBeenCalledExactlyOnceWith({
    type: "updateClips",
    updates: [{ id: 1, patch: { muted: false } }],
  })
  expect(useProjectStore.getState().project.playlist).toEqual(original)
})

it("dispatches nothing when no playlist clip is muted", async () => {
  seed({ tracks: [], clips: [audio(1), audio(2), ...nonAudio] })
  await runAction(ids[5])
  expect(dispatch).not.toHaveBeenCalled()
})

it.each(ids)("dispatches nothing for an empty playlist: %s", async (id) => {
  await runAction(id)
  expect(dispatch).not.toHaveBeenCalled()
})
