import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { Clip } from "@/bindings"
import { dispatch } from "@/lib/store/project"
import { settle } from "@/test/harness"

import { addAudioFile } from "./audio/ops"
import { attachPointer } from "./pointer"
import {
  BAR,
  project,
  PX_PER_TICK,
  ROW_HEIGHT,
  selection,
  startPlaylist,
  startSession,
  tracks,
  ui,
} from "./test-utils"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

const LOOP = "/factory/Loops/Drum loop 128.wav"

let stop: () => void
beforeEach(async () => {
  ;({ stop } = await startPlaylist())
})
afterEach(() => stop())

const clipOf = (id: number) =>
  project().playlist.clips.find((clip) => clip.id === id) as Clip
const rowOf = (clip: Clip) =>
  tracks().findIndex((track) => track.id === clip.track)

function pointer(type: string, clientX: number, clientY: number) {
  return new MouseEvent(type, { clientX, clientY, button: 0, bubbles: true })
}

describe("the grid's pointer", () => {
  it("does not take the grid moving under a still pointer for a drag", async () => {
    for (let row = 0; row < 3; row++) {
      await dispatch({ type: "addPlaylistTrack" })
    }
    const id = (await addAudioFile(LOOP, { start: 0, row: 2 })) as number
    await settle()
    ui().clearSelection()
    const row = rowOf(clipOf(id))
    expect(row).toBeGreaterThan(0)

    const { session, metrics, stop: stopSession } = startSession()
    const element = document.createElement("div")
    // The grid sits 168 pixels down, until a selected audio clip brings a
    // strip of settings in above it, which is what the real layout did.
    const top = () => (ui().selection.size > 0 ? 204 : 168)
    const detach = attachPointer(element, session, metrics, {
      localPoint: (event) => ({ x: event.clientX, y: event.clientY - top() }),
      focus: () => {},
    })

    const x = BAR * PX_PER_TICK * 0.5
    const y = 168 + row * ROW_HEIGHT + ROW_HEIGHT / 2
    element.dispatchEvent(pointer("pointerdown", x, y))
    // The press selected the clip, and the grid has moved.
    expect(selection()).toEqual([id])
    expect(top()).toBe(204)

    // Straight to the right by a bar: no vertical movement at all.
    for (let step = 1; step <= 4; step++) {
      element.dispatchEvent(
        pointer("pointermove", x + (BAR * PX_PER_TICK * step) / 4, y)
      )
    }
    element.dispatchEvent(pointer("pointerup", x + BAR * PX_PER_TICK, y))
    await settle()

    expect(clipOf(id).start).toBe(BAR)
    expect(rowOf(clipOf(id))).toBe(row)

    detach()
    stopSession()
  })
})
