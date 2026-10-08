import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type {
  Command,
  DispatchResult,
  DocumentSnapshot,
  ProjectPatch,
} from "@/bindings"
import { ActionButton } from "@/components/action-button"
import { HistoryList } from "@/features/history/history-list"
import { AppMenuBar } from "@/features/layout/menu-bar"
import type { TimeGridView } from "@/lib/canvas"
import { runAction } from "@/lib/actions"
import {
  dispatch,
  historyJump,
  loadSnapshot,
  redo,
  refetchSnapshot,
  setProjectPath,
  undo,
  useProjectStore,
} from "@/lib/store/project"
import { announceProjectReplaced } from "@/lib/store/replaced"
import { settle } from "@/test/harness"
import { MASTER_TRACK } from "@/lib/units"

import { SessionContext } from "./context"
import type { PointerInput } from "./editor"
import { attachGridInput } from "./grid-input"
import { StampMenu } from "./stamp-menu"
import { CHORD_STAMPS } from "./stamps"
import { usePianoRollStore } from "./store"
import {
  at,
  channel,
  currentPattern,
  notesOf,
  startRoll,
  undoSteps,
} from "./test-utils"

let roll: Awaited<ReturnType<typeof startRoll>>
let detachGrid: (() => void) | undefined
let restoreAnimations: (() => void) | undefined
const chord = CHORD_STAMPS[0]
const provisionalCount = () =>
  roll.editor.notes.filter((note) => note.id < 0).length

beforeEach(async () => {
  roll = await startRoll()
  await dispatch({ type: "addChannel", name: "Lead" })
  roll.show("Lead")
})

afterEach(() => {
  detachGrid?.()
  detachGrid = undefined
  restoreAnimations?.()
  restoreAnimations = undefined
  roll.stop()
  vi.restoreAllMocks()
})

function grid() {
  const element = document.createElement("canvas")
  element.tabIndex = 0
  document.body.append(element)
  const captured = new Set<number>()
  element.setPointerCapture = (id) => {
    captured.add(id)
  }
  element.hasPointerCapture = (id) => captured.has(id)
  element.releasePointerCapture = (id) => {
    captured.delete(id)
  }
  const view = Object.assign(roll.surface, {
    element,
    localPoint: (event: MouseEvent) => ({ x: event.clientX, y: event.clientY }),
    panBy: vi.fn(),
  })
  const stop = attachGridInput(roll.session, view as unknown as TimeGridView)
  roll.session.view = view as unknown as TimeGridView
  roll.session.setFocusTarget(() => element)
  detachGrid = () => {
    stop()
    roll.session.view = null
    element.remove()
  }
  return element
}

function pointer(
  element: HTMLElement,
  type: string,
  input = at(500, 60),
  button = 0
) {
  // jsdom has no PointerEvent, but the real listeners receive these same fields.
  const event = new MouseEvent(type, {
    bubbles: true,
    cancelable: true,
    clientX: input.x,
    clientY: input.y,
    shiftKey: input.shift,
    ctrlKey: input.ctrl,
    altKey: input.alt,
    button,
  })
  Object.defineProperties(event, {
    pointerId: { value: 1 },
    pointerType: { value: "mouse" },
  })
  fireEvent(element, event)
  return event
}

function documentState() {
  const { dirty, history, revision } = useProjectStore.getState()
  return structuredClone({ notes: notesOf("Lead"), dirty, history, revision })
}

