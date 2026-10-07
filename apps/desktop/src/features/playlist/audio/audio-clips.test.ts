import { toast } from "sonner"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { Clip, ClipContent } from "@/bindings"
import { runAction } from "@/lib/actions"
import type { Backend } from "@/lib/ipc"
import { dispatch, redo, undo } from "@/lib/store/project"
import { settle } from "@/test/harness"

import { duplicateSelection, paste, copySelection } from "../ops"
import {
  at,
  BAR,
  BEAT,
  click,
  drag,
  history,
  labels,
  project,
  PX_PER_TICK,
  ROW_HEIGHT,
  startPlaylist,
  startSession,
  tracks,
  ui,
} from "../test-utils"
import {
  addAudioFile,
  changeAudioClips,
  patchSelectedAudioClips,
  placeSampleClips,
  routeSelectionToNewTrack,
} from "./ops"
import { loadSamplePeaks } from "./peaks"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

/** 3.75 s, which is two bars at the demo's 128 bpm. */
const LOOP = "/factory/Loops/Drum loop 128.wav"
const LOOP_TICKS = 2 * BAR

type Audio = Extract<ClipContent, { type: "audio" }>

let backend: Backend
let stop: () => void

beforeEach(async () => {
  ;({ backend, stop } = await startPlaylist())
  vi.mocked(toast.error).mockClear()
})
afterEach(() => stop())

const steps = () => history().entries.length
const clips = () => project().playlist.clips
const clipOf = (id: number) => clips().find((clip) => clip.id === id) as Clip
const audio = (id: number) => clipOf(id).content as Audio
const rowOf = (clip: Clip) =>
  tracks().findIndex((track) => track.id === clip.track)

/** Puts the loop on the timeline. Resolves to the clip's id. */
async function addLoop(start = 0, row?: number): Promise<number> {
  const id = await addAudioFile(LOOP, { start, row })
  await settle()
  return id as number
}

/** A session whose grid has tall rows, where an audio clip shows its handles. */
function tallSession() {
  const made = startSession()
  made.surface.setViewport({ ...made.surface.viewport, rowHeight: 40 })
  return made
}

/** Where the fade handles are from the top of a row: under the title bar. */
const FADE_Y = 18

/** A pointer position in CSS pixels inside a row 40 pixels tall. */
const spot = (tick: number, row: number, y: number) => ({
  ...at(tick, row),
  y: row * 40 + y,
})

describe("dropping the same sound again", () => {
  const mixerNames = () => project().mixer.tracks.map((track) => track.name)

  it("plays every drop of one file into one mixer track", async () => {
    const send = vi.spyOn(backend, "addAudioClipFromFile")
    const first = await addLoop(0)
    const count = project().mixer.tracks.length
    const second = await addLoop(2 * BAR)
    const third = await addLoop(4 * BAR)

    // Three clips, one "Drum loop 128" track: not a staircase of three.
    expect(project().mixer.tracks).toHaveLength(count)
    expect(mixerNames().filter((name) => name === "Drum loop 128")).toHaveLength(
      1
    )
    const track = audio(first).mixerTrack
    expect([audio(second).mixerTrack, audio(third).mixerTrack]).toEqual([
      track,
      track,
    ])
    // The playlist asks for the track itself, whichever side would decide.
    expect(send.mock.calls.map((call) => call[1].mixerTrack)).toEqual([
      undefined,
      track,
      track,
    ])
    // Each drop is still one undo step.
    expect(labels()).toEqual(["Add audio clip", "Add audio clip", "Add audio clip"])
  })

  it("follows the most recent clip of the file to the track it was routed to", async () => {
    const first = await addLoop(0)
    const second = await addLoop(2 * BAR)
    const kickTrack = project().mixer.tracks[1]
    await changeAudioClips([
      { id: second, patch: { mixerTrack: kickTrack.id } },
    ])
    const third = await addLoop(4 * BAR)
    expect(audio(third).mixerTrack).toBe(kickTrack.id)
    expect(audio(first).mixerTrack).not.toBe(kickTrack.id)
  })

  it("makes a track again once the clips of the file are gone, and for another file", async () => {
    const first = await addLoop(0)
    const count = project().mixer.tracks.length
    // Another file gets a track of its own.
    const other = (await addAudioFile("/factory/Drums/Kicks/Kick 02.wav", {
      start: 0,
    })) as number
    await settle()
    expect(project().mixer.tracks).toHaveLength(count + 1)
    expect(audio(other).mixerTrack).not.toBe(audio(first).mixerTrack)

    await dispatch({ type: "removeClips", clips: [first] })
    const again = await addLoop(0)
    expect(project().mixer.tracks).toHaveLength(count + 2)
    expect(project().mixer.tracks.at(-1)?.id).toBe(audio(again).mixerTrack)
  })

  it("is the mock's rule too, when no track is asked for", async () => {
    const first = await backend.addAudioClipFromFile(LOOP, { start: 0 })
    const second = await backend.addAudioClipFromFile(LOOP, { start: 2 * BAR })
    await settle()
    const track = audio(first.created.at(-1)!).mixerTrack
    expect(audio(second.created.at(-1)!).mixerTrack).toBe(track)
    // No mixer track was made the second time: only the playlist track
    // and the clip came with the sample.
    expect(second.created).toHaveLength(3)
    const sample = audio(first.created.at(-1)!).sample
    const third = await backend.addAudioClipFromSample(sample, {
      start: 4 * BAR,
    })
    await settle()
    expect(audio(third.created.at(-1)!).mixerTrack).toBe(track)
  })
})

