// SPDX-License-Identifier: MIT
import * as React from "react"
import { fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import {
  MAX_DISPLAY_GAIN,
  Waveform,
  formatDisplayGain,
  layoutRegionHandle,
  waveformDisplayGain,
} from "./waveform"

const QUIET = new Float32Array([-0.125, 0.25, -0.25, 0.125, 0, 0, -0.05, 0.05])
const FULL = new Float32Array([-0.5, 0.5, -1, 0.95, -0.25, 0.25, 0, 0])

afterEach(() => {
  vi.restoreAllMocks()
})

describe("region handle layout", () => {
  it("centers the grab area on the position away from the edges", () => {
    expect(layoutRegionHandle("start", 0.5, 300)).toEqual({
      grab: 144,
      line: 149.5,
      flag: 149.5,
    })
    // The end handle's flag points back into the region.
    expect(layoutRegionHandle("end", 0.5, 300)).toEqual({
      grab: 144,
      line: 149.5,
      flag: 142.5,
    })
  })

  it("keeps a handle at either end whole and inside the waveform", () => {
    expect(layoutRegionHandle("start", 0, 300)).toEqual({
      grab: 0,
      line: 0,
      flag: 0,
    })
    expect(layoutRegionHandle("end", 1, 300)).toEqual({
      grab: 288,
      line: 299,
      flag: 292,
    })
    // The far ends: a start handle pushed to the right, an end to the left.
    expect(layoutRegionHandle("start", 1, 300)).toEqual({
      grab: 288,
      line: 299,
      flag: 292,
    })
    expect(layoutRegionHandle("end", 0, 300)).toEqual({
      grab: 0,
      line: 0,
      flag: 0,
    })
  })

  it("always has the line inside the grab area", () => {
    for (const edge of ["start", "end"] as const) {
      for (let step = 0; step <= 300; step += 1) {
        const { grab, line, flag } = layoutRegionHandle(edge, step / 300, 300)
        expect(grab).toBeGreaterThanOrEqual(0)
        expect(grab + 12).toBeLessThanOrEqual(300)
        expect(line).toBeGreaterThanOrEqual(grab)
        expect(line + 1).toBeLessThanOrEqual(grab + 12)
        expect(flag).toBeGreaterThanOrEqual(0)
        expect(flag + 8).toBeLessThanOrEqual(300)
      }
    }
  })
})

describe("Waveform region handles", () => {
  // The waveform lies from x = 100 to x = 400 on the page.
  const LEFT = 100
  const WIDTH = 300

  beforeEach(() => {
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null)
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(WIDTH)
  })

  function Region({ log }: { log: string[] }) {
    const [region, setRegion] = React.useState({ start: 0, end: 1 })
    return (
      <Waveform
        peaks={FULL}
        start={region.start}
        end={region.end}
        onStartChange={(start) => setRegion((was) => ({ ...was, start }))}
        onEndChange={(end) => setRegion((was) => ({ ...was, end }))}
        onGestureStart={() => log.push("start")}
        onGestureEnd={() => log.push("end")}
      />
    )
  }

  /** The handle's grab area in page coordinates, as laid out in pixels. */
  function grabArea(handle: HTMLElement) {
    expect(handle.style.left).toMatch(/px$/)
    const left = LEFT + Number.parseFloat(handle.style.left)
    return { left, right: left + Number.parseFloat(handle.style.width) }
  }

  const valueOf = (slider: HTMLElement) =>
    Number(slider.getAttribute("aria-valuenow"))

  it("can be grabbed across the full width of a handle resting at 0 and at 1", () => {
    render(<Region log={[]} />)
    const start = grabArea(screen.getByRole("slider", { name: "Region start" }))
    const end = grabArea(screen.getByRole("slider", { name: "Region end" }))
    // Twelve pixels each, none of them outside the waveform, where the
    // root would clip them.
    expect(start).toEqual({ left: LEFT, right: LEFT + 12 })
    expect(end).toEqual({ left: LEFT + WIDTH - 12, right: LEFT + WIDTH })
    // The handle's own line is inside what can be pressed.
    expect(LEFT + 0.5).toBeGreaterThan(start.left)
    expect(LEFT + WIDTH - 0.5).toBeLessThan(end.right)
    expect(LEFT + WIDTH - 0.5).toBeGreaterThan(end.left)
  })

  it("drags the end handle from a press on its line at the right edge", () => {
    const log: string[] = []
    render(<Region log={log} />)
    const end = screen.getByRole("slider", { name: "Region end" })
    const press = LEFT + WIDTH - 0.5
    fireEvent.pointerDown(end, { pointerId: 1, button: 0, clientX: press })
    fireEvent.pointerMove(end, { pointerId: 1, clientX: press - 60 })
    expect(valueOf(end)).toBeCloseTo(0.8, 10)
    // It follows the pointer: the grab area moved with it.
    expect(grabArea(end).left).toBeCloseTo(LEFT + 240 - 6, 6)
    fireEvent.pointerUp(end, { pointerId: 1 })
    expect(log).toEqual(["start", "end"])
  })

  it("drags the start handle from a press on its line at the left edge", () => {
    render(<Region log={[]} />)
    const start = screen.getByRole("slider", { name: "Region start" })
    fireEvent.pointerDown(start, { pointerId: 1, button: 0, clientX: LEFT })
    fireEvent.pointerMove(start, { pointerId: 1, clientX: LEFT + 30 })
    expect(valueOf(start)).toBeCloseTo(0.1, 10)
    fireEvent.pointerUp(start, { pointerId: 1 })
  })
})

