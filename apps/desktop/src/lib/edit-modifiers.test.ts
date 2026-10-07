import { describe, expect, it } from "vitest"

import { hintFor as rollHint } from "@/features/piano-roll/intents"
import { hintFor as playlistHint, INNER_HINTS } from "@/features/playlist/grid"

import { ignoresSnap, MODIFIER_HINTS } from "./edit-modifiers"

describe("the modifier keys of the two editors", () => {
  it("lets go of the snap for Alt and nothing else", () => {
    expect(ignoresSnap({ alt: true })).toBe(true)
    expect(ignoresSnap({ alt: false })).toBe(false)
  })

  it("is described with the same words over a note and over a clip", () => {
    const overNote = rollHint({ kind: "move" }, "draw") ?? ""
    const overClips = playlistHint("draw", "Pattern 1")
    for (const hint of [overNote, overClips]) {
      expect(hint).toContain(MODIFIER_HINTS.copy)
      expect(hint).toContain(MODIFIER_HINTS.add)
      expect(hint).toContain(MODIFIER_HINTS.free)
    }
  })

  it("is short enough for the status bar to show whole at 1440 pixels", () => {
    // About 175 characters fit there beside the engine and the file's
    // state. The longest of these used to be over 230.
    const lines = [
      ...(["draw", "paint", "select", "erase", "mute"] as const).map((tool) =>
        playlistHint(tool, "Drum loop 128")
      ),
      ...Object.values(INNER_HINTS),
      rollHint({ kind: "move" }, "draw") ?? "",
      rollHint({ kind: "draw" }, "draw") ?? "",
    ]
    for (const line of lines) {
      expect(line.length, line).toBeLessThanOrEqual(140)
    }
  })

  it("never promises that Shift copies or lets go of the snap", () => {
    const hints = [
      ...(["draw", "paint", "select", "erase"] as const).flatMap((tool) => [
        rollHint({ kind: "move" }, tool),
        rollHint({ kind: "draw" }, tool),
        rollHint({ kind: "resize", edge: "end" }, tool),
        rollHint({ kind: "resize", edge: "start" }, tool),
      ]),
      ...(["draw", "paint", "select", "erase", "mute"] as const).map((tool) =>
        playlistHint(tool, "Pattern 1")
      ),
    ]
    for (const hint of hints) {
      expect(hint).not.toMatch(/Shift\+drag copies|Shift: no snap|Hold Shift/)
    }
  })
})
