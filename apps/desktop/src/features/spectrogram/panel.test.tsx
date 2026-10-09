import { useEffect } from "react"
import { act, cleanup, render, screen } from "@testing-library/react"
import { afterEach, expect, it, vi } from "vitest"

import { registry } from "@/lib/actions"

import { closeSpectrogram, registerSpectrogramActions } from "./actions"
import { SpectrogramPanel } from "./panel"

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
  closeSpectrogram()
  vi.clearAllMocks()
})

it("opens from its action and drops the subscription on close and unregister", () => {
  const unregister = registerSpectrogramActions()
  try {
    render(<SpectrogramPanel />)
    expect(feed.start).not.toHaveBeenCalled()
    const action = registry.get("view.spectrogram")
    expect(action?.title).toBe("Show spectrogram")
    act(() => {
      void action?.run()
    })
    expect(screen.getByRole("dialog", { name: "Spectrogram" })).toBeVisible()
    expect(feed.start).toHaveBeenCalledTimes(1)
    act(() => closeSpectrogram())
    expect(feed.stop).toHaveBeenCalledTimes(1)
    act(() => {
      void action?.run()
    })
    expect(feed.start).toHaveBeenCalledTimes(2)
    act(() => unregister())
    expect(feed.stop).toHaveBeenCalledTimes(2)
    expect(registry.get("view.spectrogram")).toBeUndefined()
  } finally {
    unregister()
  }
})
