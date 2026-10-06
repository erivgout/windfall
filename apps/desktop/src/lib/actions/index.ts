import { useMemo, useSyncExternalStore } from "react"
import { useShallow } from "zustand/react/shallow"

import { reportError } from "@/lib/errors"
import { useProjectStore } from "@/lib/store/project"
import { useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"

import {
  detectMac,
  formatShortcut,
  matchEvent,
  resolveKeymap,
  type Keymap,
} from "./keymap"
import { isEnabled, registry, type Action, type AppState } from "./registry"

export { registry, isEnabled, isChecked } from "./registry"
export type { Action, AppState } from "./registry"

const isMac = detectMac()

function uiSlice(
  state: ReturnType<typeof useUiStore.getState>
): AppState["ui"] {
  return {
    theme: state.theme,
    keymap: state.keymap,
    panels: state.panels,
    centerTab: state.centerTab,
  }
}

/** The state actions look at, read right now. */
export function getAppState(): AppState {
  return {
    document: useProjectStore.getState(),
    transport: useTransportStore.getState(),
    ui: uiSlice(useUiStore.getState()),
  }
}

/**
 * The state actions look at, kept current. It changes on every edit, so use
 * it only in parts of the UI that are open briefly, such as an open menu.
 */
export function useAppState(): AppState {
  const document = useProjectStore()
  const transport = useTransportStore()
  const ui = useUiStore(useShallow(uiSlice))
  return useMemo(() => ({ document, transport, ui }), [document, transport, ui])
}

/**
 * Derives a value from the state actions look at. The component renders
 * again only when the derived value changes, so keep it a primitive.
 */
export function useAppSelector<T>(select: (state: AppState) => T): T {
  const read = () => select(getAppState())
  const value = useProjectStore(read)
  useTransportStore(read)
  useUiStore(read)
  return value
}

/** Whether an action can run right now. False for an unknown id. */
export function useActionEnabled(id: string): boolean {
  const action = useAction(id)
  return useAppSelector((state) => (action ? isEnabled(action, state) : false))
}

/** Runs an action by id, unless it is disabled. Failures are shown. */
export async function runAction(id: string): Promise<void> {
  const action = registry.get(id)
  if (!action) {
    reportError(new Error(`There is no action called "${id}".`))
    return
  }
  if (!isEnabled(action, getAppState())) return
  try {
    await action.run()
  } catch (error) {
    reportError(error, action.title)
  }
}

/** Every registered action. Renders again when actions are added or removed. */
export function useActions(): Action[] {
  return useSyncExternalStore(registry.subscribe, registry.list)
}

export function useAction(id: string): Action | undefined {
  return useActions().find((action) => action.id === id)
}

let cached: { actions: Action[]; preset: string; keymap: Keymap } | null = null

/** The keymap for the chosen preset. Rebuilt only when it can have changed. */
export function currentKeymap(): Keymap {
  const actions = registry.list()
  const preset = useUiStore.getState().keymap
  if (cached?.actions !== actions || cached.preset !== preset) {
    cached = { actions, preset, keymap: resolveKeymap(actions, preset, isMac) }
  }
  return cached.keymap
}

/** An action's main shortcut, written for this platform, or undefined. */
export function shortcutLabel(id: string): string | undefined {
  const shortcut = currentKeymap().byAction.get(id)?.[0]
  return shortcut ? formatShortcut(shortcut, isMac) : undefined
}

/** Like `shortcutLabel`, and follows keymap changes. */
export function useShortcutLabel(id: string): string | undefined {
  useActions()
  useUiStore((state) => state.keymap)
  return shortcutLabel(id)
}

/** Listens for shortcuts on the window. Returns a function that stops. */
export function installKeymap(): () => void {
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.defaultPrevented || event.isComposing) return
    const id = matchEvent(currentKeymap(), event)
    if (id === undefined) return
    // Stops the webview's own handling, such as Ctrl+S saving the page.
    event.preventDefault()
    if (event.repeat && !registry.get(id)?.repeats) return
    void runAction(id)
  }
  window.addEventListener("keydown", onKeyDown)
  return () => window.removeEventListener("keydown", onKeyDown)
}
