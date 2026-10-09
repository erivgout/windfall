import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { runAction } from "@/lib/actions"
import { newProject, openProjectPath } from "@/lib/flows/project"
import type { Backend } from "@/lib/ipc"
import { SimDocument } from "@/lib/ipc/sim/document"
import { dispatch, redo, undo, useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"

import {
  applyAdvancedFill,
  closeAdvancedFill,
  fillPreview,
  fillRequestIsCurrent,
  openAdvancedFill,
  useAdvancedFill,
  type FillOptions,
} from "./advanced-fill"
import { AdvancedFillDialog } from "./advanced-fill-dialog"
import ChannelRackPanel from "./index"
import { channel, history, notesOf, project, startRack } from "./test-utils"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

const options = (patch: Partial<FillOptions> = {}): FillOptions => ({
  rule: "euclidean",
  startStep: 0,
  endStep: 16,
  cycle: 16,
  every: 4,
  hits: 5,
  rotation: 0,
  seed: 1,
  key: 60,
  velocity: 0.8,
  gate: 1,
  replace: true,
  ...patch,
})
const steps = (settings: FillOptions, length = 16) =>
  fillPreview(settings, length)!.map((note) => note.start / 240)

describe("bounded rhythm preview", () => {
  it("distributes exact Euclidean pulses with cyclic gaps differing by at most one", () => {
    for (let cycle = 1; cycle <= 64; cycle += 1) {
      for (let hits = 1; hits <= cycle; hits += 1) {
        const result = steps(options({ cycle, hits, endStep: cycle }), cycle)
        expect(result).toHaveLength(hits)
        const gaps = result.map(
          (step, i) => (result[(i + 1) % hits] - step + cycle) % cycle || cycle
        )
        expect(Math.max(...gaps) - Math.min(...gaps)).toBeLessThanOrEqual(1)
      }
    }
    expect(steps(options({ cycle: 8, hits: 3 }))).toEqual([0, 3, 6, 8, 11, 14])
  })

  it("repeats and rotates regular cycles from the range start, truncating the final cycle", () => {
    expect(
      steps(
        options({
          rule: "regular",
          cycle: 5,
          every: 2,
          rotation: 1,
          startStep: 2,
          endStep: 13,
        })
      )
    ).toEqual([2, 3, 5, 7, 8, 10, 12])
  })

  it("produces reproducible seeded choices with exactly the requested unique hits", () => {
    for (const seed of [0, 1, 2, 0xffffffff]) {
      const settings = options({
        rule: "random",
        seed,
        cycle: 1024,
        hits: 701,
        endStep: 1024,
      })
      const first = steps(settings, 1024)
      expect(first).toHaveLength(701)
      expect(new Set(first).size).toBe(701)
      expect(first).toEqual(steps(settings, 1024))
      expect(first.every((step) => step >= 0 && step < 1024)).toBe(true)
    }
    expect(steps(options({ rule: "random", seed: 1 }))).not.toEqual(
      steps(options({ rule: "random", seed: 2 }))
    )
  })

  it("supports zero hits, full hits and the one-step boundary", () => {
    expect(steps(options({ hits: 0 }))).toEqual([])
    expect(steps(options({ hits: 16 }))).toHaveLength(16)
    expect(
      fillPreview(
        options({
          cycle: 1,
          hits: 1,
          endStep: 1,
          key: 127,
          velocity: 0,
          gate: 0.001,
        }),
        1
      )
    ).toEqual([{ start: 0, length: 1, key: 127, velocity: 0, pan: 0 }])
  })

  it.each([
    { cycle: 0 },
    { cycle: 17 },
    { cycle: NaN },
    { hits: 17 },
    { hits: -1 },
    { hits: 1.5 },
    { rotation: 16 },
    { rotation: -1 },
    { startStep: -1 },
    { startStep: 16 },
    { endStep: 17 },
    { endStep: 0 },
    { gate: 0 },
    { gate: Infinity },
    { velocity: NaN },
    { velocity: 1.01 },
    { key: 128 },
    { rule: "regular" as const, every: 0 },
    { rule: "random" as const, seed: -1 },
    { rule: "random" as const, seed: 0x100000000 },
  ])("refuses invalid options %j without building output", (patch) => {
    expect(fillPreview(options(patch), 16)).toBeNull()
  })
})

let backend: Backend
let stop = () => {}
beforeEach(async () => {
  ;({ backend, stop } = await startRack())
  closeAdvancedFill()
  useUiStore.getState().selectChannel(channel("Kick").id)
})
afterEach(() => {
  closeAdvancedFill()
  stop()
  vi.restoreAllMocks()
})

function open() {
  render(<AdvancedFillDialog />)
  act(() => openAdvancedFill())
  return screen.getByRole("dialog", { name: "Advanced step fill" })
}
const input = (name: string, value: string) =>
  fireEvent.change(screen.getByRole("spinbutton", { name }), {
    target: { value },
  })

describe("reviewed fill through the real Rust WASM document", () => {
  it("opens from the row menu without changing notes or history, and Cancel discards preview", async () => {
    const before = structuredClone(project())
    const oldHistory = structuredClone(history())
    render(<ChannelRackPanel />)
    fireEvent.contextMenu(screen.getByRole("button", { name: "Kick" }))
    fireEvent.click(
      await screen.findByRole("menuitem", { name: "Advanced step fill…" })
    )
    expect(
      await screen.findByRole("dialog", { name: "Advanced step fill" })
    ).toBeVisible()
    input("Hits per cycle", "7")
    expect(
      screen.getByRole("img", { name: "Step fill preview: 7 occupied steps" })
    ).toBeVisible()
    expect(project()).toEqual(before)
    expect(history()).toEqual(oldHistory)
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }))
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    )
    expect(project()).toEqual(before)
    expect(history()).toEqual(oldHistory)
  })

  it("applies the exact preview with key, velocity and gate, with atomic undo/redo and save/reopen", async () => {
    const before = structuredClone(notesOf("Kick"))
    const cursor = history().cursor
    const dialog = open()
    input("MIDI key", "72")
    input("Velocity (%)", "37")
    input("Note length (%)", "25")
    const previewSteps = [...dialog.querySelectorAll('[data-lit="true"]')].map(
      (cell) => Number(cell.getAttribute("data-fill-step")) - 1
    )
    fireEvent.click(screen.getByRole("button", { name: "Apply" }))
    await waitFor(() => expect(useAdvancedFill.getState().request).toBeNull())
    const after = structuredClone(notesOf("Kick"))
    expect(after.map((note) => note.start / 240)).toEqual(previewSteps)
    expect(
      after.every(
        (note) =>
          note.key === 72 &&
          note.length === 60 &&
          Math.abs(note.velocity - 0.37) < 1e-6
      )
    ).toBe(true)
    expect(history().cursor).toBe(cursor + 1)
    expect(history().entries.at(-1)?.label).toBe("Advanced step fill")
    await act(() => undo())
    expect(notesOf("Kick")).toEqual(before)
    await act(() => redo())
    expect(notesOf("Kick")).toEqual(after)
    const path = await backend.projectSave("/advanced-fill.windfall")
    await act(() => openProjectPath(path))
    expect(notesOf("Kick")).toEqual(after)
    const doc = SimDocument.create(project())
    expect(doc.project().patterns).toEqual(project().patterns)
    doc.dispose()
  })

  it("previews overlay skips and preserves every original note", async () => {
    const before = structuredClone(notesOf("Kick"))
    open()
    fireEvent.click(screen.getByRole("button", { name: "Add to range" }))
    const all = fillPreview(options({ replace: false }), 16)!
    const additions = all.filter(
      (note) => !before.some((old) => old.start === note.start)
    )
    expect(
      screen.getByText(
        `${additions.length} new hits; ${all.length - additions.length} occupied onsets skipped.`
      )
    ).toBeVisible()
    fireEvent.click(screen.getByRole("button", { name: "Apply" }))
    await waitFor(() => expect(useAdvancedFill.getState().request).toBeNull())
    expect(notesOf("Kick")).toHaveLength(before.length + additions.length)
    expect(
      before.every((old) =>
        notesOf("Kick").some(
          (note) => note.id === old.id && note.start === old.start
        )
      )
    ).toBe(true)
  })

  it("disables invalid input and stale documents without an edit", async () => {
    open()
    input("Hits per cycle", "")
    expect(
      screen.getByRole("spinbutton", { name: "Hits per cycle" })
    ).toHaveAttribute("aria-invalid", "true")
    expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
    input("Hits per cycle", "5")
    const request = useAdvancedFill.getState().request!
    await act(() =>
      dispatch({
        type: "toggleStep",
        pattern: request.pattern,
        channel: request.channel,
        step: 1,
      })
    )
    expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled()
    expect(fillRequestIsCurrent(request)).toBe(false)
    const before = structuredClone(useProjectStore.getState())
    expect(await applyAdvancedFill(request, options())).toBe(false)
    expect(useProjectStore.getState()).toEqual(before)
  })

  it("closes on history intent, even a no-op Undo", async () => {
    open()
    await act(() => undo())
    expect(useAdvancedFill.getState().request).toBeNull()
  })

  it("refuses project replacement with reused ids and revision", async () => {
    await act(() => newProject())
    useUiStore.getState().selectChannel(project().channels[0].id)
    act(() => openAdvancedFill())
    const old = useAdvancedFill.getState().request!
    await act(() => newProject())
    useUiStore.getState().selectChannel(project().channels[0].id)
    expect(project().channels[0].id).toBe(old.channel)
    expect(useProjectStore.getState().revision).toBe(old.revision)
    expect(await applyAdvancedFill(old, options())).toBe(false)
    expect(useAdvancedFill.getState().request).toBeNull()
    expect(history().entries).toHaveLength(0)
  })

  it("native admission refuses an unseen edit between UI review and dispatch", async () => {
    open()
    const request = useAdvancedFill.getState().request!
    const original = backend.dispatch.bind(backend)
    vi.spyOn(backend, "dispatch").mockImplementationOnce(async (command) => {
      await original({
        type: "toggleStep",
        pattern: request.pattern,
        channel: request.channel,
        step: 1,
      })
      return original(command)
    })
    const before = history().cursor
    expect(await act(() => applyAdvancedFill(request, options()))).toBe(false)
    expect(history().cursor).toBe(before + 1)
    expect(history().entries.at(-1)?.label).toBe("Toggle step")
  })

  it("admits only one submission while its backend reply is delayed", async () => {
    open()
    const request = useAdvancedFill.getState().request!
    const original = backend.dispatch.bind(backend)
    let resolve = () => {}
    const reply = new Promise<void>((done) => {
      resolve = done
    })
    const spy = vi
      .spyOn(backend, "dispatch")
      .mockImplementationOnce(async (command) => {
        const result = await original(command)
        await reply
        return result
      })
    let first!: Promise<boolean>
    act(() => {
      first = applyAdvancedFill(request, options())
    })
    expect(await applyAdvancedFill(request, options())).toBe(false)
    expect(spy).toHaveBeenCalledTimes(1)
    resolve()
    expect(await act(() => first)).toBe(true)
  })

  it("offers the palette action and previews the full maximum-length range", async () => {
    const pattern = project().patterns[0].id
    await act(() =>
      dispatch({
        type: "updatePattern",
        id: pattern,
        patch: { lengthSteps: 1024 },
      })
    )
    render(<AdvancedFillDialog />)
    await act(() => runAction("channel.advancedFill"))
    const dialog = screen.getByRole("dialog", { name: "Advanced step fill" })
    expect(dialog.querySelectorAll("[data-fill-step]")).toHaveLength(1024)
    expect(
      within(dialog).getByRole("spinbutton", { name: "Last step" })
    ).toHaveValue(1024)
    input("Cycle steps", "1024")
    input("Hits per cycle", "1024")
    fireEvent.click(screen.getByRole("button", { name: "Apply" }))
    await waitFor(() => expect(useAdvancedFill.getState().request).toBeNull())
    expect(notesOf("Kick")).toHaveLength(1024)
  })
})