describe("dropping a sound on the timeline", () => {
  it("adds the clip with what it needs, as one undo step", async () => {
    const before = project()
    const send = vi.spyOn(backend, "addAudioClipFromFile")
    const id = await addLoop(BAR)

    expect(send).toHaveBeenCalledWith(LOOP, {
      track: undefined,
      start: BAR,
      mixerTrack: undefined,
    })
    expect(labels()).toEqual(["Add audio clip"])
    const clip = clipOf(id)
    expect(clip).toMatchObject({ start: BAR, length: LOOP_TICKS, offset: 0 })
    // A new playlist track, a new mixer track named after the file, and
    // the sample in the pool.
    expect(tracks()).toHaveLength(1)
    expect(clip.track).toBe(tracks()[0].id)
    expect(project().mixer.tracks.at(-1)).toMatchObject({
      id: audio(id).mixerTrack,
      name: "Drum loop 128",
    })
    expect(project().samples.at(-1)).toMatchObject({
      id: audio(id).sample,
      name: "Drum loop 128",
    })
    expect(audio(id)).toMatchObject({ gain: 1, pan: 0, fadeIn: 0, pitch: 0 })
    // The new clip is the selection.
    expect([...ui().selection]).toEqual([id])

    await undo()
    expect(clips()).toEqual([])
    expect(project().samples).toEqual(before.samples)
    expect(project().mixer).toEqual(before.mixer)
    expect(tracks()).toEqual([])
    await redo()
    expect(clipOf(id).length).toBe(LOOP_TICKS)
  })

  it("lands on the track under the pointer, and makes one on an empty row", async () => {
    await dispatch({ type: "addPlaylistTrack" })
    const first = await addLoop(0, 0)
    expect(clipOf(first).track).toBe(tracks()[0].id)
    expect(tracks()).toHaveLength(1)
    // Row 5 has no track: the shell adds one at the end.
    const second = await addLoop(0, 5)
    expect(tracks()).toHaveLength(2)
    expect(clipOf(second).track).toBe(tracks()[1].id)
  })

  it("shows where it will land while it is dragged over the grid", async () => {
    const { session, stop: end } = startSession()
    expect(session.dropPreview).toBeNull()
    const place = session.previewDrop(
      { x: (2 * BAR + 900) * PX_PER_TICK, y: 3 * ROW_HEIGHT + 4, alt: false },
      "Drum loop 128",
      LOOP_TICKS
    )
    // The bar the pointer is in, on the row it is over.
    expect(place).toEqual({ row: 3, start: 2 * BAR })
    expect(session.dropPreview).toEqual({
      row: 3,
      start: 2 * BAR,
      length: LOOP_TICKS,
      name: "Drum loop 128",
    })
    // Alt lets go of the snap. Until the file is read it shows a bar.
    const free = session.previewDrop(
      { x: (2 * BAR + 900) * PX_PER_TICK, y: 4, alt: true },
      "Drum loop 128"
    )
    expect(free).toEqual({ row: 0, start: 2 * BAR + 900 })
    expect(session.dropPreview?.length).toBe(BAR)
    session.previewDrop(null)
    expect(session.dropPreview).toBeNull()
    end()
  })

  it("says why a file that cannot be read was not added", async () => {
    expect(await addAudioFile("/factory/Nope.wav", { start: 0 })).toBeNull()
    expect(toast.error).toHaveBeenCalledWith(
      "Could not add the sound to the playlist",
      expect.objectContaining({ description: expect.any(String) })
    )
    expect(steps()).toBe(0)
  })
})

