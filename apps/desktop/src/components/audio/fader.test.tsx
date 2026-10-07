// SPDX-License-Identifier: MIT
import * as React from "react"
import { fireEvent, render, screen } from "@testing-library/react"
import { afterEach, describe, expect, it, vi } from "vitest"

import {
  FADER_DB_TICKS,
  FADER_LABEL_GAP,
  FADER_MAX_GAIN,
  Fader,
  faderPositionToGain,
  faderTaper,
  fitFaderLabels,
  gainToFaderPosition,
} from "./fader"
import { dbToGain, gainToDb } from "./units"

describe("fader taper", () => {
  it("hits its anchor points", () => {
    expect(gainToFaderPosition(0)).toBe(0)
    expect(gainToFaderPosition(1)).toBe(0.78)
    expect(gainToFaderPosition(FADER_MAX_GAIN)).toBe(1)
    expect(gainToFaderPosition(dbToGain(-6))).toBeCloseTo(0.62, 10)
    expect(gainToFaderPosition(dbToGain(-12))).toBeCloseTo(0.48, 10)
    expect(gainToFaderPosition(dbToGain(-24))).toBeCloseTo(0.27, 10)
    expect(gainToFaderPosition(dbToGain(-48))).toBeCloseTo(0.07, 10)
    expect(faderPositionToGain(0)).toBe(0)
    expect(faderPositionToGain(0.78)).toBe(1)
    expect(faderPositionToGain(1)).toBe(2)
  })

  it("puts 0 dB between 75% and 80% of the travel", () => {
    const unity = gainToFaderPosition(1)
    expect(unity).toBeGreaterThanOrEqual(0.75)
    expect(unity).toBeLessThanOrEqual(0.8)
  })

  it("round-trips across the whole range", () => {
    for (let step = 0; step <= 1000; step += 1) {
      const position = step / 1000
      const gain = faderPositionToGain(position)
      expect(gain).toBeGreaterThanOrEqual(0)
      expect(gain).toBeLessThanOrEqual(FADER_MAX_GAIN)
      expect(gainToFaderPosition(gain)).toBeCloseTo(position, 9)
    }
    for (const db of [-90, -60, -48, -30, -18, -9, -3, 0, 3, 6]) {
      const gain = dbToGain(db)
      expect(
        gainToDb(faderPositionToGain(gainToFaderPosition(gain)))
      ).toBeCloseTo(db, 6)
    }
  })

  it("rises without a break", () => {
    let previous = -1
    for (let step = 0; step <= 2000; step += 1) {
      const position = gainToFaderPosition((step / 2000) * FADER_MAX_GAIN)
      expect(position).toBeGreaterThan(previous)
      previous = position
    }
  })

  it("clamps what lies outside the range", () => {
    expect(gainToFaderPosition(-1)).toBe(0)
    expect(gainToFaderPosition(5)).toBe(1)
    expect(gainToFaderPosition(Number.NaN)).toBe(0)
    expect(faderPositionToGain(-0.5)).toBe(0)
    expect(faderPositionToGain(1.5)).toBe(2)
  })

  it("works as a control scale", () => {
    expect(faderTaper.toNormalized(1, 0, 2)).toBe(0.78)
    expect(faderTaper.fromNormalized(0.78, 0, 2)).toBe(1)
  })

  it("marks +6, 0, -6, -12, -24, -48 and silence", () => {
    expect(FADER_DB_TICKS.map((tick) => tick.label)).toEqual([
      "+6",
      "0",
      "−6",
      "−12",
      "−24",
      "−48",
      "−∞",
    ])
  })
})

