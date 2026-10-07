import { act, render, waitFor } from "@testing-library/react"
import { toast } from "sonner"
import { afterEach, describe, expect, it, vi } from "vitest"

import { useEffectsUi } from "@/features/mixer/effects-ui"

import { Overlays } from "./app-shell"
import { toastInset } from "./toast-place"

afterEach(() => {
  toast.dismiss()
  vi.restoreAllMocks()
  document.body.replaceChildren()
})

/** The toaster, once a toast has come up. */
async function toaster(): Promise<HTMLElement> {
  act(() => {
    toast.success("Exported", { description: "beat.wav" })
  })
  return waitFor(() => {
    const found = document.querySelector<HTMLElement>("[data-sonner-toaster]")
    if (!found) throw new Error("no toast came up")
    return found
  })
}

/** Something in the window that toasts keep clear of, at this place. */
function corner(box: Partial<DOMRect>): HTMLElement {
  const element = document.createElement("div")
  element.setAttribute("data-toast-clear", "")
  element.getBoundingClientRect = () =>
    ({ width: 1, height: 1, ...box }) as DOMRect
  document.body.append(element)
  return element
}

describe("toasts", () => {
  it("come up at the bottom right just above the status bar, clear of the top bars", async () => {
    render(<Overlays />)
    const list = await toaster()
    // Not at the top, where they lay over search, undo and redo.
    expect(list).toHaveAttribute("data-y-position", "bottom")
    expect(list).toHaveAttribute("data-x-position", "right")
    // Above the status bar, which is 24 px tall.
    expect(list.style.getPropertyValue("--offset-bottom")).toBe("30px")
    expect(list.style.getPropertyValue("--offset-right")).toBe("6px")
    expect(list.style.getPropertyValue("--width")).toBe("340px")
    // A toast that is in the way can be closed at once.
    expect(
      list.querySelector("[data-sonner-toast] [data-close-button]")
    ).not.toBeNull()
  })

  it("wrap a long message, also one with no spaces, inside their width", async () => {
    render(<Overlays />)
    act(() => {
      toast.error("Project", {
        description:
          "Missing sample: C:\\Users\\someone\\AppData\\Local\\Temp\\audio\\tone440_10s_quiet.wav",
      })
    })
    const text = await waitFor(() => {
      const found = document.querySelector<HTMLElement>("[data-description]")
      if (!found) throw new Error("no toast came up")
      return found
    })
    expect(text.className).toContain("wrap-anywhere")
    expect(text.className).toContain("min-w-0")
  })

  it("move over to the left of the mixer's effects when those are in the corner", async () => {
    vi.stubGlobal("innerWidth", 1440)
    vi.stubGlobal("innerHeight", 900)
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
      callback(0)
      return 1
    })
    // The docked effects: the bottom right 460 by 270 pixels of the window.
    corner({
      left: 980,
      right: 1440,
      top: 606,
      bottom: 876,
      width: 460,
      height: 270,
    })
    expect(toastInset()).toBe(460)

    render(<Overlays />)
    act(() => useEffectsUi.getState().setInspectorOpen(true))
    const list = await toaster()
    await waitFor(() =>
      expect(list.style.getPropertyValue("--offset-right")).toBe("466px")
    )
    vi.unstubAllGlobals()
    act(() => useEffectsUi.getState().setInspectorOpen(false))
  })

  it("keep clear only of what is in their corner", () => {
    vi.stubGlobal("innerWidth", 1440)
    vi.stubGlobal("innerHeight", 900)
    // Enlarged into the editor area, the effects end above the mixer.
    corner({
      left: 274,
      right: 1440,
      top: 108,
      bottom: 573,
      width: 1166,
      height: 465,
    })
    // Something at the bottom, but at the left.
    corner({
      left: 0,
      right: 274,
      top: 700,
      bottom: 876,
      width: 274,
      height: 176,
    })
    // Something that is not showing.
    corner({
      left: 900,
      right: 1440,
      top: 606,
      bottom: 876,
      width: 0,
      height: 0,
    })
    expect(toastInset()).toBe(0)
    // The strip that brings the effects back, 22 pixels along the right.
    corner({
      left: 1418,
      right: 1440,
      top: 602,
      bottom: 876,
      width: 22,
      height: 274,
    })
    expect(toastInset()).toBe(22)
    vi.unstubAllGlobals()
  })
})
