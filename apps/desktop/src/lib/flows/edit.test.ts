import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { runAction } from "@/lib/actions"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { usePromptStore } from "@/lib/store/prompts"
import { settle, startTestApp } from "@/test/harness"

import { currentPatternId, deletePattern, patternLoss } from "./edit"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let stop: () => void

beforeEach(async () => {
  ;({ stop } = await startTestApp())
})
afterEach(() => stop())

const project = () => useProjectStore.getState().project
const question = () => usePromptStore.getState().confirm

/** Puts `count` clips of the demo's pattern on a new playlist track. */
async function placeClips(count: number) {
  const pattern = project().patterns[0]
  const added = await dispatch({ type: "addPlaylistTrack" })
  if (!added) throw new Error("no playlist track")
  await dispatch({
    type: "addClips",
    clips: Array.from({ length: count }, (_, index) => ({
      track: added.created[0],
      start: index * 3840,
      content: { type: "pattern" as const, pattern: pattern.id },
    })),
  })
  return pattern
}

describe("deleting a pattern", () => {
  it("asks first when clips on the playlist go with it, and says how many", async () => {
    const pattern = await placeClips(3)
    await dispatch({ type: "addPattern" })
    expect(currentPatternId()).toBe(pattern.id)
    const notes = pattern.lanes.reduce(
      (total, lane) => total + lane.notes.length,
      0
    )

    const running = runAction("pattern.delete")
    await settle()
    expect(question()?.title).toBe(`Delete ${pattern.name}?`)
    expect(question()?.description).toBe(
      `It has ${notes} notes, and 3 clips on the playlist play it and are deleted with it. Undo brings everything back.`
    )

    // Cancelled: nothing is deleted.
    question()?.resolve(null)
    await running
    expect(project().patterns.map((item) => item.id)).toContain(pattern.id)
    expect(project().playlist.clips).toHaveLength(3)

    const again = runAction("pattern.delete")
    await settle()
    question()?.resolve("delete")
    await again
    await settle()
    expect(project().patterns.map((item) => item.id)).not.toContain(pattern.id)
    expect(project().playlist.clips).toHaveLength(0)
  })

  it("asks for a pattern that only has notes", async () => {
    const pattern = project().patterns[0]
    await dispatch({ type: "addPattern" })
    expect(patternLoss(pattern.id)).toMatch(
      /^It has \d+ notes\. Undo brings the pattern back\.$/
    )
    const running = runAction("pattern.delete")
    await settle()
    expect(question()).not.toBeNull()
    question()?.resolve(null)
    await running
    expect(project().patterns).toHaveLength(2)
  })

  it("counts one clip as one", async () => {
    const added = await dispatch({ type: "addPattern" })
    const track = await dispatch({ type: "addPlaylistTrack" })
    if (!added || !track) throw new Error("setup failed")
    await dispatch({
      type: "addClips",
      clips: [
        {
          track: track.created[0],
          start: 0,
          content: { type: "pattern", pattern: added.created[0] },
        },
      ],
    })
    expect(patternLoss(added.created[0])).toBe(
      "1 clip on the playlist plays it and is deleted with it. Undo brings everything back."
    )
  })

  it("deletes an empty pattern nothing plays without asking", async () => {
    const added = await dispatch({ type: "addPattern" })
    if (!added) throw new Error("no pattern")
    const empty = added.created[0]
    expect(patternLoss(empty)).toBeNull()
    await deletePattern(empty)
    expect(question()).toBeNull()
    expect(project().patterns.map((item) => item.id)).not.toContain(empty)
  })
})
