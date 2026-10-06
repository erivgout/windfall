import { act, render } from "@testing-library/react"
import { memo, Profiler } from "react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { dispatch } from "@/lib/store/project"
import { startTestApp } from "@/test/harness"

import MixerPanel from "."
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

let stop: () => void

beforeEach(async () => {
  stubCanvas()
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
