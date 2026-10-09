import { act, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it } from "vitest"

import { runAction } from "@/lib/actions"
import { SimDocument } from "@/lib/ipc/sim/document"
import { dispatch, redo, undo } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { settle } from "@/test/harness"

import {
  assignChannelGroups,
  closeChannelGroups,
  openChannelGroups,
  useChannelGroups,
} from "./channel-groups"
import ChannelRackPanel from "./index"
import { useRackStore } from "./rack-store"
import { channel, history, project, startRack } from "./test-utils"

let app: Awaited<ReturnType<typeof startRack>>
beforeEach(async () => {
  app = await startRack()
})
afterEach(() => {
  closeChannelGroups()
  app.stop()
})

describe("rack workflow acceptance with the shared Rust WASM", () => {
  it("shows readable group names and counts instead of internal select values", async () => {
    render(<ChannelRackPanel />)
    expect(
      screen.getByRole("combobox", { name: "Channel group filter" })
    ).toHaveTextContent("All channels (4)")
    await act(async () => {
      openChannelGroups()
      await settle()
    })
    expect(
      screen.getByRole("combobox", { name: "Target channel group" })
    ).toHaveTextContent("Ungrouped")
    expect(
      screen.getByRole("combobox", { name: "Target channel group" })
    ).not.toHaveTextContent("ungrouped:")
    await act(async () => {
      closeChannelGroups()
      await dispatch({
        type: "setChannelGroup",
        channels: [channel("Kick").id],
        group: "Drums",
      })
      useRackStore.getState().setGroupFilter("Drums")
      await settle()
    })
    expect(
      screen.getByRole("combobox", { name: "Channel group filter" })
    ).toHaveTextContent("Drums (1)")
    await act(async () => {
      openChannelGroups()
      await settle()
    })
    expect(
      screen.getByRole("combobox", { name: "Target channel group" })
    ).toHaveTextContent("Drums")
    expect(
      screen.getByRole("combobox", { name: "Target channel group" })
    ).not.toHaveTextContent("group:")
  })
  it("opens original step notes in the piano roll without edits, lost properties, duplicates, or history", async () => {
    const id = channel("Kick").id
    useUiStore.getState().selectChannel(id)
    const before = structuredClone(project())
    const cursor = history().cursor
    await runAction("channel.sendStepsToPianoRoll")
    expect(useUiStore.getState().centerTab).toBe("pianoRoll")
    expect(Object.values(useRackStore.getState().noteViews)).toEqual([
      expect.objectContaining({
        view: "notes",
        lane: expect.objectContaining({ channel: id }),
      }),
    ])
    expect(project()).toEqual(before)
    expect(history().cursor).toBe(cursor)
  })

  it("mounts only group members, saves membership, merges and removes groups with one-step history", async () => {
    const kick = channel("Kick").id
    const clap = channel("Clap").id
    const before = structuredClone(project())
    await dispatch({
      type: "setChannelGroup",
      channels: [kick, clap, kick],
      group: "Drums 🥁",
    })
    expect(history().cursor).toBe(1)
    const grouped = structuredClone(project())
    expect(
      grouped.channels
        .filter((item) => item.group === "Drums 🥁")
        .map((item) => item.id)
    ).toEqual([kick, clap])
    for (const item of grouped.channels) {
      const prior = before.channels.find((value) => value.id === item.id)!
      expect(
        Object.fromEntries(
          Object.entries(item).filter(([key]) => key !== "group")
        )
      ).toEqual(
        Object.fromEntries(
          Object.entries(prior).filter(([key]) => key !== "group")
        )
      )
    }
    render(<ChannelRackPanel />)
    await act(async () => {
      useRackStore.getState().setGroupFilter("Drums 🥁")
      await settle()
    })
    expect(
      screen.getByRole("group", { name: "Kick steps" })
    ).toBeInTheDocument()
    expect(
      screen.getByRole("group", { name: "Clap steps" })
    ).toBeInTheDocument()
    expect(screen.queryByRole("group", { name: "Hat steps" })).toBeNull()
    const saved = SimDocument.create(
      (await app.backend.documentSnapshot()).project
    )
    const reopened = SimDocument.open(saved.fileText())
    expect(reopened.project().channels).toEqual(grouped.channels)
    saved.dispose()
    reopened.dispose()
    await act(async () => {
      await undo()
      await settle()
    })
    expect(project()).toEqual(before)
    await act(async () => {
      await redo()
      await settle()
    })
    expect(project()).toEqual(grouped)
    await act(async () => {
      await dispatch({
        type: "setChannelGroup",
        channels: [channel("Hat").id],
        group: "Merged",
      })
      await dispatch({
        type: "renameChannelGroup",
        name: "Drums 🥁",
        newName: "Merged",
      })
    })
    expect(
      project().channels.filter((item) => item.group === "Merged")
    ).toHaveLength(3)
    const merged = structuredClone(project())
    await act(async () => {
      await dispatch({ type: "removeChannelGroup", name: "Merged" })
    })
    expect(project().channels.every((item) => !item.group)).toBe(true)
    await act(async () => {
      await undo()
    })
    expect(project()).toEqual(merged)
  })

  it("rejects a stale manager review without changing membership or history", async () => {
    openChannelGroups()
    const captured = useChannelGroups.getState().request!
    await dispatch({
      type: "updateChannel",
      id: channel("Kick").id,
      patch: { volume: 0.25 },
    })
    const before = structuredClone(project())
    const cursor = history().cursor
    expect(
      await assignChannelGroups(captured, [channel("Kick").id], "Stale")
    ).toBe(false)
    expect(project()).toEqual(before)
    expect(history().cursor).toBe(cursor)
  })

  it("rejects out-of-scope ids and names beyond the native UTF-8 limit atomically", async () => {
    const before = structuredClone(project())
    const cursor = history().cursor
    await expect(
      app.backend.dispatch({
        type: "setChannelGroup",
        channels: [channel("Kick").id, 999999],
        group: "QA",
      })
    ).rejects.toThrow()
    await expect(
      app.backend.dispatch({
        type: "setChannelGroup",
        channels: [channel("Kick").id],
        group: "🦊".repeat(33),
      })
    ).rejects.toThrow()
    expect(project()).toEqual(before)
    expect(history().cursor).toBe(cursor)
    await dispatch({
      type: "setChannelGroup",
      channels: [channel("Kick").id],
      group: "🦊".repeat(32),
    })
    expect(channel("Kick").group).toBe("🦊".repeat(32))
  })
})
