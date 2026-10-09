import { afterEach, beforeEach, describe, expect, it } from "vitest"

import type { MeterChange } from "@/bindings"
import { RECT_VLINE, RectBatch, writeGrid } from "@/lib/canvas"
import { useProjectStore } from "@/lib/store/project"

import { GridMetrics } from "./metrics"
import { PlaylistScene } from "./scene"
import { gridSpecFor, type SnapMode } from "./snap"
import { usePlaylistStore } from "./store"
import { FakeSurface } from "./test-utils"

const FOUR_FOUR = { numerator: 4, denominator: 4 }
const SEVEN_EIGHT = { numerator: 7, denominator: 8 }
const CHANGE_TICK = 4001
const SEVEN_EIGHT_BAR = 3360
const modes: SnapMode[] = ["bar", "beat", "step", "none"]

let surface: FakeSurface
let scene: PlaylistScene
let detach: () => void

beforeEach(() => {
  useProjectStore.setState(useProjectStore.getInitialState(), true)
  usePlaylistStore.setState(usePlaylistStore.getInitialState(), true)
  surface = new FakeSurface()
  surface.viewport = { ...surface.viewport, width: 1400, pxPerTick: 0.1 }
  const metrics = new GridMetrics()
  detach = metrics.attach(surface)
  scene = new PlaylistScene(surface, metrics)
})

afterEach(() => {
  scene.destroy()
  detach()
})

function setMeters(meters: MeterChange[]) {
  useProjectStore.setState(({ project }) => ({
    project: {
      ...project,
      playlist: { ...project.playlist, timeline: { meters, markers: [] } },
    },
  }))
}

function lines() {
  const batch = new RectBatch()
  expect(surface.timeGrid).not.toBeNull()
  expect(surface.rows).not.toBeNull()
  writeGrid(
    batch,
    surface.viewport,
    surface.timeGrid!,
    surface.rows!,
    surface.theme
  )
  const result = new Map<number, number>()
  for (let index = 0; index < batch.count; index++) {
    if (batch.flags[index] & RECT_VLINE) {
      result.set(batch.start(index), batch.colors[index * 4 + 3])
    }
  }
  return result
}

describe("playlist meter grid", () => {
  it.each(modes)("keeps the old %s spacing without meter changes", (mode) => {
    usePlaylistStore.setState({ snap: mode })
    const oldSpec = gridSpecFor(mode, FOUR_FOUR)
    expect(surface.timeGrid).toEqual(oldSpec)
    const spacing = mode === "bar" ? 3840 : mode === "beat" ? 960 : 240
    const expected = Array.from(
      { length: Math.floor(14000 / spacing) + 1 },
      (_, index) => index * spacing
    )
    expect([...lines().keys()]).toEqual(expected)
    setMeters([])
    expect(surface.timeGrid).toEqual(oldSpec)
    expect([...lines().keys()]).toEqual(expected)
  })

  it.each(modes)(
    "draws %s lines from each segment's meter and origin",
    (mode) => {
      setMeters([{ id: 1, tick: CHANGE_TICK, signature: SEVEN_EIGHT }])
      // Recreate the scene so this also covers a project loaded with meters.
      scene.destroy()
      usePlaylistStore.setState({ snap: mode })
      scene = new PlaylistScene(surface, new GridMetrics())
      const drawn = lines()
      const barAlpha =
        mode === "bar" || mode === "beat"
          ? surface.theme.gridBeat.a
          : surface.theme.gridBar.a
      expect(drawn.get(3840)).toBe(barAlpha)
      expect(drawn.get(CHANGE_TICK)).toBe(surface.theme.gridBar.a)
      expect(drawn.get(CHANGE_TICK + SEVEN_EIGHT_BAR)).toBe(barAlpha)
      expect(drawn.get(CHANGE_TICK + 3840)).not.toBe(barAlpha)
      expect(
        [...drawn.keys()].filter(
          (tick) => tick >= CHANGE_TICK && tick <= CHANGE_TICK + SEVEN_EIGHT_BAR
        )
      ).toEqual(
        Array.from(
          { length: (mode === "bar" ? 1 : mode === "beat" ? 7 : 14) + 1 },
          (_, index) =>
            CHANGE_TICK +
            index *
              (mode === "bar" ? SEVEN_EIGHT_BAR : mode === "beat" ? 480 : 240)
        )
      )
      if (mode === "step" || mode === "none") {
        expect(drawn.get(960)).toBe(surface.theme.gridBeat.a)
        expect(drawn.get(CHANGE_TICK + 480)).toBe(surface.theme.gridBeat.a)
        expect(drawn.get(CHANGE_TICK + 240)).toBe(surface.theme.gridMinor.a)
      }
    }
  )

  it("refreshes bar spacing when only the meter map changes", () => {
    usePlaylistStore.setState({ snap: "bar" })
    setMeters([{ id: 1, tick: CHANGE_TICK, signature: SEVEN_EIGHT }])
    expect([...lines().keys()]).toEqual([0, 3840, CHANGE_TICK, 7361, 10721])
    setMeters([{ id: 1, tick: 0, signature: SEVEN_EIGHT }])
    expect([...lines().keys()]).toEqual([0, 3360, 6720, 10080, 13440])
    setMeters([])
    expect([...lines().keys()]).toEqual([0, 3840, 7680, 11520])
  })
})
