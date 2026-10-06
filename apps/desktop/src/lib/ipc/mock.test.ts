import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { ProjectPatch, RealtimeFrame, TransportState } from "@/bindings"
import { TEST_DIALOGS } from "@/test/harness"

import { createMockBackend } from "./mock"

function create() {
  return createMockBackend({ storage: null, dialogs: TEST_DIALOGS })
}

describe("mock backend: document", () => {
  it("starts with the demo project, clean and unsaved", async () => {
    const snapshot = await create().documentSnapshot()
    expect(snapshot).toMatchObject({ revision: 0, dirty: false, path: null })
    expect(snapshot.project.channels).toHaveLength(4)
  })

  it("emits one patch per edit with revisions that count up", async () => {
    const backend = create()
    const seen: ProjectPatch[] = []
    backend.onProjectPatch((patch) => seen.push(patch))

    const first = await backend.dispatch({ type: "addPattern" })
    const second = await backend.dispatch({ type: "addChannel" })
    await backend.undo()

    expect(seen.map((patch) => patch.revision)).toEqual([1, 2, 3])
    expect(seen[0]).toEqual(first.patch)
    expect(second.created).toHaveLength(2)
    expect(seen[2].history.cursor).toBe(1)
    expect(seen[2].channels).toHaveLength(4)
  })

  it("rejects a bad command with a plain message and emits nothing", async () => {
    const backend = create()
    const onPatch = vi.fn()
    backend.onProjectPatch(onPatch)

    await expect(
      backend.dispatch({ type: "removeChannel", id: 12345 })
    ).rejects.toThrow("channel 12345 does not exist")
    expect(onPatch).not.toHaveBeenCalled()
    expect((await backend.documentSnapshot()).revision).toBe(0)
  })

  it("resolves undo and redo to null at the ends of the history", async () => {
    const backend = create()
    expect(await backend.undo()).toBeNull()
    await backend.dispatch({ type: "addPattern" })
    expect(await backend.redo()).toBeNull()
    expect((await backend.historyJump(0)).history.cursor).toBe(0)
  })

  it("adds a channel from a file as one undo step", async () => {
    const backend = create()
    const result = await backend.addChannelFromFile(
      "/factory/Drums/Percussion/Rim 01.wav",
      0
    )
    const { project, history } = await backend.documentSnapshot()

    expect(history.entries).toEqual([{ label: "Add channel" }])
    expect(project.channels[0]).toMatchObject({ name: "Rim 01" })
    expect(project.samples.at(-1)).toMatchObject({
      id: project.channels[0].source.sample,
      path: { kind: "factory", path: "Drums/Percussion/Rim 01.wav" },
    })
    expect(result.created).toHaveLength(3)

    await backend.undo()
    const undone = await backend.documentSnapshot()
    expect(undone.project.channels).toHaveLength(4)
    expect(undone.project.samples).toHaveLength(4)
  })

  it("reuses a sample that is already in the pool", async () => {
    const backend = create()
    const before = await backend.documentSnapshot()
    const channel = before.project.channels[1].id
    await backend.setChannelSampleFromFile(
      channel,
      "/factory/Drums/Kicks/Kick 01.wav"
    )
    const after = await backend.documentSnapshot()
    expect(after.project.samples).toHaveLength(4)
    expect(after.project.channels[1].source.sample).toBe(
      before.project.channels[0].source.sample
    )
    await expect(
      backend.addChannelFromFile("/factory/Drums/Notes.txt")
    ).rejects.toThrow("is not an audio file")
  })
})

describe("mock backend: files", () => {
  it("has no path until the project is saved, then remembers it", async () => {
    const backend = create()
    await backend.dispatch({ type: "addPattern" })
    await expect(backend.projectSave()).rejects.toThrow(
      "This project has no file yet"
    )

    expect(await backend.projectSave("/projects/beat.windfall")).toBe(
      "/projects/beat.windfall"
    )
    expect(await backend.documentSnapshot()).toMatchObject({
      dirty: false,
      path: "/projects/beat.windfall",
    })
    expect(await backend.recentProjects()).toEqual(["/projects/beat.windfall"])
    expect(await backend.projectSave()).toBe("/projects/beat.windfall")
  })

  it("opens what was saved and announces the load", async () => {
    const backend = create()
    const loaded = vi.fn()
    backend.onProjectLoaded(loaded)
    await backend.dispatch({
      type: "updateSettings",
      patch: { name: "Night drive", tempoBpm: 90 },
    })
    await backend.projectSave("/projects/night.windfall")

    const fresh = await backend.projectNew()
    expect(fresh.project.channels).toHaveLength(0)
    expect(fresh).toMatchObject({ revision: 0, path: null, dirty: false })

    const opened = await backend.projectOpen("/projects/night.windfall")
    expect(opened.project.settings).toMatchObject({
      name: "Night drive",
      tempoBpm: 90,
    })
    expect(opened.history.entries).toHaveLength(0)
    expect(loaded).toHaveBeenCalledTimes(2)
    await expect(backend.projectOpen("/nowhere.windfall")).rejects.toThrow(
      'Could not open "/nowhere.windfall".'
    )
  })

  it("keeps saved projects in the storage it is given", async () => {
    const backend = createMockBackend({ dialogs: TEST_DIALOGS })
    await backend.projectSave("/projects/kept.windfall")
    const later = createMockBackend({ dialogs: TEST_DIALOGS })
    expect(await later.recentProjects()).toEqual(["/projects/kept.windfall"])
    expect((await later.projectOpen("/projects/kept.windfall")).path).toBe(
      "/projects/kept.windfall"
    )
  })
})

