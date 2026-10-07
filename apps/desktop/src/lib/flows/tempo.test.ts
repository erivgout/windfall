import { toast } from "sonner"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { getAppState, isEnabled, registry, runAction } from "@/lib/actions"
import { deviceInfo } from "@/lib/actions/builtin"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { settle, startTestApp } from "@/test/harness"

import { canScaleTempo } from "./tempo"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let stop: () => void

beforeEach(async () => {
  ;({ stop } = await startTestApp())
})
afterEach(() => {
  stop()
  vi.unstubAllGlobals()
})

const tempo = () => useProjectStore.getState().project.settings.tempoBpm
const enabled = (id: string) => {
  const action = registry.get(id)
  if (!action) throw new Error(`"${id}" is not registered`)
  return isEnabled(action, getAppState())
}
const labels = () =>
  useProjectStore.getState().history.entries.map((entry) => entry.label)

describe("the tempo actions", () => {
  it("halve and double the tempo, each as one undo step", async () => {
    expect(tempo()).toBe(128)
    await runAction("tempo.half")
    expect(tempo()).toBe(64)
    await runAction("tempo.double")
    await runAction("tempo.double")
    expect(tempo()).toBe(256)
    expect(labels()).toHaveLength(3)
  })

  it("are off where the result would be no tempo there can be", async () => {
    expect(canScaleTempo(19, 0.5)).toBe(false)
    expect(canScaleTempo(20, 0.5)).toBe(true)
    expect(canScaleTempo(261, 2)).toBe(true)
    expect(canScaleTempo(262, 2)).toBe(false)
    await dispatch({ type: "updateSettings", patch: { tempoBpm: 300 } })
    expect(enabled("tempo.double")).toBe(false)
    expect(enabled("tempo.half")).toBe(true)
  })

  it("put the tempo back to that of an empty project", async () => {
    expect(enabled("tempo.reset")).toBe(true)
    await runAction("tempo.reset")
    expect(tempo()).toBe(120)
    expect(enabled("tempo.reset")).toBe(false)
  })

  it("opens the reviewed tapper without changing tempo or history", async () => {
    await runAction("tempo.tap")
    expect(tempo()).toBe(128)
    expect(labels()).toEqual([])
    expect(useUiStore.getState().dialog).toBe("tempoTap")
  })
})

describe("copying the audio device's details", () => {
  it("puts them in one line, for a report", () => {
    expect(
      deviceInfo({
        running: true,
        host: "WASAPI",
        device: "Speakers",
        sampleRate: 48_000,
        bufferFrames: 480,
        latencyMs: 10,
        latencyFrames: 240,
        error: null,
      })
    ).toBe("WASAPI Speakers, 48 kHz, 480 samples, 10.0 ms, plus 240 samples")
    expect(
      deviceInfo({
        running: false,
        host: "WASAPI",
        device: null,
        sampleRate: 48_000,
        bufferFrames: 480,
        latencyMs: 10,
        latencyFrames: 0,
        error: "The device is gone.",
      })
    ).toBe("No audio: The device is gone.")
  })

  it("copies what the engine is running on and says so", async () => {
    const writeText = vi.fn(() => Promise.resolve())
    vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } })
    await runAction("engine.copyDeviceInfo")
    await settle()
    expect(writeText).toHaveBeenCalledWith(
      "WASAPI Speakers (simulated), 48 kHz, 256 samples, 5.3 ms"
    )
    expect(toast.success).toHaveBeenCalledWith("Copied", {
      description: "WASAPI Speakers (simulated), 48 kHz, 256 samples, 5.3 ms",
    })
  })
})
