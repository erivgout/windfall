import { act, render, screen, waitFor } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it } from "vitest"

import { useEngineStore } from "@/lib/store/engine"
import { useHintStore } from "@/lib/store/hint"
import { dispatch, undo } from "@/lib/store/project"
import { startTestApp } from "@/test/harness"

import { StatusBar } from "./status-bar"

let stop: () => void

beforeEach(async () => {
  ;({ stop } = await startTestApp())
})
afterEach(() => stop())

const engine = () =>
  screen.getByRole("button", { name: /^Audio output status/ })

describe("the hint in the status bar", () => {
  const hint = () => document.querySelector("[data-slot=status-hint]")!

  it("gives way to a notice about what was just done, and comes back", () => {
    render(<StatusBar />)
    act(() => useHintStore.setState({ text: "Drag to change the level" }))
    expect(hint()).toHaveTextContent("Drag to change the level")
    expect(hint()).not.toHaveAttribute("data-notice")
    // A line cut short by a narrow window is whole in its tooltip.
    expect(hint()).toHaveAttribute("title", "Drag to change the level")

    act(() => useHintStore.setState({ notice: "Automated by Kick volume" }))
    expect(hint()).toHaveTextContent("Automated by Kick volume")
    expect(hint()).toHaveAttribute("data-notice")
    expect(hint()).toHaveAttribute("role", "status")

    act(() => useHintStore.setState({ notice: null }))
    expect(hint()).toHaveTextContent("Drag to change the level")
    act(() => useHintStore.setState({ text: null }))
  })
})

describe("the latency in the status bar", () => {
  it("shows only the buffer while the project adds no delay", () => {
    render(<StatusBar />)
    expect(engine()).toHaveTextContent("5.3 ms")
    expect(engine()).not.toHaveTextContent("+")
  })

  it("adds what effects and instruments delay the output by", () => {
    const status = useEngineStore.getState().status
    if (!status) throw new Error("no engine status")
    useEngineStore.setState({ status: { ...status, latencyFrames: 240 } })
    render(<StatusBar />)
    expect(engine()).toHaveTextContent("5.3 ms+5.0 ms")
    expect(screen.getByTitle(/delay the output by 240 samples/)).toBeVisible()
  })

  it("follows the project: the engine is asked again after an edit", async () => {
    render(<StatusBar />)
    const [, track] = (
      await dispatch({ type: "addChannel", instrument: "subtractiveSynth" })
    )?.created ?? [0, 0]
    // The synth is 12 samples late: a quarter of a millisecond at 48 kHz.
    await waitFor(() => expect(engine()).toHaveTextContent("+0.3 ms"))
    expect(useEngineStore.getState().status?.latencyFrames).toBe(12)

    await dispatch({ type: "addEffect", track, kind: "limiter" })
    await waitFor(() => expect(engine()).toHaveTextContent("+5.3 ms"))

    await undo()
    await undo()
    await waitFor(() => expect(engine()).not.toHaveTextContent("+"))
  })
})