async function closingChoice() {
  let finish!: () => void
  const finished = new Promise<void>((resolve) => {
    finish = resolve
  })
  const observed = new Set<Element>()
  const previous = Object.getOwnPropertyDescriptor(
    Element.prototype,
    "getAnimations"
  )
  // Hold Base UI's actual exit completion, rather than guessing its CSS duration
  // or invoking the component callback directly. Opening animations finish normally.
  Object.defineProperty(Element.prototype, "getAnimations", {
    configurable: true,
    value(this: Element) {
      if (this.matches('[data-slot="dropdown-menu-content"][data-closed]')) {
        observed.add(this)
        return [{ finished }]
      }
      return []
    },
  })
  restoreAnimations = () => {
    if (previous)
      Object.defineProperty(Element.prototype, "getAnimations", previous)
    else Reflect.deleteProperty(Element.prototype, "getAnimations")
  }
  const menu = render(
    <SessionContext.Provider value={roll.session}>
      <StampMenu />
    </SessionContext.Provider>
  )
  fireEvent.click(
    screen.getByRole("button", { name: "Choose chord or scale stamp" })
  )
  fireEvent.click(await screen.findByRole("menuitem", { name: "Chords" }))
  fireEvent.click(await screen.findByRole("menuitem", { name: "Major triad" }))
  await waitFor(() => expect(observed.size).toBe(1))
  expect(roll.editor.stampState).toBeNull()
  expect(provisionalCount()).toBe(0)
  return {
    unmount: menu.unmount,
    async finish() {
      await act(async () => {
        finish()
        await finished
      })
      await waitFor(() =>
        expect(
          document.querySelector('[data-slot="dropdown-menu-content"]')
        ).toBeNull()
      )
    },
  }
}

