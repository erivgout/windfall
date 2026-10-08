import { describe, expect, it } from "vitest"

import type { Note } from "@/bindings"

import { notePreviewGeometry, previewSteps } from "./note-preview-geometry"

const note = (start: number, length: number, key = 60): Note => ({
  id: 1,
  start,
  length,
  key,
  velocity: 0.8,
  pan: 0,
})

describe("thumbnail coordinate boundaries beyond the checked document", () => {
  it.each([0, -1, NaN, Infinity, -Infinity])(
    "gives invalid/zero extent %s a finite usable view",
    (extent) => {
      const geometry = notePreviewGeometry([], extent)
      expect(geometry).toMatchObject({
        width: 20,
        height: 22,
        count: 0,
        path: "",
      })
    }
  )

  it("clips negative notes crossing zero and omits fully outside/invalid coordinates", () => {
    const geometry = notePreviewGeometry(
      [
        note(-120, 240),
        note(-240, 240),
        note(3839, 500),
        note(3840, 240),
        note(NaN, 240),
        note(0, Infinity),
        note(0, -1),
        note(0, 1, NaN),
      ],
      16
    )
    expect(geometry.path).toBe(
      "M0 9.333h10v1.417h-10ZM319.917 9.333h0.083v1.417h-0.083Z"
    )
    expect(geometry.count).toBe(2)
    expect(geometry.outside).toBe(6)
  })

  it("clamps the legal extent and untrusted pitch, and never emits NaN/Infinity", () => {
    expect(previewSteps(Number.MAX_VALUE)).toBe(1024)
    const geometry = notePreviewGeometry(
      [note(0, Number.MAX_VALUE, -100), note(100, 240, 900)],
      Number.MAX_VALUE
    )
    expect(geometry.width).toBe(20480)
    expect(geometry.path).not.toMatch(/NaN|Infinity/)
    expect(geometry.path.length).toBeLessThan(200)
    for (const value of geometry.path.match(/[\d.]+/g) ?? [])
      expect(Number.isFinite(Number(value))).toBe(true)
  })

  it("bounds even alternating dense occupancy without excluding a late pitch/time", () => {
    const notes: Note[] = []
    for (let key = 0; key < 128; key += 8) {
      for (let tick = 0; tick < 3840; tick += 15) notes.push(note(tick, 1, key))
    }
    notes.push(note(3830, 10, 127))
    const geometry = notePreviewGeometry(notes, 16)
    expect(geometry.dense).toBe(true)
    expect(geometry.count).toBe(notes.length)
    expect(geometry.path.match(/M/g)!.length).toBeLessThanOrEqual(4096)
    expect(geometry.path.includes("M318.75 1h1.25")).toBe(true)
    expect(geometry.path).not.toMatch(/NaN|Infinity/)
    expect(geometry.path.length).toBeLessThan(400000)
  })
})
