import { describe, expect, it } from "vitest"

import { BUILTIN_ACTIONS } from "./builtin"
import {
  eventChord,
  FL_KEYMAP,
  firesInOverlay,
  firesWhileTyping,
  formatShortcut,
  matchEvent,
  normalizeShortcut,
  resolveKeymap,
} from "./keymap"
import type { Action } from "./registry"

function action(id: string, defaultShortcut?: string | string[]): Action {
  return { id, title: id, section: "Test", defaultShortcut, run() {} }
}

function key(init: KeyboardEventInit, target: Element = document.body) {
  const event = new KeyboardEvent("keydown", { bubbles: true, ...init })
  target.dispatchEvent(event)
  return event
}

describe("shortcut spelling", () => {
  it("turns Mod into Ctrl or Cmd for the platform", () => {
    expect(normalizeShortcut("Mod+Shift+z", false)).toBe("Ctrl+Shift+Z")
    expect(normalizeShortcut("Mod+Shift+Z", true)).toBe("Shift+Meta+Z")
    expect(normalizeShortcut("shift+alt+F8", false)).toBe("Alt+Shift+F8")
    expect(() => normalizeShortcut("Hyper+A", false)).toThrow(
      "Unknown modifier"
    )
  })

  it("formats the way each platform writes shortcuts", () => {
    expect(formatShortcut("Mod+Shift+Z", false)).toBe("Ctrl+Shift+Z")
    expect(formatShortcut("Mod+Shift+Z", true)).toBe("⇧⌘Z")
    expect(formatShortcut("Mod+Comma", false)).toBe("Ctrl+,")
    expect(formatShortcut("NumpadAdd", false)).toBe("Num +")
    expect(formatShortcut("Mod+ArrowDown", true)).toBe("⌘↓")
    expect(formatShortcut("Alt+F8", true)).toBe("⌥ F8")
    expect(formatShortcut("Space", true)).toBe("Space")
  })
})

describe("reading key events", () => {
  it("names the key the same way shortcuts are written", () => {
    const chord = (init: KeyboardEventInit) =>
      eventChord(new KeyboardEvent("keydown", init))
    expect(chord({ key: "z", code: "KeyZ", ctrlKey: true })).toBe("Ctrl+Z")
    expect(
      chord({ key: "Z", code: "KeyZ", ctrlKey: true, shiftKey: true })
    ).toBe("Ctrl+Shift+Z")
    expect(chord({ key: " ", code: "Space" })).toBe("Space")
    expect(chord({ key: "F8", code: "F8", altKey: true })).toBe("Alt+F8")
    expect(chord({ key: "+", code: "NumpadAdd" })).toBe("NumpadAdd")
    expect(chord({ key: ",", code: "Comma", metaKey: true })).toBe("Meta+Comma")
    expect(chord({ key: "¡", code: "Digit1", altKey: true })).toBe("Alt+1")
    expect(
      chord({ key: "Shift", code: "ShiftLeft", shiftKey: true })
    ).toBeNull()
  })

  it("follows the printed letter, and the key's place when Alt changes it", () => {
    const chord = (init: KeyboardEventInit) =>
      eventChord(new KeyboardEvent("keydown", init))
    // AZERTY: the key printed Z sits where QWERTY has W.
    expect(chord({ key: "z", code: "KeyW", ctrlKey: true })).toBe("Ctrl+Z")
    // macOS: Option+C prints "ç".
    expect(chord({ key: "ç", code: "KeyC", altKey: true })).toBe("Alt+C")
  })
})

