import {
  currentKeymap,
  getAppState,
  isEnabled,
  registry,
  runAction,
} from "@/lib/actions"
import {
  detectMac,
  formatShortcut,
  isTypingTarget,
  matchEvent,
  resolveKeymap,
  type Keymap,
} from "@/lib/actions/keymap"
import { useUiStore, type KeymapPreset } from "@/lib/store/ui"

import {
  FL_ONLY_IN_ROLL,
  PIANO_ROLL_ACTIONS,
  PIANO_ROLL_FL_KEYMAP,
} from "./actions"

/*
 * The app's keymap gives each key to one action, the first that asks for
 * it. Delete, Ctrl+D, Escape and the arrows already belong to other panels,
 * so the piano roll's actions would never get them. While the piano roll
 * has the keyboard, this resolves its actions' shortcuts on their own and
 * runs them first. The shortcuts still come from the registry's actions and
 * the keymap presets; nothing is bound here by hand.
 */

const isMac = detectMac()
const keymaps = new Map<KeymapPreset, Keymap>()
const FL_SHORTCUTS: Record<string, string[]> = {
  ...PIANO_ROLL_FL_KEYMAP,
  ...FL_ONLY_IN_ROLL,
}

export function pianoRollKeymap(preset: KeymapPreset): Keymap {
  let keymap = keymaps.get(preset)
  if (!keymap) {
    const actions =
      preset === "fl"
        ? PIANO_ROLL_ACTIONS.map((action) =>
            action.id in FL_SHORTCUTS
              ? { ...action, defaultShortcut: FL_SHORTCUTS[action.id] }
              : action
          )
        : PIANO_ROLL_ACTIONS
    keymap = resolveKeymap(actions, preset, isMac)
    keymaps.set(preset, keymap)
  }
  return keymap
}

/** A piano roll action's main shortcut, written for this platform. */
export function pianoRollShortcut(
  id: string,
  preset: KeymapPreset = useUiStore.getState().keymap
): string | undefined {
  const shortcut = pianoRollKeymap(preset).byAction.get(id)?.[0]
  return shortcut ? formatShortcut(shortcut, isMac) : undefined
}

/** Like `pianoRollShortcut`, and follows a change of keymap preset. */
export function usePianoRollShortcut(id: string): string | undefined {
  const preset = useUiStore((state) => state.keymap)
  return pianoRollShortcut(id, preset)
}

/**
 * Runs the piano roll's shortcuts while the keyboard focus is inside
 * `root`, or nowhere at all. It listens on the document, which a key press
 * reaches after the focused control and before the app's keymap on the
 * window, so a fader or a text field keeps its own keys.
 */
export function installPianoRollKeys(root: HTMLElement): () => void {
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.defaultPrevented || event.isComposing) return
    const focused = document.activeElement
    const ours =
      focused === null || focused === document.body || root.contains(focused)
    if (!ours) return
    const id = matchEvent(pianoRollKeymap(useUiStore.getState().keymap), event)
    if (id === undefined) return
    const action = registry.get(id)
    // A disabled action leaves the key to whatever the app's keymap says.
    if (!action || !isEnabled(action, getAppState())) return
    event.preventDefault()
    if (event.repeat && !action.repeats) return
    void runAction(id)
  }
  document.addEventListener("keydown", onKeyDown)
  return () => document.removeEventListener("keydown", onKeyDown)
}

function isPlainSpace(event: KeyboardEvent): boolean {
  return (
    (event.key === " " || event.code === "Space") &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.metaKey &&
    !event.shiftKey
  )
}

/**
 * Keeps Space for the transport inside the piano roll. A focused button or
 * piano key would otherwise press itself, and play would seem to ignore
 * the key after every click. Enter still presses the focused control.
 */
export function installSpacePlays(root: HTMLElement): () => void {
  const owns = (event: KeyboardEvent) =>
    isPlainSpace(event) &&
    !isTypingTarget(event.target) &&
    currentKeymap().byChord.get("Space") !== undefined
  const onKeyDown = (event: KeyboardEvent) => {
    if (!owns(event)) return
    const id = currentKeymap().byChord.get("Space")
    if (id === undefined) return
    event.preventDefault()
    event.stopPropagation()
    if (!event.repeat) void runAction(id)
  }
  const onKeyUp = (event: KeyboardEvent) => {
    if (!owns(event)) return
    event.preventDefault()
    event.stopPropagation()
  }
  root.addEventListener("keydown", onKeyDown, true)
  root.addEventListener("keyup", onKeyUp, true)
  return () => {
    root.removeEventListener("keydown", onKeyDown, true)
    root.removeEventListener("keyup", onKeyUp, true)
  }
}
