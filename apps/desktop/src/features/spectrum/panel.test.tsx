import { useEffect } from "react"
import { act, cleanup, render, screen } from "@testing-library/react"
import { afterEach, expect, it, vi } from "vitest"

import { registry } from "@/lib/actions"

import { closeSpectrum, registerSpectrumActions } from "./actions"
import { SpectrumPanel } from "./panel"

const feed = vi.hoisted(() => ({ start: vi.fn(), stop: vi.fn() }))
vi.mock("@/lib/store/realtime", () => ({
  useRealtime: () => {
    useEffect(() => {
      feed.start()
      return feed.stop
    }, [])
  },
}))

afterEach(() => {
  cleanup()
  closeSpectrum()
  vi.clearAllMocks()
})

it("opens from its registered action and drops the subscription on close", () => {
  const unregister = registerSpectrumActions()
  try {
    render(<SpectrumPanel />)
    expect(feed.start).not.toHaveBeenCalled()
    const action = registry.get("view.spectrum")
    expect(action?.title).toBe("Show spectrum")
    act(() => {
      void action?.run()
    })
    expect(screen.getByRole("dialog", { name: "Spectrum" })).toBeVisible()
    expect(feed.start).toHaveBeenCalledTimes(1)
    act(() => closeSpectrum())
    expect(feed.stop).toHaveBeenCalledTimes(1)
    act(() => {
      void action?.run()
    })
    expect(feed.start).toHaveBeenCalledTimes(2)
    act(() => unregister())
    expect(feed.stop).toHaveBeenCalledTimes(2)
    expect(registry.get("view.spectrum")).toBeUndefined()
  } finally {
    unregister()
  }
})