describe("mock backend: browser and engine", () => {
  it("lists folders before files and describes samples", async () => {
    const backend = create()
    const [factory] = await backend.browserRoots()
    expect(factory).toMatchObject({ kind: "factory" })

    const top = await backend.browserList(factory.path)
    expect(top.map((entry) => entry.kind)).toEqual(["folder", "folder"])
    const hats = await backend.browserList(`${factory.path}/Drums/Hats`)
    expect(hats.map((entry) => entry.name)).toEqual([
      "Closed Hat 01.wav",
      "Closed Hat 02.wav",
      "Open Hat 01.wav",
    ])

    const info = await backend.sampleInfo(hats[0].path)
    expect(info.name).toBe("Closed Hat 01")
    expect(info.peaks.length % 2).toBe(0)
    expect(info.frames).toBe(Math.round(info.durationSecs * info.sampleRate))
    await expect(backend.sampleInfo("/factory/Drums")).rejects.toThrow()
  })

  it("adds and removes user folders but not the factory library", async () => {
    const backend = create()
    const roots = await backend.browserAddRoot("/samples/Mine/")
    expect(roots.at(-1)).toEqual({
      name: "Mine",
      path: "/samples/Mine",
      kind: "user",
    })
    await expect(backend.browserAddRoot("/samples/Mine")).rejects.toThrow(
      "already in the browser"
    )
    expect(await backend.browserRemoveRoot("/samples/Mine")).toHaveLength(1)
    await expect(backend.browserRemoveRoot("/factory")).rejects.toThrow(
      "cannot be removed"
    )
  })

  it("reports an engine error for a device that does not exist", async () => {
    const backend = create()
    const onStatus = vi.fn()
    backend.onEngineStatus(onStatus)

    expect(await backend.engineStatus()).toMatchObject({
      running: true,
      host: "WASAPI",
    })
    const failed = await backend.engineConfigure({ device: "Unplugged" })
    expect(failed.running).toBe(false)
    expect(failed.error).toContain("Unplugged")
    expect(onStatus).toHaveBeenCalledWith(failed)

    const hosts = await backend.engineDevices()
    const working = await backend.engineConfigure({
      host: hosts[1].name,
      bufferFrames: 128,
      sampleRate: 96_000,
    })
    expect(working).toMatchObject({ running: true, bufferFrames: 128 })
    expect(working.latencyMs).toBeCloseTo(1.333, 2)
  })
})

describe("mock backend: transport", () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  const flush = () => vi.advanceTimersByTimeAsync(0)

  it("moves the playhead with the tempo and loops the pattern", async () => {
    const backend = create()
    const frames: RealtimeFrame[] = []
    const stopFrames = backend.subscribeRealtime((frame) => frames.push(frame))

    await vi.advanceTimersByTimeAsync(100)
    expect(frames.at(-1)).toMatchObject({ playing: false, tick: 0 })

    void backend.transportPlay()
    await vi.advanceTimersByTimeAsync(1000)
    const playing = frames.at(-1)
    // 128 BPM is 2048 ticks a second.
    expect(playing?.tick).toBeGreaterThan(1800)
    expect(playing?.tick).toBeLessThan(2200)
    expect(playing?.meters).toHaveLength(10)
    expect(
      Math.max(...frames.flatMap((frame) => frame.meters))
    ).toBeGreaterThan(0.3)

    await vi.advanceTimersByTimeAsync(1500)
    expect(frames.at(-1)?.tick).toBeLessThan(3840)

    void backend.transportStop()
    await vi.advanceTimersByTimeAsync(50)
    expect(frames.at(-1)).toMatchObject({ playing: false, tick: 0 })
    stopFrames()
  })

  it("announces every transport change", async () => {
    const backend = create()
    const states: TransportState[] = []
    backend.onTransportState((state) => states.push(state))

    void backend.transportToggle()
    void backend.transportSet({ mode: "song" })
    void backend.transportToggle()
    await flush()

    expect(states.map((state) => [state.playing, state.mode])).toEqual([
      [true, "pattern"],
      [true, "song"],
      [false, "song"],
    ])
    await expect(backend.transportSet({ pattern: 999 })).rejects.toThrow(
      "pattern 999 does not exist"
    )
  })

  it("moves the transport off a pattern that is deleted", async () => {
    const backend = create()
    const states: TransportState[] = []
    backend.onTransportState((state) => states.push(state))
    const added = backend.dispatch({ type: "addPattern" })
    await flush()
    const id = (await added).created[0]

    void backend.transportSet({ pattern: id })
    void backend.dispatch({ type: "removePattern", id })
    await flush()
    expect(states.at(-1)?.pattern).toBe(1)
  })

  it("reports export progress until it is done", async () => {
    const backend = create()
    const fractions: number[] = []
    const done = vi.fn()
    backend.onExportProgress((progress) => {
      fractions.push(progress.fraction)
      if (progress.done) done(progress.path)
    })
    void backend.exportAudio({
      path: "/exports/beat.wav",
      format: "wav",
      bitDepth: "int24",
      sampleRate: 48_000,
      mode: "pattern",
      patternLoops: 2,
      tailSecs: 1,
    })
    await vi.advanceTimersByTimeAsync(3000)

    expect(done).toHaveBeenCalledExactlyOnceWith("/exports/beat.wav")
    expect(fractions.at(-1)).toBe(1)
    expect([...fractions].sort((a, b) => a - b)).toEqual(fractions)
  })
})