describe("what only audio clips have", () => {
  it.each<[string, Parameters<typeof patchSelectedAudioClips>[0], string]>([
    ["gain", { gain: 0.5 }, "Change clip gain"],
    ["pan", { pan: -0.75 }, "Change clip pan"],
    ["pitch", { pitch: 7 }, "Change clip pitch"],
    ["reverse", { reverse: true }, "Reverse clip"],
    ["fade in", { fadeIn: 480 }, "Change clip fade"],
    ["fade out", { fadeOut: 960 }, "Change clip fade"],
  ])("sets the %s as one undo step", async (_name, patch, label) => {
    const id = await addLoop()
    const before = audio(id)
    const count = steps()
    expect(await patchSelectedAudioClips(patch)).toBe(true)
    expect(audio(id)).toMatchObject(patch)
    expect(steps()).toBe(count + 1)
    expect(labels().at(-1)).toBe(label)
    await undo()
    expect(audio(id)).toEqual(before)
  })

  it("routes a clip to another mixer track, and to a new one", async () => {
    const id = await addLoop()
    const kick = project().mixer.tracks[1]
    await patchSelectedAudioClips({ mixerTrack: kick.id })
    expect(audio(id).mixerTrack).toBe(kick.id)
    expect(labels().at(-1)).toBe("Route clip")

    const count = steps()
    const mixerTracks = project().mixer.tracks.length
    await routeSelectionToNewTrack()
    expect(project().mixer.tracks).toHaveLength(mixerTracks + 1)
    expect(audio(id).mixerTrack).toBe(project().mixer.tracks.at(-1)?.id)
    // The new track and the routing are one step.
    expect(steps()).toBe(count + 1)
    await undo()
    expect(project().mixer.tracks).toHaveLength(mixerTracks)
    expect(audio(id).mixerTrack).toBe(kick.id)
  })

  it("brings a value into its range, as the document does", async () => {
    const id = await addLoop()
    await changeAudioClips([{ id, patch: { gain: 9, pitch: -99, pan: 3 } }])
    expect(audio(id)).toMatchObject({ gain: 2, pitch: -48, pan: 1 })
  })

  it("changes every selected audio clip in one undo step", async () => {
    const first = await addLoop(0)
    const second = await addLoop(4 * BAR)
    const pattern = project().patterns[0].id
    const added = await dispatch({
      type: "addClips",
      clips: [
        {
          track: tracks()[0].id,
          start: 8 * BAR,
          content: { type: "pattern", pattern },
        },
      ],
    })
    // A pattern clip in the selection is left alone.
    ui().select([first, second, added!.created[0]])
    const send = vi.spyOn(backend, "dispatch")
    const count = steps()
    await patchSelectedAudioClips({ gain: 0.25, fadeOut: 240 })

    expect(send).toHaveBeenCalledTimes(1)
    expect(send.mock.lastCall?.[0]).toEqual({
      type: "updateAudioClips",
      updates: [
        { id: first, patch: { gain: 0.25, fadeOut: 240 } },
        { id: second, patch: { gain: 0.25, fadeOut: 240 } },
      ],
    })
    expect(audio(first)).toMatchObject({ gain: 0.25, fadeOut: 240 })
    expect(audio(second)).toMatchObject({ gain: 0.25, fadeOut: 240 })
    expect(steps()).toBe(count + 1)
    await undo()
    expect(audio(first).gain).toBe(1)
    expect(audio(second).gain).toBe(1)
  })

  it("makes a whole knob drag one undo step for all of them", async () => {
    const first = await addLoop(0)
    const second = await addLoop(4 * BAR)
    ui().select([first, second])
    const count = steps()
    for (const gain of [0.9, 0.7, 0.5]) {
      await patchSelectedAudioClips({ gain }, 4242)
    }
    expect(steps()).toBe(count + 1)
    await undo()
    expect([audio(first).gain, audio(second).gain]).toEqual([1, 1])
  })

  it("reverses the selected audio clips with the action, and says when there are none", async () => {
    const id = await addLoop()
    await runAction("playlist.reverseClips")
    await settle()
    expect(audio(id).reverse).toBe(true)
    await runAction("playlist.reverseClips")
    await settle()
    expect(audio(id).reverse).toBe(false)
  })
})

