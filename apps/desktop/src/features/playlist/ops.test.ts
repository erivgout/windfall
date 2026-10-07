import { fireEvent } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { runAction } from "@/lib/actions"
import type { Backend } from "@/lib/ipc"
import { dispatch, undo } from "@/lib/store/project"
import { realtimeFrame, subscribeRealtime } from "@/lib/store/realtime"
import { setPlayMode, useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"
import { settle } from "@/test/harness"

import { installPlaylistKeys, playlistShortcut } from "./keys"
import {
  addClips,
  applySongCursor,
  copySelection,
  insertTrack,
  paste,
  playSong,
  seekSong,
} from "./ops"
import {
  answerConfirm,
  answerText,
  BAR,
  clips,
  history,
  labels,
  layout,
  project,
  selection,
  startPlaylist,
  STEP,
  tracks,
  ui,
} from "./test-utils"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let backend: Backend
let stop: () => void

beforeEach(async () => {
  ;({ backend, stop } = await startPlaylist())
})
afterEach(() => stop())

const pattern = () => project().patterns[0]
const steps = () => history().entries.length

async function seed(...spots: [row: number, start: number, length?: number][]) {
  const created = await addClips(
    spots.map(([row, start, length = BAR]) => ({
      row,
      start,
      length,
      offset: 0,
      muted: false,
      pattern: pattern().id,
    })),
    "Seed"
  )
  await settle()
  return created ?? []
}

async function run(id: string) {
  await runAction(id)
  await settle()
}

describe("clipboard", () => {
  it("pastes copies at the song position, snapped, on the same rows", async () => {
    const ids = await seed([0, BAR, 2 * BAR], [2, 2 * BAR])
    await dispatch({
      type: "updateClips",
      updates: [{ id: ids[0], patch: { offset: STEP, muted: true } }],
    })
    ui().select(ids)
    await run("playlist.copy")
    ui().setCursorTick(8 * BAR + 500)
    const before = steps()
    await run("playlist.paste")

    expect(clips().slice(2)).toEqual([
      expect.objectContaining({
        row: 0,
        start: 8 * BAR,
        length: 2 * BAR,
        offset: STEP,
        muted: true,
      }),
      expect.objectContaining({ row: 2, start: 9 * BAR, length: BAR }),
    ])
    expect(steps()).toBe(before + 1)
    expect(labels().at(-1)).toBe("Paste clips")
    expect(selection()).toEqual(
      clips()
        .slice(2)
        .map((clip) => clip.id)
    )
    await undo()
    expect(clips()).toHaveLength(2)
  })

  it("pastes at the playhead in song mode", async () => {
    const ids = await seed([0, 0])
    ui().select(ids)
    copySelection()
    const off = subscribeRealtime(() => {})
    await setPlayMode("song")
    await backend.transportSeek(5 * BAR + 100)
    await vi.waitFor(() => expect(realtimeFrame().tick).toBe(5 * BAR + 100))
    await paste()
    off()
    expect(layout()).toEqual([`0:0+${BAR}`, `0:${5 * BAR}+${BAR}`])
  })

  it("makes the tracks pasted clips need, in the same undo step", async () => {
    const ids = await seed([0, 0], [3, 0])
    ui().select(ids)
    await run("playlist.copy")
    for (const track of tracks().slice(1)) {
      await dispatch({ type: "removePlaylistTrack", id: track.id })
    }
    expect(tracks()).toHaveLength(1)
    const before = steps()
    ui().setCursorTick(4 * BAR)
    await run("playlist.paste")
    expect(tracks()).toHaveLength(4)
    expect(layout()).toEqual([
      `0:0+${BAR}`,
      `0:${4 * BAR}+${BAR}`,
      `3:${4 * BAR}+${BAR}`,
    ])
    expect(steps()).toBe(before + 1)
  })

  it("cuts, and pastes what was cut", async () => {
    const ids = await seed([0, 0], [0, BAR], [1, BAR])
    ui().select(ids.slice(1))
    const before = steps()
    await run("playlist.cut")
    expect(layout()).toEqual([`0:0+${BAR}`])
    expect(steps()).toBe(before + 1)
    expect(labels().at(-1)).toBe("Cut clips")
    ui().setCursorTick(3 * BAR)
    await run("playlist.paste")
    expect(layout()).toEqual([
      `0:0+${BAR}`,
      `0:${3 * BAR}+${BAR}`,
      `1:${3 * BAR}+${BAR}`,
    ])
  })

  it("leaves out copies of a pattern that was deleted since", async () => {
    const added = await dispatch({ type: "addPattern", name: "Gone" })
    const gone = added!.created[0]
    const [kept] = await seed([0, 0])
    const made = await dispatch({
      type: "addClips",
      clips: [
        {
          track: tracks()[0].id,
          start: BAR,
          content: { type: "pattern", pattern: gone },
        },
      ],
    })
    ui().select([kept, made!.created[0]])
    await run("playlist.copy")
    await dispatch({ type: "removePattern", id: gone })
    ui().setCursorTick(4 * BAR)
    await run("playlist.paste")
    expect(layout()).toEqual([`0:0+${BAR}`, `0:${4 * BAR}+${BAR}`])
  })

  it("cannot paste before anything was copied", async () => {
    await seed([0, 0])
    const before = steps()
    await run("playlist.paste")
    expect(steps()).toBe(before)
  })
})

describe("selection actions", () => {
  it("duplicates the selection right after itself and selects the copies", async () => {
    const ids = await seed([0, BAR, 2 * BAR], [1, 2 * BAR])
    ui().select(ids)
    const before = steps()
    await run("playlist.duplicate")
    expect(layout()).toEqual([
      `0:${BAR}+${2 * BAR}`,
      `1:${2 * BAR}+${BAR}`,
      `0:${3 * BAR}+${2 * BAR}`,
      `1:${4 * BAR}+${BAR}`,
    ])
    expect(steps()).toBe(before + 1)
    expect(labels().at(-1)).toBe("Duplicate clips")
    expect(selection()).toHaveLength(2)
    expect(selection().some((id) => ids.includes(id))).toBe(false)
  })

  it("selects every clip and deletes the selection in one step", async () => {
    await seed([0, 0], [1, BAR], [4, 9 * BAR])
    await run("playlist.selectAll")
    expect(selection()).toHaveLength(3)
    const before = steps()
    await run("playlist.deleteClips")
    expect(clips()).toHaveLength(0)
    expect(steps()).toBe(before + 1)
    expect(selection()).toEqual([])
  })

  it("mutes the selection, and unmutes it when all of it is muted", async () => {
    const ids = await seed([0, 0], [1, 0])
    await dispatch({
      type: "updateClips",
      updates: [{ id: ids[0], patch: { muted: true } }],
    })
    ui().select(ids)
    await run("playlist.muteClips")
    expect(clips().map((clip) => clip.muted)).toEqual([true, true])
    await run("playlist.muteClips")
    expect(clips().map((clip) => clip.muted)).toEqual([false, false])
  })

  it("nudges by the snap with the arrow actions", async () => {
    const ids = await seed([1, 2 * BAR])
    ui().select(ids)
    const before = steps()
    await run("playlist.nudgeRight")
    expect(layout()).toEqual([`1:${3 * BAR}+${BAR}`])
    ui().setSnap("beat")
    await run("playlist.nudgeLeft")
    expect(layout()).toEqual([`1:${3 * BAR - 960}+${BAR}`])
    ui().setSnap("none")
    await run("playlist.nudgeLeft")
    expect(layout()).toEqual([`1:${3 * BAR - 960 - STEP}+${BAR}`])
    expect(steps()).toBe(before + 3)
  })

  it("moves between tracks with up and down, making a track at the bottom", async () => {
    const ids = await seed([0, 0], [1, BAR])
    ui().select(ids)
    await run("playlist.moveDown")
    expect(layout()).toEqual([`1:0+${BAR}`, `2:${BAR}+${BAR}`])
    expect(tracks()).toHaveLength(3)
    await run("playlist.moveUp")
    await run("playlist.moveUp")
    expect(layout()).toEqual([`0:0+${BAR}`, `1:${BAR}+${BAR}`])
  })

  it("stops nudging at the start of the song", async () => {
    const ids = await seed([0, 0])
    ui().select(ids)
    const before = steps()
    await run("playlist.nudgeLeft")
    expect(layout()).toEqual([`0:0+${BAR}`])
    expect(steps()).toBe(before)
  })

  it("only acts while the playlist is the editor in view", async () => {
    const ids = await seed([0, 0])
    ui().select(ids)
    useUiStore.getState().showCenterTab("channelRack")
    await run("playlist.deleteClips")
    expect(clips()).toHaveLength(1)
  })
})

describe("another project", () => {
  it("drops the selection, the clipboard and the song position", async () => {
    const ids = await seed([0, 0])
    ui().select(ids)
    copySelection()
    ui().setTargetTrack(tracks()[0].id)
    ui().setCursorTick(4 * BAR)
    void runAction("file.new")
    await answerConfirm("discard")
    expect(project().playlist.clips).toHaveLength(0)
    expect(ui()).toMatchObject({
      targetTrack: null,
      cursorTick: 0,
      clipboard: [],
    })
    expect(selection()).toEqual([])
  })
})

describe("tracks", () => {
  it("adds a track at the bottom", async () => {
    await run("playlist.addTrack")
    await run("playlist.addTrack")
    expect(tracks().map((track) => track.name)).toEqual(["Track 1", "Track 2"])
    expect(ui().targetTrack).toBe(tracks()[1].id)
  })

  it("inserts an empty track above another, as one undo step", async () => {
    await seed([0, 0], [1, BAR], [2, 2 * BAR])
    await dispatch({
      type: "updatePlaylistTrack",
      id: tracks()[1].id,
      patch: { name: "Bass", muted: true },
    })
    const before = steps()
    ui().setTargetTrack(tracks()[1].id)
    await run("playlist.insertTrack")

    expect(tracks().map((track) => [track.name, track.muted])).toEqual([
      ["Track 1", false],
      ["Track 4", false],
      ["Bass", true],
      ["Track 3", false],
    ])
    expect(layout()).toEqual([
      `0:0+${BAR}`,
      `2:${BAR}+${BAR}`,
      `3:${2 * BAR}+${BAR}`,
    ])
    expect(steps()).toBe(before + 1)
    expect(labels().at(-1)).toBe("Insert track")
    await undo()
    expect(tracks().map((track) => track.name)).toEqual([
      "Track 1",
      "Bass",
      "Track 3",
    ])
    expect(layout()).toEqual([
      `0:0+${BAR}`,
      `1:${BAR}+${BAR}`,
      `2:${2 * BAR}+${BAR}`,
    ])
  })

  it("inserting below the last track just adds one", async () => {
    await seed([0, 0])
    await insertTrack(5)
    expect(tracks()).toHaveLength(2)
  })

  it("renames and mutes the target track", async () => {
    await seed([0, 0])
    ui().setTargetTrack(tracks()[0].id)
    void runAction("playlist.renameTrack")
    await answerText("  Drums ")
    expect(tracks()[0].name).toBe("Drums")
    await run("playlist.muteTrack")
    expect(tracks()[0].muted).toBe(true)
  })

  it("asks before deleting a track that has clips", async () => {
    await seed([0, 0], [0, BAR], [1, 0])
    const doomed = tracks()[0].id
    ui().setTargetTrack(doomed)
    void runAction("playlist.deleteTrack")
    await answerConfirm(null)
    expect(tracks()).toHaveLength(2)

    void runAction("playlist.deleteTrack")
    await answerConfirm("delete")
    expect(tracks()).toHaveLength(1)
    expect(layout()).toEqual([`0:0+${BAR}`])
    expect(ui().targetTrack).toBeNull()
    await undo()
    expect(clips()).toHaveLength(3)
  })

  it("deletes an empty track without asking", async () => {
    await run("playlist.addTrack")
    await run("playlist.deleteTrack")
    expect(tracks()).toHaveLength(0)
  })

  it("has nothing to act on without a target track", async () => {
    await seed([0, 0])
    await run("playlist.muteTrack")
    await run("playlist.deleteTrack")
    expect(tracks().map((track) => track.muted)).toEqual([false])
  })
})

describe("song position", () => {
  it("only remembers the place while the transport loops the pattern", async () => {
    const seek = vi.spyOn(backend, "transportSeek")
    await seekSong(6 * BAR)
    expect(seek).not.toHaveBeenCalled()
    expect(ui().cursorTick).toBe(6 * BAR)
  })

  it("moves the engine's playhead in song mode", async () => {
    await setPlayMode("song")
    const seek = vi.spyOn(backend, "transportSeek")
    await seekSong(6 * BAR)
    expect(seek).toHaveBeenCalledWith(6 * BAR)
    await seekSong(-50)
    expect(seek).toHaveBeenLastCalledWith(0)
  })

  it("plays the song from the remembered place in one go", async () => {
    await seed([0, 0, 16 * BAR])
    await seekSong(4 * BAR)
    const seek = vi.spyOn(backend, "transportSeek")
    await playSong()
    expect(useTransportStore.getState()).toMatchObject({
      mode: "song",
      playing: true,
    })
    expect(seek).toHaveBeenCalledTimes(1)
    expect(seek).toHaveBeenCalledWith(4 * BAR)
  })

  it("applies the remembered place when song mode is switched on elsewhere", async () => {
    await seekSong(2 * BAR)
    const seek = vi.spyOn(backend, "transportSeek")
    await setPlayMode("song")
    await applySongCursor()
    expect(seek).toHaveBeenCalledWith(2 * BAR)
  })

  it("toggles looping the song", async () => {
    const before = useTransportStore.getState().loopSong
    await run("playlist.loopSong")
    expect(useTransportStore.getState().loopSong).toBe(!before)
  })
})

describe("keys inside the playlist", () => {
  let root: HTMLElement
  let uninstall: () => void

  beforeEach(() => {
    root = document.createElement("div")
    document.body.append(root)
    uninstall = installPlaylistKeys(root)
  })
  afterEach(() => {
    uninstall()
    root.remove()
  })

  const press = (init: KeyboardEventInit) => {
    fireEvent.keyDown(document.body, init)
    return settle()
  }

  it("deletes the selected clips with Delete", async () => {
    const ids = await seed([0, 0], [0, BAR])
    ui().select([ids[1]])
    await press({ key: "Delete", code: "Delete" })
    expect(layout()).toEqual([`0:0+${BAR}`])
  })

  it("nudges with the arrows and duplicates with Ctrl+D", async () => {
    const ids = await seed([0, 0])
    ui().select(ids)
    await press({ key: "ArrowRight", code: "ArrowRight" })
    await press({ key: "ArrowDown", code: "ArrowDown" })
    expect(layout()).toEqual([`1:${BAR}+${BAR}`])
    await press({ key: "d", code: "KeyD", ctrlKey: true })
    expect(layout()).toEqual([`1:${BAR}+${BAR}`, `1:${2 * BAR}+${BAR}`])
  })

  it("copies and pastes with Ctrl+C and Ctrl+V, and selects all with Ctrl+A", async () => {
    await seed([0, 0])
    await press({ key: "a", code: "KeyA", ctrlKey: true })
    await press({ key: "c", code: "KeyC", ctrlKey: true })
    ui().setCursorTick(3 * BAR)
    await press({ key: "v", code: "KeyV", ctrlKey: true })
    expect(layout()).toEqual([`0:0+${BAR}`, `0:${3 * BAR}+${BAR}`])
  })

  it("switches tools with letters", async () => {
    await press({ key: "b", code: "KeyB" })
    expect(ui().tool).toBe("paint")
    await press({ key: "s", code: "KeyS" })
    expect(ui().tool).toBe("select")
    await press({ key: "e", code: "KeyE" })
    expect(ui().tool).toBe("erase")
    await press({ key: "m", code: "KeyM" })
    expect(ui().tool).toBe("mute")
    await press({ key: "d", code: "KeyD" })
    expect(ui().tool).toBe("draw")
  })

  it("uses FL Studio's letters with the FL keymap", async () => {
    useUiStore.getState().setKeymap("fl")
    await press({ key: "b", code: "KeyB" })
    expect(ui().tool).toBe("paint")
    await press({ key: "e", code: "KeyE" })
    expect(ui().tool).toBe("select")
    await press({ key: "d", code: "KeyD" })
    expect(ui().tool).toBe("erase")
    await press({ key: "t", code: "KeyT" })
    expect(ui().tool).toBe("mute")
    await press({ key: "p", code: "KeyP" })
    expect(ui().tool).toBe("draw")

    const ids = await seed([0, 0])
    ui().select(ids)
    await press({ key: "b", code: "KeyB", ctrlKey: true })
    expect(clips()).toHaveLength(2)
    expect(playlistShortcut("playlist.duplicate")).toBe("Ctrl+B")
    expect(playlistShortcut("playlist.toolDraw")).toBe("P")
  })

  it("leaves keys alone while the focus is in another panel", async () => {
    const ids = await seed([0, 0])
    ui().select(ids)
    const elsewhere = document.createElement("button")
    document.body.append(elsewhere)
    elsewhere.focus()
    fireEvent.keyDown(elsewhere, { key: "Delete", code: "Delete" })
    await settle()
    expect(clips()).toHaveLength(1)
    elsewhere.remove()
  })

  it("leaves keys alone while a name is being typed", async () => {
    const ids = await seed([0, 0])
    ui().select(ids)
    const field = document.createElement("input")
    root.append(field)
    field.focus()
    fireEvent.keyDown(field, { key: "Delete", code: "Delete" })
    fireEvent.keyDown(field, { key: "b", code: "KeyB" })
    await settle()
    expect(clips()).toHaveLength(1)
    expect(ui().tool).toBe("draw")
  })

  it("clears the selection with Escape", async () => {
    const ids = await seed([0, 0])
    ui().select(ids)
    await press({ key: "Escape", code: "Escape" })
    expect(selection()).toEqual([])
  })
})
