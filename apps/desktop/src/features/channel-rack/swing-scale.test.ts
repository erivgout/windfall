import { describe, expect, it } from "vitest"

import { nextProjectSwingScale, scaledProjectSwing } from "./swing-scale"

describe("project swing scale", () => {
  it("halves 50% swing to 0.25 and doubles it to 1", () => {
    expect(scaledProjectSwing(0.5, "halve")).toBe(0.25)
    expect(scaledProjectSwing(0.5, "double")).toBe(1)
    expect(nextProjectSwingScale(0.5, "halve")).toBe(0.25)
    expect(nextProjectSwingScale(0.5, "double")).toBe(1)
  })

  it("doubles continuously and stops at full swing", () => {
    for (const [swing, expected] of [
      [0.25, 0.5],
      [0.75, 1],
      [0.4, 0.8],
    ]) {
      expect(scaledProjectSwing(swing, "double")).toBe(expected)
      expect(nextProjectSwingScale(swing, "double")).toBe(expected)
    }
  })

  it("keeps halve of straight swing straight without a command", () => {
    expect(scaledProjectSwing(0, "halve")).toBe(0)
    expect(nextProjectSwingScale(0, "halve")).toBeNull()
  })

  it("returns null when doubling full swing", () => {
    expect(scaledProjectSwing(1, "double")).toBe(1)
    expect(nextProjectSwingScale(1, "double")).toBeNull()
  })

  it("returns null for changes smaller than 0.001", () => {
    expect(nextProjectSwingScale(0.0018, "halve")).toBeNull()
    expect(nextProjectSwingScale(0.0009, "double")).toBeNull()
    expect(nextProjectSwingScale(0.9991, "double")).toBeNull()
  })

  it("applies changes of exactly 0.001", () => {
    expect(nextProjectSwingScale(0.002, "halve")).toBe(0.001)
    expect(nextProjectSwingScale(0.001, "double")).toBe(0.002)
  })

  it("does not round scaled swing", () => {
    const swing = 0.123456789
    expect(nextProjectSwingScale(swing, "halve")).toBe(swing / 2)
    expect(nextProjectSwingScale(swing, "double")).toBe(swing * 2)
  })
})