describe("the handles of an audio clip", () => {
  it("drags the fade in to a length, as one undo step", async () => {
    const id = await addLoop()
    const { session, stop: end } = tallSession()
    const count = steps()
    // The handle is at the top left corner of the waveform, under the
    // title bar, while there is no fade.
    session.pointerDown(spot(40, 0, FADE_Y))
    session.pointerMove(spot(BEAT + 30, 0, FADE_Y))
    expect(session.badgeText).toBe("Fade in: 1 beat, 469 ms")
    // Nothing is sent while the button is down.
    expect(audio(id).fadeIn).toBe(0)
    await session.pointerUp(spot(BEAT + 30, 0, FADE_Y))
    await settle()

    expect(audio(id).fadeIn).toBe(BEAT)
    expect(steps()).toBe(count + 1)
    expect(labels().at(-1)).toBe("Change clip fade")
    expect(session.badgeText).toBeNull()
    await undo()
    expect(audio(id).fadeIn).toBe(0)
    end()
  })

  it("drags the fade out from the other corner, Alt letting go of the snap", async () => {
    const id = await addLoop()
    const { session, stop: end } = tallSession()
    // Snapped, a fade lands on a sixteenth even while clips snap to bars.
    session.pointerDown(spot(LOOP_TICKS - 40, 0, FADE_Y))
    session.pointerMove(spot(LOOP_TICKS - 700, 0, FADE_Y))
    await session.pointerUp(spot(LOOP_TICKS - 700, 0, FADE_Y))
    await settle()
    expect(audio(id).fadeOut).toBe(720)

    session.pointerDown(spot(LOOP_TICKS - 720, 0, FADE_Y))
    const to = { ...spot(LOOP_TICKS - 505, 0, FADE_Y), alt: true }
    session.pointerMove(to)
    await session.pointerUp(to)
    await settle()
    expect(audio(id).fadeOut).toBe(505)

    // It never grows past the clip.
    const handle = spot(LOOP_TICKS - 505, 0, FADE_Y)
    session.pointerDown(handle)
    session.pointerMove(spot(-4 * BAR, 0, FADE_Y))
    await session.pointerUp(spot(-4 * BAR, 0, FADE_Y))
    await settle()
    expect(audio(id).fadeOut).toBe(LOOP_TICKS)
    end()
  })

  it("drags the gain handle up and down and shows the level", async () => {
    const id = await addLoop()
    const { session, stop: end } = tallSession()
    const count = steps()
    // The gain handle is at the right end of the clip's title bar.
    const handle = {
      ...spot(LOOP_TICKS, 0, 8),
      x: LOOP_TICKS * PX_PER_TICK - 14,
    }
    session.pointerDown(handle)
    session.pointerMove({ ...handle, y: handle.y + 24 })
    expect(session.badgeText).toBe("Gain: −6.0 dB")
    await session.pointerUp({ ...handle, y: handle.y + 24 })
    await settle()

    expect(audio(id).gain).toBeCloseTo(10 ** (-6 / 20), 5)
    expect(steps()).toBe(count + 1)
    expect(labels().at(-1)).toBe("Change clip gain")
    end()
  })

  it("leaves the clip alone when a handle is only clicked", async () => {
    const id = await addLoop()
    const { session, stop: end } = tallSession()
    const count = steps()
    await click(session, spot(40, 0, FADE_Y))
    await click(session, {
      ...spot(LOOP_TICKS, 0, 8),
      x: LOOP_TICKS * PX_PER_TICK - 14,
    })
    expect(steps()).toBe(count)
    expect(audio(id)).toMatchObject({ fadeIn: 0, gain: 1 })
    end()
  })

  it("has no handles on a row too short for a title bar: the press moves the clip", async () => {
    const id = await addLoop()
    const { session, stop: end } = startSession()
    await drag(session, at(40, 0), at(BAR + 40, 0))
    expect(clipOf(id).start).toBe(BAR)
    expect(audio(id).fadeIn).toBe(0)
    end()
  })
})

