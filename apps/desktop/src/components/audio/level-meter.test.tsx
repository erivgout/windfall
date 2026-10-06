// SPDX-License-Identifier: MIT
import * as React from "react"
import { act, fireEvent, render, screen } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"

import {
  LevelMeter,
  METER_FLOOR_DB,
  advanceMeter,
  createMeterChannel,
  type LevelMeterHandle,
} from "./level-meter"

const BALLISTICS = { releaseDbPerSecond: 20, peakHoldSeconds: 1 }

describe("meter ballistics", () => {
  it("rises at once", () => {
    const channel = createMeterChannel()
    advanceMeter(channel, -6, 1 / 60, BALLISTICS)
    expect(channel.level).toBe(-6)
    expect(channel.peak).toBe(-6)
  })

  it("falls at the release rate", () => {
    const channel = createMeterChannel()
    advanceMeter(channel, 0, 0, BALLISTICS)
    advanceMeter(channel, -Infinity, 0.5, BALLISTICS)
    expect(channel.level).toBeCloseTo(-10, 10)
    advanceMeter(channel, -Infinity, 0.25, BALLISTICS)
    expect(channel.level).toBeCloseTo(-15, 10)
  })

  it("holds at a louder input while falling", () => {
    const channel = createMeterChannel()
    advanceMeter(channel, 0, 0, BALLISTICS)
    advanceMeter(channel, -3, 1, BALLISTICS)
    expect(channel.level).toBe(-3)
  })

  it("rests on the floor", () => {
    const channel = createMeterChannel()
    advanceMeter(channel, -20, 0, BALLISTICS)
    advanceMeter(channel, -Infinity, 60, BALLISTICS)
    expect(channel.level).toBe(METER_FLOOR_DB)
    expect(channel.peak).toBe(METER_FLOOR_DB)
    advanceMeter(channel, Number.NaN, 1, BALLISTICS)
    expect(channel.level).toBe(METER_FLOOR_DB)
  })

  it("holds the peak, then lets it fall", () => {
    const channel = createMeterChannel()
    advanceMeter(channel, 0, 0, BALLISTICS)
    advanceMeter(channel, -Infinity, 0.9, BALLISTICS)
    expect(channel.peak).toBe(0)
    expect(channel.level).toBeCloseTo(-18, 10)
    advanceMeter(channel, -Infinity, 0.6, BALLISTICS)
    // Half a second past the hold at 20 dB per second.
    expect(channel.peak).toBeCloseTo(-10, 10)
    advanceMeter(channel, -4, 0.1, BALLISTICS)
    expect(channel.peak).toBe(-4)
    expect(channel.peakAge).toBe(0)
  })

  it("never lets the peak sink below the level", () => {
    const channel = createMeterChannel()
    advanceMeter(channel, 0, 0, { releaseDbPerSecond: 20, peakHoldSeconds: 0 })
    for (let frame = 0; frame < 120; frame += 1) {
      advanceMeter(channel, -Infinity, 1 / 60, {
        releaseDbPerSecond: 20,
        peakHoldSeconds: 0,
      })
      expect(channel.peak).toBeGreaterThanOrEqual(channel.level)
    }
  })
})

describe("LevelMeter", () => {
  beforeEach(() => {
    // jsdom has no canvas; the meter must still track clipping without one.
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null)
  })

  function renderMeter(
    props: Partial<React.ComponentProps<typeof LevelMeter>> = {}
  ) {
    const ref = React.createRef<LevelMeterHandle>()
    const view = render(<LevelMeter ref={ref} data-testid="meter" {...props} />)
    return { ref, meter: screen.getByTestId("meter"), ...view }
  }

  it("latches the clip light until it is clicked", () => {
    const changes: boolean[] = []
    const { ref, meter } = renderMeter({
      onClipChange: (clipped) => changes.push(clipped),
    })
    // The button is hidden from assistive technology until the light is on.
    const button = meter.querySelector("button") as HTMLButtonElement
    expect(button).toHaveAttribute("aria-hidden", "true")
    expect(button).toHaveAttribute("tabindex", "-1")
    act(() => ref.current?.set(0.5, 0.9))
    expect(meter).not.toHaveAttribute("data-clipped")
    act(() => ref.current?.set(0.5, 1.2))
    expect(meter).toHaveAttribute("data-clipped")
    expect(button).toHaveAttribute("tabindex", "0")
    expect(button).toHaveAccessibleName("Clear clip indicator")
    act(() => ref.current?.set(0.1, 0.1))
    expect(meter).toHaveAttribute("data-clipped")
    fireEvent.click(button)
    expect(meter).not.toHaveAttribute("data-clipped")
    expect(changes).toEqual([true, false])
  })

  it("does not clip at exactly full scale", () => {
    const { ref, meter } = renderMeter()
    act(() => ref.current?.set(1, 1))
    expect(meter).not.toHaveAttribute("data-clipped")
    act(() => ref.current?.set(-1.01))
    expect(meter).toHaveAttribute("data-clipped")
  })

  it("honors its own clip threshold and clears through the ref", () => {
    const { ref, meter } = renderMeter({ clipGain: 2 })
    act(() => ref.current?.set(1.5, 1.5))
    expect(meter).not.toHaveAttribute("data-clipped")
    act(() => ref.current?.set(2.5, 0))
    expect(meter).toHaveAttribute("data-clipped")
    act(() => ref.current?.clearClip())
    expect(meter).not.toHaveAttribute("data-clipped")
    act(() => ref.current?.set(2.5, 0))
    act(() => ref.current?.reset())
    expect(meter).not.toHaveAttribute("data-clipped")
  })

  it("takes values through subscribe and unsubscribes on unmount", () => {
    let listener: ((left: number, right?: number) => void) | null = null
    const unsubscribe = vi.fn()
    const subscribe = vi.fn((next: (left: number, right?: number) => void) => {
      listener = next
      return unsubscribe
    })
    const { meter, unmount } = renderMeter({ subscribe })
    expect(subscribe).toHaveBeenCalledTimes(1)
    act(() => listener?.(1.5, 0))
    expect(meter).toHaveAttribute("data-clipped")
    unmount()
    expect(unsubscribe).toHaveBeenCalledTimes(1)
  })

  it("ignores values that are not numbers", () => {
    const { ref, meter } = renderMeter()
    act(() => ref.current?.set(Number.NaN, Number.NaN))
    expect(meter).not.toHaveAttribute("data-clipped")
  })
})
