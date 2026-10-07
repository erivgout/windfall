import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { startTestApp } from "@/test/harness"
import { runAction } from "@/lib/actions"
import { undo, redo, useProjectStore } from "@/lib/store/project"
import { useTransportStore } from "@/lib/store/transport"
import type { MockBackend } from "@/lib/ipc/mock"
import { TapTempoDialog } from "./tap-tempo-dialog"

let backend: MockBackend
let stop: () => void
let clock = 0
beforeEach(async () => {
  ;({ backend, stop } = await startTestApp())
  clock = 0
  vi.spyOn(performance, "now").mockImplementation(() => clock)
})
afterEach(() => {
  stop()
  vi.restoreAllMocks()
})
async function open() {
  render(<TapTempoDialog />)
  await act(() => runAction("tempo.tap"))
  return screen.getByRole("button", { name: "Tap" })
}
function tap(button: HTMLElement, time: number) {
  clock = time
  fireEvent.click(button)
}

describe("reviewed tempo tapper", () => {
  it("previews, resets and cancels without editing the project", async () => {
    const button = await open()
    const before = useProjectStore.getState()
    tap(button, 1000)
    tap(button, 1500)
    tap(button, 2000)
    expect(screen.getByRole("status")).toHaveTextContent("3 taps · 120.00 BPM")
    expect(useProjectStore.getState()).toBe(before)
    fireEvent.click(screen.getByRole("button", { name: "Reset taps" }))
    expect(screen.getByRole("button", { name: "Apply tempo" })).toBeDisabled()
    expect(screen.getByRole("status")).toHaveTextContent("0 taps")
    tap(button, 5000)
    tap(button, 5600)
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }))
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull())
    expect(useProjectStore.getState()).toBe(before)
  })
  it("applies once with undo/redo and saved project persistence", async () => {
    const button = await open()
    tap(button, 1000)
    tap(button, 1500)
    tap(button, 2000)
    fireEvent.click(screen.getByRole("button", { name: "Apply tempo" }))
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull())
    expect(useProjectStore.getState().project.settings.tempoBpm).toBe(120)
    expect(useProjectStore.getState().history.entries).toEqual([
      { label: "Change tempo" },
    ])
    await act(() => undo())
    expect(useProjectStore.getState().project.settings.tempoBpm).toBe(128)
    await act(() => redo())
    const path = await backend.projectSave("tapped.windfall")
    await act(() => backend.projectNew())
    await act(() => backend.projectOpen(path))
    expect(useProjectStore.getState().project.settings.tempoBpm).toBe(120)
  })
  it("counts keyboard taps once and keeps Space from transport playback", async () => {
    const button = await open()
    button.focus()
    clock = 1000
    fireEvent.keyDown(button, { key: " ", code: "Space" })
    fireEvent.keyDown(button, { key: " ", code: "Space", repeat: true })
    fireEvent.keyUp(button, { key: " ", code: "Space" })
    clock = 1500
    fireEvent.keyDown(button, { key: "Enter", code: "Enter" })
    fireEvent.keyUp(button, { key: "Enter", code: "Enter" })
    expect(screen.getByRole("status")).toHaveTextContent("2 taps · 120.00 BPM")
    expect(useTransportStore.getState().playing).toBe(false)
    expect(useProjectStore.getState().history.entries).toEqual([])
  })
  it("starts a fresh estimate when another project is loaded", async () => {
    const button = await open()
    tap(button, 1000)
    tap(button, 1500)
    await act(() => backend.projectNew())
    expect(screen.getByRole("status")).toHaveTextContent("0 taps")
    expect(screen.getByRole("button", { name: "Apply tempo" })).toBeDisabled()
  })
  it("ignores a delayed Apply reply after a project replacement", async () => {
    const actual = backend.dispatch.bind(backend)
    let resolve: (() => void) | undefined
    vi.spyOn(backend, "dispatch").mockImplementation(
      async (command, gesture) => {
        const result = await actual(command, gesture)
        return new Promise((done) => {
          resolve = () => done(result)
        })
      }
    )
    const button = await open()
    tap(button, 1000)
    tap(button, 1500)
    fireEvent.click(screen.getByRole("button", { name: "Apply tempo" }))
    await waitFor(() => expect(resolve).toBeDefined())
    await act(() => backend.projectNew())
    const before = useProjectStore.getState()
    await act(async () => resolve!())
    expect(useProjectStore.getState()).toBe(before)
    expect(screen.getByRole("status")).toHaveTextContent("0 taps")
    expect(screen.getByRole("button", { name: "Apply tempo" })).toBeDisabled()
  })
})
