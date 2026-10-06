import { describe, expect, it } from "vitest"

import { BUILTIN_ACTIONS } from "@/lib/actions/builtin"

import { scoreAction } from "./score"

/** Titles of the built-in actions that match, best first. */
function search(text: string): string[] {
  return BUILTIN_ACTIONS.map((action) => ({
    title: action.title,
    score: scoreAction(action.title, action.keywords ?? "", text),
  }))
    .filter((item) => item.score > 0)
    .sort((a, b) => b.score - a.score)
    .map((item) => item.title)
}

describe("scoreAction", () => {
  it("matches everything for an empty search", () => {
    expect(search("  ")).toHaveLength(BUILTIN_ACTIONS.length)
  })

  it("puts the action whose title was typed first", () => {
    expect(search("add pattern")[0]).toBe("Add pattern")
    expect(search("mixer")[0]).toBe("Mixer")
    expect(search("save")[0]).toBe("Save")
    expect(search("undo")).toEqual(["Undo"])
  })

  it("matches the starts of words in any order", () => {
    expect(search("pat add")[0]).toBe("Add pattern")
    expect(search("proj new")[0]).toBe("New project")
  })

  it("ranks a title match above a keyword match", () => {
    expect(
      scoreAction("Export audio…", "render bounce wav", "exp")
    ).toBeGreaterThan(scoreAction("Mixer", "export", "exp"))
  })

  it("finds actions by keyword", () => {
    expect(search("bounce")).toEqual(["Export audio…"])
    expect(search("theme")).toContain("Switch between light and dark")
    expect(search("fl")).toContain("Use FL Studio shortcuts")
  })

  it("accepts letters that only appear in order", () => {
    expect(search("sv as")[0]).toBe("Save as…")
    expect(search("cmdpal")[0]).toBe("Command palette…")
    expect(scoreAction("Save as…", "", "svas")).toBeLessThan(
      scoreAction("Save as…", "", "save")
    )
  })

  it("hides what does not match at all", () => {
    expect(search("zzzz")).toEqual([])
    expect(scoreAction("Play", "", "playx")).toBe(0)
  })
})
