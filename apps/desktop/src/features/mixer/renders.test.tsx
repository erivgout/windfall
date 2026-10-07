import { act, fireEvent, render, screen, within } from "@testing-library/react"
import { memo, Profiler, type ReactNode } from "react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { effectDescriptor, paramIndex } from "@/features/params"
import { newGestureId } from "@/lib/store/gesture"
import { dispatch } from "@/lib/store/project"
import { startTestApp } from "@/test/harness"

import MixerPanel from "."
import { addEffects, chain, slotButton, slotLamp } from "./effect-test-utils"
import { useEffectsUi } from "./effects-ui"
import type { MixerStripProps } from "./strip"
import {
  drag,
  flush,
  strip,
  stubCanvas,
  trackNamed,
  tracks,
} from "./test-utils"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

/** How many times anything inside each strip rendered, by track id. */
const renders = new Map<number, number>()

// Every strip gets a profiler around it. React reports to it whenever a
// component inside it renders, including renders a store subscription
// starts, which a counter in a parent would miss.
vi.mock("./strip", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./strip")>()
  const Counted = memo(function Counted(props: MixerStripProps) {
    return (
      <Profiler
        id={`strip-${props.id}`}
        onRender={() => renders.set(props.id, (renders.get(props.id) ?? 0) + 1)}
      >
        <actual.MixerStrip {...props} />
      </Profiler>
    )
  })
  return { ...actual, MixerStrip: Counted }
})

// jsdom gives every element the same empty box, so the real resize handle
// takes any press for one on itself.
vi.mock("@/components/ui/resizable", () => ({
  ResizablePanelGroup: ({ children }: { children: ReactNode }) => (
    <div className="flex">{children}</div>
  ),
  ResizablePanel: ({ children }: { children: ReactNode }) => (
    <div>{children}</div>
  ),
  ResizableHandle: () => null,
}))

let stop: () => void

beforeEach(async () => {
  stubCanvas()
  useEffectsUi.setState(useEffectsUi.getInitialState(), true)
  ;({ stop } = await startTestApp())
})
afterEach(() => {
  stop()
  vi.restoreAllMocks()
})

const count = (name: string) => renders.get(trackNamed(name).id) ?? 0

function fader(name: string): HTMLElement {
  const control = strip(name).querySelector<HTMLElement>(
    "[data-slot=fader-control]"
  )
  if (!control) throw new Error(`The strip "${name}" has no fader`)
  return control
}

describe("strip rendering", () => {
  it("renders only the strip whose fader is dragged", async () => {
    render(<MixerPanel />)
    await flush()
    for (const track of tracks()) {
      expect(renders.get(track.id)).toBeGreaterThan(0)
    }
    renders.clear()

    drag(fader("Kick"), [-4, -4, -4, -4, -4, -4])
    await flush()

    expect(trackNamed("Kick").volume).toBeLessThan(1)
    expect(count("Kick")).toBeGreaterThan(0)
    for (const name of ["Master", "Clap", "Hat", "Snare"]) {
      expect(count(name)).toBe(0)
    }
  })

  it("renders only the strip whose pan or mute changes", async () => {
    render(<MixerPanel />)
    await flush()
    renders.clear()

    const hat = trackNamed("Hat").id
    await dispatch({ type: "updateMixerTrack", id: hat, patch: { pan: 0.4 } })
    await dispatch({
      type: "updateMixerTrack",
      id: hat,
      patch: { muted: true },
    })
    await flush()

    expect(count("Hat")).toBeGreaterThan(0)
    for (const name of ["Master", "Kick", "Clap", "Snare"]) {
      expect(count(name)).toBe(0)
    }
  })

  it("renders only the two strips a selection moves between", async () => {
    render(<MixerPanel />)
    await flush()
    act(() => strip("Kick").focus())
    renders.clear()

    act(() => strip("Clap").focus())
    expect(count("Kick")).toBeGreaterThan(0)
    expect(count("Clap")).toBeGreaterThan(0)
    for (const name of ["Master", "Hat", "Snare"]) {
      expect(count(name)).toBe(0)
    }
  })

  it("renders the strip a solo changes, and no others", async () => {
    render(<MixerPanel />)
    await dispatch({
      type: "updateMixerTrack",
      id: trackNamed("Kick").id,
      patch: { solo: true },
    })
    await flush()
    renders.clear()

    // Clap joins the solo: it is the only strip whose state changes.
    await dispatch({
      type: "updateMixerTrack",
      id: trackNamed("Clap").id,
      patch: { solo: true },
    })
    await flush()
    expect(count("Clap")).toBeGreaterThan(0)
    for (const name of ["Master", "Kick", "Hat", "Snare"]) {
      expect(count(name)).toBe(0)
    }
  })
})

