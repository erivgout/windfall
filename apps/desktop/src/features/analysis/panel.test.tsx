import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { startTestApp } from "@/test/harness"
import type { MockBackend } from "@/lib/ipc/mock"
import { useProjectStore } from "@/lib/store/project"
import { usePlaylistStore } from "@/features/playlist/store"
import { AnalysisPanel } from "./index"
import type { AnalysisJob, AnalysisModel, AnalysisReview } from "./types"

const model: AnalysisModel = {
  id: "test-only-copy",
  version: "1",
  revision: "1",
  sha256: "07".repeat(32),
  bytes: "135",
  maxBytes: "4096",
  sampleRate: 48000,
  inputChannels: 2,
  maxInputFrames: "48000",
  outputs: [{ role: "copy", channels: 2 }],
  provenance: {
    origin: "authored:test",
    sourceRevision: "1",
    author: "fixture",
    licenseSpdx: "CC0-1.0",
    licenseReference: "fixture",
    adapterId: "test-only-copy",
    adapterVersion: "1",
    device: "cpu",
  },
}
const job: AnalysisJob = {
  job: "9007199254740993",
  ticket: "9007199254740994",
  request: "9007199254740995",
  sequence: "4",
  status: "ready",
  completedWork: "100",
  maximumWork: "1000",
  failure: null,
}
const review: AnalysisReview = {
  job,
  clip: 42,
  generation: "1",
  editRevision: "1",
  sourceSha256: "01".repeat(32),
  bindingSha256: "02".repeat(32),
  startFrame: "0",
  endFrame: "100",
  inputFrames: "100",
  model,
  artifacts: [
    {
      name: "copy.wav",
      role: "copy",
      frames: "100",
      channels: 2,
      sampleRate: 48000,
      frameOrigin: "0",
      bytes: "800",
      sha256: "03".repeat(32),
    },
  ],
}
let backend: MockBackend
let stop: () => void
beforeEach(async () => {
  ;({ backend, stop } = await startTestApp())
})
afterEach(() => stop())
async function nativePanel() {
  vi.spyOn(backend, "analysisCapability").mockResolvedValue({
    native: true,
    available: true,
    reason: null,
    models: [model],
  })
  vi.spyOn(backend, "analysisStatus").mockResolvedValue({
    ...job,
    status: "cancelled",
  })
  vi.spyOn(backend, "analysisCancel").mockResolvedValue({
    ...job,
    status: "cancelled",
  })
  vi.spyOn(backend, "analysisForget").mockResolvedValue()
  const view = render(<AnalysisPanel clip={42} />)
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Submit analysis" })
    ).toBeDisabled()
  )
  await screen.findByRole("option", { name: /test-only-copy/ })
  fireEvent.change(screen.getByLabelText("End frame exclusive"), {
    target: { value: "100" },
  })
  return view
}
describe("analysis app controls", () => {
  it("shows honest browser unavailability and preserves document/history", async () => {
    const before = useProjectStore.getState()
    render(<AnalysisPanel clip={42} />)
    expect(await screen.findByRole("status")).toHaveTextContent(
      "No inference runs in the browser"
    )
    expect(
      screen.getByRole("button", { name: "Submit analysis" })
    ).toBeDisabled()
    expect(
      screen.getByRole("button", { name: "Verify and import local model" })
    ).toBeDisabled()
    expect(useProjectStore.getState()).toBe(before)
  })
  it("retires a late submit after unmount instead of retaining a hidden job", async () => {
    let resolve!: (value: AnalysisJob) => void
    const submit = vi.spyOn(backend, "analysisSubmit").mockImplementation(
      () =>
        new Promise((done) => {
          resolve = done
        })
    )
    const view = await nativePanel()
    fireEvent.click(screen.getByRole("button", { name: "Submit analysis" }))
    expect(submit).toHaveBeenCalledWith({
      clip: 42,
      modelId: model.id,
      modelVersion: "1",
      modelRevision: "1",
      startFrame: "0",
      endFrame: "100",
    })
    view.unmount()
    await act(async () => resolve(job))
    expect(backend.analysisForget).toHaveBeenCalledWith(job.job)
  })
  it("discards a late review after the selected clip context changes", async () => {
    vi.spyOn(backend, "analysisSubmit").mockResolvedValue(job)
    let resolve!: (value: AnalysisReview) => void
    vi.spyOn(backend, "analysisReview").mockImplementation(
      () =>
        new Promise((done) => {
          resolve = done
        })
    )
    await nativePanel()
    fireEvent.click(screen.getByRole("button", { name: "Submit analysis" }))
    await screen.findByText(/Job 9007199254740993: ready/)
    fireEvent.click(screen.getByRole("button", { name: "Review outputs" }))
    act(() => usePlaylistStore.setState({ selection: new Set([99]) }))
    await act(async () => resolve(review))
    expect(screen.queryByText(/Source SHA256:/)).toBeNull()
    expect(screen.getByRole("alert")).toHaveTextContent("selection changed")
  })
  it("shows provenance and ignores a late apply patch after actual project replacement", async () => {
    vi.spyOn(backend, "analysisSubmit").mockResolvedValue(job)
    vi.spyOn(backend, "analysisReview").mockResolvedValue(review)
    const result = await backend.addAudioClipFromFile(
      "/factory/Loops/Drum loop 128.wav",
      { start: 0 }
    )
    await backend.projectNew()
    let resolve!: (value: typeof result) => void
    vi.spyOn(backend, "analysisApply").mockImplementation(
      () =>
        new Promise((done) => {
          resolve = done
        })
    )
    await nativePanel()
    fireEvent.click(screen.getByRole("button", { name: "Submit analysis" }))
    await screen.findByText(/Job 9007199254740993: ready/)
    fireEvent.click(screen.getByRole("button", { name: "Review outputs" }))
    await screen.findByText(/License: CC0-1.0/)
    fireEvent.click(
      screen.getByRole("button", { name: "Apply reviewed outputs" })
    )
    expect(backend.analysisApply).toHaveBeenCalledWith({
      ticket: job.ticket,
      request: job.request,
      replaceOriginal: false,
    })
    await act(() => backend.projectNew())
    const before = useProjectStore.getState()
    await act(async () => resolve(result))
    expect(useProjectStore.getState()).toBe(before)
  })
})
