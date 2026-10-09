import { describe, expect, it } from "vitest"

import { nextDialogBufferScale, scaledDialogBuffer } from "./dialog-buffer-scale"

describe("dialog buffer scale", () => {
  it.each([
    [20, 10],
    [12, 6],
    [9, 5],
    [11, 5],
  ])("halves %s ms to %s whole milliseconds", (ms, expected) => {
    expect(scaledDialogBuffer(ms, "half")).toBe(expected)
    expect(nextDialogBufferScale(ms, "half")).toBe(expected)
  })

  it.each([
    [20, 40],
    [51, 100],
    [50, 100],
  ])("doubles %s ms to %s ms within the limit", (ms, expected) => {
    expect(scaledDialogBuffer(ms, "double")).toBe(expected)
    expect(nextDialogBufferScale(ms, "double")).toBe(expected)
  })

  it("returns null when halving the 5 ms minimum", () => {
    expect(scaledDialogBuffer(5, "half")).toBe(5)
    expect(nextDialogBufferScale(5, "half")).toBeNull()
  })

  it("returns null when doubling the 100 ms maximum", () => {
    expect(scaledDialogBuffer(100, "double")).toBe(100)
    expect(nextDialogBufferScale(100, "double")).toBeNull()
  })
})
