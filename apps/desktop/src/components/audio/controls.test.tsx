// SPDX-License-Identifier: MIT
import * as React from "react"
import { act, fireEvent, render, screen } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"

import { NumberField } from "./number-field"
import { PanControl } from "./pan-control"
import { MuteSolo, ToggleLed } from "./toggle-led"
import { Waveform, drawWaveform, type WaveformHandle } from "./waveform"

describe("PanControl", () => {
  function Pan({ start = 0.5 }: { start?: number }) {
    const [pan, setPan] = React.useState(start)
    return <PanControl value={pan} onValueChange={setPan} />
  }

  it("is a -1 to 1 slider read as left, center and right", () => {
    render(<Pan start={-0.5} />)
    const slider = screen.getByRole("slider", { name: "Pan" })
    expect(slider).toHaveAttribute("aria-valuemin", "-1")
    expect(slider).toHaveAttribute("aria-valuemax", "1")
    expect(slider).toHaveAttribute("aria-valuetext", "L50")
  })

  it("returns to the center on double-click", () => {
    render(<Pan />)
    const slider = screen.getByRole("slider")
    fireEvent.doubleClick(slider)
    expect(slider).toHaveAttribute("aria-valuenow", "0")
    expect(slider).toHaveAttribute("aria-valuetext", "C")
  })

  it("types a side and an amount", () => {
    render(<Pan />)
    const slider = screen.getByRole("slider")
    fireEvent.keyDown(slider, { key: "Enter" })
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "L30" } })
    fireEvent.keyDown(screen.getByRole("textbox"), { key: "Enter" })
    expect(slider).toHaveAttribute("aria-valuenow", "-0.3")
  })
})

describe("NumberField", () => {
  function Tempo(props: Partial<React.ComponentProps<typeof NumberField>>) {
    const [tempo, setTempo] = React.useState(props.value ?? 120.5)
    return (
      <NumberField
        aria-label="Tempo"
        min={10}
        max={522}
        step={0.01}
        unit="BPM"
        {...props}
        value={tempo}
        onValueChange={setTempo}
      />
    )
  }
  const valueOf = (slider: HTMLElement) =>
    Number(slider.getAttribute("aria-valuenow"))

  it("shows the value split at the decimal point, with its unit", () => {
    render(<Tempo />)
    const slider = screen.getByRole("slider", { name: "Tempo" })
    expect(slider).toHaveAttribute("aria-valuetext", "120.50")
    expect(slider).toHaveTextContent("120.50BPM")
    expect(slider.querySelector('[data-part="whole"]')).toHaveTextContent("120")
    expect(slider.querySelector('[data-part="fraction"]')).toHaveTextContent(
      ".50"
    )
  })

  it("drags by one per four pixels", () => {
    render(<Tempo />)
    const slider = screen.getByRole("slider")
    fireEvent.pointerDown(slider, { pointerId: 1, button: 0, clientY: 100 })
    fireEvent.pointerMove(slider, { pointerId: 1, clientY: 60 })
    fireEvent.pointerUp(slider, { pointerId: 1 })
    expect(valueOf(slider)).toBeCloseTo(130.5, 6)
  })

  it("drags the whole part in whole steps and keeps the decimals", () => {
    render(<Tempo splitDrag />)
    const slider = screen.getByRole("slider")
    const whole = slider.querySelector('[data-part="whole"]') as HTMLElement
    fireEvent.pointerDown(whole, { pointerId: 1, button: 0, clientY: 100 })
    fireEvent.pointerMove(whole, { pointerId: 1, clientY: 90 })
    expect(valueOf(slider)).toBe(123.5)
    fireEvent.pointerMove(whole, { pointerId: 1, clientY: 89 })
    expect(valueOf(slider)).toBe(123.5)
    fireEvent.pointerUp(whole, { pointerId: 1 })
  })

  it("drags the decimals by single steps", () => {
    render(<Tempo splitDrag />)
    const slider = screen.getByRole("slider")
    const fraction = slider.querySelector(
      '[data-part="fraction"]'
    ) as HTMLElement
    fireEvent.pointerDown(fraction, { pointerId: 1, button: 0, clientY: 100 })
    fireEvent.pointerMove(fraction, { pointerId: 1, clientY: 80 })
    fireEvent.pointerUp(fraction, { pointerId: 1 })
    expect(valueOf(slider)).toBeCloseTo(120.55, 6)
  })

  it("steps by one with the arrows and by the step with Shift", () => {
    render(<Tempo />)
    const slider = screen.getByRole("slider")
    fireEvent.keyDown(slider, { key: "ArrowUp" })
    expect(valueOf(slider)).toBe(121.5)
    fireEvent.keyDown(slider, { key: "ArrowDown", shiftKey: true })
    expect(valueOf(slider)).toBe(121.49)
    fireEvent.keyDown(slider, { key: "PageUp" })
    expect(valueOf(slider)).toBe(131.49)
  })

  it("opens typing on double-click and clamps what is typed", () => {
    render(<Tempo />)
    const slider = screen.getByRole("slider")
    fireEvent.doubleClick(slider)
    const input = screen.getByRole("textbox")
    expect(input).toHaveValue("120.50")
    fireEvent.change(input, { target: { value: "9000" } })
    fireEvent.keyDown(input, { key: "Enter" })
    expect(valueOf(slider)).toBe(522)
  })

  it("nudges with the wheel when focused", () => {
    render(<Tempo />)
    const slider = screen.getByRole("slider")
    act(() => slider.focus())
    fireEvent.wheel(slider, { deltaY: -100 })
    expect(valueOf(slider)).toBe(121.5)
  })

  it("handles whole numbers without a decimal part", () => {
    render(<Tempo value={8} min={1} max={64} step={1} unit={undefined} />)
    const slider = screen.getByRole("slider")
    expect(slider).toHaveTextContent(/^8$/)
    expect(slider.querySelector('[data-part="fraction"]')).toBeNull()
  })
})

