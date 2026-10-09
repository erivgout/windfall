import { useEffect } from "react"
import { act, cleanup, render, screen } from "@testing-library/react"
import { afterEach, expect, it, vi } from "vitest"

import { registry } from "@/lib/actions"

import { closePhaseMeter, registerPhaseMeterActions } from "./actions"
import { PhaseMeterPanel } from "./panel"

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
  closePhaseMeter()
  vi.clearAllMocks()
})

it("opens from its registered action and drops the subscription on close", () => {
  const unregister = registerPhaseMeterActions()
  try {
    render(<PhaseMeterPanel />)
    expect(feed.start).not.toHaveBeenCalled()
    const action = registry.get("view.phase-meter")
    expect(action?.title).toBe("Show phase meter")
    expect(action?.section).toBe("View")
    act(() => {
      void action?.run()
    })
    expect(screen.getByRole("dialog", { name: "Phase meter" })).toBeVisible()
    expect(feed.start).toHaveBeenCalledTimes(1)
    act(() => closePhaseMeter())
    expect(feed.stop).toHaveBeenCalledTimes(1)
    act(() => {
      void action?.run()
    })
    expect(feed.start).toHaveBeenCalledTimes(2)
    act(() => unregister())
    expect(feed.stop).toHaveBeenCalledTimes(2)
    expect(registry.get("view.phase-meter")).toBeUndefined()
  } finally {
    unregister()
  }
})
