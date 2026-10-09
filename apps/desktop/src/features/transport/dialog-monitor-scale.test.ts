import { describe, expect, it } from "vitest"

import { nextDialogMonitorScale, scaledDialogMonitor } from "./dialog-monitor-scale"

describe("dialog monitor scale", () => {
  it.each([
    [0.5, 0.25],
    [0.253, 0.1265],
  ])("halves stored gain %s to %s without rounding", (gain, expected) => {
    expect(scaledDialogMonitor(gain, "half")).toBe(expected)
    expect(nextDialogMonitorScale(gain, "half")).toBe(expected)
  })

  it.each([
    [0.5, 1],
    [0.25, 0.5],
    [0.6, 1],
  ])("doubles stored gain %s to %s up to full level", (gain, expected) => {
    expect(scaledDialogMonitor(gain, "double")).toBe(expected)
    expect(nextDialogMonitorScale(gain, "double")).toBe(expected)
  })

  it.each(["half", "double"] as const)("keeps silence at zero for %s", (factor) => {
    expect(scaledDialogMonitor(0, factor)).toBe(0)
    expect(nextDialogMonitorScale(0, factor)).toBeNull()
  })

  it("returns null when doubling full level", () => {
    expect(scaledDialogMonitor(1, "double")).toBe(1)
    expect(nextDialogMonitorScale(1, "double")).toBeNull()
  })

  it("returns null for gain changes smaller than 0.001", () => {
    expect(nextDialogMonitorScale(0.0018, "half")).toBeNull()
    expect(nextDialogMonitorScale(0.0009, "double")).toBeNull()
    expect(nextDialogMonitorScale(0.9995, "double")).toBeNull()
  })

  it("applies gain changes of exactly 0.001", () => {
    expect(nextDialogMonitorScale(0.002, "half")).toBe(0.001)
    expect(nextDialogMonitorScale(0.001, "double")).toBe(0.002)
  })
})
