import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type {
  DocumentSnapshot,
  EngineStatus,
  TransportState,
} from "@/bindings"
import { backend, setBackend, type Backend } from "@/lib/ipc"
import { createMockBackend } from "@/lib/ipc/mock"
import { settle, startTestApp, TEST_DIALOGS } from "@/test/harness"

import { connectStores } from "./connect"
import { configureEngine, useEngineStore } from "./engine"
import { dispatch, useProjectStore } from "./project"
import {
  play,
  setPlayMode,
  stop as stopPlayback,
  togglePlayback,
  useTransportStore,
} from "./transport"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

const { toast } = await import("sonner")

/** A promise settled from outside, to choose when a reply arrives. */
function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((onResolve) => {
    resolve = onResolve
  })
  return { promise, resolve }
}

/** Events the test sends by hand, in place of the backend's own. */
function emitter<T>() {
  const handlers = new Set<(value: T) => void>()
  return {
    on(handler: (value: T) => void) {
      handlers.add(handler)
      return () => void handlers.delete(handler)
    },
    emit(value: T) {
      for (const handler of handlers) handler(value)
    },
  }
}

let stop: () => void
let app: Awaited<ReturnType<typeof startTestApp>>

beforeEach(async () => {
  vi.mocked(toast.error).mockClear()
  app = await startTestApp()
  stop = app.stop
})
afterEach(() => stop())

describe("how a project was being played", () => {
  it("comes back with the project: a song reopens in Song mode", async () => {
    const backend = app.backend
    await setPlayMode("song")
    await backend.transportSet({ loopSong: false })
    await settle()
    const path = await backend.projectSave("/projects/song.windfall")

    await backend.projectNew()
    await settle()
    expect(useTransportStore.getState().mode).toBe("pattern")

    // The reply of the open is not what sets the store: the event is, and
    // it comes after the project it is about.
    const order: string[] = []
    const offLoaded = backend.onProjectLoaded(() => order.push("project"))
    const offTransport = backend.onTransportState((state) =>
      order.push(`transport ${state.mode}`)
    )
    await backend.projectOpen(path)
    await settle()
    expect(order).toEqual(["project", "transport song"])
    expect(useTransportStore.getState()).toMatchObject({
      mode: "song",
      loopSong: false,
      playing: false,
      pattern: useProjectStore.getState().project.patterns[0].id,
    })
    offLoaded()
    offTransport()
  })

  it("is the pattern for a project saved that way, whatever was playing before", async () => {
    const backend = app.backend
    const path = await backend.projectSave("/projects/beat.windfall")
    await setPlayMode("song")
    expect(useTransportStore.getState().mode).toBe("song")
    await backend.projectOpen(path)
    await settle()
    expect(useTransportStore.getState().mode).toBe("pattern")
  })

  it("takes the transport from the event that follows project:loaded, not from an older reply", async () => {
    const loaded = emitter<DocumentSnapshot>()
    const transports = emitter<TransportState>()
    await connectTo({
      onProjectLoaded: loaded.on,
      onTransportState: transports.on,
    })
    const snapshot = await backend.documentSnapshot()
    expect(useTransportStore.getState().mode).toBe("pattern")
    // The shell opens a song: project first, then how it was being played.
    loaded.emit(snapshot)
    transports.emit({
      playing: false,
      mode: "song",
      pattern: snapshot.project.patterns[0].id,
      loopSong: false,
    })
    expect(useTransportStore.getState()).toMatchObject({
      mode: "song",
      loopSong: false,
    })
  })
})

/** Connects the stores to a mock with some of its calls replaced. */
async function connectTo(overrides: Partial<Backend>) {
  stop()
  const mock = createMockBackend({ storage: null, dialogs: TEST_DIALOGS })
  setBackend({ ...mock, ...overrides })
  useTransportStore.setState(useTransportStore.getInitialState(), true)
  useEngineStore.setState(useEngineStore.getInitialState(), true)
  stop = connectStores()
  await settle()
}

const transport = () => useTransportStore.getState()

describe("transport store", () => {
  it("follows the backend through replies and events", async () => {
    await play()
    expect(transport().playing).toBe(true)
    await stopPlayback()
    expect(transport().playing).toBe(false)
    await setPlayMode("song")
    expect(transport().mode).toBe("song")
  })

  it("does not light Play for a play the backend refused", async () => {
    await setPlayMode("song")
    await togglePlayback()
    expect(transport().playing).toBe(false)
    expect(toast.error).toHaveBeenCalledWith(
      expect.stringContaining("The playlist is empty")
    )
    await play()
    expect(transport().playing).toBe(false)

    // With a clip to play, the same call starts the song.
    const track = await dispatch({ type: "addPlaylistTrack" })
    if (!track) throw new Error("could not add a track")
    const pattern = useProjectStore.getState().project.patterns[0].id
    await dispatch({
      type: "addClips",
      clips: [
        {
          track: track.created[0],
          start: 0,
          content: { type: "pattern", pattern },
        },
      ],
    })
    await togglePlayback()
    expect(transport().playing).toBe(true)
  })

  it("keeps a newer event when the stale reply of the command arrives after it", async () => {
    const reply = deferred<TransportState>()
    const events = emitter<TransportState>()
    await connectTo({
      transportPlay: () => reply.promise,
      onTransportState: events.on,
    })
    const base = transport()

    // The engine starts and stops at once: the events say so first, and
    // the reply, made while it was still playing, comes in last.
    const playing = play()
    events.emit({ ...base, playing: true })
    events.emit({ ...base, playing: false })
    reply.resolve({ ...base, playing: true })
    await playing

    expect(transport().playing).toBe(false)
  })

  it("uses the reply when no event has said anything since", async () => {
    const reply = deferred<TransportState>()
    const events = emitter<TransportState>()
    await connectTo({
      transportPlay: () => reply.promise,
      onTransportState: events.on,
    })
    const base = transport()

    const playing = play()
    reply.resolve({ ...base, playing: true })
    await playing
    expect(transport().playing).toBe(true)

    // An event after the reply is newer still.
    events.emit({ ...base, playing: false })
    expect(transport().playing).toBe(false)
  })
})

describe("engine store", () => {
  it("keeps a newer status event over the reply of a configure call", async () => {
    const reply = deferred<EngineStatus>()
    const events = emitter<EngineStatus>()
    await connectTo({
      engineConfigure: () => reply.promise,
      onEngineStatus: events.on,
    })
    const status = useEngineStore.getState().status
    if (!status) throw new Error("the engine never reported")

    const configuring = configureEngine({ sampleRate: 96_000 })
    // The device was unplugged while the call was on its way.
    const failed = { ...status, running: false, error: "The device is gone." }
    events.emit(failed)
    reply.resolve({ ...status, sampleRate: 96_000 })
    await configuring

    expect(useEngineStore.getState().status).toEqual(failed)
    // What was asked for is remembered either way.
    expect(useEngineStore.getState().request).toEqual({ sampleRate: 96_000 })
  })

  it("uses the reply of a configure call when no event came in", async () => {
    await connectTo({ onEngineStatus: () => () => undefined })
    await configureEngine({ bufferFrames: 512 })
    expect(useEngineStore.getState().status).toMatchObject({
      running: true,
      bufferFrames: 512,
    })
  })
})
