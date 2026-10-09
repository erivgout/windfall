import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it } from "vitest"

import type { PluginBinding } from "@/bindings"
import { PluginControls } from "@/features/plugins/controls"
import { SimDocument } from "@/lib/ipc/sim/document"
import { dispatch, redo, undo, useProjectStore } from "@/lib/store/project"
import { startTestApp } from "@/test/harness"

import { SidechainPanel } from "./sidechains"
import { flush, history, project, trackNamed } from "./test-utils"

let app: Awaited<ReturnType<typeof startTestApp>>
beforeEach(async () => {
  app = await startTestApp()
})
afterEach(() => app.stop())

async function fileRoundTrip() {
  const snapshot = await app.backend.documentSnapshot()
  const doc = SimDocument.create(snapshot.project)
  const reopened = SimDocument.open(doc.fileText())
  expect(reopened.project()).toEqual(snapshot.project)
  doc.dispose()
  reopened.dispose()
}

async function openSelector(
  user: ReturnType<typeof userEvent.setup>,
  name: string
) {
  const trigger = await screen.findByRole("combobox", { name })
  await user.click(trigger)
  const listbox = await screen.findByRole("listbox")
  await waitFor(() => {
    expect(trigger).toHaveAttribute("aria-expanded", "true")
    expect(listbox).toBeVisible()
  })
}

function HostedControls({ effect }: { effect: number }) {
  const binding = useProjectStore((state) =>
    state.project.plugins?.find(
      (item) => item.target.type === "effect" && item.target.effect === effect
    )
  )
  return binding ? <PluginControls binding={binding} /> : null
}

