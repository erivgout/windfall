import { describe, expect, it } from "vitest"

import type { AutomationPoint, Project } from "@/bindings"

import { rangeNormalized, TEMPO_RANGE } from "./curve"
import { compileLanes } from "./lanes"
import { songTempoMap, TempoMap } from "./tempo-map"

const BAR = 3840
const bpm = (tempo: number) => rangeNormalized(TEMPO_RANGE, tempo)

const point = (
  tick: number,
  tempo: number,
  more: Partial<AutomationPoint> = {}
): AutomationPoint => ({
  tick,
  value: bpm(tempo),
  curve: 0,
  hold: false,
  ...more,
})

type Song = Pick<Project, "playlist" | "automations" | "settings">

function song(
  points: AutomationPoint[],
  start: number,
  length: number,
  tempoBpm = 120
): Song {
  return {
    settings: {
      name: "",
      tempoBpm,
      timeSignature: { numerator: 4, denominator: 4 },
      swing: 0,
    },
    automations: [
      { id: 1, name: "Tempo", color: 0, target: { type: "tempo" }, points },
    ],
    playlist: {
      tracks: [{ id: 10, name: "Track 1", muted: false }],
      clips: [
        {
          id: 2,
          track: 10,
          start,
          length,
          offset: 0,
          muted: false,
          content: { type: "automation", automation: 1 },
        },
      ],
    },
  }
}

function mapOf(project: Song, end: number): TempoMap {
  const lane = compileLanes(project).find(
    (item) => item.target.type === "tempo"
  )
  return new TempoMap(lane ?? null, project.settings.tempoBpm, end)
}

describe("a steady tempo", () => {
  it("takes two seconds a bar at 120 bpm", () => {
    const map = new TempoMap(null, 120, 0)
    expect(map.steady).toBe(true)
    expect(map.secondsAt(BAR)).toBeCloseTo(2, 12)
    expect(map.tickAt(3)).toBeCloseTo(1.5 * BAR, 9)
    expect(map.tempoAt(123456)).toBe(120)
  })
})

describe("a tempo that changes", () => {
  it("stretches time by the ratio to the stored tempo", () => {
    // A bar at the stored tempo, then a bar at half of it, twice as long.
    const map = mapOf(song([point(0, 60)], BAR, BAR), 2 * BAR)
    expect(map.steady).toBe(false)
    expect(map.secondsAt(BAR)).toBeCloseTo(2, 9)
    expect(map.secondsAt(2 * BAR)).toBeCloseTo(6, 9)
    expect(map.tempoAt(BAR / 2)).toBe(120)
    expect(map.tempoAt(BAR + 10)).toBeCloseTo(60, 4)
    // After the clip the tempo holds what the clip left.
    expect(map.secondsAt(3 * BAR)).toBeCloseTo(10, 9)
  })

  it("sums a ramp in closed form", () => {
    // 60 to 180 bpm over four bars: 16 beats, 60 / 120 * ln(3) * 16 s.
    const map = mapOf(
      song([point(0, 60), point(4 * BAR, 180)], 0, 4 * BAR),
      4 * BAR
    )
    const expected = ((16 * 60) / 120) * Math.log(3)
    expect(map.secondsAt(4 * BAR)).toBeCloseTo(expected, 4)
    expect(map.tempoAt(2 * BAR)).toBeCloseTo(120, 3)
  })

  it("goes from seconds back to the same tick", () => {
    const map = mapOf(
      song(
        [
          point(0, 90, { curve: 0.6 }),
          point(BAR, 200, { hold: true }),
          point(2 * BAR, 40),
          point(3 * BAR, 140),
        ],
        960,
        4 * BAR
      ),
      6 * BAR
    )
    for (const tick of [0, 500, 960, 2000, 4800, 9000, 15360, 20000, 30000]) {
      expect(map.tickAt(map.secondsAt(tick))).toBeCloseTo(tick, 4)
    }
    // Time only ever goes forward.
    let last = -1
    for (let tick = 0; tick <= 6 * BAR; tick += 97) {
      const seconds = map.secondsAt(tick)
      expect(seconds).toBeGreaterThan(last)
      last = seconds
    }
  })

  it("keeps a jump a jump", () => {
    const map = mapOf(
      song([point(0, 120), point(BAR, 120), point(BAR, 240)], 0, 2 * BAR),
      2 * BAR
    )
    expect(map.secondsAt(BAR)).toBeCloseTo(2, 6)
    expect(map.secondsAt(2 * BAR)).toBeCloseTo(3, 6)
  })
})

describe("songTempoMap", () => {
  it("is kept until the playlist, the automations or the tempo change", () => {
    const project = song([point(0, 60)], 0, BAR)
    const first = songTempoMap(project)
    expect(songTempoMap({ ...project })).toBe(first)
    const faster = {
      ...project,
      settings: { ...project.settings, tempoBpm: 140 },
    }
    expect(songTempoMap(faster)).not.toBe(first)
    expect(songTempoMap({ ...project, automations: [] }).steady).toBe(true)
  })
})
