import { describe, expect, it } from "vitest"

import { duplicatePage } from "./duplicate-page"

const pages = [
  { title: "Verse", body: "First line\nSecond line" },
  { title: "Chorus", body: "  Refrain  " },
  { title: "Bridge", body: "Ending" },
]

describe("duplicatePage", () => {
  it("inserts a copy with the same title and body immediately after the chosen page", () => {
    const result = duplicatePage(pages, 1, 8)

    expect(result?.slice(0, 3)).toEqual([pages[0], pages[1], pages[1]])
    expect(result?.[2]).not.toBe(pages[1])
    expect(result).toHaveLength(4)
  })

  it("keeps the following pages in order", () => {
    expect(duplicatePage(pages, 0, 8)).toEqual([
      pages[0],
      pages[0],
      pages[1],
      pages[2],
    ])
  })

  it("does not mutate the input array or its pages", () => {
    const input = Object.freeze(pages.map((page) => Object.freeze({ ...page })))
    const result = duplicatePage(input, 1, 8)

    expect(input).toEqual(pages)
    expect(result).not.toBe(input)
  })

  it("returns null for an empty list", () => {
    expect(duplicatePage([], 0, 8)).toBeNull()
  })

  it("returns null for an index past the end", () => {
    expect(duplicatePage(pages, pages.length, 8)).toBeNull()
  })

  it("returns null for a negative or noninteger index", () => {
    expect(duplicatePage(pages, -1, 8)).toBeNull()
    expect(duplicatePage(pages, 0.5, 8)).toBeNull()
  })

  it("returns null when the list is already at maxPages", () => {
    expect(duplicatePage(pages, 1, pages.length)).toBeNull()
  })
})