describe("stamp choices during the real menu exit", () => {
  it("arms once and focuses the grid only after an ordinary menu exit completes", async () => {
    const element = grid()
    const focus = vi.spyOn(roll.session, "focusGrid")
    const before = undoSteps()
    const choice = await closingChoice()
    expect(focus).not.toHaveBeenCalled()
    await choice.finish()
    expect(roll.editor.stampState?.stamp).toBe(chord)
    expect(focus).toHaveBeenCalledOnce()
    expect(element).toHaveFocus()
    expect(notesOf("Lead")).toEqual([])
    expect(undoSteps()).toBe(before)
  })

  it.each(
    (["root", "mode", "guide", "pitch snap"] as const).flatMap((setting) =>
      [false, true].map((returnToOriginal) => ({ setting, returnToOriginal }))
    )
  )(
    "revokes a choice after a $setting change (return=$returnToOriginal)",
    async ({ setting, returnToOriginal }) => {
      grid()
      const before = documentState()
      const focus = vi.spyOn(roll.session, "focusGrid")
      const choice = await closingChoice()
      act(() => {
        const state = usePianoRollStore.getState()
        switch (setting) {
          case "root":
            state.setScaleRoot(2)
            if (returnToOriginal) state.setScaleRoot(0)
            break
          case "mode":
            state.setScaleId("minor")
            if (returnToOriginal) state.setScaleId("major")
            break
          case "guide":
            state.setHighlightScale(true)
            if (returnToOriginal) state.setHighlightScale(false)
            break
          case "pitch snap":
            state.setSnapToScale(true)
            if (returnToOriginal) state.setSnapToScale(false)
            break
        }
      })
      await choice.finish()
      expect(roll.editor.stampState).toBeNull()
      expect(roll.editor.busy).toBe(false)
      expect(focus).not.toHaveBeenCalled()
      expect(documentState()).toEqual(before)
    }
  )

  it.each(
    (["tempo", "master gain", "other channel"] as const).flatMap((edit) =>
      (["undo", "redo", "round trip", "jump"] as const).map((operation) => ({
        edit,
        operation,
      }))
    )
  )(
    "revokes a choice on non-lane $edit $operation",
    async ({ edit, operation }) => {
      await dispatch({
        type: "addNotes",
        pattern: currentPattern().id,
        channel: channel("Lead").id,
        notes: [{ start: 480, key: 60, length: 240, velocity: 0.4, pan: -0.2 }],
      })
      const command: Command =
        edit === "tempo"
          ? { type: "updateSettings", patch: { tempoBpm: 100 } }
          : edit === "master gain"
            ? {
                type: "updateMixerTrack",
                id: MASTER_TRACK,
                patch: { volume: 0.4 },
              }
            : {
                type: "updateChannel",
                id: channel("Kick").id,
                patch: { volume: 0.4 },
              }
      await dispatch(command)
      if (operation === "redo") await undo()
      const lane = notesOf("Lead")
      const beforeCursor = undoSteps()
      grid()
      const focus = vi.spyOn(roll.session, "focusGrid")
      const choice = await closingChoice()
      await act(async () => {
        if (operation === "jump") await historyJump(beforeCursor - 1)
        else if (operation === "redo") await redo()
        else {
          await runAction("edit.undo")
          expect(undoSteps()).toBe(beforeCursor - 1)
          expect(notesOf("Lead")).toBe(lane)
          if (operation === "round trip") await runAction("edit.redo")
        }
      })
      expect(undoSteps()).toBe(
        beforeCursor +
          (operation === "redo" ? 1 : operation === "round trip" ? 0 : -1)
      )
      expect(notesOf("Lead")).toBe(lane)
      const afterNavigation = documentState()
      await choice.finish()
      expect(roll.editor.stampState).toBeNull()
      expect(roll.editor.busy).toBe(false)
      expect(focus).not.toHaveBeenCalled()
      expect(documentState()).toEqual(afterNavigation)
    }
  )

  it.each([
    { operation: "undo", source: "toolbar" },
    { operation: "redo", source: "toolbar" },
    { operation: "undo", source: "keyboard" },
    { operation: "redo", source: "keyboard" },
    { operation: "undo", source: "menu" },
    { operation: "redo", source: "menu" },
    { operation: "historyJump", source: "store" },
    { operation: "historyJump", source: "history list" },
  ] as const)(
    "revokes a choice before a delayed $operation reply from $source",
    async ({ operation, source }) => {
      await dispatch({
        type: "addNotes",
        pattern: currentPattern().id,
        channel: channel("Lead").id,
        notes: [{ start: 480, key: 60, length: 240 }],
      })
      await dispatch({ type: "updateSettings", patch: { tempoBpm: 100 } })
      if (operation === "redo") await undo()
      const lane = notesOf("Lead")
      const element = grid()
      const focus = vi.spyOn(roll.session, "focusGrid")
      const choice = await closingChoice()
      const before = documentState()
      let reply!: (patch: ProjectPatch | null) => void
      const held = new Promise<ProjectPatch | null>((resolve) => {
        reply = resolve
      })
      const original =
        operation === "historyJump"
          ? roll.backend.historyJump.bind(
              roll.backend,
              before.history.cursor - 1
            )
          : roll.backend[operation].bind(roll.backend)
      let busyAtRequest: boolean | undefined
      // Hold IPC before executing it: neither its event nor its reply can update
      // the mirror until after the menu's actual Animation.finished resolves.
      const request = vi
        .spyOn(roll.backend, operation)
        .mockImplementationOnce(() => {
          busyAtRequest = roll.editor.busy
          return held
        })
      try {
        if (source === "toolbar") {
          render(<ActionButton action={`edit.${operation}`} />)
          fireEvent.click(
            screen.getByRole("button", {
              name: operation === "undo" ? "Undo" : "Redo",
            })
          )
        } else if (source === "keyboard") {
          element.focus()
          const event = new KeyboardEvent("keydown", {
            key: "z",
            code: "KeyZ",
            ctrlKey: true,
            shiftKey: operation === "redo",
            bubbles: true,
            cancelable: true,
          })
          fireEvent(element, event)
          expect(event.defaultPrevented).toBe(true)
        } else if (source === "menu") {
          render(<AppMenuBar />)
          fireEvent.click(screen.getByRole("menuitem", { name: "Edit" }))
          fireEvent.click(
            await screen.findByRole("menuitem", {
              name: operation === "undo" ? /^Undo/ : /^Redo/,
            })
          )
        } else if (source === "history list") {
          render(<HistoryList />)
          fireEvent.click(screen.getByRole("button", { name: "Add note" }))
        } else {
          void historyJump(before.history.cursor - 1)
        }
        await waitFor(() => expect(request).toHaveBeenCalledOnce())
        expect(busyAtRequest).toBe(false)
        expect(documentState()).toEqual(before)
        expect(notesOf("Lead")).toBe(lane)
        await choice.finish()
        expect(roll.editor.stampState).toBeNull()
        expect(roll.editor.busy).toBe(false)
        expect(focus).not.toHaveBeenCalled()
        expect(documentState()).toEqual(before)
      } finally {
        await act(async () => {
          reply(await original())
          await settle()
        })
      }
      expect(undoSteps()).toBe(
        before.history.cursor + (operation === "redo" ? 1 : -1)
      )
      expect(notesOf("Lead")).toBe(lane)
      expect(roll.editor.stampState).toBeNull()
    }
  )

  it("keeps a valid choice through unchanged scale values and unrelated UI notifications", async () => {
    const element = grid()
    const focus = vi.spyOn(roll.session, "focusGrid")
    const before = documentState()
    const choice = await closingChoice()
    act(() => {
      const state = usePianoRollStore.getState()
      state.setScaleRoot(state.scaleRoot)
      state.setScaleId(state.scaleId)
      state.setHighlightScale(state.highlightScale)
      state.setSnapToScale(state.snapToScale)
      state.setGhosts(false)
      state.setFollow(true)
      setProjectPath("/projects/same-document.windfall")
    })
    await choice.finish()
    expect(roll.editor.stampState?.stamp).toBe(chord)
    expect(focus).toHaveBeenCalledOnce()
    expect(element).toHaveFocus()
    expect(documentState()).toEqual(before)
  })

  it("does not treat ordinary non-lane dispatch as history navigation", async () => {
    grid()
    const choice = await closingChoice()
    await act(async () => {
      await dispatch({ type: "updateSettings", patch: { tempoBpm: 100 } })
    })
    const beforeCompletion = documentState()
    await choice.finish()
    expect(roll.editor.stampState?.stamp).toBe(chord)
    expect(documentState()).toEqual(beforeCompletion)
  })

  it("revokes a choice on a no-op history jump without editing the document", async () => {
    grid()
    const choice = await closingChoice()
    const focus = vi.spyOn(roll.session, "focusGrid")
    const before = documentState()
    await act(async () => {
      await historyJump(before.history.cursor)
    })
    const afterNavigation = documentState()
    expect(afterNavigation).toMatchObject({
      notes: before.notes,
      dirty: before.dirty,
      history: before.history,
    })
    await choice.finish()
    expect(roll.editor.stampState).toBeNull()
    expect(focus).not.toHaveBeenCalled()
    expect(documentState()).toEqual(afterNavigation)
  })

  it.each([
    { tool: "draw", occupied: false },
    { tool: "draw", occupied: true },
    { tool: "select", occupied: false },
    { tool: "select", occupied: true },
  ] as const)(
    "cancels a pending choice before a $tool right-click (occupied=$occupied)",
    async ({ tool, occupied }) => {
      if (occupied) {
        await dispatch({
          type: "addNotes",
          pattern: currentPattern().id,
          channel: channel("Lead").id,
          notes: [
            { start: 480, key: 60, length: 240, velocity: 0.4, pan: -0.2 },
          ],
        })
        roll.editor.selectAll()
      }
      usePianoRollStore.getState().setTool(tool)
      await roll.backend.projectSave("/projects/pending-right-click.windfall")
      await settle()
      const element = grid()
      const selection = [...roll.editor.selection]
      const before = documentState()
      expect(before.dirty).toBe(false)
      const choice = await closingChoice()
      const menu = vi.fn()
      // A parent context-menu handler must not receive the cancellation click.
      const container = document.createElement("div")
      element.replaceWith(container)
      container.append(element)
      container.addEventListener("contextmenu", menu)
      const context = new MouseEvent("contextmenu", {
        bubbles: true,
        cancelable: true,
        button: 2,
        clientX: at(600, 60).x,
        clientY: at(600, 60).y,
      })
      try {
        await act(async () => {
          pointer(element, "pointerdown", at(600, 60), 2)
          fireEvent(element, context)
          pointer(element, "pointerup", at(600, 60), 2)
          await settle()
        })
        const afterClick = documentState()
        const captured = element.hasPointerCapture(1)
        await choice.finish()
        expect(afterClick).toEqual(before)
        expect(documentState()).toEqual(before)
        expect(roll.editor.stampState).toBeNull()
        expect(roll.editor.busy).toBe(false)
        expect([...roll.editor.selection]).toEqual(selection)
        expect(captured).toBe(false)
        expect(context.defaultPrevented).toBe(true)
        expect(menu).not.toHaveBeenCalled()
      } finally {
        container.replaceWith(element)
      }
    }
  )

  it.each([
    "blur",
    "tool action",
    "tool setter",
    "Escape",
    "lane round trip",
    "lane edit",
    "pattern length",
    "time signature",
    "project replacement",
    "disposal",
    "menu unmount",
  ] as const)(
    "cannot arm a choice after %s cancels it during delayed closing",
    async (reason) => {
      grid()
      const choice = await closingChoice()
      const focus = vi.spyOn(roll.session, "focusGrid")
      const before = undoSteps()
      await act(async () => {
        switch (reason) {
          case "blur":
            fireEvent(window, new Event("blur"))
            break
          case "tool action":
            await runAction("pianoRoll.toolPaint")
            break
          case "tool setter":
            usePianoRollStore.getState().setTool("paint")
            usePianoRollStore.getState().setTool("draw")
            break
          case "Escape":
            fireEvent.keyDown(window, { key: "Escape" })
            break
          case "lane round trip":
            roll.show("Kick")
            roll.show("Lead")
            break
          case "lane edit":
            await dispatch({
              type: "addNotes",
              pattern: currentPattern().id,
              channel: channel("Lead").id,
              notes: [{ start: 0, key: 50, length: 240 }],
            })
            await undo()
            break
          case "pattern length":
            await dispatch({
              type: "updatePattern",
              id: currentPattern().id,
              patch: { lengthSteps: 32 },
            })
            await undo()
            break
          case "time signature":
            await dispatch({
              type: "updateSettings",
              patch: { timeSignature: { numerator: 3, denominator: 4 } },
            })
            await undo()
            break
          case "project replacement":
            announceProjectReplaced()
            break
          case "disposal":
            roll.editor.dispose()
            roll.editor.attach(roll.surface)
            break
          case "menu unmount":
            choice.unmount()
            break
        }
      })
      expect(roll.editor.stampState).toBeNull()
      // The tool action itself focuses the grid; its later menu completion must not.
      focus.mockClear()
      await choice.finish()
      expect(roll.editor.stampState).toBeNull()
      expect(roll.editor.busy).toBe(false)
      expect(focus).not.toHaveBeenCalled()
      expect(provisionalCount()).toBe(0)
      expect(notesOf("Lead")).toEqual([])
      expect(undoSteps()).toBe(before)
    }
  )
})

