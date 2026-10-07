import { act, render, screen, waitFor, within } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import {
  selectEqBand,
  setEqRange,
  useEqView,
} from "@/features/effects/eq/eq-view"
import MixerPanel from "@/features/mixer"
import { useRackStore } from "@/features/channel-rack/rack-store"
import { useEffectsUi } from "@/features/mixer/effects-ui"
import { useMixerUi } from "@/features/mixer/mixer-ui"
import { peakOf } from "@/features/mixer/peaks"
import { strip, trackNamed } from "@/features/mixer/test-utils"
import {
  createSession,
  readContext,
} from "@/features/piano-roll/create-session"
import { forgetSavedViews } from "@/features/piano-roll/session"
import { usePianoRollStore } from "@/features/piano-roll/store"
import PlaylistPanel from "@/features/playlist"
import { GridMetrics, initialView } from "@/features/playlist/metrics"
import { usePlaylistStore } from "@/features/playlist/store"
import { MasterMeterSlot } from "@/features/transport/master-meter-slot"
import { TimeGridView } from "@/lib/canvas"
import { createFake2D, createFakeGl } from "@/lib/canvas/test-utils"
import type { MockBackend } from "@/lib/ipc/mock"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { onProjectReplaced } from "@/lib/store/replaced"
import { useUiStore } from "@/lib/store/ui"
import { settle, startTestApp } from "@/test/harness"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let backend: MockBackend
let stop: () => void

beforeEach(async () => {
  ;({ backend, stop } = await startTestApp())
})
afterEach(() => {
  stop()
  vi.restoreAllMocks()
  document.body.replaceChildren()
})

async function flush() {
  await act(async () => {
    await settle()
  })
}

/** File > New, as the shell does it: a snapshot and a `project:loaded`. */
async function newProject() {
  await act(async () => {
    await backend.projectNew()
    await settle()
  })
}

const SOON = { timeout: 3000 }

describe("the project-replaced signal", () => {
  it("fires after New and after Open, with the new project already in the store", async () => {
    const seen: string[] = []
    const off = onProjectReplaced(() =>
      seen.push(useProjectStore.getState().project.settings.name)
    )
    // Starting up and editing replace nothing.
    await dispatch({ type: "updateSettings", patch: { name: "First song" } })
    expect(seen).toEqual([])

    const path = await backend.projectSave("/projects/first.windfall")
    await newProject()
    expect(seen).toHaveLength(1)
    expect(seen[0]).not.toBe("First song")

    await act(async () => {
      await backend.projectOpen(path)
      await settle()
    })
    expect(seen).toEqual([seen[0], "First song"])

    off()
    await newProject()
    expect(seen).toHaveLength(2)
  })
})

