import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { Clip, RealtimeFrame } from "@/bindings"
import { TEST_DIALOGS } from "@/test/harness"

import { createMockBackend } from "./mock"

/*
 * The three shell commands that put audio and automation on the playlist,
 * and what the made-up engine reports while such a song plays.
 */

const KICK = "/factory/Drums/Kicks/Kick 01.wav"
const LOOP = "/factory/Loops/Drum loop 128.wav"

function create() {
  return createMockBackend({ storage: null, dialogs: TEST_DIALOGS })
}

async function projectOf(backend: ReturnType<typeof create>) {
  return (await backend.documentSnapshot()).project
}

const audioOf = (clip: Clip | undefined) =>
  clip?.content.type === "audio" ? clip.content : null

describe("mock backend: audio clips", () => {
  it("adds a clip from a file with the tracks it needs, as one undo step", async () => {
    const backend = create()
    const before = await projectOf(backend)
    const result = await backend.addAudioClipFromFile(LOOP, { start: 960 })
    const { project, history } = await backend.documentSnapshot()

    expect(history.entries).toEqual([{ label: "Add audio clip" }])
    // The sample, the playlist track, the mixer track, and the clip last.
    const [sample, track, mixerTrack, clip] = result.created
    expect(result.created).toHaveLength(4)
    expect(project.samples.at(-1)).toMatchObject({
      id: sample,
      name: "Drum loop 128",
    })
    expect(project.playlist.tracks.at(-1)?.id).toBe(track)
    expect(project.mixer.tracks.at(-1)).toMatchObject({
      id: mixerTrack,
      name: "Drum loop 128",
    })
    expect(project.mixer.tracks).toHaveLength(before.mixer.tracks.length + 1)
    // 3.75 s at 128 bpm is 8 beats.
    expect(project.playlist.clips).toEqual([
      {
        id: clip,
        track,
        start: 960,
        length: 7680,
        offset: 0,
        muted: false,
        content: {
          type: "audio",
          sample,
          mixerTrack,
          gain: 1,
          pan: 0,
          fadeIn: 0,
          fadeOut: 0,
          reverse: false,
          pitch: 0,
        },
      },
    ])

    await backend.undo()
    expect(await projectOf(backend)).toEqual({
      ...before,
      nextId: project.nextId,
    })
  })

  it("reuses the sample and the tracks it is given", async () => {
    const backend = create()
    const before = await projectOf(backend)
    const kick = before.samples.find((sample) => sample.name === "Kick")!
    const [track] = (await backend.dispatch({ type: "addPlaylistTrack" }))
      .created
    const mixerTrack = before.mixer.tracks[1].id

    const result = await backend.addAudioClipFromFile(KICK, {
      track,
      start: 0,
      mixerTrack,
    })
    // The sample the project already had, and the clip.
    expect(result.created).toEqual([kick.id, result.created[1]])
    const project = await projectOf(backend)
    expect(project.samples).toHaveLength(before.samples.length)
    expect(project.mixer.tracks).toHaveLength(before.mixer.tracks.length)
    expect(audioOf(project.playlist.clips[0])).toMatchObject({
      sample: kick.id,
      mixerTrack,
    })
  })

  it("adds a clip of a sample in the pool", async () => {
    const backend = create()
    const before = await projectOf(backend)
    const kick = before.samples.find((sample) => sample.name === "Kick")!
    const result = await backend.addAudioClipFromSample(kick.id, { start: 0 })
    // The playlist track, the mixer track, and the clip last.
    expect(result.created).toHaveLength(3)
    const project = await projectOf(backend)
    const clip = project.playlist.clips[0]
    expect(clip.id).toBe(result.created[2])
    expect(clip.track).toBe(result.created[0])
    expect(audioOf(clip)?.mixerTrack).toBe(result.created[1])
    // 0.42 s at 128 bpm, rounded up to a whole tick.
    expect(clip.length).toBe(Math.ceil(0.42 * 128 * 16))
    expect(project.mixer.tracks.at(-1)?.name).toBe("Kick")
  })

  it("refuses a file it cannot read and a sample it does not have", async () => {
    const backend = create()
    await expect(
      backend.addAudioClipFromFile("/factory/Notes.txt", { start: 0 })
    ).rejects.toThrow("is not an audio file Windfall can read")
    await expect(
      backend.addAudioClipFromFile("/factory/Drums/Nope.wav", { start: 0 })
    ).rejects.toThrow("is not an audio file Windfall can read")
    await expect(
      backend.addAudioClipFromSample(999, { start: 0 })
    ).rejects.toThrow("sample 999 does not exist")
    expect((await backend.documentSnapshot()).history.entries).toEqual([])
  })

  it("refuses a sample whose audio is not there", async () => {
    const backend = create()
    const [missing] = (
      await backend.dispatch({
        type: "addSample",
        name: "Gone",
        path: { kind: "external", path: "/elsewhere/Gone.wav" },
      })
    ).created
    await expect(
      backend.addAudioClipFromSample(missing, { start: 0 })
    ).rejects.toThrow('The audio of "Gone" is not loaded')
  })
})