describe("waveform display gain", () => {
  it("fills the height with the loudest peak", () => {
    expect(waveformDisplayGain(QUIET)).toBe(4)
    expect(waveformDisplayGain(new Float32Array([-2, 1]))).toBe(0.5)
  })

  it("leaves silence flat and does not blow up a noise floor", () => {
    expect(waveformDisplayGain(new Float32Array(8))).toBe(1)
    expect(waveformDisplayGain(new Float32Array([-0.0001, 0.0001]))).toBe(
      MAX_DISPLAY_GAIN
    )
    expect(waveformDisplayGain([])).toBe(1)
  })

  it("prints as a magnification", () => {
    expect(formatDisplayGain(4)).toBe("×4")
    expect(formatDisplayGain(2.512)).toBe("×2.5")
    expect(formatDisplayGain(12.4)).toBe("×12")
    expect(formatDisplayGain(100)).toBe("×100")
    expect(formatDisplayGain(0.5)).toBe("×0.5")
  })
})

describe("Waveform normalizing", () => {
  let columns: number[][]

  beforeEach(() => {
    columns = []
    const context = {
      fillStyle: "",
      clearRect() {},
      fillRect: (...rect: number[]) => columns.push(rect),
    } as unknown as CanvasRenderingContext2D
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(context)
    // Four columns, one per bucket, 100 pixels tall.
    vi.spyOn(
      HTMLCanvasElement.prototype,
      "getBoundingClientRect"
    ).mockReturnValue({ width: 4, height: 100 } as DOMRect)
  })

  /** Heights of the waveform's columns, without the center line. */
  const heights = () =>
    columns.filter((rect) => rect[2] === 1).map((rect) => rect[3])

  const gainLabel = (container: HTMLElement) =>
    container.querySelector('[data-slot="waveform-gain"]')

  it("draws a quiet sample at full height by default and says by how much", () => {
    const { container } = render(<Waveform peaks={QUIET} />)
    // The loudest bucket reaches 0.25 on one side and 0.125 on the other.
    expect(Math.max(...heights())).toBe(75)
    expect(gainLabel(container)).toHaveTextContent("×4")
  })

  it("draws at full scale when asked not to normalize", () => {
    const { container } = render(<Waveform peaks={QUIET} normalize={false} />)
    expect(Math.max(...heights())).toBe(20)
    expect(gainLabel(container)).toBeNull()
  })

  it("puts no label on a sample that is close to full scale", () => {
    const { container } = render(<Waveform peaks={FULL} />)
    expect(Math.max(...heights())).toBe(98)
    expect(gainLabel(container)).toBeNull()
  })

  it("keeps a silent sample flat", () => {
    const { container } = render(<Waveform peaks={new Float32Array(8)} />)
    expect(Math.max(...heights())).toBe(1)
    expect(gainLabel(container)).toBeNull()
  })
})