describe("presets", () => {
  const actions = [
    action("edit.undo", "Mod+Z"),
    action("edit.redo", ["Mod+Shift+Z", "Mod+Y"]),
    action("file.new", "Mod+N"),
    action("view.mixer", "Mod+J"),
    action("view.commandPalette", "Mod+K"),
    action("takes.f9", "F9"),
  ]

  it("uses each action's own shortcuts in the Windfall preset", () => {
    const keymap = resolveKeymap(actions, "windfall", false)
    expect(keymap.byChord.get("Ctrl+Z")).toBe("edit.undo")
    expect(keymap.byChord.get("Ctrl+Y")).toBe("edit.redo")
    expect(keymap.byAction.get("edit.redo")).toEqual(["Mod+Shift+Z", "Mod+Y"])
    expect(resolveKeymap(actions, "windfall", true).byChord.get("Meta+Z")).toBe(
      "edit.undo"
    )
  })

  it("puts FL Studio's shortcuts first in the FL preset", () => {
    const keymap = resolveKeymap(actions, "fl", false)
    expect(keymap.byChord.get("Ctrl+Alt+Z")).toBe("edit.undo")
    expect(keymap.byChord.get("Ctrl+Z")).toBe("edit.undo")
    expect(keymap.byChord.get("F9")).toBe("view.mixer")
    expect(keymap.byAction.get("view.mixer")).toEqual(["F9"])
  })

  it("keeps Windfall shortcuts FL does not use, and drops the ones it does", () => {
    const keymap = resolveKeymap(actions, "fl", false)
    expect(keymap.byChord.get("Ctrl+K")).toBe("view.commandPalette")
    expect(keymap.byChord.get("Ctrl+Shift+Z")).toBe("edit.redo")
    expect(keymap.byAction.has("takes.f9")).toBe(false)
  })

  it("leaves New project without a key in the FL preset", () => {
    const keymap = resolveKeymap(actions, "fl", false)
    expect(keymap.byAction.has("file.new")).toBe(false)
    expect(keymap.byChord.has("Ctrl+N")).toBe(false)
  })

  it("only maps actions that exist, with no key bound twice", () => {
    const ids = new Set(BUILTIN_ACTIONS.map((item) => item.id))
    for (const id of Object.keys(FL_KEYMAP)) expect(ids).toContain(id)

    for (const preset of ["windfall", "fl"] as const) {
      const keymap = resolveKeymap(BUILTIN_ACTIONS, preset, false)
      const bound = [...keymap.byAction.values()].flat()
      expect(bound).toHaveLength(keymap.byChord.size)
    }
    const windfall = resolveKeymap(BUILTIN_ACTIONS, "windfall", false)
    const declared = BUILTIN_ACTIONS.flatMap(
      (item) => item.defaultShortcut ?? []
    )
    expect(windfall.byChord.size).toBe(declared.length)
  })
})

describe("matching", () => {
  const keymap = resolveKeymap(
    [
      action("transport.toggle", "Space"),
      action("edit.undo", "Mod+Z"),
      action("file.save", "Mod+S"),
      action("view.mixer", "F9"),
      action("transport.toggleMode", "L"),
    ],
    "windfall",
    false
  )

  function inside(html: string): Element {
    document.body.innerHTML = html
    const target = document.body.querySelector("[data-target]")
    if (!target) throw new Error("no target in the markup")
    return target
  }

  it("matches a key press on the page", () => {
    expect(matchEvent(keymap, key({ key: " ", code: "Space" }))).toBe(
      "transport.toggle"
    )
    expect(
      matchEvent(keymap, key({ key: "s", code: "KeyS", ctrlKey: true }))
    ).toBe("file.save")
    expect(matchEvent(keymap, key({ key: "q", code: "KeyQ" }))).toBeUndefined()
  })

  it("lets typing reach a text field", () => {
    const input = inside("<input data-target />")
    expect(
      matchEvent(keymap, key({ key: " ", code: "Space" }, input))
    ).toBeUndefined()
    expect(
      matchEvent(keymap, key({ key: "l", code: "KeyL" }, input))
    ).toBeUndefined()
    expect(
      matchEvent(keymap, key({ key: "z", code: "KeyZ", ctrlKey: true }, input))
    ).toBeUndefined()
    expect(
      matchEvent(keymap, key({ key: "s", code: "KeyS", ctrlKey: true }, input))
    ).toBe("file.save")
    expect(matchEvent(keymap, key({ key: "F9", code: "F9" }, input))).toBe(
      "view.mixer"
    )
  })

  it("leaves plain keys to an open menu or dialog but not Ctrl shortcuts", () => {
    const item = inside('<div role="menu"><div data-target></div></div>')
    expect(
      matchEvent(keymap, key({ key: "l", code: "KeyL" }, item))
    ).toBeUndefined()
    expect(
      matchEvent(keymap, key({ key: "z", code: "KeyZ", ctrlKey: true }, item))
    ).toBe("edit.undo")
  })

  it("does not steal Space from a focused button", () => {
    const button = inside("<button data-target>Mute</button>")
    expect(
      matchEvent(keymap, key({ key: " ", code: "Space" }, button))
    ).toBeUndefined()
    expect(matchEvent(keymap, key({ key: "l", code: "KeyL" }, button))).toBe(
      "transport.toggleMode"
    )
  })

  it("knows which shortcuts are safe where", () => {
    expect(firesWhileTyping("Ctrl+S")).toBe(true)
    expect(firesWhileTyping("Ctrl+V")).toBe(false)
    expect(firesWhileTyping("Ctrl+Alt+Z")).toBe(true)
    expect(firesWhileTyping("Alt+C")).toBe(false)
    expect(firesInOverlay("Ctrl+Z")).toBe(true)
    expect(firesInOverlay("Space")).toBe(false)
  })
})