describe("strip rendering with effects", () => {
  const all = ["Master", "Kick", "Clap", "Hat", "Snare"]

  /** A chain on every track, the master included. */
  async function fillMixer() {
    for (const name of all) {
      await addEffects(name, "eq", "compressor", "limiter", "reverb", "delay")
    }
  }

  it("renders no strip while a setting of an effect is dragged", async () => {
    render(<MixerPanel />)
    await fillMixer()
    renders.clear()

    const kick = trackNamed("Kick").id
    const [eq, compressor] = chain("Kick")
    const frequency = paramIndex(effectDescriptor("eq"), "peak2.frequencyHz")
    const threshold = paramIndex(effectDescriptor("compressor"), "thresholdDb")
    const gesture = newGestureId()
    for (let step = 1; step <= 12; step += 1) {
      await dispatch(
        {
          type: "setEffectParam",
          track: kick,
          effect: eq.id,
          param: frequency,
          value: 1000 + step * 50,
        },
        gesture
      )
      await dispatch(
        {
          type: "setEffectParam",
          track: kick,
          effect: compressor.id,
          param: threshold,
          value: -18 - step,
        },
        gesture
      )
    }
    await dispatch({
      type: "setEffectParams",
      track: kick,
      effect: eq.id,
      params: { ...effectDescriptor("eq").defaults, outputGainDb: 3 },
    })
    await flush()

    expect(chain("Kick")[1].params).toMatchObject({ thresholdDb: -30 })
    for (const name of all) expect(count(name), name).toBe(0)
  })

  it("renders no strip while a knob in the inspector is dragged", async () => {
    render(<MixerPanel />)
    await fillMixer()
    const reverb = chain("Clap")[3]
    fireEvent.click(slotButton("Clap", reverb.id))
    await flush()
    const panel = screen
      .getByRole("complementary", { name: "Effects" })
      .querySelector<HTMLElement>(`[data-effect-row="${reverb.id}"]`)
    if (!panel) throw new Error("The inspector does not show the reverb")
    renders.clear()

    drag(within(panel).getByRole("slider", { name: "Decay" }), [5, 5, 5, 5])
    drag(
      within(panel).getByRole("slider", { name: "Reverb dry/wet mix" }),
      [-5, -5, -5]
    )
    await flush()

    expect(chain("Clap")[3].params).not.toEqual(reverb.params)
    expect(chain("Clap")[3].mix).toBeLessThan(1)
    for (const name of all) expect(count(name), name).toBe(0)
  })

  it("renders only the strip whose fader is dragged when every track has effects", async () => {
    render(<MixerPanel />)
    await fillMixer()
    renders.clear()

    drag(fader("Hat"), [-4, -4, -4, -4])
    await flush()

    expect(trackNamed("Hat").volume).toBeLessThan(1)
    expect(count("Hat")).toBeGreaterThan(0)
    for (const name of ["Master", "Kick", "Clap", "Snare"]) {
      expect(count(name), name).toBe(0)
    }
  })

  it("renders only the strip whose effect is switched off or reordered", async () => {
    render(<MixerPanel />)
    await fillMixer()
    renders.clear()

    fireEvent.click(slotLamp("Snare", chain("Snare")[0].id))
    await dispatch({
      type: "moveEffect",
      track: trackNamed("Snare").id,
      effect: chain("Snare")[0].id,
      index: 2,
    })
    await flush()

    expect(chain("Snare")[2].enabled).toBe(false)
    expect(count("Snare")).toBeGreaterThan(0)
    for (const name of ["Master", "Kick", "Clap", "Hat"]) {
      expect(count(name), name).toBe(0)
    }
  })
})