describe("stamp preview pointer presence through real grid input", () => {
  it.each(["draw", "select"] as const)(
    "preserves ordinary %s right-click behavior without a stamp choice",
    async (tool) => {
      await dispatch({
        type: "addNotes",
        pattern: currentPattern().id,
        channel: channel("Lead").id,
        notes: [{ start: 480, key: 60, length: 240 }],
      })
      usePianoRollStore.getState().setTool(tool)
      await roll.backend.projectSave("/projects/ordinary-right-click.windfall")
      await settle()
      const element = grid()
      const before = documentState()
      const context = new MouseEvent("contextmenu", {
        bubbles: true,
        cancelable: true,
        button: 2,
      })
      const menu = vi.fn()
      document.body.addEventListener("contextmenu", menu)
      try {
        pointer(element, "pointerdown", at(600, 60), 2)
        fireEvent(element, context)
        pointer(element, "pointerup", at(600, 60), 2)
        await settle()
        if (tool === "draw") {
          expect(notesOf("Lead")).toEqual([])
          expect(undoSteps()).toBe(before.history.cursor + 1)
          expect(useProjectStore.getState().dirty).toBe(true)
          expect(context.defaultPrevented).toBe(true)
          expect(menu).not.toHaveBeenCalled()
        } else {
          expect(documentState()).toEqual(before)
          expect(roll.editor.selectionCount).toBe(1)
          expect(context.defaultPrevented).toBe(false)
          expect(menu).toHaveBeenCalledOnce()
        }
      } finally {
        document.body.removeEventListener("contextmenu", menu)
      }
    }
  )

  it("right-click cancels a captured stamp gesture and releases capture without an edit", async () => {
    const element = grid()
    const before = documentState()
    roll.editor.armStamp(chord)
    pointer(element, "pointerdown")
    expect(element.hasPointerCapture(1)).toBe(true)
    expect(provisionalCount()).toBe(3)
    pointer(element, "pointerdown", at(500, 60), 2)
    expect(element.hasPointerCapture(1)).toBe(false)
    pointer(element, "pointerup", at(500, 60), 2)
    await settle()
    expect(roll.editor.stampState).toBeNull()
    expect(provisionalCount()).toBe(0)
    expect(documentState()).toEqual(before)
  })

  it("preserves a valid captured stamp drag outside the grid, including snap bypass and wheel refresh", async () => {
    const element = grid()
    const before = documentState()
    usePianoRollStore.getState().setSnapToScale(true)
    roll.editor.armStamp(chord)
    pointer(element, "pointerdown")
    expect(element.hasPointerCapture(1)).toBe(true)
    const outside = { ...at(17000, 61), x: 850 }
    pointer(element, "pointermove", outside)
    pointer(element, "pointerleave", outside)
    expect(roll.editor.notes.map((note) => note.key)).toEqual([60, 64, 67])
    fireEvent.keyDown(window, { key: "Alt", altKey: true })
    expect(roll.editor.notes.map((note) => note.key)).toEqual([61, 65, 68])
    fireEvent.keyUp(window, { key: "Alt" })
    expect(roll.editor.notes.map((note) => note.key)).toEqual([60, 64, 67])
    fireEvent.keyDown(window, { key: "Alt", altKey: true })
    fireEvent.wheel(element, {
      clientX: outside.x,
      clientY: outside.y,
      deltaY: 16,
    })
    expect(roll.editor.notes.map((note) => note.key)).toEqual([61, 65, 68])
    expect(documentState()).toEqual(before)
    pointer(element, "pointerup", { ...outside, alt: true })
    await settle()
    expect(notesOf("Lead")).toMatchObject(
      [61, 65, 68].map((key) => ({ start: 17000, key, length: 240 }))
    )
    expect(undoSteps()).toBe(before.history.cursor + 1)
    expect(roll.editor.selectionCount).toBe(3)
    expect(provisionalCount()).toBe(0)
    expect(element.hasPointerCapture(1)).toBe(false)
  })

  it("does not capture a rejected root-125 stamp press", async () => {
    roll.surface.viewport = { ...roll.surface.viewport, scrollRow: 0 }
    const element = grid()
    roll.editor.armStamp(chord)
    const before = documentState()
    pointer(element, "pointerdown", { ...at(500, 125), y: 40 })
    expect(roll.editor.stampState?.error).toContain("MIDI")
    expect(element.hasPointerCapture(1)).toBe(false)
    expect(documentState()).toEqual(before)
    expect(provisionalCount()).toBe(0)
  })

  it.each(["Alt", "Shift", "Control", "wheel"] as const)(
    "keeps a rejected press idle through leave, outside move and %s, then resumes on re-entry",
    async (refresh) => {
      roll.surface.viewport = { ...roll.surface.viewport, scrollRow: 0 }
      const element = grid()
      await roll.backend.projectSave("/projects/rejected-stamp.windfall")
      await settle()
      const before = documentState()
      const rejected = { ...at(500, 125), y: 40 }
      const outside = { ...rejected, y: roll.surface.viewport.height + 16 }
      roll.editor.armStamp(chord)
      pointer(element, "pointerdown", rejected)
      expect(roll.editor.stampState?.error).toContain("MIDI")
      pointer(element, "pointerleave", outside)
      pointer(element, "pointermove", outside)
      const afterMove = provisionalCount()
      // Check the refresh independently even when the old captured move revived it.
      pointer(element, "pointerleave", outside)
      expect(provisionalCount()).toBe(0)
      if (refresh === "wheel") {
        fireEvent.wheel(element, {
          clientX: outside.x,
          clientY: outside.y,
          deltaY: 16,
        })
      } else {
        fireEvent.keyDown(window, {
          key: refresh,
          altKey: refresh === "Alt",
          shiftKey: refresh === "Shift",
          ctrlKey: refresh === "Control",
        })
      }
      const afterRefresh = provisionalCount()
      if (refresh !== "wheel") fireEvent.keyUp(window, { key: refresh })
      expect(afterMove).toBe(0)
      expect(afterRefresh).toBe(0)
      expect(provisionalCount()).toBe(0)
      expect(element.hasPointerCapture(1)).toBe(false)
      expect(documentState()).toEqual(before)

      const returned = { ...rejected, y: 120 }
      pointer(element, "pointerenter", returned)
      expect(roll.editor.notes.map((note) => note.key)).toEqual([120, 124, 127])
      pointer(element, "pointerup", returned)
      await settle()
      expect(documentState()).toEqual(before)
      pointer(element, "pointerdown", returned)
      expect(element.hasPointerCapture(1)).toBe(true)
      pointer(element, "pointerup", returned)
      await settle()
      expect(notesOf("Lead").map((note) => note.key)).toEqual([120, 124, 127])
      expect(roll.editor.selectionCount).toBe(3)
      expect(undoSteps()).toBe(before.history.cursor + 1)
      expect(element.hasPointerCapture(1)).toBe(false)
    }
  )

  it.each(["Alt", "Shift", "Control"] as const)(
    "keeps a left preview hidden for %s down/up, and resumes on re-entry",
    (key) => {
      const element = grid()
      roll.editor.armStamp(chord)
      pointer(element, "pointerenter")
      pointer(element, "pointermove")
      expect(provisionalCount()).toBe(3)
      pointer(element, "pointerleave")
      expect(provisionalCount()).toBe(0)
      fireEvent.keyDown(window, {
        key,
        altKey: key === "Alt",
        shiftKey: key === "Shift",
        ctrlKey: key === "Control",
      })
      expect(provisionalCount()).toBe(0)
      fireEvent.keyUp(window, { key })
      expect(provisionalCount()).toBe(0)
      pointer(element, "pointerenter", at(960, 62))
      expect(roll.editor.notes.map((note) => note.key)).toEqual([62, 66, 69])
      expect(notesOf("Lead")).toEqual([])
    }
  )

  it("continues modifier updates on a captured drag outside the grid and commits its final pitch", async () => {
    const element = grid()
    usePianoRollStore.getState().setSnapToScale(true)
    const start = at(500, 60)
    // A captured event outside the right edge is still part of this draw gesture.
    const outside: PointerInput = { ...at(17000, 61), x: 850 }
    pointer(element, "pointerdown", start)
    expect(element.hasPointerCapture(1)).toBe(true)
    pointer(element, "pointermove", outside)
    pointer(element, "pointerleave", outside)
    expect(roll.surface.dragOffset.rows).toBeCloseTo(0)
    fireEvent.keyDown(window, { key: "Alt", altKey: true })
    expect(roll.surface.dragOffset.rows).toBe(-1)
    fireEvent.keyUp(window, { key: "Alt" })
    expect(roll.surface.dragOffset.rows).toBeCloseTo(0)
    fireEvent.keyDown(window, { key: "Shift", shiftKey: true })
    expect(roll.surface.dragOffset.rows).toBeCloseTo(0)
    fireEvent.keyUp(window, { key: "Shift" })
    expect(roll.surface.dragOffset.rows).toBeCloseTo(0)
    fireEvent.keyDown(window, { key: "Alt", altKey: true })
    pointer(element, "pointerup", { ...outside, alt: true })
    await settle()
    expect(element.hasPointerCapture(1)).toBe(false)
    expect(notesOf("Lead")).toMatchObject([
      { start: 16980, key: 61, length: 240 },
    ])
  })

  it("updates Ctrl copy mode on a captured note drag outside the grid", async () => {
    await dispatch({
      type: "addNotes",
      pattern: currentPattern().id,
      channel: channel("Lead").id,
      notes: [{ start: 480, key: 60, length: 240 }],
    })
    const element = grid()
    pointer(element, "pointerdown", at(600, 60))
    pointer(element, "pointermove", at(17160, 62))
    pointer(element, "pointerleave", at(17160, 62))
    fireEvent.keyDown(window, { key: "Control", ctrlKey: true })
    expect(roll.editor.drag?.duplicate).toBe(true)
    fireEvent.keyUp(window, { key: "Control" })
    expect(roll.editor.drag?.duplicate).toBe(false)
    fireEvent.keyDown(window, { key: "Control", ctrlKey: true })
    pointer(element, "pointerup", at(17160, 62, { ctrl: true }))
    await settle()
    expect(notesOf("Lead")).toMatchObject([
      { start: 480, key: 60, length: 240 },
      { start: 17040, key: 62, length: 240 },
    ])
    expect(roll.editor.selectionCount).toBe(1)
  })
})

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

