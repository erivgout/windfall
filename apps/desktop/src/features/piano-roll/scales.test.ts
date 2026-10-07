import { afterEach, describe, expect, it } from "vitest"

import { keyToRow, pianoRows } from "@/lib/canvas"

import {
  inScale,
  nearestScaleKey,
  scaleMoveKeys,
  scaleRows,
  SCALES,
} from "./scales"
import { usePianoRollStore } from "./store"

const major = { root: 0, id: "major" } as const

afterEach(() => {
  usePianoRollStore.setState(usePianoRollStore.getInitialState(), true)
  localStorage.removeItem("windfall.pianoRoll")
})

describe("scale pitch policy", () => {
  it("uses every root and named scale within the inclusive MIDI bounds", () => {
    for (const definition of SCALES) {
      for (let root = 0; root < 12; root++) {
        const scale = { root, id: definition.id }
        for (const key of [-20, 0, 1, 60, 61, 126, 127, 200]) {
          const snapped = nearestScaleKey(key, scale)!
          expect(snapped).toBeGreaterThanOrEqual(0)
          expect(snapped).toBeLessThanOrEqual(127)
          expect(inScale(snapped, scale)).toBe(true)
          const candidates = Array.from(
            { length: 128 },
            (_, index) => index
          ).filter((candidate) => inScale(candidate, scale))
          expect(snapped).toBe(
            candidates.sort(
              (a, b) => Math.abs(a - key) - Math.abs(b - key) || a - b
            )[0]
          )
        }
      }
    }
  })

  it("chooses the lower equal-distance key and never leaves narrow bounds", () => {
    expect(nearestScaleKey(61, major)).toBe(60)
    expect(nearestScaleKey(61, major, 61, 63)).toBe(62)
    expect(nearestScaleKey(61, major, 61, 61)).toBeNull()
    expect(nearestScaleKey(NaN, major)).toBeNull()
  })

  it("preserves chord intervals, leaves time-only edits alone and advances keyboard nudges", () => {
    const limits = { minKeys: -60, maxKeys: 60 }
    expect(scaleMoveKeys(60, 1, limits, major)).toBe(0)
    expect(scaleMoveKeys(60, 1, limits, major, true)).toBe(2)
    expect(scaleMoveKeys(62, -1, limits, major, true)).toBe(-2)
    expect(scaleMoveKeys(61, 0, limits, major)).toBe(0)
    expect(scaleMoveKeys(60, 1, limits, null)).toBe(1)
    expect(
      scaleMoveKeys(126, 1, { minKeys: -126, maxKeys: 0 }, major, true)
    ).toBe(0)
    // A chromatic chord spanning the whole keyboard cannot be translated.
    expect(scaleMoveKeys(1, 1, { minKeys: 0, maxKeys: 0 }, major)).toBe(0)
  })

  it("keeps default piano shading and makes chosen roots visible through shared row styles", () => {
    expect(scaleRows(null)).toEqual(pianoRows())
    const rows = scaleRows({ root: 2, id: "minor" })
    for (let key = 0; key < 128; key++) {
      expect(rows.shaded![keyToRow(key)]).toBe(
        inScale(key, { root: 2, id: "minor" }) ? 0 : 1
      )
      expect(rows.strong![keyToRow(key)]).toBe((key + 1) % 12 === 2 ? 1 : 0)
    }
  })
})

describe("retained piano preferences", () => {
  it("retains validated roots, scales and independent options across rehydration", async () => {
    usePianoRollStore.getState().setScaleRoot(9)
    usePianoRollStore.getState().setScaleId("dorian")
    usePianoRollStore.getState().setHighlightScale(true)
    usePianoRollStore.getState().setSnapToScale(true)
    const saved = localStorage.getItem("windfall.pianoRoll")!
    usePianoRollStore.setState(usePianoRollStore.getInitialState(), true)
    localStorage.setItem("windfall.pianoRoll", saved)
    await usePianoRollStore.persist.rehydrate()
    expect(usePianoRollStore.getState()).toMatchObject({
      scaleRoot: 9,
      scaleId: "dorian",
      highlightScale: true,
      snapToScale: true,
    })
  })

  it("rejects malformed saved values and never merges transient fields or methods", async () => {
    usePianoRollStore.setState(usePianoRollStore.getInitialState(), true)
    localStorage.setItem(
      "windfall.pianoRoll",
      JSON.stringify({
        version: 1,
        state: {
          scaleRoot: 900,
          scaleId: "unknown",
          highlightScale: "true",
          snapToScale: {},
          snap: "wrong",
          laneKind: "wrong",
          laneHeight: null,
          ghosts: "yes",
          lastLength: -100,
          setScaleRoot: null,
        },
      })
    )
    await usePianoRollStore.persist.rehydrate()
    expect(usePianoRollStore.getState()).toMatchObject({
      scaleRoot: 0,
      scaleId: "major",
      highlightScale: false,
      snapToScale: false,
      lastLength: 240,
      laneKind: "velocity",
    })
    expect(usePianoRollStore.getState().setScaleRoot).toBeTypeOf("function")
    usePianoRollStore.getState().setScaleRoot(-1)
    expect(usePianoRollStore.getState().scaleRoot).toBe(0)
  })
})
