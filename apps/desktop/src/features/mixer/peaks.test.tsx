import { render, screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { MasterMeterSlot } from "@/features/transport/master-meter-slot"
import type { Backend } from "@/lib/ipc"
import { dispatch } from "@/lib/store/project"
import { realtimeFrame } from "@/lib/store/realtime"
import { startTestApp } from "@/test/harness"

import MixerPanel from "."
import { formatPeak } from "./level-section"
import {
  peakOf,
  resetAllPeaks,
  resetPeak,
  subscribePeak,
  watchPeaks,
} from "./peaks"
import { flush, strip, stubCanvas, trackNamed } from "./test-utils"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let backend: Backend
let stop: () => void

beforeEach(async () => {
  stubCanvas()
  ;({ backend, stop } = await startTestApp())
})
afterEach(() => {
  stop()
  vi.restoreAllMocks()
})

const peak = (name: string) =>
  within(strip(name)).getByRole("button", { name: /^Peak/ })
const clipLight = (name: string) =>
  within(strip(name)).getByRole("button", { name: /^Clip light/, hidden: true })

// The mock plays in real time, so these wait on its clock. The first hit
// lands at once; the margin is for a busy machine.
const SOON = { timeout: 3000 }

/** Stops playback and waits for the meters to fall back to rest. */
async function stopAndSettle() {
  await backend.transportStop()
  await waitFor(() => expect(Math.max(0, ...realtimeFrame().meters)).toBe(0), {
    timeout: 4000,
  })
  await flush()
}

describe("formatPeak", () => {
  it("prints dB without the unit", () => {
    expect(formatPeak(1)).toBe("0.0")
    expect(formatPeak(0.5)).toBe("−6.0")
    expect(formatPeak(1.5)).toBe("+3.5")
    expect(formatPeak(0)).toBe("−∞")
  })
})

describe("peak readout", () => {
  it("starts at silence", () => {
    render(<MixerPanel />)
    expect(peak("Kick")).toHaveTextContent("Peak −∞")
    expect(peak("Kick")).not.toHaveAttribute("data-clipped")
    expect(clipLight("Kick")).not.toHaveAttribute("data-clipped")
  })

  it("holds the loudest level while the pattern plays, until it is clicked", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    await backend.transportPlay()
    await waitFor(
      () => expect(peakOf(trackNamed("Kick").id)).toBeGreaterThan(0),
      SOON
    )
    await stopAndSettle()

    const held = peakOf(trackNamed("Kick").id)
    expect(peak("Kick")).toHaveTextContent(`Peak ${formatPeak(held)}`)

    await user.click(peak("Kick"))
    expect(peak("Kick")).toHaveTextContent("Peak −∞")
    expect(peakOf(trackNamed("Kick").id)).toBe(0)
  })

  it("latches the clip light above 0 dB and clears it on a click", async () => {
    const user = userEvent.setup()
    render(<MixerPanel />)
    // Turned up to +6 dB the kick goes over.
    await dispatch({
      type: "updateMixerTrack",
      id: trackNamed("Kick").id,
      patch: { volume: 2 },
    })
    await backend.transportPlay()
    await waitFor(
      () => expect(peak("Kick")).toHaveAttribute("data-clipped"),
      SOON
    )
    await stopAndSettle()

    expect(clipLight("Kick")).toHaveAttribute("data-clipped")
    expect(clipLight("Kick")).toHaveAttribute("tabindex", "0")
    expect(peak("Hat")).not.toHaveAttribute("data-clipped")

    await user.click(clipLight("Kick"))
    expect(clipLight("Kick")).not.toHaveAttribute("data-clipped")
    expect(peak("Kick")).not.toHaveAttribute("data-clipped")
    expect(peak("Kick")).toHaveTextContent("Peak −∞")
  })

  it("resets every readout from the command palette action", async () => {
    render(<MixerPanel />)
    await backend.transportPlay()
    await waitFor(
      () => expect(peakOf(trackNamed("Hat").id)).toBeGreaterThan(0),
      SOON
    )
    await stopAndSettle()

    resetAllPeaks()
    expect(peak("Hat")).toHaveTextContent("Peak −∞")
    expect(peak("Master")).toHaveTextContent("Peak −∞")
  })
})

describe("held peaks", () => {
  it("tells a listener the held value at once and after a reset", () => {
    const stopWatching = watchPeaks()
    const seen: number[] = []
    const off = subscribePeak(7, (value) => seen.push(value))
    expect(seen).toEqual([0])
    resetPeak(7)
    // Nothing was held, so there is nothing to tell.
    expect(seen).toEqual([0])
    off()
    stopWatching()
  })
})

describe("transport master meter", () => {
  it("is a horizontal stereo meter", () => {
    render(<MasterMeterSlot />)
    const slot = screen.getByRole("group", { name: "Master level" })
    expect(slot).toHaveAttribute("data-slot", "master-meter")
    expect(slot.querySelector("[data-slot=level-meter]")).toHaveAttribute(
      "data-orientation",
      "horizontal"
    )
  })
})
