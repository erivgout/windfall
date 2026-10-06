import type { TransportState } from "@/bindings"
import type { ProjectState } from "@/lib/store/project"
import type { KeymapPreset, SidePanel, CenterTab, Theme } from "@/lib/store/ui"

/** What an action may look at to decide whether it is enabled or checked. */
export type AppState = {
  document: ProjectState
  transport: TransportState
  ui: {
    theme: Theme
    keymap: KeymapPreset
    panels: Record<SidePanel, boolean>
    centerTab: CenterTab
  }
}

/**
 * One thing the user can do. Menus, the command palette, context menus and
 * the keymap are all built from these; nothing binds a key or builds a menu
 * item any other way.
 */
export type Action = {
  /** Stable name, such as `file.save`. Keymaps refer to it. */
  id: string
  /** What menus and the palette show. Sentence case, no trailing dots. */
  title: string
  /** Groups actions in the palette: File, Edit, Transport, View and so on. */
  section: string
  /**
   * Shortcut in the Windfall keymap, such as `Mod+Shift+Z`. `Mod` is Ctrl on
   * Windows and Linux and Cmd on macOS. Several shortcuts may be given.
   */
  defaultShortcut?: string | string[]
  /** Extra words the palette search matches. */
  keywords?: string
  /** Runs again and again while the key is held, as undo does. */
  repeats?: boolean
  /** Leave out for an action that is always available. */
  enabled?(state: AppState): boolean
  /** For actions that are on or off, such as "Show browser". */
  checked?(state: AppState): boolean
  run(): void | Promise<void>
}

export type Registry = {
  /** Adds actions. Returns a function that removes them again. */
  register(actions: Action[]): () => void
  get(id: string): Action | undefined
  /** Every action, in the order registered. The array changes when the set does. */
  list(): Action[]
  subscribe(listener: () => void): () => void
}

export function createRegistry(): Registry {
  const actions = new Map<string, Action>()
  const listeners = new Set<() => void>()
  let snapshot: Action[] = []

  function changed() {
    snapshot = [...actions.values()]
    for (const listener of listeners) listener()
  }

  return {
    register(added) {
      for (const action of added) {
        if (actions.has(action.id)) {
          throw new Error(`Action "${action.id}" is registered twice`)
        }
      }
      for (const action of added) actions.set(action.id, action)
      changed()
      return () => {
        for (const action of added) {
          if (actions.get(action.id) === action) actions.delete(action.id)
        }
        changed()
      }
    },
    get: (id) => actions.get(id),
    list: () => snapshot,
    subscribe(listener) {
      listeners.add(listener)
      return () => {
        listeners.delete(listener)
      }
    },
  }
}

export function isEnabled(action: Action, state: AppState): boolean {
  return action.enabled ? action.enabled(state) : true
}

export function isChecked(action: Action, state: AppState): boolean {
  return action.checked ? action.checked(state) : false
}

/** The app's one registry. */
export const registry = createRegistry()
