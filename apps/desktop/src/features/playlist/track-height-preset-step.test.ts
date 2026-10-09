import { describe, expect, it } from "vitest"

import { DEFAULT_ROW_HEIGHT, MIN_ROW_HEIGHT, TALL_ROW_HEIGHT } from "./layout"
import { nextTrackHeightPreset } from "./track-height-preset-step"
import { TRACK_HEIGHT_PRESETS } from "./track-height-presets"

const [follow, short, normal, tall] = TRACK_HEIGHT_PRESETS

describe("track height preset stepping", () => {
  it("uses Follow and the shared layout heights in order", () => {
    expect(TRACK_HEIGHT_PRESETS.map((item) => item.height)).toEqual([
      0,
      MIN_ROW_HEIGHT,
      DEFAULT_ROW_HEIGHT,
      TALL_ROW_HEIGHT,
    ])
  })

  it.each([
    ["Follow", follow.height, null, short.height],
    ["Short", short.height, follow.height, normal.height],
    ["Normal", normal.height, short.height, tall.height],
    ["Tall", tall.height, normal.height, null],
    ["10", 10, follow.height, short.height],
    ["20", 20, short.height, normal.height],
    ["50", 50, normal.height, tall.height],
    ["-1", -1, null, follow.height],
    ["100", 100, tall.height, null],
  ] as const)(
    "steps %s to its neighboring presets without wrapping",
    (_label, height, previous, next) => {
      expect(nextTrackHeightPreset(height, "previous")).toBe(previous)
      expect(nextTrackHeightPreset(height, "next")).toBe(next)
    }
  )

  it("treats a missing height as Follow", () => {
    const track: { height?: number } = {}
    expect(nextTrackHeightPreset(track.height ?? 0, "previous")).toBeNull()
    expect(nextTrackHeightPreset(track.height ?? 0, "next")).toBe(short.height)
  })

  it.each([NaN, Infinity, -Infinity])(
    "returns null in both directions for %s",
    (height) => {
      expect(nextTrackHeightPreset(height, "previous")).toBeNull()
      expect(nextTrackHeightPreset(height, "next")).toBeNull()
    }
  )

  it("keeps a height just below Normal between Short and Normal", () => {
    const height = normal.height - 0.0001
    expect(nextTrackHeightPreset(height, "previous")).toBe(short.height)
    expect(nextTrackHeightPreset(height, "next")).toBe(normal.height)
  })

  it("keeps a height just above Normal between Normal and Tall", () => {
    const height = normal.height + 0.0001
    expect(nextTrackHeightPreset(height, "previous")).toBe(normal.height)
    expect(nextTrackHeightPreset(height, "next")).toBe(tall.height)
  })
})
