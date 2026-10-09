import { act, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { readParam } from "@/features/params"
import { SimDocument } from "@/lib/ipc/sim/document"
import { dispatch, redo, undo } from "@/lib/store/project"
import {
  DEFAULT_TRACK_PROCESSING,
  TRACK_PROCESSING_INFO,
} from "@/lib/track-processing"
import { startTestApp } from "@/test/harness"

import { flush, history, project, trackNamed } from "./test-utils"
import { TrackProcessingPanel } from "./track-processing"

let app: Awaited<ReturnType<typeof startTestApp>>
beforeEach(async () => {
  app = await startTestApp()
  await dispatch({
    type: "addEffect",
    track: trackNamed("Kick").id,
    kind: "compressor",
  })
})
afterEach(() => {
  app.stop()
  vi.restoreAllMocks()
})

describe("integrated track controls against fresh shared Rust document WASM", () => {
  it.each(TRACK_PROCESSING_INFO.map((info, param) => ({ info, param })))(
    "edits $info.name with exact descriptor index, native bounds and history while preserving slots and other tracks",
    async ({ info, param }) => {
      const track = trackNamed("Kick").id
      render(<TrackProcessingPanel track={track} />)
      fireEvent.click(
        screen.getByRole("button", { name: "Track EQ and stereo" })
      )
      const before = structuredClone(project())
      const cursor = history().cursor
      const send = vi.spyOn(app.backend, "dispatch")
      if (info.kind === "toggle")
        fireEvent.click(
          screen.getByRole("button", { name: info.name })
        )
      else
        fireEvent.keyDown(screen.getByRole("slider", { name: info.name }), {
          key: "End",
        })
      await flush()
      expect(send).toHaveBeenCalled()
      expect(send.mock.calls[0][0]).toMatchObject({
        type: "setTrackParam",
        id: track,
        param,
      })
      const changed = structuredClone(project())
      const processing =
        trackNamed("Kick").processing ?? DEFAULT_TRACK_PROCESSING
      expect(readParam(processing, info)).toBeCloseTo(
        info.kind === "toggle" ? 1 - info.default : info.max,
        5
      )
      expect(history().cursor).toBe(cursor + 1)
      expect(changed.mixer.tracks.filter((item) => item.id !== track)).toEqual(
        before.mixer.tracks.filter((item) => item.id !== track)
      )
      expect(trackNamed("Kick").effects).toEqual(
        before.mixer.tracks.find((item) => item.id === track)!.effects
      )
      expect(changed.channels).toEqual(before.channels)
      expect(changed.patterns).toEqual(before.patterns)
      const snapshot = await app.backend.documentSnapshot()
      const doc = SimDocument.create(snapshot.project)
      const reopened = SimDocument.open(doc.fileText())
      expect(reopened.project()).toEqual(snapshot.project)
      doc.dispose()
      reopened.dispose()
      await act(async () => {
        await undo()
      })
      expect(project()).toEqual(before)
      await act(async () => {
        await redo()
      })
      expect(project()).toEqual(changed)
      if (info.kind !== "toggle") {
        fireEvent.keyDown(screen.getByRole("slider", { name: info.name }), {
          key: "Home",
        })
        await flush()
        expect(
          readParam(
            trackNamed("Kick").processing ?? DEFAULT_TRACK_PROCESSING,
            info
          )
        ).toBeCloseTo(info.min, 5)
      }
    }
  )

  it.each(["Master", "Kick"])(
    "provides all three EQ bands and utilities on %s and resets only integrated settings in one undo step",
    async (name) => {
      const track = trackNamed(name).id
      render(<TrackProcessingPanel track={track} />)
      fireEvent.click(
        screen.getByRole("button", { name: "Track EQ and stereo" })
      )
      for (const label of ["Low shelf", "Mid bell", "High shelf"])
        expect(screen.getByRole("group", { name: label })).toBeInTheDocument()
      for (const label of [
        "Invert left polarity",
        "Invert right polarity",
        "Swap left and right",
      ]) {
        fireEvent.click(screen.getByRole("button", { name: label }))
        await flush()
      }
      fireEvent.keyDown(screen.getByRole("slider", { name: "Mid gain" }), {
        key: "End",
      })
      await flush()
      const before = structuredClone(project())
      const cursor = history().cursor
      fireEvent.click(
      screen.getByRole("button", { name: "Reset" })
      )
      await flush()
      expect(trackNamed(name).processing ?? DEFAULT_TRACK_PROCESSING).toEqual(
        DEFAULT_TRACK_PROCESSING
      )
      expect(history().cursor).toBe(cursor + 1)
      expect(trackNamed(name).effects).toEqual(
        before.mixer.tracks.find((item) => item.id === track)!.effects
      )
      expect(
        project().mixer.tracks.filter((item) => item.id !== track)
      ).toEqual(before.mixer.tracks.filter((item) => item.id !== track))
      await act(async () => {
        await undo()
      })
      expect(project()).toEqual(before)
    }
  )
})
