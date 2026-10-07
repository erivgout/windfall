import type { KeymapPreset } from "@/lib/store/ui"

import type { Action } from "./registry"

/*
 * A shortcut is written as modifiers and one key joined by "+", such as
 * "Mod+Shift+Z". "Mod" is Ctrl on Windows and Linux and Cmd on macOS. Keys
 * are letters, digits, F1 to F24, Space, Comma, Enter, Escape, Delete, the
 * arrows (ArrowUp...) and numpad keys by their code (NumpadAdd...).
 */

/**
 * FL Studio's default shortcuts for the actions Windfall has, taken from the
 * shortcut list in the FL Studio manual. `null` leaves an action without a
 * key: FL uses Ctrl+N for "Save new version", so it must not start a new
 * project. Actions not listed keep their Windfall shortcut when it is free.
 */
export const FL_KEYMAP: Record<string, string[] | null> = {
  "file.new": null,
  "file.open": ["Mod+O"],
  "file.save": ["Mod+S"],
  "file.saveAs": ["Mod+Shift+S"],
  "file.export": ["Mod+R"],
  "edit.undo": ["Mod+Alt+Z", "Mod+Z"],
  "transport.toggle": ["Space"],
  "transport.toggleMode": ["L"],
  "pattern.next": ["NumpadAdd"],
  "pattern.previous": ["NumpadSubtract"],
  "pattern.add": ["F4"],
  "pattern.rename": ["F2"],
  "view.playlist": ["F5"],
  "view.channelRack": ["F6"],
  "view.pianoRoll": ["F7"],
  "view.browser": ["Alt+F8"],
  "view.mixer": ["F9"],
  "options.settings": ["F10"],
}

/**
 * Adds FL Studio shortcuts for a panel's own actions, in the same form as
 * `FL_KEYMAP`. Call it before the panel registers those actions.
 */
export function addFlShortcuts(shortcuts: Record<string, string[] | null>) {
  Object.assign(FL_KEYMAP, shortcuts)
}

export const KEYMAP_PRESETS: {
  id: KeymapPreset
  name: string
  about: string
}[] = [
  { id: "windfall", name: "Windfall", about: "Shortcuts most apps share." },
  {
    id: "fl",
    name: "FL Studio",
    about: "FL Studio's defaults, for people switching over.",
  },
]

export function detectMac(): boolean {
  return (
    typeof navigator !== "undefined" &&
    /mac|iphone|ipad/i.test(navigator.platform)
  )
}

const MODIFIER_ORDER = ["Ctrl", "Alt", "Shift", "Meta"] as const
type Modifier = (typeof MODIFIER_ORDER)[number]

function chord(modifiers: Set<Modifier>, key: string): string {
  return [...MODIFIER_ORDER.filter((name) => modifiers.has(name)), key].join(
    "+"
  )
}

/** Rewrites a shortcut into the one spelling events are compared against. */
export function normalizeShortcut(shortcut: string, isMac: boolean): string {
  const parts = shortcut.split("+").map((part) => part.trim())
  const key = parts.pop() ?? ""
  const modifiers = new Set<Modifier>()
  for (const part of parts) {
    const name = part.toLowerCase()
    if (name === "mod") modifiers.add(isMac ? "Meta" : "Ctrl")
    else if (name === "ctrl" || name === "control") modifiers.add("Ctrl")
    else if (name === "alt" || name === "option") modifiers.add("Alt")
    else if (name === "shift") modifiers.add("Shift")
    else if (name === "meta" || name === "cmd") modifiers.add("Meta")
    else throw new Error(`Unknown modifier "${part}" in shortcut "${shortcut}"`)
  }
  return chord(modifiers, key.length === 1 ? key.toUpperCase() : key)
}

type KeyEventLike = Pick<
  KeyboardEvent,
  "key" | "code" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey"
>

function keyName(event: KeyEventLike): string | null {
  const { key, code } = event
  if (["Control", "Alt", "Shift", "Meta", "AltGraph"].includes(key)) return null
  if (code.startsWith("Numpad")) return code
  if (code.startsWith("Digit")) return code.slice(5)
  if (code === "Space" || key === " ") return "Space"
  if (/^F\d{1,2}$/.test(code)) return code
  if (key === ",") return "Comma"
  // The printed letter wins so shortcuts follow the keyboard layout. With
  // Alt held macOS prints a symbol instead, and then the key's place decides.
  if (/^[a-z]$/i.test(key)) return key.toUpperCase()
  if (/^Key[A-Z]$/.test(code)) return code.slice(3)
  return key
}

/** The shortcut an event stands for, or null for a bare modifier key. */
export function eventChord(event: KeyEventLike): string | null {
  const key = keyName(event)
  if (key === null) return null
  const modifiers = new Set<Modifier>()
  if (event.ctrlKey) modifiers.add("Ctrl")
  if (event.altKey) modifiers.add("Alt")
  if (event.shiftKey) modifiers.add("Shift")
  if (event.metaKey) modifiers.add("Meta")
  return chord(modifiers, key)
}

const KEY_LABELS: Record<string, string> = {
  Comma: ",",
  NumpadAdd: "Num +",
  NumpadSubtract: "Num -",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Escape: "Esc",
  Delete: "Del",
}

