// SPDX-License-Identifier: MIT
import * as React from "react"
import { fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it } from "vitest"

import {
  DEFAULT_ENVELOPE_LIMITS,
  EnvelopeEditor,
  constrainEnvelope,
  envelopePatch,
  envelopeSpanMs,
  stepEnvelopeTime,
  type EnvelopeValues,
} from "./envelope-editor"

const START: EnvelopeValues = {
  attackMs: 10,
  decayMs: 200,
  sustain: 0.5,
  releaseMs: 300,
}

describe("envelope values", () => {
  it("clamps every field to its range", () => {
    expect(
      constrainEnvelope({
        attackMs: -5,
        decayMs: 99999,
        sustain: 1.4,
        releaseMs: Number.NaN,
      })
    ).toEqual({
      attackMs: 0,
      decayMs: DEFAULT_ENVELOPE_LIMITS.maxDecayMs,
      sustain: 1,
      releaseMs: 0,
    })
    expect(constrainEnvelope({ ...START, sustain: -2 }).sustain).toBe(0)
  })

  it("honors custom limits", () => {
    const limits = { maxAttackMs: 100, maxDecayMs: 100, maxReleaseMs: 100 }
    expect(constrainEnvelope(START, limits)).toEqual({
      attackMs: 10,
      decayMs: 100,
      sustain: 0.5,
      releaseMs: 100,
    })
  })

  it("rounds times to a tenth of a millisecond and sustain to a thousandth", () => {
    const rounded = constrainEnvelope({
      attackMs: 1.2345,
      decayMs: 200.06,
      sustain: 0.33333,
      releaseMs: 300,
    })
    expect(rounded).toEqual({
      attackMs: 1.2,
      decayMs: 200.1,
      sustain: 0.333,
      releaseMs: 300,
    })
  })

  it("patches only what changed", () => {
    expect(envelopePatch(START, START)).toEqual({})
    expect(
      envelopePatch(START, { ...START, decayMs: 250, sustain: 0.4 })
    ).toEqual({ decayMs: 250, sustain: 0.4 })
  })

  it("fits the time axis to a round length", () => {
    expect(envelopeSpanMs(START)).toBe(1000)
    expect(
      envelopeSpanMs({ attackMs: 1, decayMs: 10, sustain: 1, releaseMs: 10 })
    ).toBe(100)
    expect(
      envelopeSpanMs({
        attackMs: 500,
        decayMs: 500,
        sustain: 1,
        releaseMs: 2000,
      })
    ).toBe(5000)
    for (const total of [30, 140, 900, 2600, 19000]) {
      const span = envelopeSpanMs({
        attackMs: total / 3,
        decayMs: total / 3,
        sustain: 0.5,
        releaseMs: total / 3,
      })
      expect(span * 0.75).toBeGreaterThanOrEqual(total - 1e-6)
    }
  })

  it("steps times by a share of the value, with a floor", () => {
    expect(stepEnvelopeTime(1000, 1, "normal")).toBe(1050)
    expect(stepEnvelopeTime(4, 1, "normal")).toBe(5)
    expect(stepEnvelopeTime(4, -1, "fine")).toBeCloseTo(3.9, 10)
    expect(stepEnvelopeTime(100, 1, "coarse")).toBe(125)
    expect(stepEnvelopeTime(0, 1, "coarse")).toBe(10)
  })
})

function renderEditor(
  initial: EnvelopeValues = START,
  props: Partial<React.ComponentProps<typeof EnvelopeEditor>> = {}
) {
  const patches: Partial<EnvelopeValues>[] = []
  const order: string[] = []
  function Harness() {
    const [values, setValues] = React.useState(initial)
    return (
      <EnvelopeEditor
        {...values}
        {...props}
        onChange={(patch) => {
          patches.push(patch)
          order.push("change")
          setValues((was) => ({ ...was, ...patch }))
        }}
        onGestureStart={() => order.push("start")}
        onGestureEnd={() => order.push("end")}
      />
    )
  }
  render(<Harness />)
  const node = (name: string) => screen.getByRole("slider", { name })
  return { patches, order, node }
}

const valueOf = (node: HTMLElement) =>
  Number(node.getAttribute("aria-valuenow"))

