import { describe, expect, it } from "vitest"

import { nextLoopMode } from "./loop-mode-step"

describe("sampler loop mode stepping", () => {
  it.each<{
    mode: string | undefined
    direction: "previous" | "next"
    expected: "off" | "forward" | "pingPong" | null
  }>([
    { mode: "off", direction: "previous", expected: null },
    { mode: "off", direction: "next", expected: "forward" },
    { mode: "forward", direction: "previous", expected: "off" },
    { mode: "forward", direction: "next", expected: "pingPong" },
    { mode: "pingPong", direction: "previous", expected: "forward" },
    { mode: "pingPong", direction: "next", expected: null },
    { mode: "unknown", direction: "previous", expected: null },
    { mode: "unknown", direction: "next", expected: null },
    { mode: undefined, direction: "previous", expected: null },
    { mode: undefined, direction: "next", expected: "forward" },
  ])(
    "returns $expected for $mode moved $direction",
    ({ mode, direction, expected }) => {
      expect(nextLoopMode(mode, direction)).toBe(expected)
    }
  )
})