describe("native stamp replies across document recovery", () => {
  it("selects all created stamp notes after a held revision-gap snapshot recovers", async () => {
    const original = roll.backend.documentSnapshot.bind(roll.backend)
    const old = await original()
    await roll.backend.dispatch({
      type: "updateSettings",
      patch: { name: "Missed event" },
    })
    loadSnapshot(old)
    const recovery = deferred<DocumentSnapshot>()
    const snapshot = vi
      .spyOn(roll.backend, "documentSnapshot")
      .mockReturnValueOnce(recovery.promise)
    roll.editor.armStamp(chord)
    roll.editor.pointerDown(at(500, 60), "left")
    roll.editor.pointerUp(at(500, 60))
    await settle()
    expect(snapshot).toHaveBeenCalledOnce()
    expect(notesOf("Lead")).toEqual([])
    expect(roll.editor.selectionCount).toBe(0)
    const resumed = refetchSnapshot()
    recovery.resolve(await original())
    await resumed
    await settle()
    expect(notesOf("Lead").map((note) => note.key)).toEqual([60, 64, 67])
    expect([...roll.editor.selection].sort()).toEqual(
      notesOf("Lead")
        .map((note) => note.id)
        .sort()
    )
    expect(roll.editor.stampState).toBeNull()
  })

  it.each(["lane switch", "project replacement", "disposal"] as const)(
    "does not apply late created selection after %s",
    async (reason) => {
      const original = roll.backend.dispatch.bind(roll.backend)
      const committed = deferred<DispatchResult>()
      const reply = deferred<DispatchResult>()
      vi.spyOn(roll.backend, "dispatch").mockImplementationOnce(
        async (...args) => {
          committed.resolve(await original(...args))
          return reply.promise
        }
      )
      roll.editor.armStamp(chord)
      roll.editor.pointerDown(at(500, 60), "left")
      roll.editor.pointerUp(at(500, 60))
      const result = await committed.promise
      switch (reason) {
        case "lane switch":
          roll.show("Kick")
          break
        case "project replacement":
          await roll.backend.projectNew()
          break
        case "disposal":
          roll.editor.dispose()
          break
      }
      const project = useProjectStore.getState().project
      const selection = [...roll.editor.selection]
      reply.resolve(result)
      await settle()
      expect([...roll.editor.selection]).toEqual(selection)
      expect(roll.editor.selectionCount).toBe(0)
      expect(useProjectStore.getState().project).toEqual(project)
      expect(roll.editor.stampState).toBeNull()
    }
  )
})