describe("what the tools remember", () => {
  it("gives the first note of a new project the default length and velocity", async () => {
    // A long, quiet note was the last one touched in the old project.
    usePianoRollStore.getState().rememberNote(16 * 960, 0.3)
    expect(usePianoRollStore.getState().lastLength).toBe(16 * 960)

    await newProject()
    const roll = usePianoRollStore.getState()
    expect(roll.lastLength).toBe(240)
    expect(roll.lastVelocity).toBe(0.8)
    // What is a matter of taste stays as it was set.
    usePianoRollStore.getState().setTool("paint")
    await newProject()
    expect(usePianoRollStore.getState().tool).toBe("paint")
    usePianoRollStore.getState().setTool("draw")
  })

  it("places the pattern again in the playlist, not a sound of the old project", async () => {
    const sample = useProjectStore.getState().project.samples[0].id
    usePlaylistStore.getState().setBrush({ type: "audio", sample })
    usePlaylistStore.getState().setClipboard([
      {
        row: 0,
        start: 0,
        length: 3840,
        offset: 0,
        muted: false,
        content: { type: "pattern", pattern: 1 },
      },
    ])
    await newProject()
    expect(usePlaylistStore.getState().brush).toEqual({ type: "pattern" })
    // Its clips named patterns and samples by ids that mean others now.
    expect(usePlaylistStore.getState().clipboard).toEqual([])
  })

  it("closes a rename or a color choice that was for a track or channel of the old project", async () => {
    const track = useProjectStore.getState().project.mixer.tracks[1].id
    const channel = useProjectStore.getState().project.channels[0].id
    useMixerUi.setState({ renaming: track, coloring: track, focusing: track })
    useRackStore.getState().openColorPicker(channel)
    await newProject()
    expect(useMixerUi.getState()).toEqual({
      renaming: null,
      coloring: null,
      focusing: null,
    })
    expect(useRackStore.getState().colorPickerFor).toBeNull()
  })

  it("keeps copied notes, which name nothing by id, and pastes them into the new project", async () => {
    const before = useProjectStore.getState().project
    const session = createSession()
    session.setEditing(before.patterns[0].id, before.channels[0].id)
    session.editor.setContext(
      readContext(before.patterns[0].id, before.channels[0].id)
    )
    session.editor.selectAll()
    session.editor.copy()
    const copied = usePianoRollStore.getState().clipboardCount
    expect(copied).toBeGreaterThan(0)
    session.dispose()

    await newProject()
    expect(usePianoRollStore.getState().clipboardCount).toBe(copied)
    const project = useProjectStore.getState().project
    const pattern = project.patterns[0].id
    const channel = project.channels[0].id
    const notes = () =>
      useProjectStore
        .getState()
        .project.patterns[0].lanes.find((item) => item.channel === channel)
        ?.notes ?? []
    const had = notes().length
    const next = createSession()
    next.setEditing(pattern, channel)
    next.editor.setContext(readContext(pattern, channel))
    await act(async () => {
      await next.editor.paste({ at: "playhead", tick: 0 })
      await settle()
    })
    // The notes land in the pattern and channel that are open now.
    expect(notes()).toHaveLength(had + copied)
    next.dispose()
  })
})

describe("what the mixer holds", () => {
  beforeEach(() => {
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null)
  })

  it("drops held peaks and clip lights when another project loads", async () => {
    render(
      <>
        <MasterMeterSlot />
        <MixerPanel />
      </>
    )
    const peak = (name: string) =>
      within(strip(name)).getByRole("button", { name: /^Peak/ })
    const transportMeter = () =>
      screen
        .getByRole("group", { name: "Master level" })
        .querySelector("[data-slot=level-meter]")

    const kick = useProjectStore
      .getState()
      .project.channels.find((channel) => channel.name === "Kick")
    if (!kick) throw new Error("The demo has no kick")
    await dispatch({
      type: "updateChannel",
      id: kick.id,
      patch: { volume: 1 },
    })
    await dispatch({
      type: "updateMixerTrack",
      id: trackNamed("Kick").id,
      patch: { volume: 2 },
    })
    await dispatch({
      type: "updateMixerTrack",
      id: trackNamed("Master").id,
      patch: { volume: 2 },
    })
    await backend.transportPlay()
    await waitFor(() => {
      expect(peak("Kick")).toHaveAttribute("data-clipped")
      expect(peak("Master")).toHaveAttribute("data-clipped")
      expect(transportMeter()).toHaveAttribute("data-clipped")
    }, SOON)
    await backend.transportStop()
    const kickTrack = trackNamed("Kick").id

    await newProject()
    // The new project's kick is on a track with the same id, and nothing
    // has played yet.
    expect(trackNamed("Kick Punch").id).toBe(kickTrack)
    expect(peakOf(kickTrack)).toBe(0)
    expect(peakOf(trackNamed("Master").id)).toBe(0)
    expect(peak("Master")).toHaveTextContent("Peak −∞")
    expect(peak("Master")).not.toHaveAttribute("data-clipped")
    expect(peak("Kick Punch")).toHaveTextContent("Peak −∞")
    expect(peak("Kick Punch")).not.toHaveAttribute("data-clipped")
    expect(
      strip("Master").querySelector("[data-slot=level-meter-clip]")
    ).not.toHaveAttribute("data-clipped")
    expect(transportMeter()).not.toHaveAttribute("data-clipped")
  })

  it("forgets folded effects, the selected effect and each equaliser's view", async () => {
    useEffectsUi.setState({ collapsed: [7, 9], selectedEffect: 7 })
    selectEqBand(7, "highShelf")
    setEqRange(7, 12)

    await newProject()
    expect(useEffectsUi.getState().collapsed).toEqual([])
    expect(useEffectsUi.getState().selectedEffect).toBeNull()
    expect(useEqView.getState()).toEqual({ band: {}, range: {} })
  })
})

