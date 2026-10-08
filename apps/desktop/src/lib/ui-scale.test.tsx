import { useState } from "react"
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, describe, expect, it, vi } from "vitest"

import { Fader } from "@/components/audio/fader"
import { Knob } from "@/components/audio/knob"
import { PluginControls } from "@/features/plugins/controls"
import { SettingsDialog } from "@/features/settings/settings-dialog"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { startTestApp } from "@/test/harness"
import { useApplicationScale } from "./ui-scale-root"
import { applyUiScale, UI_SCALES } from "./ui-scale"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

afterEach(() => {
  applyUiScale(100)
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

function ScaleRoot() {
  useApplicationScale()
  return <SettingsDialog />
}

describe("display preference", () => {
  it("uses the real Settings select and restore without editing the document or calling the backend", async () => {
    const app = await startTestApp()
    try {
      useUiStore.getState().openDialog("settings")
      render(<ScaleRoot />)
      await waitFor(() =>
        expect(
          screen.getByRole("combobox", { name: "Buffer size" })
        ).toBeEnabled()
      )
      const dispatch = vi.spyOn(app.backend, "dispatch")
      const configure = vi.spyOn(app.backend, "engineConfigure")
      const before = useProjectStore.getState()
      const user = userEvent.setup()
      for (const value of UI_SCALES) {
        await user.click(
          screen.getByRole("combobox", { name: "Interface scale" })
        )
        await user.click(
          await screen.findByRole("option", { name: `${value}%` })
        )
        expect(document.documentElement.dataset.uiScale).toBe(String(value))
        expect(
          JSON.parse(localStorage.getItem("windfall.ui")!).state.uiScale
        ).toBe(value)
      }
      await user.click(screen.getByRole("button", { name: "Restore 100%" }))
      expect(useUiStore.getState().uiScale).toBe(100)
      expect(document.documentElement.style.zoom).toBe("1")
      // A preference change from another window uses the same root contract.
      localStorage.setItem(
        "windfall.ui",
        JSON.stringify({ version: 1, state: { uiScale: 125 } })
      )
      fireEvent(window, new StorageEvent("storage", { key: "windfall.ui" }))
      await waitFor(() =>
        expect(document.documentElement.dataset.uiScale).toBe("125")
      )
      expect(useProjectStore.getState()).toBe(before)
      expect(dispatch).not.toHaveBeenCalled()
      expect(configure).not.toHaveBeenCalled()
    } finally {
      app.stop()
    }
  })

  it.each([75, 125, 150, 200])(
    "retains %i on rehydration and keeps it out of project state",
    async (value) => {
      useUiStore.setState(useUiStore.getInitialState(), true)
      localStorage.setItem(
        "windfall.ui",
        JSON.stringify({
          version: 1,
          state: { uiScale: value, theme: "light" },
        })
      )
      await useUiStore.persist.rehydrate()
      expect(useUiStore.getState().uiScale).toBe(value)
      expect(useUiStore.getState().theme).toBe("light")
    }
  )

  it.each([undefined, null, "150", 0, 74, 201, 125.5, {}, [150]])(
    "recovers invalid stored scale %j",
    async (uiScale) => {
      useUiStore.setState(useUiStore.getInitialState(), true)
      localStorage.setItem(
        "windfall.ui",
        JSON.stringify({ version: 1, state: { uiScale } })
      )
      await useUiStore.persist.rehydrate()
      expect(useUiStore.getState().uiScale).toBe(100)
    }
  )

  it("recovers malformed JSON, and an unsupported zoom stays at 100%", async () => {
    useUiStore.setState(useUiStore.getInitialState(), true)
    localStorage.setItem("windfall.ui", "{broken")
    await useUiStore.persist.rehydrate()
    expect(useUiStore.getState().uiScale).toBe(100)
    vi.stubGlobal("CSS", { supports: () => false })
    applyUiScale(200)
    expect(document.documentElement.dataset.uiScale).toBe("100")
  })
})

function Controls() {
  const [knob, setKnob] = useState(0)
  const [fader, setFader] = useState(0)
  return (
    <>
      <Knob
        aria-label="Scale knob"
        value={knob}
        onValueChange={setKnob}
        dragRange={100}
      />
      <Fader
        aria-label="Scale fader"
        value={fader}
        min={0}
        max={1}
        scale="linear"
        onValueChange={setFader}
      />
    </>
  )
}

function PluginRoot() {
  useApplicationScale()
  const binding = useProjectStore((state) => state.project.plugins?.[0])
  return binding ? <PluginControls binding={binding} /> : null
}

describe.each([75, 125, 150, 200] as const)(
  "hosted parameter editor at %i%%",
  (scale) => {
    it("inherits application scale and edits through the real generic plugin control", async () => {
      const app = await startTestApp()
      try {
        await dispatch({
          type: "addPluginInstrument",
          plugin: {
            target: { type: "instrument", channel: 0 },
            format: "clap",
            path: "/fixture/synth.clap",
            id: "scale.fixture",
            name: "Scale fixture",
            state: [],
            parameters: [
              {
                id: 43,
                name: "Cutoff",
                min: 100,
                max: 8000,
                value: 400,
                stepped: false,
                readOnly: false,
                automatable: true,
              },
            ],
          },
        })
        useUiStore.getState().setUiScale(scale)
        render(<PluginRoot />)
        const control = await screen.findByRole("spinbutton", {
          name: "Cutoff",
        })
        // A committed number-field edit, through its real DOM handlers.
        fireEvent.focus(control)
        fireEvent.change(control, { target: { value: "1200" } })
        fireEvent.blur(control)
        await waitFor(() =>
          expect(
            useProjectStore.getState().project.plugins?.[0].parameters[0].value
          ).toBe(1200)
        )
        expect(document.documentElement.dataset.uiScale).toBe(String(scale))
      } finally {
        app.stop()
      }
    })
  }
)

describe.each([75, 125, 150, 200])("real controls at %i%%", (scale) => {
  it("drags knob and fader to both endpoints in scaled pixels and preserves keyboard endpoints", () => {
    applyUiScale(scale)
    class Pointer extends MouseEvent {
      readonly pointerType = "mouse"
      readonly pointerId = 1
    }
    vi.stubGlobal("PointerEvent", Pointer)
    render(<Controls />)
    const knob = screen.getByRole("slider", { name: "Scale knob" })
    const fader = screen.getByRole("slider", { name: "Scale fader" })
    Object.defineProperty(fader, "clientHeight", { value: 112 })
    for (const control of [knob, fader]) {
      fireEvent.pointerDown(control, { button: 0, clientY: 400 })
      fireEvent.pointerMove(control, { clientY: 400 - scale })
      fireEvent.pointerUp(control)
      expect(control).toHaveAttribute("aria-valuenow", "1")
      fireEvent.pointerDown(control, { button: 0, clientY: 400 })
      fireEvent.pointerMove(control, { clientY: 400 + scale })
      fireEvent.pointerUp(control)
      expect(control).toHaveAttribute("aria-valuenow", "0")
      act(() => control.focus())
      fireEvent.keyDown(control, { key: "End" })
      expect(control).toHaveAttribute("aria-valuenow", "1")
      fireEvent.keyDown(control, { key: "Home" })
      expect(control).toHaveAttribute("aria-valuenow", "0")
    }
  })
})
