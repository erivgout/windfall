import { describe, expect, it } from "vitest"

import { nextFlaggedChannelId } from "./select-next"

const channels = Object.freeze([
  Object.freeze({ id: 3, muted: true, solo: false }),
  Object.freeze({ id: 1, muted: false, solo: true }),
  Object.freeze({ id: 4, muted: false, solo: false }),
  Object.freeze({ id: 2, muted: true, solo: true }),
])

describe("nextFlaggedChannelId", () => {
  it("chooses the next muted channel after the current one", () => {
    expect(nextFlaggedChannelId(channels, 3, "muted")).toBe(2)
  })

  it("wraps to a muted channel before the current one", () => {
    expect(nextFlaggedChannelId(channels, 2, "muted")).toBe(3)
  })

  it("returns null when the only muted channel is already current", () => {
    expect(nextFlaggedChannelId(channels.slice(0, 3), 3, "muted")).toBeNull()
  })

  it("returns the first muted channel for a null current id", () => {
    expect(nextFlaggedChannelId(channels, null, "muted")).toBe(3)
  })

  it("returns null when no channel is muted", () => {
    expect(nextFlaggedChannelId(channels.slice(1, 3), 1, "muted")).toBeNull()
  })

  it("does not return a solo channel for the muted flag", () => {
    expect(nextFlaggedChannelId(channels, 3, "muted")).toBe(2)
    expect(nextFlaggedChannelId(channels, 3, "solo")).toBe(1)
  })

  it("returns the first match when the current id is missing", () => {
    expect(nextFlaggedChannelId(channels, 99, "muted")).toBe(3)
    expect(nextFlaggedChannelId(channels, 99, "solo")).toBe(1)
  })

  it("returns null for an empty list", () => {
    expect(nextFlaggedChannelId([], null, "muted")).toBeNull()
    expect(nextFlaggedChannelId([], 3, "solo")).toBeNull()
  })

  it("wraps for solo and excludes the current solo channel", () => {
    expect(nextFlaggedChannelId(channels, 2, "solo")).toBe(1)
    expect(nextFlaggedChannelId(channels.slice(0, 3), 1, "solo")).toBeNull()
  })

  it("does not mutate the input", () => {
    nextFlaggedChannelId(channels, 2, "muted")
    nextFlaggedChannelId(channels, 2, "solo")
    expect(channels).toEqual([
      { id: 3, muted: true, solo: false },
      { id: 1, muted: false, solo: true },
      { id: 4, muted: false, solo: false },
      { id: 2, muted: true, solo: true },
    ])
  })
})
