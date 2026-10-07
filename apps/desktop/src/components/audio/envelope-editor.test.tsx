// SPDX-License-Identifier: MIT
import * as React from "react"
import { fireEvent, render, screen, within } from "@testing-library/react"
import { describe, expect, it } from "vitest"

import {
  DEFAULT_ENVELOPE_LIMITS,
  EnvelopeEditor,
  constrainEnvelope,
  envelopeFall,
  envelopePatch,
  envelopeRise,
  envelopeSpanMs,
  fitEnvelopeLabels,
  stepEnvelopeTime,
  type EnvelopeAxisLabel,
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

describe("the shape of a stage", () => {
  it("falls from all of the way to none of it, landing exactly", () => {
    expect(envelopeFall(0)).toBe(1)
    expect(envelopeFall(1)).toBe(0)
    expect(envelopeFall(-1)).toBe(1)
    expect(envelopeFall(2)).toBe(0)
    // Just short of the end it is all but there, with no step at the end.
    expect(envelopeFall(0.999)).toBeLessThan(0.00001)
  })

  it("covers 60 dB of the fall over the stage, so half the time is 30 dB", () => {
    // The engine's curve: (1 + f) (f / (1 + f))^p - f with f = 0.001.
    const expected = 1.001 * Math.sqrt(0.001 / 1.001) - 0.001
    expect(envelopeFall(0.5)).toBeCloseTo(expected, 12)
    expect(envelopeFall(0.5)).toBeGreaterThan(0.03)
    expect(envelopeFall(0.5)).toBeLessThan(0.032)
    // A tenth of the time takes half of the level away.
    expect(envelopeFall(0.1)).toBeCloseTo(0.5, 1)
  })

  it("only ever falls", () => {
    let previous = envelopeFall(0)
    for (let step = 1; step <= 100; step += 1) {
      const now = envelopeFall(step / 100)
      expect(now).toBeLessThan(previous)
      previous = now
    }
  })

  it("rises to full level exactly, fast at first", () => {
    expect(envelopeRise(0)).toBe(0)
    expect(envelopeRise(1)).toBe(1)
    // Aiming 30% past full level: 1.3 (1 - (0.3 / 1.3)^p).
    expect(envelopeRise(0.5)).toBeCloseTo(1.3 * (1 - Math.sqrt(0.3 / 1.3)), 12)
    expect(envelopeRise(0.5)).toBeGreaterThan(0.5)
    let previous = 0
    for (let step = 1; step <= 100; step += 1) {
      const now = envelopeRise(step / 100)
      expect(now).toBeGreaterThan(previous)
      previous = now
    }
  })
})

describe("the labels of the time axis", () => {
  // What a 500 ms axis is marked with, on a plot `width` pixels wide.
  const marks = (width: number): EnvelopeAxisLabel[] =>
    ["0", "100 ms", "200 ms", "300 ms", "400 ms", "500 ms"].map(
      (text, index) => ({ x: 12 + (width * index) / 5, text })
    )
  // The room a label takes, the way the component reckons it.
  const box = (label: EnvelopeAxisLabel, all: EnvelopeAxisLabel[]) => {
    const width = label.text.length * 5.6
    const left =
      label === all[0]
        ? label.x
        : label === all[all.length - 1]
          ? label.x - width
          : label.x - width / 2
    return [left, left + width] as const
  }

  it("all fit on a wide editor", () => {
    expect(fitEnvelopeLabels(marks(296)).map((label) => label.text)).toEqual([
      "0",
      "100 ms",
      "200 ms",
      "300 ms",
      "400 ms",
      "500 ms",
    ])
  })

  it("are thinned on a narrow one until none touches another", () => {
    const all = marks(136)
    const kept = fitEnvelopeLabels(all)
    expect(kept.length).toBeLessThan(all.length)
    // The ends stay: where the axis starts and how long it is.
    expect(kept[0].text).toBe("0")
    expect(kept[kept.length - 1].text).toBe("500 ms")
    for (let index = 1; index < kept.length; index += 1) {
      const [, previousRight] = box(kept[index - 1], all)
      const [left] = box(kept[index], all)
      expect(left).toBeGreaterThan(previousRight)
    }
  })

  it("keeps only the start when not even two fit", () => {
    expect(fitEnvelopeLabels(marks(20)).map((label) => label.text)).toEqual([
      "0",
    ])
    expect(fitEnvelopeLabels([])).toEqual([])
  })
})

describe("EnvelopeEditor", () => {
  const shapeOf = (container: HTMLElement) =>
    container.querySelector("[data-slot=envelope-shape]")?.getAttribute("d") ??
    ""
  const tailOf = (container: HTMLElement) =>
    container.querySelector("[data-slot=envelope-tail]")?.getAttribute("d") ??
    ""
  const points = (path: string) => path.split(/[ML]/).length - 1

  it("draws straight lines unless told the envelope is curved", () => {
    const { container } = render(<EnvelopeEditor {...START} />)
    // Attack and decay: start, peak, peak again, sustain.
    expect(points(shapeOf(container))).toBe(4)
    expect(points(tailOf(container))).toBe(2)
  })

  it("draws the decay and the release as the curves the engine plays", () => {
    const straight = render(<EnvelopeEditor {...START} />)
    const linearShape = shapeOf(straight.container)
    const linearTail = tailOf(straight.container)
    straight.unmount()

    const { container } = render(
      <EnvelopeEditor {...START} curve="exponential" />
    )
    // The attack stays a line; the decay is drawn through many points.
    expect(points(shapeOf(container))).toBeGreaterThan(20)
    expect(points(tailOf(container))).toBeGreaterThan(20)
    expect(shapeOf(container)).not.toBe(linearShape)
    expect(tailOf(container)).not.toBe(linearTail)
    // The curves still start and end on the nodes, where the lines did.
    const ends = (path: string) => {
      const all = path.split(/[ML]/).filter(Boolean)
      return [all[0].trim(), all[all.length - 1].trim()]
    }
    expect(ends(tailOf(container))).toEqual(ends(linearTail))
    expect(ends(shapeOf(container))).toEqual(ends(linearShape))

    // Half way through the release the level has all but gone.
    const coords = tailOf(container)
      .split(/[ML]/)
      .filter(Boolean)
      .map((pair) => pair.trim().split(" ").map(Number))
    const [, top] = coords[0]
    const [, bottom] = coords[coords.length - 1]
    const [, middle] = coords[(coords.length - 1) / 2]
    expect((bottom - middle) / (bottom - top)).toBeCloseTo(envelopeFall(0.5), 2)
  })

  it("rounds the attack as well for an envelope that has one", () => {
    const sharp = render(<EnvelopeEditor {...START} curve="exponential" />)
    const sharpShape = shapeOf(sharp.container)
    sharp.unmount()
    const { container } = render(<EnvelopeEditor {...START} curve="rounded" />)
    expect(points(shapeOf(container))).toBeGreaterThan(points(sharpShape))
    expect(container.firstElementChild).toHaveAttribute("data-curve", "rounded")
  })

  it("leaves Delete and Backspace on a node to the app", () => {
    const changes: Partial<EnvelopeValues>[] = []
    render(
      <EnvelopeEditor
        {...START}
        defaults={{ releaseMs: 50 }}
        onChange={(patch) => changes.push(patch)}
      />
    )
    const release = screen.getByRole("slider", { name: "Release" })
    expect(fireEvent.keyDown(release, { key: "Delete" })).toBe(true)
    expect(fireEvent.keyDown(release, { key: "Backspace" })).toBe(true)
    expect(changes).toEqual([])
    // A double-click is what puts a node back.
    fireEvent.doubleClick(release)
    expect(changes).toEqual([{ releaseMs: 50 }])
  })

  it("is a group, so a label on it names the three nodes together", () => {
    render(
      <EnvelopeEditor
        aria-label="Envelope shape"
        attackMs={10}
        decayMs={200}
        sustain={0.5}
        releaseMs={300}
      />
    )
    const group = screen.getByRole("group", { name: "Envelope shape" })
    expect(group).toHaveAttribute("data-slot", "envelope-editor")
    expect(within(group).getAllByRole("slider")).toHaveLength(3)
  })

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