describe("where the piano roll was scrolled to", () => {
  beforeEach(() => {
    forgetSavedViews()
    const gl = createFakeGl()
    const drawn = new WeakMap<
      HTMLCanvasElement,
      ReturnType<typeof createFake2D>
    >()
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
      function (this: HTMLCanvasElement, kind: string) {
        if (kind === "webgl2") return gl.context
        let fake = drawn.get(this)
        if (!fake) {
          fake = createFake2D()
          drawn.set(this, fake)
        }
        return fake.context
      }
    )
  })

  async function openLane(key: string) {
    const container = document.createElement("div")
    Object.defineProperty(container, "clientWidth", { value: 800 })
    Object.defineProperty(container, "clientHeight", { value: 400 })
    document.body.append(container)
    const view = await TimeGridView.create(container, { autoRender: false })
    const session = createSession()
    session.attachView(view)
    session.showLane(key)
    return { session, view }
  }

  const scrolled = { scrollTick: 4800, pxPerTick: 0.25 }

  it("comes back for the same lane, but not in another project", async () => {
    const first = await openLane("1:2")
    const opening = first.view.viewport
    first.view.setViewport({ ...opening, ...scrolled })
    first.session.dispose()

    // Coming back to the lane in the same project finds it as it was left.
    const second = await openLane("1:2")
    expect(second.view.viewport).toMatchObject(scrolled)
    second.session.dispose()

    await newProject()
    const third = await openLane("1:2")
    expect(third.view.viewport.scrollTick).toBe(opening.scrollTick)
    expect(third.view.viewport.pxPerTick).toBe(opening.pxPerTick)
    third.session.dispose()
  })

  it("is not written back by the editor that was open when the project changed", async () => {
    const open = await openLane("1:2")
    const opening = open.view.viewport
    open.view.setViewport({ ...opening, ...scrolled })

    await newProject()
    // The panel starts over; the old editor goes away after the signal.
    open.session.dispose()

    const next = await openLane("1:2")
    expect(next.view.viewport.scrollTick).toBe(opening.scrollTick)
    expect(next.view.viewport.pxPerTick).toBe(opening.pxPerTick)
    next.session.dispose()
  })
})

describe("where the playlist was scrolled to", () => {
  it("is kept between tabs and dropped with the project", async () => {
    const fresh = initialView()
    const metrics = new GridMetrics()
    metrics.setViewport({ ...metrics.viewport, scrollTick: 7680 })
    expect(initialView()).toMatchObject({ scrollTick: 7680 })

    await newProject()
    expect(initialView()).toEqual(fresh)
    // The panel that was open cannot bring its view back either.
    metrics.setViewport({ ...metrics.viewport, scrollTick: 3840 })
    expect(initialView()).toEqual(fresh)
  })

  it("starts the panel over, and drops the selection and the clipboard", async () => {
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null)
    useUiStore.getState().showCenterTab("playlist")
    render(<PlaylistPanel />)
    await flush()
    const before = document.querySelector("[data-slot=playlist]")
    expect(before).not.toBeNull()
    usePlaylistStore.setState({ selection: new Set([4, 5]), cursorTick: 960 })

    await newProject()
    const after = document.querySelector("[data-slot=playlist]")
    expect(after).not.toBeNull()
    expect(after).not.toBe(before)
    expect(usePlaylistStore.getState().selection.size).toBe(0)
    expect(usePlaylistStore.getState().cursorTick).toBe(0)
  })
})