describe("ToggleLed", () => {
  it("is a pressed-state button that reports the next state", () => {
    const calls: boolean[] = []
    const { rerender } = render(
      <ToggleLed pressed={false} onPressedChange={(on) => calls.push(on)}>
        M
      </ToggleLed>
    )
    const button = screen.getByRole("button", { name: "M" })
    expect(button).toHaveAttribute("aria-pressed", "false")
    fireEvent.click(button)
    rerender(
      <ToggleLed pressed onPressedChange={(on) => calls.push(on)}>
        M
      </ToggleLed>
    )
    expect(button).toHaveAttribute("aria-pressed", "true")
    fireEvent.click(button)
    expect(calls).toEqual([true, false])
  })

  it("does not fire when disabled", () => {
    const calls: boolean[] = []
    render(
      <ToggleLed
        pressed={false}
        disabled
        onPressedChange={(on) => calls.push(on)}
        aria-label="Arm"
      />
    )
    fireEvent.click(screen.getByRole("button", { name: "Arm" }))
    expect(calls).toEqual([])
  })

  it("pairs mute and solo", () => {
    const calls: string[] = []
    render(
      <MuteSolo
        muted={false}
        solo
        onMutedChange={(muted) => calls.push(`mute ${muted}`)}
        onSoloChange={(solo) => calls.push(`solo ${solo}`)}
      />
    )
    expect(screen.getByRole("button", { name: "Mute" })).toHaveAttribute(
      "aria-pressed",
      "false"
    )
    expect(screen.getByRole("button", { name: "Solo" })).toHaveAttribute(
      "aria-pressed",
      "true"
    )
    fireEvent.click(screen.getByRole("button", { name: "Mute" }))
    fireEvent.click(screen.getByRole("button", { name: "Solo" }))
    expect(calls).toEqual(["mute true", "solo false"])
  })
})

describe("Waveform", () => {
  beforeEach(() => {
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null)
  })

  const PEAKS = new Float32Array([-0.5, 0.5, -1, 1, -0.25, 0.25, 0, 0])

  function Region() {
    const [region, setRegion] = React.useState({ start: 0.2, end: 0.8 })
    return (
      <Waveform
        peaks={PEAKS}
        start={region.start}
        end={region.end}
        minRegion={0.05}
        onStartChange={(start) => setRegion((was) => ({ ...was, start }))}
        onEndChange={(end) => setRegion((was) => ({ ...was, end }))}
      />
    )
  }
  const valueOf = (slider: HTMLElement) =>
    Number(slider.getAttribute("aria-valuenow"))

  it("shows no handles without a region", () => {
    render(<Waveform peaks={PEAKS} />)
    expect(screen.queryAllByRole("slider")).toHaveLength(0)
  })

  it("exposes the region handles as sliders that cannot cross", () => {
    render(<Region />)
    const start = screen.getByRole("slider", { name: "Region start" })
    const end = screen.getByRole("slider", { name: "Region end" })
    expect(start).toHaveAttribute("aria-valuenow", "0.2")
    expect(start).toHaveAttribute("aria-valuetext", "20.0%")
    expect(start).toHaveAttribute("aria-orientation", "horizontal")
    expect(Number(start.getAttribute("aria-valuemax"))).toBeCloseTo(0.75, 10)
    expect(Number(end.getAttribute("aria-valuemin"))).toBeCloseTo(0.25, 10)
    fireEvent.keyDown(start, { key: "End" })
    expect(valueOf(start)).toBeCloseTo(0.75, 10)
    fireEvent.keyDown(end, { key: "Home" })
    expect(valueOf(end)).toBeCloseTo(0.8, 10)
    expect(valueOf(end)).toBeGreaterThan(valueOf(start))
  })

  it("moves a handle with the arrow keys and resets it on double-click", () => {
    render(<Region />)
    const start = screen.getByRole("slider", { name: "Region start" })
    fireEvent.keyDown(start, { key: "ArrowRight" })
    expect(valueOf(start)).toBeGreaterThan(0.2)
    fireEvent.doubleClick(start)
    expect(valueOf(start)).toBe(0)
  })

  it("moves the playhead through its ref", () => {
    const ref = React.createRef<WaveformHandle>()
    const { container } = render(<Waveform ref={ref} peaks={PEAKS} />)
    const playhead = container.querySelector<HTMLElement>(
      '[data-slot="waveform-playhead"]'
    )
    act(() => ref.current?.setPlayhead(0.25))
    expect(Number.parseFloat(playhead?.style.left ?? "")).toBe(25)
    expect(playhead?.style.display).toBe("block")
    act(() => ref.current?.setPlayhead(7))
    expect(Number.parseFloat(playhead?.style.left ?? "")).toBe(100)
    act(() => ref.current?.setPlayhead(null))
    expect(playhead?.style.display).toBe("none")
  })

  it("draws one column per pixel from the min and max pairs", () => {
    const rects: number[][] = []
    const context = {
      fillStyle: "",
      fillRect: (...rect: number[]) => rects.push(rect),
    } as unknown as CanvasRenderingContext2D
    drawWaveform(context, PEAKS, 4, 100, "red")
    expect(rects).toEqual([
      [0, 25, 1, 50],
      [1, 0, 1, 100],
      [2, 37, 1, 26],
      [3, 50, 1, 1],
    ])
    rects.length = 0
    // Fewer columns than buckets: each column covers the extremes of its buckets.
    drawWaveform(context, PEAKS, 2, 100, "red")
    expect(rects).toEqual([
      [0, 0, 1, 100],
      [1, 37, 1, 26],
    ])
  })
})