describe("mounted detector routing and truthful auxiliary input selection", () => {
  it("adds, bounds gain, removes and changes detector destinations independently of audible sends, with undo and save", async () => {
    const user = userEvent.setup()
    const from = trackNamed("Kick").id
    const clap = trackNamed("Clap").id
    await dispatch({ type: "setSend", from, to: clap, gain: 0.5 })
    const before = structuredClone(project())
    const cursor = history().cursor
    render(<SidechainPanel track={from} />)
    await user.click(
      screen.getByRole("button", { name: "Sidechain · 0 in / 0 out" })
    )
    await openSelector(user, "Add detector-only sidechain")
    expect(screen.queryByRole("option", { name: "Kick" })).toBeNull()
    await user.click(await screen.findByRole("option", { name: "Clap" }))
    await waitFor(() => expect(screen.queryByRole("listbox")).toBeNull())
    await flush()
    expect(trackNamed("Kick").sidechains).toEqual([{ target: clap, gain: 1 }])
    expect(trackNamed("Kick").sends).toEqual(
      before.mixer.tracks.find((track) => track.id === from)!.sends
    )
    const level = screen.getByRole("slider", {
      name: "Sidechain level to Clap",
    })
    fireEvent.keyDown(level, { key: "End" })
    fireEvent.keyUp(level, { key: "End" })
    await flush()
    expect(trackNamed("Kick").sidechains).toEqual([{ target: clap, gain: 2 }])
    fireEvent.keyDown(level, { key: "Home" })
    fireEvent.keyUp(level, { key: "Home" })
    await flush()
    expect(trackNamed("Kick").sidechains).toEqual([{ target: clap, gain: 0 }])
    expect(history().cursor).toBe(cursor + 3)
    await user.click(
      screen.getByRole("button", { name: "Remove sidechain to Clap" })
    )
    await flush()
    expect(trackNamed("Kick").sidechains ?? []).toEqual([])
    await openSelector(user, "Add detector-only sidechain")
    await user.click(await screen.findByRole("option", { name: "Hat" }))
    await waitFor(() => expect(screen.queryByRole("listbox")).toBeNull())
    await flush()
    expect(trackNamed("Kick").sidechains).toEqual([
      { target: trackNamed("Hat").id, gain: 1 },
    ])
    expect(history().cursor).toBe(cursor + 5)
    expect(project().channels).toEqual(before.channels)
    expect(project().patterns).toEqual(before.patterns)
    expect(project().mixer.tracks.filter((track) => track.id !== from)).toEqual(
      before.mixer.tracks.filter((track) => track.id !== from)
    )
    await fileRoundTrip()
    await act(async () => {
      for (let i = 0; i < 5; i++) await undo()
    })
    expect(project()).toEqual(before)
  })

  it("filters cycle-producing detector destinations and native admission rejects cycles, self and Master sources without editing", async () => {
    const user = userEvent.setup()
    const kick = trackNamed("Kick").id
    const clap = trackNamed("Clap").id
    await dispatch({ type: "setSidechain", from: kick, to: clap, gain: 1 })
    render(<SidechainPanel track={clap} />)
    await user.click(
      screen.getByRole("button", { name: "Sidechain · 1 in / 0 out" })
    )
    expect(
      await screen.findByRole("button", {
        name: "Remove detector input from Kick",
      })
    ).toBeVisible()
    await openSelector(user, "Add detector-only sidechain")
    expect(
      await screen.findByRole("option", { name: "Master" })
    ).toBeInTheDocument()
    expect(screen.queryByRole("option", { name: "Kick" })).toBeNull()
    expect(screen.queryByRole("option", { name: "Clap" })).toBeNull()
    const before = structuredClone(project())
    const cursor = history().cursor
    for (const [from, to] of [
      [clap, kick],
      [clap, clap],
      [0, clap],
    ]) {
      await expect(
        app.backend.dispatch({ type: "setSidechain", from, to, gain: 1 })
      ).rejects.toThrow()
    }
    expect(project()).toEqual(before)
    expect(history().cursor).toBe(cursor)
    await user.keyboard("{Escape}")
    await waitFor(() => expect(screen.queryByRole("listbox")).toBeNull())
    await user.click(
      screen.getByRole("button", { name: "Remove detector input from Kick" })
    )
    await flush()
    expect(trackNamed("Kick").sidechains ?? []).toEqual([])
    expect(history().cursor).toBe(cursor + 1)
    await fileRoundTrip()
    await act(async () => {
      await undo()
    })
    expect(project()).toEqual(before)
  })

  it.each(["clap", "vst3"])(
    "uses %s native fixture input index 1/Detector/Mono, retains unavailable selection visibly and clears stale ownership",
    async (format) => {
      const user = userEvent.setup()
      // Mirrors the real desktop bridge's Mono Detector fixture, not invented ports.
      const binding: PluginBinding = {
        target: { type: "effect", effect: 0 },
        format,
        path: `/plugins/detector.${format}`,
        id: "native.detector",
        name: "Native detector",
        state: [1, 2, 3],
        parameters: [],
        // A saved port can disappear after a plugin update; keep it visible.
        sidechainInput: 7,
        auxiliaryInputs: [{ index: 1, name: "Detector", channels: 1 }],
      }
      await dispatch({
        type: "addPluginEffect",
        track: trackNamed("Clap").id,
        plugin: binding,
      })
      const added = project().plugins!.find(
        (item) => item.target.type === "effect"
      )!
      if (added.target.type !== "effect")
        throw new Error("Missing effect binding")
      const effect = added.target.effect
      render(<HostedControls effect={effect} />)
      expect(
        screen.getByRole("combobox", { name: "Plugin sidechain input" })
      ).toHaveTextContent("Saved input 8 (unavailable)")
      const before = structuredClone(project())
      const cursor = history().cursor
      await openSelector(user, "Plugin sidechain input")
      expect(
        await screen.findByRole("option", {
          name: "Saved input 8 (unavailable)",
        })
      ).toHaveAttribute("aria-disabled", "true")
      await user.click(
        await screen.findByRole("option", { name: "Detector · Mono" })
      )
      await waitFor(() => expect(screen.queryByRole("listbox")).toBeNull())
      await flush()
      expect(project().plugins![0].sidechainInput).toBe(1)
      expect(project().plugins![0].state).toEqual(before.plugins![0].state)
      expect(project().mixer).toEqual(before.mixer)
      expect(history().cursor).toBe(cursor + 1)
      await fileRoundTrip()
      await act(async () => {
        await undo()
      })
      expect(project()).toEqual(before)
      await act(async () => {
        await redo()
      })
      await flush()
      const valid = structuredClone(project())
      const validCursor = history().cursor
      await expect(
        app.backend.dispatch({
          type: "setPluginSidechainInput",
          target: added.target,
          input: 7,
        })
      ).rejects.toThrow("no such auxiliary input")
      expect(project()).toEqual(valid)
      expect(history().cursor).toBe(validCursor)
      expect(
        screen.getByRole("combobox", { name: "Plugin sidechain input" })
      ).toHaveTextContent("Detector · Mono")
      await openSelector(user, "Plugin sidechain input")
      expect(
        screen.queryByRole("option", { name: "Saved input 8 (unavailable)" })
      ).toBeNull()
      await user.click(
        await screen.findByRole("option", { name: "First auxiliary input" })
      )
      await waitFor(() => expect(screen.queryByRole("listbox")).toBeNull())
      await flush()
      expect(project().plugins![0].sidechainInput).toBeUndefined()
      await openSelector(user, "Plugin sidechain input")
      expect(
        await screen.findByRole("option", { name: "Detector · Mono" })
      ).toBeVisible()
      await act(async () => {
        await dispatch({
          type: "removeEffect",
          track: trackNamed("Clap").id,
          effect,
        })
      })
      expect(
        screen.queryByRole("combobox", { name: "Plugin sidechain input" })
      ).toBeNull()
      expect(
        screen.queryByRole("option", { name: "Detector · Mono" })
      ).toBeNull()
      const removed = structuredClone(project())
      const removedCursor = history().cursor
      await expect(
        app.backend.dispatch({
          type: "setPluginSidechainInput",
          target: added.target,
          input: 1,
        })
      ).rejects.toThrow()
      expect(project()).toEqual(removed)
      expect(history().cursor).toBe(removedCursor)
    }
  )
})