describe("fader labels", () => {
  const placed = FADER_DB_TICKS.map((tick) => ({
    ...tick,
    position: gainToFaderPosition(tick.value),
  }))
  const labels = (length: number, gap = FADER_LABEL_GAP.vertical) =>
    fitFaderLabels(placed, length, gap)
      .map((mark) => mark.label)
      .filter((label) => label !== undefined)

  it("labels every mark when there is room", () => {
    expect(labels(160)).toEqual(["+6", "0", "−6", "−12", "−24", "−48", "−∞"])
  })

  it("drops labels as the travel shortens, the least important first", () => {
    expect(labels(120)).toEqual(["+6", "0", "−6", "−12", "−24", "−∞"])
    expect(labels(60)).toEqual(["+6", "0", "−12", "−24", "−∞"])
    expect(labels(47)).toEqual(["+6", "0", "−12", "−∞"])
    expect(labels(40)).toEqual(["0", "−12", "−∞"])
    expect(labels(20)).toEqual(["0", "−∞"])
    expect(labels(5)).toEqual(["0"])
  })

  it("never puts two labels closer than the gap", () => {
    for (let length = 1; length <= 400; length += 1) {
      for (const gap of [10, 20]) {
        const kept = fitFaderLabels(placed, length, gap).filter(
          (mark) => mark.label !== undefined
        )
        expect(kept.length).toBeGreaterThan(0)
        for (const a of kept) {
          for (const b of kept) {
            if (a === b) continue
            expect(
              Math.abs(a.position - b.position) * length
            ).toBeGreaterThanOrEqual(gap)
          }
        }
      }
    }
  })

  it("keeps every mark, with or without its label", () => {
    const fitted = fitFaderLabels(placed, 20, 10)
    expect(fitted.map((mark) => mark.value)).toEqual(
      placed.map((mark) => mark.value)
    )
  })

  it("prefers strong marks to the ends in any set of marks", () => {
    const marks = [0, 25, 50, 75, 100].map((value) => ({
      value,
      label: String(value),
      strong: value === 25,
      position: value / 100,
    }))
    const kept = (length: number) =>
      fitFaderLabels(marks, length, 10)
        .filter((mark) => mark.label !== undefined)
        .map((mark) => mark.value)
    // The strong mark at 25 keeps its label and the end beside it gives way.
    expect(kept(30)).toEqual([25, 100])
    expect(kept(39)).toEqual([25, 100])
    expect(kept(40)).toEqual([0, 25, 50, 75, 100])
  })
})

function GainFader(props: Partial<React.ComponentProps<typeof Fader>>) {
  const [gain, setGain] = React.useState(props.value ?? 1)
  return (
    <Fader aria-label="Level" {...props} value={gain} onValueChange={setGain} />
  )
}

