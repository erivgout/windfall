import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { ChannelId, NoteInit } from "@/bindings"
import { TooltipProvider } from "@/components/ui/tooltip"
import { CommandPalette } from "@/features/palette/command-palette"
import PianoRollPanel from "@/features/piano-roll"
import { currentSession } from "@/features/piano-roll/session"
import {
  actionForEvent,
  disabledReason,
  getAppState,
  isChecked,
  isEnabled,
  registry,
  runAction,
  shortcutLabel,
} from "@/lib/actions"
import { sourceSample } from "@/lib/channel-source"
import { SAMPLE_DRAG_TYPE } from "@/lib/dnd"
import { useHintStore } from "@/lib/store/hint"
import { buildProject, emptyProject } from "@/lib/ipc/sim/project"
import { dispatch, redo, undo, useProjectStore } from "@/lib/store/project"
import { getProjectGeneration } from "@/lib/store/replaced"
import { setTransportPattern, useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"
import { applyUiScale } from "@/lib/ui-scale"
import { settle } from "@/test/harness"

import ChannelRackPanel from "./index"
import { LEFT_WIDTH, PITCH_VAR, STEPS_INSET } from "./layout"
import { NOTE_VIEW_ACTION_IDS, OPEN_NOTE_PREVIEW_ACTION } from "./actions"
import {
  notePreviewActionForTarget,
  runNotePreviewAction,
  type NotePreviewTarget,
} from "./note-preview-target"
import { useRackStore } from "./rack-store"
import {
  channel,
  dragData,
  history,
  notesOf,
  project,
  startRack,
  stepButtons,
  stepGrid,
} from "./test-utils"

const renders = vi.hoisted(() => new Map<number, number>())
vi.mock("@/lib/store/selectors", async (original) => {
  const actual = await original<typeof import("@/lib/store/selectors")>()
  return {
    ...actual,
    useChannel(id: ChannelId | null) {
      if (id !== null) renders.set(id, (renders.get(id) ?? 0) + 1)
      return actual.useChannel(id)
    },
  }
})
vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

function fixture(
  notes: NoteInit[] = [
    { start: 120, length: 480, key: 60 },
    { start: 120, length: 240, key: 72 },
    { start: 500, length: 960, key: 67 },
    { start: 3600, length: 480, key: 64, velocity: 0 },
    { start: 4800, length: 240, key: 60 },
  ]
) {
  const base = emptyProject("Rack preview")
  return buildProject(base, (run) => {
    const [drum] = run({ type: "addChannel", name: "Drum" })
    const [lead] = run({ type: "addChannel", name: "Lead" })
    run({
      type: "toggleStep",
      pattern: base.patterns[0].id,
      channel: drum,
      step: 4,
    })
    if (notes.length)
      run({
        type: "addNotes",
        pattern: base.patterns[0].id,
        channel: lead,
        notes,
      })
  })
}

let app: Awaited<ReturnType<typeof startRack>>
beforeEach(async () => {
  app = await startRack({ project: fixture() })
})
afterEach(() => {
  app.stop()
  applyUiScale(100)
  vi.restoreAllMocks()
})

function preview(name = "Lead") {
  return screen.getByRole("button", {
    name: `${name} note preview. Open in piano roll`,
  })
}
function shape(name = "Lead") {
  return preview(name).querySelector("path")!.getAttribute("d")!
}
function rectangles(name = "Lead") {
  return [
    ...shape(name).matchAll(/M([\d.]+) ([\d.]+)h([\d.]+)v([\d.]+)h-[\d.]+Z/g),
  ].map((match) => match.slice(1).map(Number))
}
async function view(name: string, label: string) {
  await userEvent.click(
    screen.getByRole("button", { name: `${name} row view` })
  )
  await userEvent.click(
    await screen.findByRole(
      label === "Open in piano roll" ? "menuitem" : "menuitemcheckbox",
      { name: new RegExp(`^${label}`) }
    )
  )
  await waitFor(() =>
    expect(screen.queryByRole("menu")).not.toBeInTheDocument()
  )
}
async function edit(notes: NoteInit[], name = "Lead") {
  await act(async () => {
    const old = notesOf(name)
    await dispatch({
      type: "batch",
      label: "Replace lane",
      commands: [
        {
          type: "removeNotes",
          pattern: project().patterns[0].id,
          channel: channel(name).id,
          notes: old.map((note) => note.id),
        },
        {
          type: "addNotes",
          pattern: project().patterns[0].id,
          channel: channel(name).id,
          notes,
        },
      ],
    })
  })
}
function Editors() {
  const tab = useUiStore((state) => state.centerTab)
  return (
    <TooltipProvider>
      {tab === "pianoRoll" ? <PianoRollPanel /> : <ChannelRackPanel />}
    </TooltipProvider>
  )
}

describe("the rack note preview backed by the Rust WASM document", () => {
  it("discovers canonical row commands in the real palette with current-lane disabled reasons and shortcuts", async () => {
    render(
      <>
        <ChannelRackPanel />
        <CommandPalette />
      </>
    )
    const before = useProjectStore.getState()
    const sent = vi.spyOn(app.backend, "dispatch")
    await userEvent.keyboard("{Control>}k{/Control}")
    for (const id of [...NOTE_VIEW_ACTION_IDS, OPEN_NOTE_PREVIEW_ACTION]) {
      const action = registry.get(id)!
      const option = screen.getByRole("option", {
        name: new RegExp(`^${action.title}`),
      })
      expect(option).toHaveAttribute("aria-disabled", "true")
      expect(option).toHaveTextContent("Select a channel in a pattern")
      expect(action.scope).toBe("channelRack")
    }
    await act(async () =>
      useUiStore.getState().selectChannel(channel("Drum").id)
    )
    const notes = screen.getByRole("option", { name: /^Show notes/ })
    expect(notes).not.toHaveAttribute("aria-disabled", "true")
    expect(
      within(notes).getByText(shortcutLabel(NOTE_VIEW_ACTION_IDS[2])!)
    ).toBeVisible()
    expect(
      screen.getByRole("option", { name: /^Open in piano roll/ })
    ).toHaveTextContent("Alt+3")
    await act(async () => useUiStore.getState().setKeymap("fl"))
    expect(
      screen.getByRole("option", { name: /^Open in piano roll/ })
    ).toHaveTextContent("F7")
    await userEvent.click(notes)
    expect(preview("Drum")).toBeInTheDocument()
    await waitFor(() => expect(preview("Drum")).toHaveFocus())
    expect(rectangles("Drum")[0][0]).toBe(80)
    expect(useUiStore.getState().selectedChannel).toBe(channel("Drum").id)
    expect(useProjectStore.getState().project).toBe(before.project)
    expect(useProjectStore.getState().history).toBe(before.history)
    expect(useProjectStore.getState().dirty).toBe(false)
    expect(sent).not.toHaveBeenCalled()
  })

  it("uses scoped registry shortcuts and invalidates current-lane checked state", async () => {
    render(<ChannelRackPanel />)
    await act(async () =>
      useUiStore.getState().selectChannel(channel("Lead").id)
    )
    preview().focus()
    const before = useProjectStore.getState()
    const sent = vi.spyOn(app.backend, "dispatch")
    expect(
      actionForEvent(
        new KeyboardEvent("keydown", { key: "2", ctrlKey: true, altKey: true })
      )?.id
    ).toBe(NOTE_VIEW_ACTION_IDS[1])
    const version = registry.stateVersion()
    await userEvent.keyboard("{Control>}{Alt>}2{/Alt}{/Control}")
    expect(stepButtons("Lead")).toHaveLength(16)
    await waitFor(() => expect(stepButtons("Lead")[0]).toHaveFocus())
    expect(registry.stateVersion()).toBeGreaterThan(version)
    expect(
      isChecked(registry.get(NOTE_VIEW_ACTION_IDS[1])!, getAppState())
    ).toBe(true)
    stepButtons("Lead")[0].focus()
    await userEvent.keyboard("{Control>}{Alt>}3{/Alt}{/Control}")
    expect(preview()).toBeInTheDocument()
    await waitFor(() => expect(preview()).toHaveFocus())
    await userEvent.click(screen.getByRole("button", { name: "Lead row view" }))
    const notes = await screen.findByRole("menuitemcheckbox", {
      name: /^Show notes/,
    })
    expect(notes).toHaveAttribute("aria-checked", "true")
    expect(notes).toHaveTextContent(shortcutLabel(NOTE_VIEW_ACTION_IDS[2])!)
    await userEvent.keyboard("{Escape}")
    await act(async () => useUiStore.getState().showCenterTab("pianoRoll"))
    expect(
      actionForEvent(
        new KeyboardEvent("keydown", { key: "2", ctrlKey: true, altKey: true })
      )
    ).toBeUndefined()
    expect(useProjectStore.getState().project).toBe(before.project)
    expect(useProjectStore.getState().history).toBe(before.history)
    expect(sent).not.toHaveBeenCalled()
  })

  it("owns the innermost thumbnail context menu with captured metadata and no primary opening", async () => {
    render(<ChannelRackPanel />)
    await act(async () =>
      useUiStore.getState().selectChannel(channel("Drum").id)
    )
    const before = useProjectStore.getState()
    const sent = vi.spyOn(app.backend, "dispatch")
    const transport = vi.spyOn(app.backend, "transportToggle")
    fireEvent.pointerDown(preview(), { button: 2, pointerId: 1 })
    fireEvent.contextMenu(preview(), { button: 2 })
    // Keep the rack's existing pointer-down row selection; opening the menu
    // itself never navigates. Subsequent selection must not retarget it.
    expect(useUiStore.getState().selectedChannel).toBe(channel("Lead").id)
    await act(async () =>
      useUiStore.getState().selectChannel(channel("Drum").id)
    )
    expect(
      await screen.findByRole("menuitemcheckbox", {
        name: /^Automatic steps or notes/,
      })
    ).toHaveAttribute("aria-checked", "true")
    expect(screen.getAllByRole("menu")).toHaveLength(1)
    expect(
      screen.queryByRole("menuitem", { name: /Add channel/ })
    ).not.toBeInTheDocument()
    expect(
      screen.getByRole("menuitem", { name: /^Open in piano roll/ })
    ).toHaveTextContent(shortcutLabel("view.pianoRoll")!)
    expect(useUiStore.getState().selectedChannel).toBe(channel("Lead").id)
    expect(useUiStore.getState().centerTab).toBe("channelRack")
    expect(transport).not.toHaveBeenCalled()
    await userEvent.click(
      screen.getByRole("menuitemcheckbox", { name: /^Show steps/ })
    )
    await waitFor(() => expect(stepButtons("Lead")[0]).toHaveFocus())
    expect(useUiStore.getState().selectedChannel).toBe(channel("Lead").id)
    expect(stepButtons("Drum")).toHaveLength(16)
    expect(useProjectStore.getState().project).toBe(before.project)
    expect(useProjectStore.getState().history).toBe(before.history)
    expect(useProjectStore.getState().dirty).toBe(false)
    expect(sent).not.toHaveBeenCalled()
  })

  it.each(["click", "Enter"])(
    "opens the captured context lane using %s after close with existing piano session focus",
    async (method) => {
      render(<Editors />)
      await act(async () =>
        useUiStore.getState().selectChannel(channel("Drum").id)
      )
      const before = useProjectStore.getState()
      const sent = vi.spyOn(app.backend, "dispatch")
      fireEvent.contextMenu(preview())
      const opening = await screen.findByRole("menuitem", {
        name: /^Open in piano roll/,
      })
      if (method === "click") await userEvent.click(opening)
      else {
        opening.focus()
        await userEvent.keyboard("{Enter}")
      }
      await waitFor(() =>
        expect(
          screen.getByRole("application", { name: "Note grid" })
        ).toHaveFocus()
      )
      expect(currentSession()?.editing).toEqual({
        patternId: project().patterns[0].id,
        channelId: channel("Lead").id,
      })
      expect(useProjectStore.getState().project).toBe(before.project)
      expect(useProjectStore.getState().history).toBe(before.history)
      expect(sent).not.toHaveBeenCalled()
    }
  )

  it("keeps row checked state and execution bound when another lane is selected", async () => {
    render(<ChannelRackPanel />)
    await view("Lead", "Show notes")
    await act(async () =>
      useUiStore.getState().selectChannel(channel("Drum").id)
    )
    await userEvent.click(screen.getByRole("button", { name: "Lead row view" }))
    expect(
      screen.getByRole("menuitemcheckbox", { name: /^Show notes/ })
    ).toHaveAttribute("aria-checked", "true")
    expect(
      screen.getByRole("menuitemcheckbox", {
        name: /^Automatic steps or notes/,
      })
    ).toHaveAttribute("aria-checked", "false")
    await userEvent.click(
      screen.getByRole("menuitemcheckbox", { name: /^Show steps/ })
    )
    expect(stepButtons("Lead")).toHaveLength(16)
    expect(stepButtons("Drum")).toHaveLength(16)
    expect(useUiStore.getState().selectedChannel).toBe(channel("Lead").id)
  })

  it.each(["pattern", "New", "Open", "selection"] as const)(
    "does not restore pending view focus into a successor %s",
    async (successor) => {
      render(<ChannelRackPanel />)
      await app.backend.projectSave("/Focus.windfall")
      await act(async () =>
        useUiStore.getState().selectChannel(channel("Lead").id)
      )
      let focusFrame: FrameRequestCallback | undefined
      const actualFrame = globalThis.requestAnimationFrame
      // Control timing only. The command, project replacement and mirror are
      // real; this checks the window between a view command and its UI frame.
      const frameSpy = vi
        .spyOn(globalThis, "requestAnimationFrame")
        .mockImplementation((callback) => {
          if (!focusFrame) {
            focusFrame = callback
            return 0
          }
          return actualFrame(callback)
        })
      await act(async () => {
        await runAction(NOTE_VIEW_ACTION_IDS[1])
      })
      expect(stepButtons("Lead")).toHaveLength(16)
      expect(focusFrame).toBeDefined()
      frameSpy.mockRestore()
      await act(async () => {
        if (successor === "pattern") {
          const added = await dispatch({ type: "addPattern" })
          await setTransportPattern(added!.created[0])
        } else if (successor === "New") await app.backend.projectNew()
        else if (successor === "Open")
          await app.backend.projectOpen("/Focus.windfall")
        else useUiStore.getState().selectChannel(channel("Drum").id)
      })
      const state = useProjectStore.getState()
      const selected = useUiStore.getState().selectedChannel
      const focused = document.activeElement
      const sent = vi.spyOn(app.backend, "dispatch")
      await act(async () => {
        focusFrame!(0)
      })
      expect(useUiStore.getState().selectedChannel).toBe(selected)
      expect(document.activeElement).toBe(focused)
      expect(useProjectStore.getState().project).toBe(state.project)
      expect(useProjectStore.getState().history).toBe(state.history)
      expect(sent).not.toHaveBeenCalled()
    }
  )

  it.each(["pattern", "New", "Open", "delete"] as const)(
    "rejects captured registry metadata and stale context items after %s",
    async (replacement) => {
      render(<ChannelRackPanel />)
      await app.backend.projectSave("/Captured.windfall")
      const target: NotePreviewTarget = {
        pattern: project().patterns[0].id,
        channel: channel("Lead").id,
        generation: getProjectGeneration(),
      }
      const canonical = registry.get(NOTE_VIEW_ACTION_IDS[1])!
      const bound = notePreviewActionForTarget(canonical, target)
      const opening = registry.get(OPEN_NOTE_PREVIEW_ACTION)!
      fireEvent.contextMenu(preview())
      const stale = await screen.findByRole("menuitemcheckbox", {
        name: /^Show steps/,
      })
      await act(async () => {
        if (replacement === "pattern") {
          const added = await dispatch({ type: "addPattern" })
          await setTransportPattern(added!.created[0])
        } else if (replacement === "New") await app.backend.projectNew()
        else if (replacement === "Open")
          await app.backend.projectOpen("/Captured.windfall")
        else await dispatch({ type: "removeChannel", id: target.channel })
      })
      expect(screen.queryByRole("menu")).not.toBeInTheDocument()
      const state = useProjectStore.getState()
      const selected = useUiStore.getState().selectedChannel
      const sent = vi.spyOn(app.backend, "dispatch")
      expect(isEnabled(bound, getAppState())).toBe(false)
      expect(disabledReason(bound, getAppState())).toBe(
        replacement === "pattern"
          ? "Pattern changed"
          : replacement === "delete"
            ? "Channel no longer exists"
            : "Project changed"
      )
      expect(await runNotePreviewAction(target, canonical)).toBe(false)
      expect(await runNotePreviewAction(target, opening)).toBe(false)
      fireEvent.click(stale)
      expect(useUiStore.getState().selectedChannel).toBe(selected)
      expect(useUiStore.getState().centerTab).toBe("channelRack")
      expect(useRackStore.getState().noteViews).toEqual({})
      expect(useProjectStore.getState().project).toBe(state.project)
      expect(useProjectStore.getState().history).toBe(state.history)
      expect(useProjectStore.getState().dirty).toBe(state.dirty)
      expect(sent).not.toHaveBeenCalled()
    }
  )

  it("replaces only non-step lanes with real timing, duration, pitch, chord and overlap rectangles", () => {
    render(<ChannelRackPanel />)
    expect(stepButtons("Drum")).toHaveLength(16)
    expect(
      screen.queryByRole("group", { name: "Lead steps" })
    ).not.toBeInTheDocument()
    const rects = rectangles()
    expect(rects).toHaveLength(4)
    // The pattern spans 3840 ticks / 320 units: these expectations are
    // independently calculated from the actual commanded notes.
    expect(rects[0][0]).toBe(10)
    expect(rects[0][2]).toBe(40)
    expect(rects[1][0]).toBe(10)
    expect(rects[1][2]).toBe(20)
    expect(rects[1][1]).toBeLessThan(rects[0][1])
    expect(rects[2][0]).toBeCloseTo(500 / 12, 3)
    expect(rects[2][2]).toBe(80)
    expect(rects[2][1]).toBeLessThan(rects[0][1])
    expect(rects[2][1]).toBeGreaterThan(rects[1][1])
    // The silent final note still draws, clipped exactly at pattern end.
    expect(rects[3][0]).toBe(300)
    expect(rects[3][2]).toBe(20)
    expect(preview()).toHaveTextContent(
      "4 notes in the pattern; 1 outside the preview"
    )
    expect(notesOf("Lead")).toHaveLength(5)
  })

  it.each([
    ["off-grid", [{ start: 1, length: 240, key: 60 }]],
    ["pitch", [{ start: 0, length: 240, key: 61 }]],
    ["duration", [{ start: 0, length: 480, key: 60 }]],
    [
      "stacked steps",
      [
        { start: 0, length: 240, key: 60 },
        { start: 0, length: 240, key: 60 },
      ],
    ],
    ["outside the pattern", [{ start: 4800, length: 240, key: 72 }]],
  ] satisfies [string, NoteInit[]][])(
    "automatically detects %s notes",
    async (_, notes) => {
      await edit(notes)
      render(<ChannelRackPanel />)
      expect(preview()).toBeInTheDocument()
    }
  )

  it("switches real Steps/Notes/Automatic controls without editing or saving view choices", async () => {
    render(<ChannelRackPanel />)
    const before = useProjectStore.getState()
    const sent = vi.spyOn(app.backend, "dispatch")
    await view("Lead", "Show steps")
    expect(stepButtons("Lead")).toHaveLength(16)
    await view("Drum", "Show notes")
    expect(rectangles("Drum")[0][0]).toBe(80)
    await view("Lead", "Automatic steps or notes")
    expect(preview()).toBeInTheDocument()
    expect(useProjectStore.getState().project).toBe(before.project)
    expect(useProjectStore.getState().history).toBe(before.history)
    expect(useProjectStore.getState().dirty).toBe(false)
    expect(sent).not.toHaveBeenCalled()
    expect(localStorage.getItem("windfall.rack")).not.toContain("noteViews")
  })

  it("reacts to edit, undo and redo, preserving step-note identity", async () => {
    render(<ChannelRackPanel />)
    const before = shape()
    const initialIds = notesOf("Lead").map((note) => note.id)
    await edit([{ start: 0, length: 240, key: 60 }])
    expect(stepButtons("Lead")[0]).toHaveAttribute("aria-pressed", "true")
    await act(async () => {
      await undo()
    })
    expect(shape()).toBe(before)
    expect(notesOf("Lead").map((note) => note.id)).toEqual(initialIds)
    await act(async () => {
      await redo()
    })
    await view("Lead", "Show notes")
    expect(rectangles()).toHaveLength(1)
    expect(notesOf("Lead")[0]).toMatchObject({ start: 0, length: 240, key: 60 })
    expect(history().entries).toHaveLength(1)
  })

  it("shows empty notes explicitly and updates pitch extremes/zero velocity through checked commands", async () => {
    render(<ChannelRackPanel />)
    await edit([])
    await view("Lead", "Show notes")
    expect(shape()).toBe("")
    expect(preview()).toHaveTextContent("No notes in this pattern")
    await edit([
      { start: 0, length: 1, key: 0, velocity: 0 },
      { start: 3839, length: 240000, key: 127 },
    ])
    const rects = rectangles()
    expect(rects).toHaveLength(2)
    expect(rects[1][1]).toBeLessThan(rects[0][1])
    expect(rects.every((rect) => rect.every(Number.isFinite))).toBe(true)
    expect(rects[1][0] + rects[1][2]).toBeCloseTo(320, 3)
  })

  it.each(["click", "Enter", "menu"])(
    "opens the exact current lane using %s and focuses the existing piano grid with no history",
    async (method) => {
      const second = await dispatch({ type: "addPattern", name: "Second" })
      await dispatch({
        type: "addNotes",
        pattern: second!.created[0],
        channel: channel("Lead").id,
        notes: [{ start: 500, length: 480, key: 70 }],
      })
      await setTransportPattern(second!.created[0])
      render(<Editors />)
      const before = useProjectStore.getState()
      const sent = vi.spyOn(app.backend, "dispatch")
      const transport = vi.spyOn(app.backend, "transportSet")
      if (method === "click") await userEvent.click(preview())
      else if (method === "Enter") {
        preview().focus()
        await userEvent.keyboard("{Enter}")
      } else await view("Lead", "Open in piano roll")
      await waitFor(() =>
        expect(
          screen.getByRole("application", { name: "Note grid" })
        ).toHaveFocus()
      )
      expect(currentSession()?.editing).toEqual({
        patternId: second!.created[0],
        channelId: channel("Lead").id,
      })
      expect(currentSession()?.editor.notes).toMatchObject([
        { start: 500, length: 480, key: 70 },
      ])
      expect(useProjectStore.getState().project).toBe(before.project)
      expect(useProjectStore.getState().history).toBe(before.history)
      expect(useProjectStore.getState().dirty).toBe(before.dirty)
      expect(sent).not.toHaveBeenCalled()
      expect(transport).not.toHaveBeenCalled()
    }
  )

  it("hands arrow focus between names, thumbnails and step rows; Space remains transport", async () => {
    render(<ChannelRackPanel />)
    const leadName = screen.getByRole("button", { name: "Lead, no sample" })
    leadName.focus()
    await userEvent.keyboard("{ArrowRight}")
    expect(preview()).toHaveFocus()
    expect(useHintStore.getState().text).toContain("Note preview in rack")
    await userEvent.keyboard("{ArrowLeft}")
    expect(leadName).toHaveFocus()
    preview().focus()
    await userEvent.keyboard("{ArrowUp}")
    expect(stepButtons("Drum")[0]).toHaveFocus()
    await userEvent.keyboard("{ArrowDown}")
    expect(preview()).toHaveFocus()
    const sent = vi.spyOn(app.backend, "dispatch")
    const play = vi.spyOn(app.backend, "transportToggle")
    await userEvent.keyboard(" ")
    expect(play).toHaveBeenCalledTimes(1)
    expect(sent).not.toHaveBeenCalled()
    expect(useUiStore.getState().centerTab).toBe("channelRack")
    fireEvent.keyDown(preview(), { key: "Enter", repeat: true })
    expect(useUiStore.getState().centerTab).toBe("channelRack")
  })

  it("tracks pattern changes and keeps an explicit choice only for its own lane", async () => {
    render(<ChannelRackPanel />)
    await view("Lead", "Show steps")
    let id = 0
    await act(async () => {
      const added = await dispatch({ type: "addPattern" })
      id = added!.created[0]
      await dispatch({
        type: "addNotes",
        pattern: id,
        channel: channel("Lead").id,
        notes: [{ start: 960, length: 240, key: 75 }],
      })
      await setTransportPattern(id)
    })
    expect(rectangles()[0][0]).toBe(80)
    await act(async () => {
      await setTransportPattern(project().patterns[0].id)
    })
    expect(stepGrid("Lead")).toBeInTheDocument()
    await act(async () => {
      await setTransportPattern(id)
    })
    expect(rectangles()[0][0]).toBe(80)
  })

  it("save/reopen and New reset view preferences and pressed targets even when numeric ids repeat", async () => {
    render(<ChannelRackPanel />)
    const initial = shape()
    const ids = project().channels.map((item) => item.id)
    await app.backend.projectSave("/Rack preview.windfall")
    await view("Lead", "Show steps")
    const generation = getProjectGeneration()
    await act(async () => {
      await app.backend.projectOpen("/Rack preview.windfall")
    })
    expect(getProjectGeneration()).toBeGreaterThan(generation)
    expect(project().channels.map((item) => item.id)).toEqual(ids)
    expect(useRackStore.getState().noteViews).toEqual({})
    expect(shape()).toBe(initial)
    const old = preview()
    fireEvent.pointerDown(old, { pointerId: 1, button: 0 })
    await act(async () => {
      await app.backend.projectNew()
    })
    fireEvent.pointerUp(old, { pointerId: 1 })
    fireEvent.click(old, { detail: 1 })
    expect(useUiStore.getState().centerTab).toBe("channelRack")
    expect(useRackStore.getState().noteViews).toEqual({})
    expect(
      screen.queryByRole("button", {
        name: "Lead note preview. Open in piano roll",
      })
    ).not.toBeInTheDocument()
    await act(async () => {
      await app.backend.projectOpen("/Rack preview.windfall")
    })
    expect(shape()).toBe(initial)
    expect(useProjectStore.getState().dirty).toBe(false)
  })

  it("forgets deleted channel/pattern choices, closes replaced menus and refuses cancelled/stale presses", async () => {
    render(<ChannelRackPanel />)
    const old = preview()
    fireEvent.pointerDown(old, { pointerId: 1, button: 0 })
    fireEvent.pointerCancel(old, { pointerId: 1 })
    fireEvent.click(old, { detail: 1 })
    expect(useUiStore.getState().centerTab).toBe("channelRack")
    await view("Lead", "Show notes")
    const id = channel("Lead").id
    await act(async () => {
      await dispatch({ type: "removeChannel", id })
    })
    expect(useRackStore.getState().noteViews).toEqual({})
    await act(async () => {
      await undo()
    })
    expect(preview()).toBeInTheDocument()
    await view("Lead", "Show steps")
    await app.backend.projectSave("/Saved.windfall")
    await userEvent.click(screen.getByRole("button", { name: "Lead row view" }))
    const item = screen.getByRole("menuitem", { name: /^Open in piano roll/ })
    await act(async () => {
      await app.backend.projectOpen("/Saved.windfall")
    })
    expect(screen.queryByRole("menu")).not.toBeInTheDocument()
    fireEvent.click(item)
    expect(useUiStore.getState().centerTab).toBe("channelRack")
    const second = await dispatch({ type: "addPattern" })
    await act(async () => {
      await setTransportPattern(second!.created[0])
    })
    await view("Lead", "Show notes")
    await act(async () => {
      await dispatch({ type: "removePattern", id: second!.created[0] })
    })
    expect(useRackStore.getState().noteViews).toEqual({})
  })

  it("isolates note edits and view changes from adjacent row renders", async () => {
    render(<ChannelRackPanel />)
    await act(async () => {
      useUiStore.getState().selectChannel(channel("Lead").id)
      await settle()
    })
    const drum = channel("Drum").id
    const before = renders.get(drum)
    const note = notesOf("Lead")[0]
    await act(async () => {
      await dispatch({
        type: "updateNotes",
        pattern: project().patterns[0].id,
        channel: channel("Lead").id,
        updates: [{ id: note.id, patch: { start: 360 } }],
      })
    })
    expect(rectangles().some((rect) => rect[0] === 30)).toBe(true)
    await view("Lead", "Show steps")
    expect(renders.get(drum)).toBe(before)
  })

  it("retains notes while muting, soloing, mixing, routing, reordering and replacing the sample on a preview row", async () => {
    render(<ChannelRackPanel />)
    const before = notesOf("Lead")
    const path = shape()
    await userEvent.click(screen.getByRole("button", { name: "Lead on" }))
    expect(preview()).toHaveClass("opacity-40")
    await act(async () => {
      await runAction("channel.solo")
    })
    expect(channel("Lead").solo).toBe(true)
    const knob = screen.getByRole("slider", { name: "Lead channel volume" })
    const volume = channel("Lead").volume
    fireEvent.pointerDown(knob, { pointerId: 1, button: 0, clientY: 100 })
    fireEvent.pointerMove(knob, { pointerId: 1, clientY: 60 })
    fireEvent.pointerUp(knob, { pointerId: 1 })
    await act(settle)
    expect(channel("Lead").volume).toBeGreaterThan(volume)
    const pan = screen.getByRole("slider", { name: "Lead channel pan" })
    fireEvent.keyDown(pan, { key: "ArrowUp" })
    await act(settle)
    expect(channel("Lead").pan).not.toBe(0)
    const track = channel("Lead").mixerTrack
    await act(async () => {
      await runAction("channel.routeToNewTrack")
      await runAction("channel.moveUp")
    })
    expect(channel("Lead").mixerTrack).not.toBe(track)
    expect(project().channels[0].name).toBe("Lead")
    const button = screen.getByRole("button", { name: "Lead, no sample" })
    fireEvent.drop(button, {
      dataTransfer: dragData(
        SAMPLE_DRAG_TYPE,
        JSON.stringify({
          path: "/factory/Drums/Kicks/Kick 02.wav",
          name: "Kick 02",
        })
      ),
    })
    await act(settle)
    expect(sourceSample(channel("Lead").source)).not.toBeNull()
    expect(notesOf("Lead")).toEqual(before)
    expect(shape()).toBe(path)
  })

  it("bounds dense valid lanes to one path and lets the last note contribute", async () => {
    const notes: NoteInit[] = Array.from({ length: 3000 }, () => ({
      start: 0,
      length: 240,
      key: 60,
    }))
    notes.push({ start: 3600, length: 240, key: 72 })
    await edit(notes)
    render(<ChannelRackPanel />)
    expect(preview().querySelectorAll("path")).toHaveLength(1)
    expect(rectangles()).toHaveLength(2)
    expect(rectangles().some((rect) => rect[0] === 300)).toBe(true)
    expect(preview()).toHaveTextContent("dense notes are combined")
    expect(notesOf("Lead")).toHaveLength(3001)
    expect(shape().length).toBeLessThan(400000)
  })

  it("moves the shared ruler and thumbnail playheads without row renders or rebuilding note geometry", async () => {
    render(<ChannelRackPanel />)
    const lead = channel("Lead").id
    const before = renders.get(lead)
    const path = preview().querySelector("path")!
    const geometry = shape()
    const caret = preview().querySelector<HTMLElement>(
      '[data-slot="rack-note-playhead"]'
    )!
    await act(async () => {
      await app.backend.transportPlay()
    })
    await waitFor(() => expect(caret.style.visibility).toBe("visible"))
    await waitFor(() => expect(caret.style.left).toContain("var(--rack-pitch)"))
    const ruler = screen.getByRole("img", { name: "Ruler: 16 steps" })
    expect(
      ruler.querySelector<HTMLElement>('[data-slot="rack-playhead"]')?.style
        .visibility
    ).toBe("visible")
    expect(preview().querySelector("path")).toBe(path)
    expect(shape()).toBe(geometry)
    expect(renders.get(lead)).toBe(before)
    await act(async () => {
      await app.backend.transportStop()
    })
    await waitFor(() => expect(caret.style.visibility).toBe("hidden"))
  })

  it("abandons a pressed thumbnail and an open menu when the pattern changes", async () => {
    render(<ChannelRackPanel />)
    const old = preview()
    fireEvent.pointerDown(old, { button: 0, pointerId: 1 })
    let second = 0
    await act(async () => {
      const added = await dispatch({ type: "addPattern" })
      second = added!.created[0]
      await setTransportPattern(second)
    })
    fireEvent.click(old, { detail: 1 })
    expect(useUiStore.getState().centerTab).toBe("channelRack")
    await userEvent.click(screen.getByRole("button", { name: "Lead row view" }))
    await act(async () => {
      await setTransportPattern(project().patterns[0].id)
    })
    expect(screen.queryByRole("menu")).not.toBeInTheDocument()
    expect(useRackStore.getState().noteViews).toEqual({})
  })

  it.each([75, 100, 150, 200] as const)(
    "keeps logical timeline width/ruler/scroll and step input at %s%%",
    async (scale) => {
      applyUiScale(scale)
      render(<ChannelRackPanel />)
      const grid = stepGrid("Drum")
      const thumb = preview().parentElement!
      expect(thumb.style.width).toBe(grid.style.width)
      expect(preview().querySelector("svg")).toHaveAttribute(
        "viewBox",
        "0 0 320 22"
      )
      const scroller = document.querySelector<HTMLElement>(
        '[data-slot="rack-scroll"]'
      )!
      expect(scroller.style.getPropertyValue(PITCH_VAR)).toBe("20px")
      expect(
        (grid.closest("[data-channel-row]")!.lastElementChild as HTMLElement)
          .style.paddingLeft
      ).toBe(`${STEPS_INSET}px`)
      const line = document.querySelector<HTMLElement>(
        '[data-slot="rack-bar-line"]'
      )
      expect(line).toBeNull() // One bar; the shared ruler still starts at tick zero.
      const pinned = document.querySelector<HTMLElement>("[data-channel-row]")!
        .firstElementChild as HTMLElement
      expect(pinned.style.width).toBe(`${LEFT_WIDTH}px`)
      scroller.scrollLeft = 120
      expect(rectangles()[0][0]).toBe(10)
      expect(rectangles()[2][2]).toBe(80)
      // Real StepGrid ratio input cancels application zoom exactly once.
      grid.getBoundingClientRect = () => ({
        left: 100,
        top: 0,
        width: (320 * scale) / 100,
        height: (22 * scale) / 100,
        right: 100 + (320 * scale) / 100,
        bottom: (22 * scale) / 100,
        x: 100,
        y: 0,
        toJSON() {},
      })
      const before = history().cursor
      fireEvent.pointerDown(grid, {
        pointerId: 1,
        button: 0,
        clientX: 100 + (2.5 * 20 * scale) / 100,
      })
      fireEvent.pointerUp(grid, { pointerId: 1 })
      await act(settle)
      expect(notesOf("Drum").some((note) => note.start === 480)).toBe(true)
      expect(history().cursor).toBe(before + 1)
      const caret = preview().querySelector<HTMLElement>(
        '[data-slot="rack-note-playhead"]'
      )!
      const path = shape()
      await act(async () => {
        await app.backend.transportPlay()
      })
      await waitFor(() => expect(caret.style.visibility).toBe("visible"))
      const rulerCaret = document.querySelector<HTMLElement>(
        '[data-slot="rack-playhead"]'
      )!
      const headerX = Number(
        rulerCaret.style.transform.match(/translateX\(([\d.]+)px\)/)![1]
      )
      const previewStep = Number(caret.style.left.match(/\* ([\d.]+)/)![1])
      expect(previewStep * 20).toBeCloseTo(headerX, 8)
      expect(shape()).toBe(path)
      await act(async () => {
        await app.backend.transportStop()
      })
      await waitFor(() => expect(caret.style.visibility).toBe("hidden"))
      await userEvent.click(preview())
      expect(useUiStore.getState().selectedChannel).toBe(channel("Lead").id)
      expect(useTransportStore.getState().pattern).toBe(
        project().patterns[0].id
      )
    }
  )
})
