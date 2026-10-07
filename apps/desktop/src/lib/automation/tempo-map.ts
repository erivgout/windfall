import type { Project } from "@/bindings"
import { PPQ } from "@/lib/units"

import { rangeValue, TEMPO_RANGE } from "./curve"
import {
  compileLanes,
  laneCorners,
  laneValueAt,
  laneValueBefore,
  type Lane,
} from "./lanes"

/*
 * How long the song takes to get to each of its ticks when automation
 * moves the tempo on the way. This mirrors the engine's tempo map
 * (`windfall-engine/src/tempo.rs`): a row of segments in each of which the
 * tempo moves in a straight line, where the time has a closed form. A bent
 * stretch of the curve is followed with straight pieces a sixteenth of a
 * beat long, as the engine does.
 */

const GRAIN_TICKS = 60

type Segment = {
  tick: number
  /** Seconds from the start of the song to the segment's start. */
  seconds: number
  /** Infinite for the last. */
  length: number
  from: number
  to: number
}

function slope(segment: Segment): number {
  return Number.isFinite(segment.length) && segment.length > 0
    ? (segment.to - segment.from) / (segment.from * segment.length)
    : 0
}

/** Seconds from a segment's start to `ticks` ticks into it. */
function secondsInto(segment: Segment, ticks: number): number {
  // With the tempo at `from * (1 + slope * x)`, the time is
  // `60 / (from * PPQ) * ln(1 + slope * ticks) / slope`.
  const bend = slope(segment) * ticks
  const stretch = Math.abs(bend) < 1e-9 ? 1 - bend / 2 : Math.log1p(bend) / bend
  return ((ticks * 60) / (segment.from * PPQ)) * stretch
}

/** The inverse: ticks into a segment after `seconds` of it. */
function ticksInto(segment: Segment, seconds: number): number {
  const plain = (seconds * segment.from * PPQ) / 60
  const bend = slope(segment) * plain
  const stretch = Math.abs(bend) < 1e-9 ? 1 + bend / 2 : Math.expm1(bend) / bend
  return plain * stretch
}

/** The tempo along a song, and the time it takes to get to each tick. */
export class TempoMap {
  /** True when no automation moves the tempo: every tick takes the same time. */
  readonly steady: boolean
  private readonly segments: Segment[]

  /**
   * `lane` is the lane of the tempo, or null when the tempo is not
   * automated. `base` is the stored tempo, which holds wherever the lane
   * has nothing to say. `end` is the last tick of the song.
   */
  constructor(lane: Lane | null, base: number, end: number) {
    const songEnd = Math.max(0, end)
    this.steady = lane === null
    this.segments = []
    if (!lane) {
      this.segments.push({
        tick: 0,
        seconds: 0,
        length: Infinity,
        from: base,
        to: base,
      })
      return
    }
    const real = (value: number | undefined | null) =>
      value === undefined || value === null
        ? base
        : rangeValue(TEMPO_RANGE, value)
    const corners = laneCorners(lane, songEnd, GRAIN_TICKS)
    let seconds = 0
    for (let index = 0; index + 1 < corners.length; index += 1) {
      const start = corners[index]
      const length = corners[index + 1] - start
      // What the segment sets out with and what it arrives at, so a jump
      // at a corner stays a jump between two segments.
      const segment: Segment = {
        tick: start,
        seconds,
        length,
        from: real(laneValueAt(lane, start)?.value),
        to: real(laneValueBefore(lane, corners[index + 1])),
      }
      seconds += secondsInto(segment, length)
      this.segments.push(segment)
    }
    const last = real(laneValueAt(lane, songEnd)?.value)
    this.segments.push({
      tick: songEnd,
      seconds,
      length: Infinity,
      from: last,
      to: last,
    })
  }

  private segmentAt(tick: number): Segment {
    const segments = this.segments
    let low = 0
    let high = segments.length
    while (low < high) {
      const middle = (low + high) >> 1
      if (segments[middle].tick <= tick) low = middle + 1
      else high = middle
    }
    return segments[Math.max(0, low - 1)]
  }

  /** Seconds from the start of the song to a tick of it. */
  secondsAt(tick: number): number {
    const at = Math.max(0, tick)
    const segment = this.segmentAt(at)
    return segment.seconds + secondsInto(segment, at - segment.tick)
  }

  /** The tick the song has reached after `seconds`. */
  tickAt(seconds: number): number {
    if (seconds <= 0) return 0
    const segments = this.segments
    let low = 0
    let high = segments.length
    while (low < high) {
      const middle = (low + high) >> 1
      if (segments[middle].seconds <= seconds) low = middle + 1
      else high = middle
    }
    const segment = segments[Math.max(0, low - 1)]
    return segment.tick + ticksInto(segment, seconds - segment.seconds)
  }

  /** The tempo at a tick of the song, in beats per minute. */
  tempoAt(tick: number): number {
    const at = Math.max(0, tick)
    const segment = this.segmentAt(at)
    if (!Number.isFinite(segment.length) || segment.length <= 0) {
      return segment.from
    }
    const part = Math.min(1, Math.max(0, (at - segment.tick) / segment.length))
    return segment.from + (segment.to - segment.from) * part
  }
}

function songEnd(project: Pick<Project, "playlist">): number {
  let end = 0
  for (const clip of project.playlist.clips) {
    end = Math.max(end, clip.start + clip.length)
  }
  return end
}

type Source = Pick<Project, "playlist" | "automations" | "settings">

let cachedFor: {
  playlist: Source["playlist"]
  automations: Source["automations"]
  tempo: number
} | null = null
let cached: TempoMap | null = null

/**
 * The tempo map of a project's song. It is kept until the playlist, the
 * automations or the stored tempo change, so it can be asked every frame.
 */
export function songTempoMap(project: Source): TempoMap {
  const tempo = project.settings.tempoBpm
  if (
    cached &&
    cachedFor &&
    cachedFor.playlist === project.playlist &&
    cachedFor.automations === project.automations &&
    cachedFor.tempo === tempo
  ) {
    return cached
  }
  const lane =
    compileLanes(project).find((item) => item.target.type === "tempo") ?? null
  cached = new TempoMap(lane, tempo, songEnd(project))
  cachedFor = {
    playlist: project.playlist,
    automations: project.automations,
    tempo,
  }
  return cached
}
