import { afterEach, describe, expect, it, vi } from "vitest"
import { analyzeInWorker } from "./analyze"
import type { SliceAnalysisRequest } from "./types"

const request: SliceAnalysisRequest = {
  clip: {
    id: 1,
    track: 1,
    start: 0,
    length: 960,
    offset: 0,
    muted: false,
    content: {
      type: "audio",
      sample: 1,
      mixerTrack: 0,
      gain: 1,
      pan: 0,
      fadeIn: 0,
      fadeOut: 0,
      reverse: false,
      pitch: 0,
    },
  },
  sampleRate: 1000,
  channels: 1,
  samples: [0],
  tempo: 120,
  swing: 0,
  options: { mode: "grid", gridTicks: 960 },
}
class WorkerStub {
  static latest: WorkerStub
  onmessage: ((event: { data: unknown }) => void) | null = null
  onerror: ((event: { message: string }) => void) | null = null
  postMessage = vi.fn()
  terminate = vi.fn()
  constructor() {
    WorkerStub.latest = this
  }
}
afterEach(() => {
  vi.unstubAllGlobals()
  vi.useRealTimers()
})
describe("slicer worker lifecycle", () => {
  it("waits until the WASM worker is ready before sending its request", async () => {
    vi.stubGlobal("Worker", WorkerStub)
    const pending = analyzeInWorker(request)
    const worker = WorkerStub.latest
    expect(worker.postMessage).not.toHaveBeenCalled()
    worker.onmessage!({ data: { ready: true } })
    expect(worker.postMessage).toHaveBeenCalledWith(request)
    const analysis = { markers: [], peaks: [0] }
    worker.onmessage!({ data: { analysis } })
    await expect(pending).resolves.toEqual(analysis)
    expect(worker.terminate).toHaveBeenCalledOnce()
  })
  it("terminates on analysis failure or missing worker initialization", async () => {
    vi.stubGlobal("Worker", WorkerStub)
    let pending = analyzeInWorker(request)
    let failure = expect(pending).rejects.toThrow("failed to load")
    WorkerStub.latest.onerror!({ message: "failed to load" })
    await failure
    expect(WorkerStub.latest.terminate).toHaveBeenCalledOnce()
    vi.useFakeTimers()
    pending = analyzeInWorker(request)
    failure = expect(pending).rejects.toThrow("timed out")
    await vi.advanceTimersByTimeAsync(60_000)
    await failure
    expect(WorkerStub.latest.terminate).toHaveBeenCalledOnce()
  })
})
