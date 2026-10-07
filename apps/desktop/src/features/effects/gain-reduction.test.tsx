import { act, render, screen } from "@testing-library/react"
import { Profiler } from "react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { RealtimeFrame } from "@/bindings"
import type { MockBackend } from "@/lib/ipc/mock"
import { gainReductionFeed } from "@/lib/store/realtime"
import { startTestApp } from "@/test/harness"

import {
  advanceReduction,
  createReductionState,
  formatReduction,
  GainReductionBar,
  GainReductionMeter,
  reductionPosition,
} from "./gain-reduction"

const QUIET: RealtimeFrame = {
  playing: true,
  tick: 0,
  meters: [],
  cpu: 0,
  xruns: 0,
  voices: 0,
  gainReductions: [],
  automated: [],
}

let stop: () => void
let backend: MockBackend
/** Hands a frame to the app as the engine would. */
let push: (frame: RealtimeFrame) => void
let pending: FrameRequestCallback[]
let now: number

/** Runs one animation frame, `ms` later. */
function frame(ms = 16) {
  now += ms
  const due = pending
  pending = []
  act(() => {
    for (const callback of due) callback(now)
  })
}

const reading = (effect: number, db: number): RealtimeFrame => ({
  ...QUIET,
  gainReductions: [{ effect, db }],
})

beforeEach(async () => {
  ;({ stop, backend } = await startTestApp())
  pending = []
  now = 1000
  push = () => {
    throw new Error("Nothing has subscribed to the realtime feed")
  }
  vi.spyOn(backend, "subscribeRealtime").mockImplementation((handler) => {
    push = handler
    return () => {}
  })
  vi.spyOn(performance, "now").mockImplementation(() => now)
  vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
    pending.push(callback)
    return pending.length
  })
  vi.stubGlobal("cancelAnimationFrame", () => {
    pending = []
  })
})
afterEach(() => {
  stop()
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

describe("the gain reduction feed", () => {
  it("reads one effect's reduction once per animation frame", () => {
    const seen: number[] = []
    const unsubscribe = gainReductionFeed(7)((db) => seen.push(db))
    frame()
    expect(seen).toEqual([0])

    push({
      ...QUIET,
      gainReductions: [
        { effect: 3, db: 9 },
        { effect: 7, db: 4.5 },
      ],
    })
    frame()
    expect(seen).toEqual([0, 4.5])
    unsubscribe()
  })

  it("keeps the deepest reading between two draws", () => {
    const seen: number[] = []
    const unsubscribe = gainReductionFeed(7)((db) => seen.push(db))
    push(reading(7, 2))
    push(reading(7, 8))
    push(reading(7, 1))
    frame()
    expect(seen).toEqual([8])
    // The next draw starts over.
    push(reading(7, 1))
    frame()
    expect(seen).toEqual([8, 1])
    unsubscribe()
  })

  it("reads 0 for an effect the engine does not list", () => {
    const seen: number[] = []
    const unsubscribe = gainReductionFeed(99)((db) => seen.push(db))
    push(reading(7, 12))
    frame()
    expect(seen).toEqual([0])
    unsubscribe()
  })
})

describe("the gain reduction meter", () => {
  it("follows the feed without a single React render", () => {
    let renders = 0
    render(
      <Profiler id="meters" onRender={() => (renders += 1)}>
        <GainReductionMeter effect={7} label="Compressor" />
        <ul>
          <li>
            <GainReductionBar effect={7} />
          </li>
        </ul>
      </Profiler>
    )
    const mounted = renders
    const fill = document.querySelector<HTMLElement>(
      "[data-slot=gain-reduction-fill]"
    )
    const bar = document.querySelector<HTMLElement>(
      "[data-slot=gain-reduction-bar] > span"
    )
    const value = document.querySelector("[data-slot=gain-reduction-value]")
    expect(fill?.style.transform).toBe("scaleY(0)")
    expect(bar?.style.transform).toBe("scaleX(0)")
    expect(value).toHaveTextContent("0.0")

    push(reading(7, 6))
    frame()
    // 6 dB of the 24 the meter shows.
    expect(fill?.style.transform).toBe("scaleY(0.2500)")
    expect(value).toHaveTextContent("−6.0")
    expect(bar?.style.transform).toBe(`scaleX(${(0.25 ** 0.6).toFixed(3)})`)

    push(reading(7, 13.2))
    frame()
    expect(fill?.style.transform).toBe("scaleY(0.5500)")
    expect(value).toHaveTextContent("−13.2")

    // It lets go: the bar falls back at once, the figure holds.
    push(reading(7, 0))
    frame(100)
    expect(fill?.style.transform).toBe("scaleY(0.3500)")
    expect(value).toHaveTextContent("−13.2")

    for (let step = 0; step < 100; step += 1) {
      push(reading(7, 0))
      frame(50)
    }
    expect(fill?.style.transform).toBe("scaleY(0.0000)")
    expect(value).toHaveTextContent("0.0")

    expect(renders).toBe(mounted)
    expect(
      screen.getByRole("group", { name: "Compressor gain reduction" })
    ).toBeVisible()
  })

  it("shows nothing for another effect's readings", () => {
    render(<GainReductionMeter effect={8} label="Limiter" />)
    push(reading(7, 10))
    frame()
    expect(
      document.querySelector<HTMLElement>("[data-slot=gain-reduction-fill]")
        ?.style.transform
    ).toBe("scaleY(0.0000)")
  })
})

describe("meter ballistics", () => {
  it("rises at once and falls at a steady rate", () => {
    const state = createReductionState()
    advanceReduction(state, 10, 0, 16)
    expect(state.level).toBe(10)
    // 48 dB a second: 6 dB in an eighth of one, and never below nothing.
    advanceReduction(state, 0, 141, 125)
    expect(state.level).toBeCloseTo(4)
    advanceReduction(state, 0, 391, 250)
    expect(state.level).toBe(0)
    advanceReduction(state, 10, 300, 50)
    advanceReduction(state, 4, 400, 100)
    expect(state.level).toBeCloseTo(5.2)
  })

  it("holds the deepest reading for a while, then lets it fall", () => {
    const state = createReductionState()
    advanceReduction(state, 9, 0, 16)
    advanceReduction(state, 2, 1000, 16)
    expect(state.peak).toBe(9)
    advanceReduction(state, 2, 1300, 100)
    expect(state.peak).toBeLessThan(9)
    expect(state.peak).toBeGreaterThanOrEqual(state.level)
    // A deeper reading takes the hold over.
    advanceReduction(state, 15, 1400, 100)
    expect(state.peak).toBe(15)
  })

  it("ignores readings that are not numbers or are below zero", () => {
    const state = createReductionState()
    advanceReduction(state, Number.NaN, 0, 16)
    advanceReduction(state, -3, 16, 16)
    expect(state).toMatchObject({ level: 0, peak: 0 })
  })

  it("places and prints a reduction", () => {
    expect(reductionPosition(0)).toBe(0)
    expect(reductionPosition(12)).toBe(0.5)
    expect(reductionPosition(40)).toBe(1)
    expect(formatReduction(0)).toBe("0.0")
    expect(formatReduction(0.04)).toBe("0.0")
    expect(formatReduction(4.25)).toBe("−4.3")
  })
})