describe("mock backend: automate", () => {
  it("makes the automation, a track and a clip as one undo step", async () => {
    const backend = create()
    const before = await projectOf(backend)
    const track = before.mixer.tracks[1]
    const result = await backend.automate({
      type: "trackVolume",
      track: track.id,
    })
    const { project, history } = await backend.documentSnapshot()

    expect(history.entries).toEqual([{ label: "Create automation clip" }])
    const [automation, playlistTrack, clip] = result.created
    expect(project.automations).toHaveLength(1)
    expect(project.automations[0]).toMatchObject({
      id: automation,
      target: { type: "trackVolume", track: track.id },
    })
    // One point at the value the fader has now, so nothing changes yet.
    expect(project.automations[0].points).toHaveLength(1)
    expect(project.automations[0].points[0].value).toBeCloseTo(
      Math.sqrt(track.volume / 2),
      6
    )
    // From the top of the song for four bars, on a new track at the end.
    expect(project.playlist.clips).toEqual([
      {
        id: clip,
        track: playlistTrack,
        start: 0,
        length: 4 * 3840,
        offset: 0,
        muted: false,
        content: { type: "automation", automation },
      },
    ])
    expect(project.playlist.tracks.at(-1)?.id).toBe(playlistTrack)

    await backend.undo()
    const undone = await projectOf(backend)
    expect(undone.automations).toEqual([])
    expect(undone.playlist).toEqual(before.playlist)
  })

  it("covers the whole song when it is longer than four bars", async () => {
    const backend = create()
    await backend.addAudioClipFromFile(LOOP, { start: 20 * 3840 })
    await backend.automate({ type: "tempo" })
    const project = await projectOf(backend)
    expect(project.playlist.clips.at(0)).toMatchObject({
      start: 0,
      length: 20 * 3840 + 7680,
      content: { type: "automation" },
    })
    expect(project.automations[0].name).toBe("Tempo")
  })

  it("refuses a target the project does not have", async () => {
    const backend = create()
    await expect(
      backend.automate({ type: "channelVolume", channel: 4242 })
    ).rejects.toThrow("4242")
    expect((await projectOf(backend)).automations).toEqual([])
  })
})

describe("mock backend: the song playing", () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  async function playing(backend: ReturnType<typeof create>) {
    const frames: RealtimeFrame[] = []
    const stop = backend.subscribeRealtime((frame) => frames.push(frame))
    void backend.transportSet({ mode: "song", loopSong: false })
    void backend.transportPlay()
    await vi.advanceTimersByTimeAsync(0)
    return { frames, stop }
  }

  it("reports what the curves are doing, read at the playhead", async () => {
    const backend = create()
    const track = (await projectOf(backend)).mixer.tracks[1].id
    const [automation] = (await backend.automate({ type: "trackPan", track }))
      .created
    await backend.dispatch({
      type: "setAutomationPoints",
      id: automation,
      points: [
        { tick: 0, value: 0, curve: 0, hold: false },
        { tick: 4 * 3840, value: 1, curve: 0, hold: false },
      ],
    })

    const { frames, stop } = await playing(backend)
    expect(frames.at(-1)?.automated ?? []).toEqual([])
    await vi.advanceTimersByTimeAsync(1000)
    const frame = frames.at(-1)!
    expect(frame.playing).toBe(true)
    expect(frame.automated).toHaveLength(1)
    expect(frame.automated[0].automation).toBe(automation)
    expect(frame.automated[0].value).toBeCloseTo(frame.tick / (4 * 3840), 9)

    // Nothing is automated in pattern mode, or once the song has stopped.
    void backend.transportSet({ mode: "pattern" })
    await vi.advanceTimersByTimeAsync(50)
    expect(frames.at(-1)?.automated).toEqual([])
    void backend.transportStop()
    await vi.advanceTimersByTimeAsync(50)
    expect(frames.at(-1)?.automated).toEqual([])
    stop()
  })

  it("moves the playhead by the tempo curve", async () => {
    const backend = create()
    const [automation] = (await backend.automate({ type: "tempo" })).created
    // 60 bpm for the whole clip, under a stored tempo of 128.
    await backend.dispatch({
      type: "setAutomationPoints",
      id: automation,
      points: [{ tick: 0, value: 50 / 512, curve: 0, hold: false }],
    })
    const { frames, stop } = await playing(backend)
    await vi.advanceTimersByTimeAsync(1000)
    // A beat a second is 960 ticks, where 128 bpm would be 2048.
    expect(frames.at(-1)?.tick).toBeGreaterThan(900)
    expect(frames.at(-1)?.tick).toBeLessThan(1020)
    stop()
  })

  it("shows an audio clip sounding on the mixer track it plays into", async () => {
    const backend = create()
    const result = await backend.addAudioClipFromFile(LOOP, { start: 0 })
    const mixerTrack = result.created[2]
    const project = await projectOf(backend)
    const index = project.mixer.tracks.findIndex(
      (track) => track.id === mixerTrack
    )

    const { frames, stop } = await playing(backend)
    await vi.advanceTimersByTimeAsync(500)
    const level = frames.at(-1)!.meters[index * 2]
    expect(level).toBeGreaterThan(0.3)
    // The master hears it too.
    expect(frames.at(-1)!.meters[0]).toBeGreaterThan(0.3)

    // A muted clip is silent.
    await backend.dispatch({
      type: "updateClips",
      updates: [{ id: result.created[3], patch: { muted: true } }],
    })
    await vi.advanceTimersByTimeAsync(1500)
    expect(frames.at(-1)!.meters[index * 2]).toBeLessThan(0.01)
    stop()
  })
})