describe("the clip machinery on audio clips", () => {
  it("trims the left edge with the offset, so the audio stays in place", async () => {
    const id = await addLoop(2 * BAR)
    const { session, stop: end } = startSession()
    await drag(session, at(2 * BAR + 20, 0), at(3 * BAR, 0))
    expect(clipOf(id)).toMatchObject({
      start: 3 * BAR,
      offset: BAR,
      length: BAR,
    })
    expect(labels().at(-1)).toBe("Trim clip")

    // Back out again, and no further than where the audio begins.
    await drag(session, at(3 * BAR + 20, 0), at(0, 0))
    expect(clipOf(id)).toMatchObject({
      start: 2 * BAR,
      offset: 0,
      length: LOOP_TICKS,
    })
    end()
  })

  it("cuts with the right edge, and may run past the end of the audio", async () => {
    const id = await addLoop()
    const { session, stop: end } = startSession()
    await drag(session, at(LOOP_TICKS - 20, 0), at(BAR, 0))
    expect(clipOf(id)).toMatchObject({ start: 0, length: BAR, offset: 0 })
    await drag(session, at(BAR - 20, 0), at(5 * BAR, 0))
    expect(clipOf(id).length).toBe(5 * BAR)
    end()
  })

  it("moves, mutes and deletes like any clip", async () => {
    const id = await addLoop()
    const { session, stop: end } = startSession()
    await drag(session, at(BAR, 0), at(3 * BAR, 2))
    expect(clipOf(id).start).toBe(2 * BAR)
    expect(rowOf(clipOf(id))).toBe(2)

    ui().select([id])
    await runAction("playlist.muteClips")
    await settle()
    expect(clipOf(id).muted).toBe(true)
    await runAction("playlist.deleteClips")
    await settle()
    expect(clips()).toEqual([])
    await undo()
    expect(clipOf(id).muted).toBe(true)
    end()
  })

  it("copies a clip with its settings: duplicate, Ctrl+drag and paste", async () => {
    const id = await addLoop()
    await changeAudioClips([
      { id, patch: { gain: 0.5, pitch: -5, reverse: true, fadeIn: 240 } },
    ])
    await dispatch({
      type: "updateClips",
      updates: [{ id, patch: { offset: 480, length: BAR } }],
    })
    const original = clipOf(id)

    ui().select([id])
    await duplicateSelection()
    await settle()
    const { session, stop: end } = startSession()
    await drag(session, at(BAR / 2, 0), at(BAR / 2 + 4 * BAR, 1, { mod: true }))
    ui().select([id])
    copySelection()
    ui().setCursorTick(12 * BAR)
    await paste()
    await settle()

    expect(clips()).toHaveLength(4)
    for (const copy of clips()) {
      expect(copy.content).toEqual(original.content)
      expect(copy).toMatchObject({ offset: 480, length: BAR })
    }
    expect(clips().map((clip) => clip.start)).toEqual([
      0,
      BAR,
      4 * BAR,
      12 * BAR,
    ])
    // Each copy was one command, and one undo step.
    expect(labels().slice(-3)).toEqual([
      "Duplicate clip",
      "Clone clip",
      "Paste clip",
    ])
    end()
  })

  it("keeps the ticks when the tempo changes, so the audio ends elsewhere", async () => {
    const id = await addLoop()
    await dispatch({ type: "updateSettings", patch: { tempoBpm: 64 } })
    expect(clipOf(id)).toMatchObject({ start: 0, length: LOOP_TICKS })
  })
})

