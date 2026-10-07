import type { SampleAsset, SampleId } from "@/bindings"
import { backend, errorMessage } from "@/lib/ipc"
import { onProjectReplaced } from "@/lib/store/replaced"

/*
 * The waveforms drawn inside audio clips. A sample's overview is asked for
 * once and kept; the drawing reads it through a pyramid of coarser copies,
 * so a clip costs the same to draw however far the timeline is zoomed out.
 */

/** A waveform overview at several resolutions. */
export type PeakPyramid = {
  /**
   * Min and max pairs. Level 0 is the overview as it came; each level
   * after it has half the buckets, down to one.
   */
  readonly levels: readonly Float32Array[]
  /** The largest absolute value in the file's overview. */
  readonly peak: number
}

export function buildPyramid(peaks: ArrayLike<number>): PeakPyramid {
  const base = new Float32Array(peaks.length - (peaks.length % 2))
  let peak = 0
  for (let index = 0; index < base.length; index += 1) {
    base[index] = peaks[index]
    peak = Math.max(peak, Math.abs(peaks[index]))
  }
  const levels = [base]
  let current = base
  while (current.length > 2) {
    const buckets = Math.ceil(current.length / 4)
    const next = new Float32Array(buckets * 2)
    for (let bucket = 0; bucket < buckets; bucket += 1) {
      const a = bucket * 4
      // The last bucket of an odd level has only one child.
      const b = Math.min(a + 2, current.length - 2)
      next[bucket * 2] = Math.min(current[a], current[b])
      next[bucket * 2 + 1] = Math.max(current[a + 1], current[b + 1])
    }
    levels.push(next)
    current = next
  }
  return { levels, peak }
}

const range: [number, number] = [0, 0]

/**
 * The lowest and highest value of the file between two positions (0 to 1,
 * in either order). The result is written to a pair that is reused: read
 * it before the next call. A stretch narrower than a bucket is read between
 * the two buckets around it, so a waveform zoomed far in is still a smooth
 * outline and not a row of blocks.
 */
export function peakRange(
  pyramid: PeakPyramid,
  from: number,
  to: number
): readonly [min: number, max: number] {
  const base = pyramid.levels[0]
  const buckets = base.length / 2
  if (buckets === 0) {
    range[0] = 0
    range[1] = 0
    return range
  }
  const low = Math.min(1, Math.max(0, Math.min(from, to))) * buckets
  const high = Math.min(1, Math.max(0, Math.max(from, to))) * buckets
  const span = high - low

  if (span < 1) {
    // Between the centers of two buckets.
    const center = (low + high) / 2 - 0.5
    const left = Math.min(buckets - 1, Math.max(0, Math.floor(center)))
    const right = Math.min(buckets - 1, left + 1)
    const share = Math.min(1, Math.max(0, center - left))
    range[0] = base[left * 2] + (base[right * 2] - base[left * 2]) * share
    range[1] =
      base[left * 2 + 1] + (base[right * 2 + 1] - base[left * 2 + 1]) * share
    return range
  }

  // The coarsest level that still has two buckets or more in the stretch.
  const level = Math.min(
    pyramid.levels.length - 1,
    Math.max(0, Math.floor(Math.log2(span)) - 1)
  )
  const data = pyramid.levels[level]
  const scale = 2 ** level
  const first = Math.floor(low / scale)
  const last = Math.min(data.length / 2 - 1, Math.ceil(high / scale) - 1)
  let min = Infinity
  let max = -Infinity
  for (let bucket = first; bucket <= last; bucket += 1) {
    min = Math.min(min, data[bucket * 2])
    max = Math.max(max, data[bucket * 2 + 1])
  }
  range[0] = min === Infinity ? 0 : min
  range[1] = max === -Infinity ? 0 : max
  return range
}

export type SamplePeaks =
  | { status: "loading" }
  | { status: "ready"; durationSecs: number; pyramid: PeakPyramid }
  /** The file is gone, or cannot be read. */
  | { status: "missing"; message: string }

type Entry = { peaks: SamplePeaks; failedAt: number }

/** How long a sample that could not be read is left alone before a retry. */
const RETRY_MS = 4000

// Keyed by the sample's object: the store keeps it until the sample
// changes, so a sample that is pointed at another file is read again.
let entries = new WeakMap<SampleAsset, Entry>()
const listeners = new Set<(sample: SampleId) => void>()

onProjectReplaced(() => {
  entries = new WeakMap()
})

/** Calls `listener` when a sample's waveform has arrived or failed. */
export function onPeaksChanged(listener: (sample: SampleId) => void) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

function load(asset: SampleAsset, entry: Entry) {
  void backend
    .sampleInfoById(asset.id)
    .then(
      (info): SamplePeaks => ({
        status: "ready",
        durationSecs: info.durationSecs,
        pyramid: buildPyramid(info.peaks),
      }),
      (error: unknown): SamplePeaks => ({
        status: "missing",
        message: errorMessage(error),
      })
    )
    .then((peaks) => {
      // Another project may have been opened while the file was read.
      if (entries.get(asset) !== entry) return
      entry.peaks = peaks
      entry.failedAt = peaks.status === "missing" ? Date.now() : 0
      for (const listener of [...listeners]) listener(asset.id)
    })
}

/**
 * The waveform of a sample of the project. The first call starts reading
 * it and answers "loading"; `onPeaksChanged` says when to ask again.
 */
export function samplePeaks(asset: SampleAsset): SamplePeaks {
  let entry = entries.get(asset)
  if (!entry) {
    entry = { peaks: { status: "loading" }, failedAt: 0 }
    entries.set(asset, entry)
    load(asset, entry)
  } else if (
    entry.peaks.status === "missing" &&
    Date.now() - entry.failedAt > RETRY_MS
  ) {
    // The file may have been put back since.
    entry.failedAt = Date.now()
    load(asset, entry)
  }
  return entry.peaks
}

/** The length of a sample in seconds, once its file has been read. */
export function sampleDuration(asset: SampleAsset | undefined): number | null {
  if (!asset) return null
  const peaks = samplePeaks(asset)
  return peaks.status === "ready" ? peaks.durationSecs : null
}

/** Reads a sample's file now and resolves once that is done. */
export async function loadSamplePeaks(
  asset: SampleAsset
): Promise<SamplePeaks> {
  const now = samplePeaks(asset)
  if (now.status !== "loading") return now
  return new Promise((resolve) => {
    const stop = onPeaksChanged((sample) => {
      if (sample !== asset.id) return
      const next = samplePeaks(asset)
      if (next.status === "loading") return
      stop()
      resolve(next)
    })
  })
}