describe("Fader", () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it("labels only the dB marks that fit a short fader, and keeps the lines", () => {
    // 60 pixels of travel.
    vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(60)
    const { container } = render(<GainFader />)
    const scale = container.querySelector('[data-slot="fader-scale"]')
    const shown = [...(scale?.querySelectorAll("span") ?? [])]
      .map((span) => span.textContent)
      .filter((text) => text !== "")
    expect(shown).toEqual(["+6", "0", "−12", "−24", "−∞"])
    expect(scale?.querySelectorAll("span:empty")).toHaveLength(
      FADER_DB_TICKS.length * 2 - shown.length
    )
  })

  it("measures a horizontal fader along its width", () => {
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(100)
    vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(16)
    const { container } = render(<GainFader orientation="horizontal" />)
    const scale = container.querySelector('[data-slot="fader-scale"]')
    const shown = [...(scale?.querySelectorAll("span") ?? [])]
      .map((span) => span.textContent)
      .filter((text) => text !== "")
    // 20 pixels apart at least: 0 dB, the ends, then −12 and −24.
    expect(shown).toEqual(["+6", "0", "−12", "−24", "−∞"])
  })

  it("comes in two thicknesses, in both directions", () => {
    const control = (props: Partial<React.ComponentProps<typeof Fader>>) => {
      const { container, unmount } = render(<GainFader {...props} />)
      const classes = [
        ...(container.querySelector('[data-slot="fader-control"]')?.classList ??
          []),
      ]
      unmount()
      return classes
    }
    expect(control({})).toContain("w-7")
    expect(control({ size: "sm" })).toContain("w-5")
    expect(control({ size: "sm" })).not.toContain("w-7")
    expect(control({ orientation: "horizontal" })).toContain("h-7")
    expect(control({ orientation: "horizontal", size: "sm" })).toContain("h-5")
    expect(control({ orientation: "horizontal", size: "sm" })).not.toContain(
      "h-7"
    )
  })

  it("is a gain slider with a dB readout by default", () => {
    render(<GainFader value={0.5} showValue />)
    const slider = screen.getByRole("slider", { name: "Level" })
    expect(slider).toHaveAttribute("aria-valuemin", "0")
    expect(slider).toHaveAttribute("aria-valuemax", "2")
    expect(slider).toHaveAttribute("aria-valuenow", "0.5")
    expect(slider).toHaveAttribute("aria-valuetext", "−6.0 dB")
    expect(slider).toHaveAttribute("aria-orientation", "vertical")
    expect(screen.getByText("−6.0 dB")).toBeInTheDocument()
    expect(screen.getByText("−48")).toBeInTheDocument()
  })

  it("returns to 0 dB on double-click", () => {
    render(<GainFader value={0.2} />)
    const slider = screen.getByRole("slider")
    fireEvent.doubleClick(slider)
    expect(slider).toHaveAttribute("aria-valuenow", "1")
  })

  it("follows the pointer along its travel", () => {
    render(<GainFader value={1} />)
    const slider = screen.getByRole("slider")
    // 214 pixels tall with a 14 pixel cap leaves 200 pixels of travel.
    Object.defineProperty(slider, "clientHeight", { value: 214 })
    fireEvent.pointerDown(slider, { pointerId: 1, button: 0, clientY: 100 })
    // 0 dB sits at 78%, so -6 dB (62%) is 32 pixels further down.
    fireEvent.pointerMove(slider, { pointerId: 1, clientY: 132 })
    expect(Number(slider.getAttribute("aria-valuenow"))).toBeCloseTo(
      dbToGain(-6),
      6
    )
    fireEvent.pointerMove(slider, { pointerId: 1, clientY: 20 })
    expect(slider).toHaveAttribute("aria-valuenow", "2")
    fireEvent.pointerMove(slider, { pointerId: 1, clientY: 400 })
    expect(slider).toHaveAttribute("aria-valuenow", "0")
    // The cap stays under a pointer that went past the end: coming back
    // to where it started returns to the starting value.
    fireEvent.pointerMove(slider, { pointerId: 1, clientY: 100 })
    expect(Number(slider.getAttribute("aria-valuenow"))).toBeCloseTo(1, 6)
    fireEvent.pointerUp(slider, { pointerId: 1 })
  })

  it("types decibels", () => {
    render(<GainFader value={1} />)
    const slider = screen.getByRole("slider")
    fireEvent.keyDown(slider, { key: "Enter" })
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "-12" } })
    fireEvent.keyDown(screen.getByRole("textbox"), { key: "Enter" })
    expect(Number(slider.getAttribute("aria-valuenow"))).toBeCloseTo(
      dbToGain(-12),
      10
    )
  })

  it("takes its own range, scale and marks", () => {
    render(
      <GainFader
        value={25}
        min={0}
        max={100}
        step={5}
        orientation="horizontal"
        ticks={[{ value: 50, label: "half" }]}
      />
    )
    const slider = screen.getByRole("slider")
    expect(slider).toHaveAttribute("aria-valuemax", "100")
    expect(slider).toHaveAttribute("aria-valuetext", "25")
    expect(slider).toHaveAttribute("aria-orientation", "horizontal")
    expect(screen.getByText("half")).toBeInTheDocument()
    fireEvent.keyDown(slider, { key: "ArrowRight" })
    expect(slider).toHaveAttribute("aria-valuenow", "30")
  })

  it("renders a meter beside the travel", () => {
    render(<GainFader meter={<div data-testid="meter" />} />)
    expect(screen.getByTestId("meter").parentElement).toHaveAttribute(
      "data-slot",
      "fader-meter"
    )
  })
})
