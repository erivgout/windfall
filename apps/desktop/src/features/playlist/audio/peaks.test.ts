import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { useProjectStore } from "@/lib/store/project"
import { dispatch } from "@/lib/store/project"
import { settle } from "@/test/harness"

import { startPlaylist } from "../test-utils"
import {
  buildPyramid,
  loadSamplePeaks,
  onPeaksChanged,
  peakRange,
  sampleDuration,
  samplePeaks,
} from "./peaks"

/** Eight buckets whose height grows with the bucket: 0.1, 0.2 ... 0.8. */
const ramp = Array.from({ length: 8 }, (_, bucket) => {
  const height = (bucket + 1) / 10
  return [-height, height]
}).flat()

describe("the pyramid", () => {
  it("halves the buckets level by level, keeping the extremes", () => {
    const pyramid = buildPyramid(ramp)
    expect(pyramid.levels.map((level) => level.length / 2)).toEqual([
      8, 4, 2, 1,
    ])
    expect(pyramid.peak).toBeCloseTo(0.8, 6)
    expect([...pyramid.levels[1]].map((value) => +value.toFixed(2))).toEqual([
      -0.2, 0.2, -0.4, 0.4, -0.6, 0.6, -0.8, 0.8,
    ])
    expect(pyramid.levels[3][1]).toBeCloseTo(0.8, 6)
  })

  it("copes with an odd number of buckets and with none", () => {
    const odd = buildPyramid([-0.1, 0.1, -0.5, 0.5, -0.3, 0.3])
    expect(odd.levels.map((level) => level.length / 2)).toEqual([3, 2, 1])
    expect(odd.levels[2][1]).toBeCloseTo(0.5, 6)
    const none = buildPyramid([])
    expect([...peakRange(none, 0, 1)]).toEqual([0, 0])
  })
})

describe("peakRange", () => {
  const pyramid = buildPyramid(ramp)

  it("covers a stretch of the file, whichever way round it is given", () => {
    const [min, max] = peakRange(pyramid, 0, 0.5)
    expect(max).toBeCloseTo(0.4, 6)
    expect(min).toBeCloseTo(-0.4, 6)
    expect(peakRange(pyramid, 0.5, 0)[1]).toBeCloseTo(0.4, 6)
    expect(peakRange(pyramid, 0, 1)[1]).toBeCloseTo(0.8, 6)
  })

  it("reads between two buckets when the stretch is narrower than one", () => {
    // The middle of the third bucket, exactly.
    expect(peakRange(pyramid, 2.5 / 8, 2.5 / 8)[1]).toBeCloseTo(0.3, 6)
    // Half way between the third and the fourth.
    expect(peakRange(pyramid, 3 / 8, 3 / 8)[1]).toBeCloseTo(0.35, 6)
    // The two ends do not read past the file.
    expect(peakRange(pyramid, 0, 0)[1]).toBeCloseTo(0.1, 6)
    expect(peakRange(pyramid, 1, 1)[1]).toBeCloseTo(0.8, 6)
  })

  it("holds positions outside the file to its ends", () => {
    expect(peakRange(pyramid, -3, -2)[1]).toBeCloseTo(0.1, 6)
    expect(peakRange(pyramid, 0.75, 9)[1]).toBeCloseTo(0.8, 6)
  })
})

describe("the waveforms of the project's samples", () => {
  let stop: () => void
  beforeEach(async () => {
    ;({ stop } = await startPlaylist())
  })
  afterEach(() => stop())

  const samples = () => useProjectStore.getState().project.samples

  it("reads a sample once and says when it has arrived", async () => {
    const [kick] = samples()
    const changed = vi.fn()
    const off = onPeaksChanged(changed)
    expect(samplePeaks(kick)).toEqual({ status: "loading" })
    expect(sampleDuration(kick)).toBeNull()
    await settle()

    expect(changed).toHaveBeenCalledWith(kick.id)
    const peaks = samplePeaks(kick)
    expect(peaks.status).toBe("ready")
    expect(sampleDuration(kick)).toBe(0.42)
    expect(samplePeaks(kick)).toBe(peaks)
    expect(changed).toHaveBeenCalledTimes(1)
    off()
  })

  it("resolves once a sample has been read", async () => {
    const [, clap] = samples()
    const peaks = await loadSamplePeaks(clap)
    expect(peaks.status).toBe("ready")
    expect((await loadSamplePeaks(clap)).status).toBe("ready")
  })

  it("says a sample is missing when its file cannot be read", async () => {
    await dispatch({
      type: "addSample",
      name: "Gone",
      path: { kind: "external", path: "/elsewhere/Gone.wav" },
    })
    const gone = samples().at(-1)!
    const peaks = await loadSamplePeaks(gone)
    expect(peaks.status).toBe("missing")
    expect(sampleDuration(gone)).toBeNull()
  })
})
