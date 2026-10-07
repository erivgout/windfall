import type { Command, DispatchResult, SampleId, SampleInfo } from "@/bindings"
import { analyzeInWorker } from "@/features/slicer/analyze"
import type {
  SliceAnalysis,
  SliceAudio,
  SliceOptions,
  SliceReview,
} from "@/features/slicer/types"
import { sim } from "./wasm"
import type { SimDocument } from "./document"

export type SlicerMockOptions = {
  /** Optional decoded fixtures for headless tests. Browser library audio is simulated. */
  slicerAudio?: Record<SampleId, SliceAudio>
}

/** Browser samples are made-up fixtures, like the existing browser waveform. */
function fixture(info: SampleInfo): SliceAudio {
  const sampleRate = 8_000
  const frames = Math.ceil(info.durationSecs * sampleRate)
  if (frames > 32_000_000)
    throw new Error("The simulated clip is too long to analyze.")
  return {
    sampleRate,
    channels: 1,
    samples: Array.from({ length: frames }, (_, frame) => {
      const phase = frame % 4_000
      return Math.sin(frame * 0.1) * Math.exp(-phase / 180) * 0.8
    }),
  }
}

export function createSlicerMock(
  options: SlicerMockOptions,
  document: () => SimDocument,
  dispatch: (command: Command) => DispatchResult,
  info: (sample: SampleId) => SampleInfo
) {
  let ticket = 0
  let pending: {
    review: SliceReview
    original: SimDocument
    revision: number
    clip: import("@/bindings").Clip
    audio: SliceAudio
  } | null = null
  const audio = new Map<SampleId, SliceAudio>()
  let audioDocument = document()
  const source = (sample: SampleId) => {
    if (document() !== audioDocument) {
      audio.clear()
      audioDocument = document()
    }
    const supplied = options.slicerAudio?.[sample]
    if (supplied) return supplied
    if (!audio.has(sample)) audio.set(sample, fixture(info(sample)))
    return audio.get(sample)!
  }
  return {
    async sliceAnalyze(
      id: number,
      settings: SliceOptions
    ): Promise<SliceReview> {
      const token = ++ticket
      pending = null
      const original = document()
      const revision = original.snapshot(null).revision
      const project = original.project()
      const clip = project.playlist.clips.find((c) => c.id === id)
      if (!clip || clip.content.type !== "audio")
        throw new Error("Select one playlist audio clip to slice.")
      const buffer = source(clip.content.sample)
      const request = {
        ...buffer,
        clip,
        tempo: project.settings.tempoBpm,
        swing: project.settings.swing,
        options: settings,
      }
      // jsdom has no Workers; tests call the very same Rust export synchronously.
      const analysis =
        typeof Worker === "undefined"
          ? sim.call<SliceAnalysis>("slice_analyze", 0, request)
          : await analyzeInWorker(request)
      if (
        token !== ticket ||
        document() !== original ||
        original.snapshot(null).revision !== revision ||
        source(clip.content.sample) !== buffer
      ) {
        throw new Error(
          "The project or source changed. Analyze the clip again."
        )
      }
      const review = { token, clip: id, lengthTicks: clip.length, analysis }
      pending = { review, original, revision, clip, audio: buffer }
      return review
    },
    async sliceApply(
      token: number,
      markers: number[]
    ): Promise<DispatchResult> {
      const current = pending
      if (!current || current.review.token !== token || token !== ticket)
        throw new Error("This slice review expired. Analyze the clip again.")
      const doc = document()
      const sample =
        current.clip.content.type === "audio" ? current.clip.content.sample : 0
      if (
        doc !== current.original ||
        doc.snapshot(null).revision !== current.revision ||
        source(sample) !== current.audio
      ) {
        throw new Error(
          "The project or source changed. Analyze the clip again."
        )
      }
      if (
        markers.some(
          (tick) =>
            !current.review.analysis.markers.some((m) => m.tick === tick)
        )
      )
        throw new Error(
          "Apply accepts only markers from the current slice review."
        )
      const project = doc.project()
      const command = sim.call<Command>("slice_command", 0, {
        clip: current.clip,
        markers,
        tempo: project.settings.tempoBpm,
        swing: project.settings.swing,
      })
      const result = dispatch(command)
      pending = null
      return result
    },
    async sliceDiscard(token: number): Promise<void> {
      if (ticket === token) {
        pending = null
        ticket++
      }
    },
  }
}
