import { useReducer } from "react"

import {
  contextSeparator,
  type ContextItem,
} from "@/components/context-actions"
import { getAppState, isEnabled, registry, runAction } from "@/lib/actions"

import { pianoRollShortcut } from "./shortcuts"

/**
 * Context menu entries for registry actions. Piano roll actions are turned
 * into entries that carry the shortcut they have inside the piano roll,
 * which the app's keymap does not know when another panel owns the key.
 * Other ids pass through. Build the list when the menu is about to open,
 * because it holds each action's enabled state at that moment.
 */
export function rollMenu(entries: readonly ContextItem[]): ContextItem[] {
  const state = getAppState()
  return entries.map((entry) => {
    if (typeof entry !== "string" || !entry.startsWith("pianoRoll.")) {
      return entry
    }
    const action = registry.get(entry)
    if (!action) return entry
    return {
      title: action.title,
      disabled: !isEnabled(action, state),
      shortcut: pianoRollShortcut(entry),
      run: () => runAction(entry),
    }
  })
}

export const NOTE_MENU: ContextItem[] = [
  "pianoRoll.cut",
  "pianoRoll.copy",
  "pianoRoll.paste",
  "pianoRoll.duplicate",
  "pianoRoll.delete",
  contextSeparator,
  "pianoRoll.selectAll",
  "pianoRoll.deselect",
  contextSeparator,
  "pianoRoll.quantize",
  "pianoRoll.quantizeEnds",
  "pianoRoll.octaveUp",
  "pianoRoll.octaveDown",
  contextSeparator,
  "pianoRoll.zoomSelection",
  "pianoRoll.zoomFit",
]

export const VIEW_MENU: ContextItem[] = [
  "pianoRoll.zoomFit",
  "pianoRoll.zoomSelection",
  contextSeparator,
  "pianoRoll.ghosts",
  "pianoRoll.follow",
]

/**
 * Menu entries that are fresh when the menu opens. Put `refresh` on the
 * element's `onContextMenuCapture`: it renders the owner once more, in the
 * same update that opens the menu.
 */
export function useRollMenu(entries: readonly ContextItem[]) {
  const [, refresh] = useReducer((count: number) => count + 1, 0)
  return { items: rollMenu(entries), refresh }
}