describe("EnvelopeEditor", () => {
  it("exposes three slider nodes with their values", () => {
    const { node } = renderEditor()
    expect(node("Attack")).toHaveAttribute("aria-valuenow", "10")
    expect(node("Attack")).toHaveAttribute("aria-valuetext", "10 ms")
    expect(node("Attack")).toHaveAttribute("aria-valuemin", "0")
    expect(node("Attack")).toHaveAttribute("aria-valuemax", "5000")
    expect(node("Decay and sustain")).toHaveAttribute(
      "aria-valuetext",
      "200 ms, sustain 50%"
    )
    expect(node("Release")).toHaveAttribute("aria-valuemax", "10000")
    expect(node("Release")).toHaveAttribute("tabindex", "0")
  })

  it("changes times with left and right, as one gesture per key hold", () => {
    const { patches, order, node } = renderEditor()
    fireEvent.keyDown(node("Attack"), { key: "ArrowRight" })
    fireEvent.keyDown(node("Attack"), { key: "ArrowRight" })
    fireEvent.keyUp(node("Attack"), { key: "ArrowRight" })
    expect(patches).toEqual([{ attackMs: 11 }, { attackMs: 12 }])
    expect(order).toEqual(["start", "change", "change", "end"])
    fireEvent.keyDown(node("Release"), { key: "ArrowLeft" })
    expect(valueOf(node("Release"))).toBe(285)
  })

  it("changes sustain with up and down on the decay node", () => {
    const { patches, node } = renderEditor()
    const decay = node("Decay and sustain")
    fireEvent.keyDown(decay, { key: "ArrowUp" })
    fireEvent.keyDown(decay, { key: "ArrowDown", shiftKey: true })
    fireEvent.keyDown(decay, { key: "ArrowRight" })
    expect(patches).toEqual([
      { sustain: 0.51 },
      { sustain: 0.509 },
      { decayMs: 210 },
    ])
  })

  it("never emits a value outside its range", () => {
    const { patches, node } = renderEditor({
      attackMs: 4990,
      decayMs: 3,
      sustain: 0.995,
      releaseMs: 1,
    })
    fireEvent.keyDown(node("Attack"), { key: "PageUp" })
    fireEvent.keyDown(node("Attack"), { key: "PageUp" })
    fireEvent.keyDown(node("Decay and sustain"), { key: "ArrowUp" })
    fireEvent.keyDown(node("Decay and sustain"), { key: "ArrowUp" })
    fireEvent.keyDown(node("Release"), { key: "PageDown" })
    fireEvent.keyDown(node("Release"), { key: "PageDown" })
    fireEvent.keyDown(node("Release"), { key: "End" })
    expect(patches).toEqual([
      { attackMs: 5000 },
      { sustain: 1 },
      { releaseMs: 0 },
      { releaseMs: 10000 },
    ])
  })

  it("jumps to the ends with Home and End", () => {
    const { node } = renderEditor()
    fireEvent.keyDown(node("Attack"), { key: "End" })
    expect(valueOf(node("Attack"))).toBe(5000)
    fireEvent.keyDown(node("Attack"), { key: "Home" })
    expect(valueOf(node("Attack"))).toBe(0)
  })

  it("drags a node along the time axis and the level", () => {
    const { patches, order, node } = renderEditor()
    const decay = node("Decay and sustain")
    // Without layout the plot is 320 by 128: 296 by 100 inside its padding,
    // and the axis is one second long.
    fireEvent.pointerDown(decay, {
      pointerId: 1,
      button: 0,
      clientX: 100,
      clientY: 50,
    })
    fireEvent.pointerMove(decay, { pointerId: 1, clientX: 129.6, clientY: 30 })
    fireEvent.pointerUp(decay, { pointerId: 1 })
    expect(patches).toHaveLength(1)
    expect(patches[0].decayMs).toBeCloseTo(300, 0)
    expect(patches[0].sustain).toBeCloseTo(0.7, 3)
    expect(order).toEqual(["start", "change", "end"])
  })

  it("drags only the time of the attack and release nodes", () => {
    const { patches, node } = renderEditor()
    const release = node("Release")
    fireEvent.pointerDown(release, {
      pointerId: 1,
      button: 0,
      clientX: 0,
      clientY: 0,
    })
    fireEvent.pointerMove(release, {
      pointerId: 1,
      clientX: -2000,
      clientY: 500,
    })
    fireEvent.pointerUp(release, { pointerId: 1 })
    expect(patches).toEqual([{ releaseMs: 0 }])
  })

  it("restores a node's defaults on double-click", () => {
    const { patches, node } = renderEditor(START, {
      defaults: { decayMs: 120, sustain: 0.8, releaseMs: 50 },
    })
    fireEvent.doubleClick(node("Decay and sustain"))
    expect(patches).toEqual([{ decayMs: 120, sustain: 0.8 }])
    fireEvent.doubleClick(node("Attack"))
    expect(patches).toHaveLength(1)
  })

  it("ignores input when disabled", () => {
    const { patches, node } = renderEditor(START, { disabled: true })
    fireEvent.keyDown(node("Attack"), { key: "ArrowRight" })
    fireEvent.pointerDown(node("Attack"), { pointerId: 1, button: 0 })
    fireEvent.pointerMove(node("Attack"), { pointerId: 1, clientX: 50 })
    expect(patches).toEqual([])
    expect(node("Attack")).toHaveAttribute("aria-disabled", "true")
    expect(node("Attack")).toHaveAttribute("tabindex", "-1")
  })
})