describe("the rules of the document", () => {
  it("will not remove a sample a clip still plays", async () => {
    const id = await addLoop()
    const sample = audio(id).sample
    expect(await dispatch({ type: "removeSample", id: sample })).toBeNull()
    expect(toast.error).toHaveBeenCalled()
    expect(project().samples.some((item) => item.id === sample)).toBe(true)
  })

  it("sends a clip to the master when its mixer track is removed", async () => {
    const id = await addLoop()
    await dispatch({ type: "removeMixerTrack", id: audio(id).mixerTrack })
    expect(audio(id).mixerTrack).toBe(0)
    await undo()
    expect(audio(id).mixerTrack).not.toBe(0)
  })
})

describe("placing a sample with the tools", () => {
  const kick = () => project().samples.find((item) => item.name === "Kick")!
  /** 0.42 s at 128 bpm. */
  const KICK_TICKS = Math.ceil(0.42 * 128 * 16)

  it("draws one clip of the picked sample through the shell", async () => {
    ui().setBrush({ type: "audio", sample: kick().id })
    await loadSamplePeaks(kick())
    const send = vi.spyOn(backend, "addAudioClipFromSample")
    const { session, stop: end } = startSession()
    session.pointerDown(at(2 * BAR + 100, 1))
    // The ghost is as long as the sound lasts.
    expect(session.ghosts).toEqual([
      expect.objectContaining({ row: 1, start: 2 * BAR, length: KICK_TICKS }),
    ])
    await session.pointerUp(at(2 * BAR + 100, 1))
    await settle()

    expect(send).toHaveBeenCalledWith(kick().id, {
      track: undefined,
      start: 2 * BAR,
      mixerTrack: undefined,
    })
    expect(labels()).toEqual(["Add audio clip"])
    expect(clips()).toHaveLength(1)
    expect(clips()[0]).toMatchObject({ start: 2 * BAR, length: KICK_TICKS })
    expect(project().mixer.tracks.at(-1)?.name).toBe("Kick")
    end()
  })

  it("plays later clips of a sample into the track the first one made", async () => {
    const first = await placeSampleClips(
      kick(),
      [brushed(0, 0)],
      "Add audio clip"
    )
    const count = project().mixer.tracks.length
    const second = await placeSampleClips(
      kick(),
      [brushed(0, BAR)],
      "Add audio clip"
    )
    expect(project().mixer.tracks).toHaveLength(count)
    expect(audio(second![0]).mixerTrack).toBe(audio(first![0]).mixerTrack)
  })

  it("paints clips back to back with their tracks, as one undo step", async () => {
    ui().setBrush({ type: "audio", sample: kick().id })
    ui().setTool("paint")
    ui().setSnap("none")
    await loadSamplePeaks(kick())
    const before = project()
    const { session, stop: end } = startSession()
    session.pointerDown(at(0, 2))
    session.pointerMove(at(3 * KICK_TICKS + 10, 2))
    expect(session.ghosts).toHaveLength(4)
    await session.pointerUp(at(3 * KICK_TICKS + 10, 2))
    await settle()

    expect(clips().map((clip) => [clip.start, clip.length])).toEqual([
      [0, KICK_TICKS],
      [KICK_TICKS, KICK_TICKS],
      [2 * KICK_TICKS, KICK_TICKS],
      [3 * KICK_TICKS, KICK_TICKS],
    ])
    // Three playlist tracks for row 2, one mixer track for the sound.
    expect(tracks()).toHaveLength(3)
    const mixerTrack = project().mixer.tracks.at(-1)!
    expect(mixerTrack.name).toBe("Kick")
    expect(
      clips().every(
        (clip) => (clip.content as Audio).mixerTrack === mixerTrack.id
      )
    ).toBe(true)
    expect(labels()).toEqual(["Paint clips"])

    await undo()
    expect(clips()).toEqual([])
    expect(tracks()).toEqual([])
    expect(project().mixer).toEqual(before.mixer)
    end()
  })

  function brushed(row: number, start: number) {
    return {
      row,
      start,
      length: 1,
      offset: 0,
      muted: false,
      content: {
        type: "audio",
        sample: kick().id,
        mixerTrack: 0,
        gain: 1,
        pan: 0,
        fadeIn: 0,
        fadeOut: 0,
        reverse: false,
        pitch: 0,
      },
    } as const
  }
})
