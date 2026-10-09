import {
  act,
  fireEvent,
  render,
  screen,
  within,
  waitFor,
} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { StrictMode } from "react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { SimDocument } from "@/lib/ipc/sim/document"
import { dispatch, redo, undo } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { startTestApp } from "@/test/harness"

import MixerPanel from "."
import { registerMixerActions } from "./actions"
import { useEffectsUi } from "./effects-ui"
import { useMixerUi } from "./mixer-ui"
import {
  drag,
  flush,
  history,
  project,
  sizeMixer,
  strip,
  stubCanvas,
  trackNamed,
} from "./test-utils"

let app: Awaited<ReturnType<typeof startTestApp>>
beforeEach(async () => {
  stubCanvas()
  sizeMixer(1000, 640)
  useMixerUi.setState(useMixerUi.getInitialState(), true)
  app = await startTestApp()
  await dispatch({
    type: "batch",
    commands: [
      {
        type: "updateMixerTrack",
        id: trackNamed("Kick").id,
        patch: { dock: "left", volume: 0.5, pan: 0.8 },
      },
      {
        type: "updateMixerTrack",
        id: trackNamed("Clap").id,
        patch: { dock: "middle", volume: 1, pan: -0.9 },
      },
      {
        type: "updateMixerTrack",
        id: trackNamed("Hat").id,
        patch: { dock: "right", volume: 0.25, pan: 0.1 },
      },
    ],
  })
  await dispatch({
    type: "addEffect",
    track: trackNamed("Kick").id,
    kind: "compressor",
  })
})
afterEach(() => {
  app.stop()
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

function select(
  name: string,
  modifiers: { ctrlKey?: boolean; shiftKey?: boolean } = {}
) {
  const event = new MouseEvent("pointerdown", {
    bubbles: true,
    cancelable: true,
    button: 0,
    ...modifiers,
  })
  Object.defineProperty(event, "pointerId", { value: 9 })
  Object.defineProperty(event, "pointerType", { value: "mouse" })
  fireEvent(strip(name), event)
}

describe("mounted mixer selection and layouts against shared Rust WASM", () => {
  it("keeps boot and inspector action holders registered exactly once through StrictMode dock remounts", async () => {
    const user = userEvent.setup()
    const releaseBoot = registerMixerActions()
    try {
      useUiStore.setState({ mixerLayout: "extraLarge" })
      useEffectsUi.setState({ inspectorOpen: true })
      const mounted = render(
        <StrictMode>
          <MixerPanel />
        </StrictMode>
      )
      select("Kick")
      select("Clap", { ctrlKey: true })
      select("Snare", { ctrlKey: true })
      await user.click(screen.getByRole("button", { name: "3 selected" }))
      await user.click(screen.getByRole("button", { name: "Dock left" }))
      await flush()
      for (const name of ["Kick", "Clap", "Snare"])
        expect(
          strip(name).closest("[data-slot=mixer-dock-left]")
        ).not.toBeNull()
      expect(strip("Hat")).toBeInTheDocument()
      mounted.unmount()
      const reopened = render(
        <StrictMode>
          <MixerPanel />
        </StrictMode>
      )
      expect(strip("Hat")).toBeInTheDocument()
      reopened.unmount()
    } finally {
      releaseBoot()
      useEffectsUi.setState({ inspectorOpen: false })
    }
  })
  it("extends and contracts keyboard ranges across dock boundaries while focus follows the primary strip", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    const before = structuredClone(project())
    const cursor = history().cursor
    select("Kick")
    act(() => strip("Kick").focus())
    await user.keyboard("{Shift>}{ArrowRight}{ArrowRight}{ArrowRight}{/Shift}")
    expect(useMixerUi.getState().selected).toEqual(
      ["Kick", "Clap", "Snare", "Hat"].map((name) => trackNamed(name).id)
    )
    expect(strip("Hat")).toHaveFocus()
    expect(useUiStore.getState().selectedTrack).toBe(trackNamed("Hat").id)
    await user.keyboard("{Shift>}{ArrowLeft}{ArrowLeft}{/Shift}")
    expect(useMixerUi.getState().selected).toEqual([
      trackNamed("Kick").id,
      trackNamed("Clap").id,
    ])
    expect(strip("Clap")).toHaveFocus()
    expect(project()).toEqual(before)
    expect(history().cursor).toBe(cursor)
  })

  it("rejects a captured group move after the model order changes without losing the intervening edit", async () => {
    render(<MixerPanel />)
    select("Kick")
    select("Hat", { ctrlKey: true })
    const expected = project().mixer.tracks.map((track) => track.id)
    const ids = useMixerUi.getState().selected.slice()
    await dispatch({
      type: "moveMixerTracks",
      expected,
      ids: [trackNamed("Clap").id],
      before: null,
    })
    await flush()
    const changed = structuredClone(project())
    const cursor = history().cursor
    await expect(
      app.backend.dispatch({
        type: "moveMixerTracks",
        expected,
        ids,
        before: null,
      })
    ).rejects.toThrow()
    expect(project()).toEqual(changed)
    expect(history().cursor).toBe(cursor)
    expect(useMixerUi.getState().selected).toEqual(ids)
    await act(async () => {
      await undo()
    })
    expect(project().mixer.tracks.map((track) => track.id)).toEqual(expected)
  })

  it("keeps independent dock scroll positions and aligns strip bottoms as differing scrollbar sizes change on resize", async () => {
    for (const dock of ["left", "middle", "right"] as const) {
      await dispatch({
        type: "batch",
        commands: Array.from({ length: 8 }, (_, index) => ({
          type: "addMixerTrack" as const,
          name: `${dock} ${index}`,
        })),
      })
      await dispatch({
        type: "batch",
        commands: project()
          .mixer.tracks.filter((track) => track.name.startsWith(`${dock} `))
          .map((track) => ({
            type: "updateMixerTrack" as const,
            id: track.id,
            patch: { dock },
          })),
      })
    }
    const before = structuredClone(project())
    const cursor = history().cursor
    let leftBar = 16
    let rightBar = 8
    let dockWidth = 160
    const callbacks = new Set<ResizeObserverCallback>()
    vi.stubGlobal(
      "ResizeObserver",
      class {
        constructor(callback: ResizeObserverCallback) {
          callbacks.add(callback)
        }
        observe() {}
        unobserve() {}
        disconnect() {}
      }
    )
    const dock = (element: Element) =>
      element instanceof HTMLElement &&
      element.dataset.slot?.startsWith("mixer-dock-")
    vi.spyOn(Element.prototype, "clientWidth", "get").mockImplementation(
      function (this: Element) {
        return dock(this) ? dockWidth : 0
      }
    )
    vi.spyOn(Element.prototype, "clientHeight", "get").mockImplementation(
      function (this: Element) {
        return dock(this)
          ? 640 -
              ((this as HTMLElement).dataset.slot === "mixer-dock-left"
                ? leftBar
                : (this as HTMLElement).dataset.slot === "mixer-dock-right"
                  ? rightBar
                  : 0)
          : 0
      }
    )
    vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(
      function (this: HTMLElement) {
        return dock(this) ? 640 : 0
      }
    )
    const mounted = render(<MixerPanel />)
    const left = screen.getByRole("group", { name: "left mixer dock" })
    const middle = screen.getByRole("group", { name: "middle mixer dock" })
    const right = screen.getByRole("group", { name: "right mixer dock" })
    expect(left.firstElementChild).not.toHaveStyle({
      height: "calc(100% - 16px)",
    })
    expect(middle.firstElementChild).toHaveStyle({
      height: "calc(100% - 16px)",
    })
    expect(right.firstElementChild).toHaveStyle({ height: "calc(100% - 8px)" })
    expect(
      mounted.container.querySelector("[data-slot=mixer-master]")
    ).toHaveStyle({ paddingBottom: "16px" })
    fireEvent.wheel(left, { deltaY: 80 })
    fireEvent.wheel(right, { deltaY: 160 })
    fireEvent.scroll(left)
    fireEvent.scroll(right)
    expect([left.scrollLeft, middle.scrollLeft, right.scrollLeft]).toEqual([
      80, 0, 160,
    ])
    expect(project()).toEqual(before)
    expect(history().cursor).toBe(cursor)
    leftBar = 0
    rightBar = 12
    dockWidth = 100
    await act(async () => {
      for (const callback of callbacks) callback([], {} as ResizeObserver)
    })
    expect(left.firstElementChild).toHaveStyle({ height: "calc(100% - 12px)" })
    expect(middle.firstElementChild).toHaveStyle({
      height: "calc(100% - 12px)",
    })
    expect(right.firstElementChild).not.toHaveStyle({
      height: "calc(100% - 8px)",
    })
    expect(
      mounted.container.querySelector("[data-slot=mixer-master]")
    ).toHaveStyle({ paddingBottom: "12px" })
    expect([left.scrollLeft, middle.scrollLeft, right.scrollLeft]).toEqual([
      80, 0, 160,
    ])
    mounted.unmount()
    vi.unstubAllGlobals()
  })
  it("toggles Ctrl members and extends Shift ranges in visual order across all docks", async () => {
    render(<MixerPanel />)
    expect(strip("Kick").closest("[data-slot=mixer-dock-left]")).not.toBeNull()
    expect(
      strip("Clap").closest("[data-slot=mixer-dock-middle]")
    ).not.toBeNull()
    expect(strip("Hat").closest("[data-slot=mixer-dock-right]")).not.toBeNull()
    const before = structuredClone(project())
    const cursor = history().cursor
    select("Kick")
    select("Hat", { ctrlKey: true })
    expect(useMixerUi.getState().selected).toEqual([
      trackNamed("Kick").id,
      trackNamed("Hat").id,
    ])
    select("Kick", { ctrlKey: true })
    expect(useMixerUi.getState().selected).toEqual([trackNamed("Hat").id])
    select("Hat")
    select("Clap", { shiftKey: true })
    expect(useMixerUi.getState().selected).toEqual([
      trackNamed("Clap").id,
      trackNamed("Snare").id,
      trackNamed("Hat").id,
    ])
    expect(useUiStore.getState().selectedTrack).toBe(trackNamed("Clap").id)
    for (const name of ["Clap", "Snare", "Hat"])
      expect(strip(name)).toHaveAttribute("data-group-selected", "true")
    expect(project()).toEqual(before)
    expect(history().cursor).toBe(cursor)
  })

  it("applies captured gain ratios and bounded pan deltas to selected tracks in one undo step per gesture", async () => {
    render(<MixerPanel />)
    select("Kick")
    select("Clap", { ctrlKey: true })
    select("Hat", { ctrlKey: true })
    const before = structuredClone(project())
    const cursor = history().cursor
    drag(
      within(strip("Kick")).getByRole("slider", { name: "Kick volume" }),
      [-8, -8]
    )
    await flush()
    const ratio = trackNamed("Kick").volume / 0.5
    expect(trackNamed("Clap").volume).toBeCloseTo(ratio, 6)
    expect(trackNamed("Hat").volume).toBeCloseTo(0.25 * ratio, 6)
    expect(trackNamed("Snare").volume).toBe(1)
    expect(history().cursor).toBe(cursor + 1)
    await act(async () => {
      await undo()
    })
    expect(project()).toEqual(before)
    await act(async () => {
      await redo()
    })
    const gainAdjusted = structuredClone(project())
    drag(
      within(strip("Kick")).getByRole("slider", { name: "Kick pan" }),
      [10, 10]
    )
    await flush()
    expect(trackNamed("Kick").pan).toBe(1)
    expect(trackNamed("Clap").pan).toBeCloseTo(-0.7, 6)
    expect(trackNamed("Hat").pan).toBeCloseTo(0.3, 6)
    expect(history().cursor).toBe(cursor + 2)
    await act(async () => {
      await undo()
    })
    expect(project()).toEqual(gainAdjusted)
  })

  it("moves a cross-dock group as a stable block through the toolbar and preserves IDs, settings, ownership and saved docks", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    select("Kick")
    select("Hat", { ctrlKey: true })
    const before = structuredClone(project())
    const cursor = history().cursor
    await user.click(screen.getByRole("button", { name: "2 selected" }))
    await user.click(
      screen.getByRole("combobox", { name: "Move selected tracks" })
    )
    await user.click(screen.getByRole("option", { name: "End of mixer" }))
    await flush()
    const ids = before.mixer.tracks.map((track) => track.id)
    const moving = [trackNamed("Kick").id, trackNamed("Hat").id]
    expect(project().mixer.tracks.map((track) => track.id)).toEqual([
      ...ids.filter((id) => !moving.includes(id)),
      ...moving,
    ])
    expect(
      project()
        .mixer.tracks.map((track) => ({ ...track }))
        .sort((a, b) => a.id - b.id)
    ).toEqual([...before.mixer.tracks].sort((a, b) => a.id - b.id))
    expect(project().channels).toEqual(before.channels)
    expect(project().automations).toEqual(before.automations)
    expect(history().cursor).toBe(cursor + 1)
    const doc = SimDocument.create(
      (await app.backend.documentSnapshot()).project
    )
    const reopened = SimDocument.open(doc.fileText())
    expect(reopened.project().mixer).toEqual(
      (await app.backend.documentSnapshot()).project.mixer
    )
    doc.dispose()
    reopened.dispose()
    await act(async () => {
      await undo()
    })
    expect(project()).toEqual(before)
  })

  it("offers all eight explicit size layouts, mounts their exact widths, and rehydrates the saved preference", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    const before = structuredClone(project())
    for (const [label, value, width] of [
      ["Compact", "compact", 36],
      ["Compact 2", "compact2", 48],
      ["Narrow", "narrow", 64],
      ["Standard", "standard", 80],
      ["Wide", "wide", 104],
      ["Extra wide", "extraWide", 128],
      ["Large", "large", 152],
      ["Extra large", "extraLarge", 176],
    ] as const) {
      await user.click(
        screen.getByRole("combobox", { name: "Mixer size layout" })
      )
      await user.click(await screen.findByRole("option", { name: label }))
      await waitFor(() => expect(screen.queryByRole("listbox")).toBeNull())
      await flush()
      expect(useUiStore.getState().mixerLayout).toBe(value)
      expect(strip("Clap").parentElement).toHaveStyle({ width: `${width}px` })
      const saved = localStorage.getItem("windfall.ui")!
      await act(async () => {
        useUiStore.setState({ mixerLayout: "adaptive" })
        localStorage.setItem("windfall.ui", saved)
        await useUiStore.persist.rehydrate()
      })
      expect(useUiStore.getState().mixerLayout).toBe(value)
      expect(strip("Clap").parentElement).toHaveStyle({ width: `${width}px` })
    }
    expect(project()).toEqual(before)
  })

  it("routes and docks only selected inserts through mounted group controls with undo and saved routing intact", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    select("Kick")
    select("Hat", { ctrlKey: true })
    const before = structuredClone(project())
    const cursor = history().cursor
    await user.click(screen.getByRole("button", { name: "2 selected" }))
    await user.click(
      screen.getByRole("combobox", { name: "Route selected tracks" })
    )
    expect(screen.queryByRole("option", { name: "Kick" })).toBeNull()
    expect(screen.queryByRole("option", { name: "Hat" })).toBeNull()
    expect(screen.getByRole("option", { name: "Master" })).toBeInTheDocument()
    await user.click(screen.getByRole("option", { name: "Clap" }))
    await waitFor(() => expect(screen.queryByRole("listbox")).toBeNull())
    await flush()
    const expected = structuredClone(before)
    for (const track of expected.mixer.tracks) {
      if ([trackNamed("Kick").id, trackNamed("Hat").id].includes(track.id))
        track.output = trackNamed("Clap").id
    }
    expect(project()).toEqual(expected)
    expect(history().cursor).toBe(cursor + 1)
    await user.click(screen.getByRole("button", { name: "Dock middle" }))
    await flush()
    for (const track of expected.mixer.tracks) {
      if ([trackNamed("Kick").id, trackNamed("Hat").id].includes(track.id))
        // The native file/patch format omits the default middle dock.
        delete track.dock
    }
    expect(project()).toEqual(expected)
    expect(history().cursor).toBe(cursor + 2)
    const doc = SimDocument.create(
      (await app.backend.documentSnapshot()).project
    )
    const reopened = SimDocument.open(doc.fileText())
    expect(reopened.project().mixer).toEqual(
      (await app.backend.documentSnapshot()).project.mixer
    )
    doc.dispose()
    reopened.dispose()
    await act(async () => {
      await undo()
      await undo()
    })
    expect(project()).toEqual(before)
  })
})
