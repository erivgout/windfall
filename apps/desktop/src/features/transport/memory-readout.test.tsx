import { StrictMode } from "react"
import { act, cleanup, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, expect, it, vi } from "vitest"

import type { Backend } from "@/lib/ipc"
import { settle, startTestApp } from "@/test/harness"
import { MemoryReadout } from "./memory-readout"
import { PerformanceReadout } from "./performance-readout"

let backend: Backend
let stop: () => void

beforeEach(async () => {
  ;({ backend, stop } = await startTestApp())
  vi.useFakeTimers()
})

afterEach(() => {
  cleanup()
  stop()
  vi.useRealTimers()
  vi.restoreAllMocks()
})

async function flush() {
  await act(settle)
}

it("shows honest browser unavailability beside the existing CPU readout", async () => {
  render(<PerformanceReadout />)
  await flush()
  expect(screen.getByLabelText("Host RAM: unavailable")).toHaveTextContent(
    "RAM—"
  )
  expect(screen.getByText("CPU")).toBeVisible()
  expect(screen.getByText("0%")).toBeVisible()
  expect(screen.getByLabelText("Host RAM: unavailable")).toHaveAttribute(
    "title",
    "Desktop host resident memory; excludes webviews and plugin helpers"
  )
})

it("updates native resident bytes in binary units once each completed poll", async () => {
  const query = vi
    .spyOn(backend, "processMemory")
    .mockResolvedValueOnce(256 * 1024 * 1024)
    .mockResolvedValueOnce(1.5 * 1024 * 1024 * 1024)
    .mockResolvedValue(0)
  render(<MemoryReadout />)
  await flush()
  expect(screen.getByLabelText("Host RAM: 256 MiB")).toBeVisible()
  await act(() => vi.advanceTimersByTimeAsync(999))
  expect(query).toHaveBeenCalledTimes(1)
  await act(() => vi.advanceTimersByTimeAsync(1))
  expect(screen.getByLabelText("Host RAM: 1.5 GiB")).toBeVisible()
  await act(() => vi.advanceTimersByTimeAsync(1000))
  expect(screen.getByLabelText("Host RAM: 0 MiB")).toBeVisible()
})

it("does not queue another request while the current native query is pending", async () => {
  let complete!: (bytes: number) => void
  const pending = new Promise<number>((resolve) => {
    complete = resolve
  })
  const query = vi.spyOn(backend, "processMemory").mockReturnValue(pending)
  render(<MemoryReadout />)
  await act(() => vi.advanceTimersByTimeAsync(5000))
  expect(query).toHaveBeenCalledTimes(1)
  await act(async () => complete(64 * 1024 * 1024))
  expect(screen.getByLabelText("Host RAM: 64 MiB")).toBeVisible()
  await act(() => vi.advanceTimersByTimeAsync(999))
  expect(query).toHaveBeenCalledTimes(1)
  await act(() => vi.advanceTimersByTimeAsync(1))
  expect(query).toHaveBeenCalledTimes(2)
})

it("clears stale values after query failure and recovers on the next poll", async () => {
  vi.spyOn(backend, "processMemory")
    .mockResolvedValueOnce(32 * 1024 * 1024)
    .mockRejectedValueOnce(new Error("OS query failed"))
    .mockResolvedValue(48 * 1024 * 1024)
  render(<MemoryReadout />)
  await flush()
  expect(screen.getByLabelText("Host RAM: 32 MiB")).toBeVisible()
  await act(() => vi.advanceTimersByTimeAsync(1000))
  expect(screen.getByLabelText("Host RAM: unavailable")).toBeVisible()
  await act(() => vi.advanceTimersByTimeAsync(1000))
  expect(screen.getByLabelText("Host RAM: 48 MiB")).toBeVisible()
})

it.each([NaN, Infinity, -1, 0.5, Number.MAX_SAFE_INTEGER + 1])(
  "refuses invalid byte counts (%s)",
  async (value) => {
    vi.spyOn(backend, "processMemory").mockResolvedValue(value)
    render(<MemoryReadout />)
    await flush()
    expect(screen.getByLabelText("Host RAM: unavailable")).toBeVisible()
  }
)

it("stops polling on unmount even when an old request completes later", async () => {
  let complete!: (bytes: number) => void
  const query = vi.spyOn(backend, "processMemory").mockReturnValue(
    new Promise((resolve) => {
      complete = resolve
    })
  )
  const view = render(<MemoryReadout />)
  view.unmount()
  await act(async () => complete(64 * 1024 * 1024))
  await act(() => vi.advanceTimersByTimeAsync(5000))
  expect(query).toHaveBeenCalledTimes(1)
})

it("ignores the retired Strict Mode effect's late value and timer", async () => {
  let complete!: (bytes: number) => void
  const query = vi
    .spyOn(backend, "processMemory")
    .mockReturnValueOnce(
      new Promise((resolve) => {
        complete = resolve
      })
    )
    .mockResolvedValue(96 * 1024 * 1024)
  render(
    <StrictMode>
      <MemoryReadout />
    </StrictMode>
  )
  await flush()
  expect(query).toHaveBeenCalledTimes(2)
  expect(screen.getByLabelText("Host RAM: 96 MiB")).toBeVisible()
  await act(async () => complete(8 * 1024 * 1024))
  expect(screen.getByLabelText("Host RAM: 96 MiB")).toBeVisible()
  await act(() => vi.advanceTimersByTimeAsync(1000))
  expect(query).toHaveBeenCalledTimes(3)
})