const MAC_MODIFIERS: Record<Modifier, string> = {
  Ctrl: "⌃",
  Alt: "⌥",
  Shift: "⇧",
  Meta: "⌘",
}

/** A shortcut the way the platform writes it: `Ctrl+Shift+Z` or `⇧⌘Z`. */
export function formatShortcut(shortcut: string, isMac: boolean): string {
  const parts = normalizeShortcut(shortcut, isMac).split("+")
  const key = parts.pop() ?? ""
  const label = KEY_LABELS[key] ?? key
  if (isMac) {
    const symbols = parts
      .map((part) => MAC_MODIFIERS[part as Modifier])
      .join("")
    return label.length > 1 && symbols ? `${symbols} ${label}` : symbols + label
  }
  return [...parts.map((part) => (part === "Meta" ? "Win" : part)), label].join(
    "+"
  )
}

function shortcutList(value: string | string[] | undefined): string[] {
  if (value === undefined) return []
  return Array.isArray(value) ? value : [value]
}

export type Keymap = {
  /** Normalized chord to the id of the action it runs. */
  byChord: Map<string, string>
  /** Action id to its shortcuts as written, the main one first. */
  byAction: Map<string, string[]>
}

/**
 * Works out which key runs which action. A preset's own bindings come
 * first; an action the preset does not mention keeps its Windfall shortcut
 * unless the preset already uses that key for something else.
 */
export function resolveKeymap(
  actions: Action[],
  preset: KeymapPreset,
  isMac: boolean
): Keymap {
  const byChord = new Map<string, string>()
  const byAction = new Map<string, string[]>()
  const overrides = preset === "fl" ? FL_KEYMAP : {}
  const known = new Set(actions.map((action) => action.id))

  const bind = (id: string, shortcuts: string[]) => {
    for (const shortcut of shortcuts) {
      const normalized = normalizeShortcut(shortcut, isMac)
      if (byChord.has(normalized)) continue
      byChord.set(normalized, id)
      byAction.set(id, [...(byAction.get(id) ?? []), shortcut])
    }
  }

  for (const [id, shortcuts] of Object.entries(overrides)) {
    if (shortcuts && known.has(id)) bind(id, shortcuts)
  }
  for (const action of actions) {
    if (action.id in overrides) continue
    bind(action.id, shortcutList(action.defaultShortcut))
  }
  return { byChord, byAction }
}

const TEXT_EDITING_KEYS = new Set(["Z", "Y", "A", "C", "X", "V"])

const TEXT_ENTRY_SELECTOR =
  "input, textarea, select, [contenteditable=''], [contenteditable='true'], [role='textbox'], [role='combobox']"

const OVERLAY_SELECTOR =
  "[role='menu'], [role='listbox'], [role='dialog'], [role='alertdialog']"

const ACTIVATED_BY_KEY_SELECTOR =
  "button, a[href], summary, [role='button'], [role='tab'], [role='menuitem'], [role='option'], [role='switch'], [role='checkbox'], [role='radio'], [role='slider'], [role='separator']"

function splitChord(normalized: string) {
  const modifiers = normalized.split("+")
  const key = modifiers.pop() ?? ""
  return {
    key,
    isFunctionKey: /^F\d{1,2}$/.test(key),
    hasCommand: modifiers.includes("Ctrl") || modifiers.includes("Meta"),
    hasAlt: modifiers.includes("Alt"),
  }
}

/**
 * Whether a shortcut may fire while the user types in a field. Plain keys
 * must reach the field, and so must the editing shortcuts every text box
 * has, such as undo and paste.
 */
export function firesWhileTyping(normalized: string): boolean {
  const { key, isFunctionKey, hasCommand, hasAlt } = splitChord(normalized)
  if (isFunctionKey) return true
  if (!hasCommand) return false
  return !(TEXT_EDITING_KEYS.has(key) && !hasAlt)
}

/**
 * Whether a shortcut may fire while a menu, popover or dialog has the focus.
 * Those use plain keys to move around, so only shortcuts with Ctrl or Cmd
 * and the function keys get through.
 */
export function firesInOverlay(normalized: string): boolean {
  const { isFunctionKey, hasCommand } = splitChord(normalized)
  return isFunctionKey || hasCommand
}

export function isTypingTarget(target: EventTarget | null): boolean {
  return (
    target instanceof Element && target.closest(TEXT_ENTRY_SELECTOR) !== null
  )
}

function isOverlayTarget(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest(OVERLAY_SELECTOR) !== null
}

/** True when the focused control uses this key itself, as a button uses Space. */
export function targetUsesKey(
  target: EventTarget | null,
  normalized: string
): boolean {
  if (!(target instanceof Element)) return false
  if (normalized !== "Space" && normalized !== "Enter") return false
  return target.closest(ACTIVATED_BY_KEY_SELECTOR) !== null
}

/** The action a key press should run, if any. */
export function matchEvent(
  keymap: Keymap,
  event: KeyboardEvent
): string | undefined {
  const normalized = eventChord(event)
  if (normalized === null) return undefined
  if (isTypingTarget(event.target)) {
    if (!firesWhileTyping(normalized)) return undefined
  } else if (isOverlayTarget(event.target) && !firesInOverlay(normalized)) {
    return undefined
  }
  if (targetUsesKey(event.target, normalized)) return undefined
  return keymap.byChord.get(normalized)
}
