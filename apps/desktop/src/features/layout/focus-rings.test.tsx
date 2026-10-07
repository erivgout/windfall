import { act, render, screen } from "@testing-library/react"
import type { ReactNode } from "react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { useRackStore } from "@/features/channel-rack/rack-store"
import { useEffectsUi } from "@/features/mixer/effects-ui"
import { usePianoRollStore } from "@/features/piano-roll/store"
import { usePlaylistStore } from "@/features/playlist/store"
import { setBackend } from "@/lib/ipc"
import { createMockBackend, type MockBackend } from "@/lib/ipc/mock"
import { useEngineStore } from "@/lib/store/engine"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { usePromptStore } from "@/lib/store/prompts"
import { useTransportStore } from "@/lib/store/transport"
import { type CenterTab, useUiStore } from "@/lib/store/ui"
import { useWarningsStore } from "@/lib/store/warnings"
import { settle, TEST_DIALOGS } from "@/test/harness"

import { AppShell } from "./app-shell"

/*
 * Everything the keyboard can land on has to show that it has the focus.
 * The stylesheet outlines whatever is focused, so the ones at risk are those
 * that switch that outline off, and they have to draw something in its
 * place. jsdom draws nothing, so this reads the classes; the look itself was
 * checked in a browser, in both themes.
 */

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
  Toaster: () => null,
}))

vi.mock("@/components/ui/resizable", () => ({
  ResizablePanelGroup: ({ children }: { children: ReactNode }) => (
    <div className="flex">{children}</div>
  ),
  ResizablePanel: ({ children }: { children: ReactNode }) => (
    <div>{children}</div>
  ),
  ResizableHandle: () => null,
}))

let backend: MockBackend

beforeEach(() => {
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null)
  backend = createMockBackend({ storage: null, dialogs: TEST_DIALOGS })
  setBackend(backend)
  for (const store of [
    useProjectStore,
    useTransportStore,
    useEngineStore,
    useUiStore,
    usePromptStore,
    useWarningsStore,
    useEffectsUi,
    useRackStore,
    usePianoRollStore,
    usePlaylistStore,
  ]) {
    ;(store as { setState(state: unknown, replace: true): void }).setState(
      store.getInitialState(),
      true
    )
  }
})
afterEach(() => {
  backend.dispose()
  vi.restoreAllMocks()
})

const FOCUSABLE = "button, a[href], input, select, textarea, [tabindex]"
/** Roles the arrow keys move between, which are focusable at tabindex -1. */
const ROVING =
  /^(button|slider|tab|menuitem|menuitemcheckbox|menuitemradio|option|treeitem|switch|checkbox|radio|gridcell|application)$/

const REMOVES_OUTLINE = /(^|\s)outline-(none|hidden)(\s|$)/
/** A ring, an outline or a frame of its own while it has the focus. */
const DRAWS_FOCUS =
  /(^|\s)(focus-visible:(ring-|outline-\d|border-ring)|focus-frame(\s|$))/
/** A text entry that is framed for as long as it is open. */
const FRAMED_ENTRY = /(^|\s)(ring-\d|border-ring)(\s|$)/
/** Menu entries are lit, the way the pointer lights them. */
const LIT_ENTRY = /(^|\s)(focus:bg-|data-highlighted:bg-)/

function takesFocus(element: Element) {
  if (element.hasAttribute("disabled")) return false
  if (element.getAttribute("tabindex") !== "-1") return true
  const tag = element.tagName.toLowerCase()
  if (tag === "button" || tag === "input" || tag === "a") return true
  return ROVING.test(element.getAttribute("role") ?? "")
}

/** Why an element would not show its focus, or null when it does. */
function problem(element: Element): string | null {
  const classes = element.getAttribute("class") ?? ""
  if (!REMOVES_OUTLINE.test(classes)) return null
  if (/(^|\s)focus-visible:outline-\d/.test(classes)) {
    // `outline-none` also sets the style the width alone would inherit.
    return /(^|\s)focus-visible:outline-solid(\s|$)/.test(classes)
      ? null
      : "its outline has a width and no style"
  }
  if (DRAWS_FOCUS.test(classes)) return null
  // Drawn by a part inside it, or by the frame around it.
  if (element.querySelector('[class*="group-focus-visible"]')) return null
  if (element.closest('[class*="has-focus-visible:"]')) return null
  const tag = element.tagName.toLowerCase()
  if ((tag === "input" || tag === "textarea") && FRAMED_ENTRY.test(classes)) {
    return null
  }
  if (LIT_ENTRY.test(classes)) return null
  return "it removes the outline and draws nothing in its place"
}

function unmarked() {
  const found: string[] = []
  for (const element of document.querySelectorAll(FOCUSABLE)) {
    if (!takesFocus(element)) continue
    const why = problem(element)
    if (why === null) continue
    const name =
      element.getAttribute("aria-label") ??
      element.getAttribute("data-slot") ??
      element.textContent?.trim().slice(0, 24) ??
      ""
    found.push(`<${element.tagName.toLowerCase()}> "${name}": ${why}`)
  }
  return [...new Set(found)]
}

async function openWindow() {
  render(<AppShell />)
  await act(settle)
  const master = useProjectStore.getState().project.mixer.tracks[0]
  await act(async () => {
    await dispatch({ type: "addEffect", track: master.id, kind: "eq" })
    await dispatch({ type: "addEffect", track: master.id, kind: "compressor" })
    useUiStore.getState().selectTrack(master.id)
    useEffectsUi.getState().setInspectorOpen(true)
    useRackStore.getState().setInspectorOpen(true)
    useUiStore
      .getState()
      .selectChannel(useProjectStore.getState().project.channels[0].id)
    await settle()
  })
}

async function showTab(tab: CenterTab) {
  await act(async () => {
    useUiStore.getState().showCenterTab(tab)
    await settle()
  })
}

describe("keyboard focus", () => {
  it("is drawn by everything in the window that can take it", async () => {
    await openWindow()
    expect(unmarked()).toEqual([])
    await showTab("playlist")
    expect(unmarked()).toEqual([])
    await showTab("pianoRoll")
    expect(unmarked()).toEqual([])
  }, 30_000)

  it("frames the two editors over the canvas that fills them", async () => {
    await openWindow()
    await showTab("playlist")
    // An outline of their own would be painted under the canvas.
    expect(
      screen.getByRole("application", { name: "Song timeline" }).className
    ).toContain("focus-frame")
    await showTab("pianoRoll")
    expect(
      screen.getByRole("application", { name: "Note grid" }).className
    ).toContain("focus-frame")
  }, 30_000)
})
