import { fireEvent } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { attachGridInput } from "@/features/piano-roll/grid-input"
import { usePianoRollStore } from "@/features/piano-roll/store"
import {
  at,
  brief,
  notesOf,
  startRoll,
  TEST_VIEWPORT,
} from "@/features/piano-roll/test-utils"
import { GridMetrics } from "@/features/playlist/metrics"
import { attachPointer } from "@/features/playlist/pointer"
import { PlaylistSession } from "@/features/playlist/session"
import { startPlaylist, clips, project } from "@/features/playlist/test-utils"
import { usePlaylistStore } from "@/features/playlist/store"
import { dispatch, receivePatch } from "@/lib/store/project"
import { applyUiScale } from "@/lib/ui-scale"
import { settle } from "@/test/harness"
import { createFake2D } from "./test-utils"
import { TimeGridView } from "./time-grid-view"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))
beforeEach(() => {
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
    () => createFake2D().context
  )
  // Session label painters use the 2D context, too.
})
afterEach(() => {
  applyUiScale(100)
  vi.restoreAllMocks()
  document.body.replaceChildren()
})

async function grid(scale: number) {
  applyUiScale(scale)
  const container = document.createElement("div")
  Object.defineProperties(container, {
    clientWidth: { value: 800 },
    clientHeight: { value: 320 },
  })
  container.getBoundingClientRect = () =>
    new DOMRect(100, 50, (800 * scale) / 100, (320 * scale) / 100)
  document.body.append(container)
  const view = await TimeGridView.create(container, {
    renderer: "canvas2d",
    autoRender: false,
    initial: TEST_VIEWPORT,
  })
  return {
    view,
    pointer(type: string, point: { x: number; y: number }, ctrl = false) {
      const e = new MouseEvent(type, {
        bubbles: true,
        button: 0,
        clientX: 100 + (point.x * scale) / 100,
        clientY: 50 + (point.y * scale) / 100,
        ctrlKey: ctrl,
      })
      Object.defineProperties(e, {
        pointerId: { value: 1 },
        pointerType: { value: "mouse" },
      })
      fireEvent(view.element, e)
    },
  }
}

describe.each([75, 125, 150, 200])("real editor inputs at %i%%", (scale) => {
  it("draws, hits, moves, and box-selects notes through the actual view and grid handlers", async () => {
    const roll = await startRoll()
    await dispatch({ type: "addChannel", name: "Lead" })
    roll.show("Lead")
    const g = await grid(scale)
    roll.session.attachView(g.view)
    const detach = attachGridInput(roll.session, g.view)
    try {
      g.pointer("pointerdown", at(960, 60))
      g.pointer("pointerup", at(960, 60))
      await settle()
      expect(brief(notesOf("Lead"))).toEqual(["960:60:240"])
      const point = at(1080, 60)
      expect(g.view.hitTest(point.x, point.y)?.id).toBe(notesOf("Lead")[0].id)
      g.pointer("pointerdown", at(1080, 60))
      g.pointer("pointermove", at(2040, 62))
      g.pointer("pointerup", at(2040, 62))
      await settle()
      expect(brief(notesOf("Lead"))).toEqual(["1920:62:240"])
      usePianoRollStore.getState().setTool("select")
      g.pointer("pointerdown", { x: 20, y: 120 }, true)
      g.pointer("pointermove", { x: 200, y: 180 }, true)
      g.pointer("pointerup", { x: 200, y: 180 }, true)
      expect(roll.editor.selectionCount).toBe(1)
      // Pixel wheel movement is normalized, line-wheel movement stays logical.
      fireEvent.wheel(g.view.element, { deltaY: (16 * scale) / 100 })
      expect(g.view.viewport.scrollRow).toBeCloseTo(58)
      fireEvent.wheel(g.view.element, { deltaY: 1, deltaMode: 1 })
      expect(g.view.viewport.scrollRow).toBeCloseTo(58 + 1 / 16)
      fireEvent.wheel(g.view.element, { deltaY: 2, deltaMode: 2 })
      expect(g.view.viewport.scrollRow).toBeCloseTo(58 + 3 / 16)
    } finally {
      detach()
      roll.session.attachView(null)
      g.view.destroy()
      roll.stop()
    }
  })

  it("places and moves playlist clips, then hits and moves an automation point through actual pointer handlers", async () => {
    const app = await startPlaylist()
    const g = await grid(scale)
    g.view.setViewport({
      ...g.view.viewport,
      scrollRow: 0,
      pxPerTick: 0.025,
      rowHeight: 92,
    })
    const metrics = new GridMetrics()
    const detachMetrics = metrics.attach(g.view)
    g.view.setViewport({
      ...g.view.viewport,
      scrollRow: 0,
      pxPerTick: 0.025,
      rowHeight: 92,
    })
    let time = 1000
    const session = new PlaylistSession(g.view, metrics, () => time)
    const detach = attachPointer(g.view.element, session, metrics, {
      localPoint: (e) => g.view.localPoint(e),
      focus: () => {},
    })
    try {
      g.pointer("pointerdown", { x: 100, y: 40 })
      g.pointer("pointerup", { x: 100, y: 40 })
      await settle()
      expect(clips()).toHaveLength(1)
      const first = clips()[0]
      g.pointer("pointerdown", { x: 110, y: 40 })
      g.pointer("pointermove", { x: 206, y: 40 })
      g.pointer("pointerup", { x: 206, y: 40 })
      await settle()
      expect(clips()[0].start).toBe(first.start + 3840)
      const automated = await app.backend.automate({
        type: "trackVolume",
        track: project().mixer.tracks[1].id,
      })
      receivePatch(automated.patch)
      await settle()
      usePlaylistStore.getState().setTool("draw")
      const clip = project().playlist.clips.find(
        (c) => c.content.type === "automation"
      )!
      const row = project().playlist.tracks.findIndex(
        (t) => t.id === clip.track
      )
      const y = row * 92 + 53
      g.pointer("pointerdown", { x: 96, y })
      g.pointer("pointerup", { x: 96, y })
      await settle()
      expect(project().automations[0].points.some((p) => p.tick === 3840)).toBe(
        true
      )
      time += 2000
      g.pointer("pointerdown", { x: 96, y })
      g.pointer("pointermove", { x: 192, y: y - 17.5 })
      g.pointer("pointerup", { x: 192, y: y - 17.5 })
      await settle()
      expect(
        project().automations[0].points.some(
          (p) => p.tick === 7680 && p.value > 0.5
        )
      ).toBe(true)
    } finally {
      detach()
      session.destroy()
      detachMetrics()
      g.view.destroy()
      app.stop()
    }
  })
})
