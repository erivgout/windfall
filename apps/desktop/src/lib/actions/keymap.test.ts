import { describe, expect, it } from "vitest"

import { BUILTIN_ACTIONS } from "./builtin"
import {
  eventChord,
  FL_KEYMAP,
  firesInOverlay,
  firesWhileTyping,
  formatShortcut,
  isTransportChord,
  normalizeShortcut,
  resolveChord,
  resolveInChain,
  resolveKeymap,
  shortcutAllowed,
} from "./keymap"
import type { Action } from "./registry"

function action(
  id: string,
  defaultShortcut?: string | string[],
  scope?: Action["scope"]
): Action {
  return { id, title: id, section: "Test", defaultShortcut, scope, run() {} }
}

const everything = () => true

describe("shortcut spelling", () => {
  it("turns Mod into Ctrl or Cmd for the platform", () => {
    expect(normalizeShortcut("Mod+Shift+z", false)).toBe("Ctrl+Shift+Z")
    expect(normalizeShortcut("Mod+Shift+Z", true)).toBe("Shift+Meta+Z")
    expect(normalizeShortcut("shift+alt+F8", false)).toBe("Alt+Shift+F8")
    expect(() => normalizeShortcut("Hyper+A", false)).toThrow(
      "Unknown modifier"
    )
  })

  it("spells a shortcut one way however its modifiers were written", () => {
    expect(normalizeShortcut("Shift+Mod+Alt+A", false)).toBe("Ctrl+Alt+Shift+A")
    expect(normalizeShortcut("Cmd+Option+a", true)).toBe("Alt+Meta+A")
    expect(normalizeShortcut("control+A", true)).toBe("Ctrl+A")
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

  it("matches Mod to Ctrl on Windows and to Cmd on macOS", () => {
    const save = [action("file.save", "Mod+S")]
    const ctrl = eventChord(
      new KeyboardEvent("keydown", { key: "s", code: "KeyS", ctrlKey: true })
    )
    const cmd = eventChord(
      new KeyboardEvent("keydown", { key: "s", code: "KeyS", metaKey: true })
    )
    if (ctrl === null || cmd === null) throw new Error("no chord")

    const windows = resolveKeymap(save, {}, false)
    expect(resolveChord(windows, ctrl, [], everything)).toBe("file.save")
    expect(resolveChord(windows, cmd, [], everything)).toBeUndefined()

    const mac = resolveKeymap(save, {}, true)
    expect(resolveChord(mac, cmd, [], everything)).toBe("file.save")
    expect(resolveChord(mac, ctrl, [], everything)).toBeUndefined()
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
  const global = (keymap: ReturnType<typeof resolveKeymap>) =>
    keymap.byScope.get("global") ?? new Map<string, string>()

  it("uses each action's own shortcuts in the Windfall preset", () => {
    const keymap = resolveKeymap(actions, {}, false)
    expect(global(keymap).get("Ctrl+Z")).toBe("edit.undo")
    expect(global(keymap).get("Ctrl+Y")).toBe("edit.redo")
    expect(keymap.byAction.get("edit.redo")).toEqual(["Mod+Shift+Z", "Mod+Y"])
    expect(global(resolveKeymap(actions, {}, true)).get("Meta+Z")).toBe(
      "edit.undo"
    )
  })

  it("puts FL Studio's shortcuts first in the FL preset", () => {
    const keymap = resolveKeymap(actions, FL_KEYMAP, false)
    expect(global(keymap).get("Ctrl+Alt+Z")).toBe("edit.undo")
    expect(global(keymap).get("Ctrl+Z")).toBe("edit.undo")
    expect(global(keymap).get("F9")).toBe("view.mixer")
    expect(keymap.byAction.get("view.mixer")).toEqual(["F9"])
  })

  it("keeps Windfall shortcuts FL does not use, and drops the ones it does", () => {
    const keymap = resolveKeymap(actions, FL_KEYMAP, false)
    expect(global(keymap).get("Ctrl+K")).toBe("view.commandPalette")
    expect(global(keymap).get("Ctrl+Shift+Z")).toBe("edit.redo")
    expect(keymap.byAction.has("takes.f9")).toBe(false)
  })

  it("leaves New project without a key in the FL preset", () => {
    const keymap = resolveKeymap(actions, FL_KEYMAP, false)
    expect(keymap.byAction.has("file.new")).toBe(false)
    expect(global(keymap).has("Ctrl+N")).toBe(false)
  })

  it("only maps actions that exist, with no key bound twice", () => {
    const ids = new Set(BUILTIN_ACTIONS.map((item) => item.id))
    for (const id of Object.keys(FL_KEYMAP)) expect(ids).toContain(id)

    for (const preset of [{}, FL_KEYMAP]) {
      const keymap = resolveKeymap(BUILTIN_ACTIONS, preset, false)
      const bound = [...keymap.byAction.values()].flat()
      expect(bound).toHaveLength(global(keymap).size)
    }
    const windfall = resolveKeymap(BUILTIN_ACTIONS, {}, false)
    const declared = BUILTIN_ACTIONS.flatMap(
      (item) => item.defaultShortcut ?? []
    )
    expect(global(windfall).size).toBe(declared.length)
  })
})

describe("scopes", () => {
  const actions = [
    action("channel.delete", "Delete", "channelRack"),
    action("channel.duplicate", "Mod+D", "channelRack"),
    action("notes.delete", ["Delete", "Backspace"], "pianoRoll"),
    action("notes.duplicate", "Mod+D", "pianoRoll"),
    action("notes.draw", "D", "pianoRoll"),
    action("clips.draw", "D", "playlist"),
    action("clips.duplicate", "Mod+D", "playlist"),
    action("view.browser", "Mod+B"),
    action("pattern.rename"),
    action("channel.rename", "F2", "channelRack"),
  ]
  const fl = {
    "notes.draw": ["P"],
    "clips.draw": ["P"],
    "notes.duplicate": ["Mod+B"],
    "clips.duplicate": ["Mod+B"],
    "view.browser": ["Alt+F8"],
    "pattern.rename": ["F2"],
  }

  it("lets every panel bind the same key to its own action", () => {
    const keymap = resolveKeymap(actions, {}, false)
    expect(keymap.byScope.get("channelRack")?.get("Delete")).toBe(
      "channel.delete"
    )
    expect(keymap.byScope.get("pianoRoll")?.get("Delete")).toBe("notes.delete")
    expect(keymap.byScope.get("global")?.has("Delete")).toBe(false)
    // Each action keeps its shortcut for its label, whoever else uses it.
    for (const id of ["channel.duplicate", "notes.duplicate"]) {
      expect(keymap.byAction.get(id)).toEqual(["Mod+D"])
    }
    expect(keymap.byAction.get("clips.duplicate")).toEqual(["Mod+D"])
  })

  it("runs the action of the innermost scope that binds the key", () => {
    const keymap = resolveKeymap(actions, {}, false)
    const run = (scopes: Action["scope"][], chord = "Ctrl+D") =>
      resolveChord(
        keymap,
        chord,
        scopes.flatMap((scope) => scope ?? []),
        everything
      )
    expect(run(["pianoRoll"])).toBe("notes.duplicate")
    expect(run(["channelRack"])).toBe("channel.duplicate")
    expect(run(["playlist"])).toBe("clips.duplicate")
    expect(run(["mixer"])).toBeUndefined()
    expect(run([])).toBeUndefined()
    // A scope inside another one is asked first.
    expect(run(["pianoRoll", "channelRack"], "Delete")).toBe("notes.delete")
    expect(run(["channelRack", "pianoRoll"], "Delete")).toBe("channel.delete")
    // What no scope binds is looked up among the global shortcuts.
    expect(run(["pianoRoll"], "Ctrl+B")).toBe("view.browser")
  })

  it("falls outward past an action that cannot run", () => {
    const keymap = resolveKeymap(actions, {}, false)
    const without = (disabled: string) => (id: string) => id !== disabled
    expect(
      resolveChord(
        keymap,
        "Delete",
        ["pianoRoll", "channelRack"],
        without("notes.delete")
      )
    ).toBe("channel.delete")
    expect(
      resolveChord(keymap, "Delete", ["pianoRoll"], without("notes.delete"))
    ).toBeUndefined()
    expect(
      resolveChord(keymap, "Ctrl+B", ["pianoRoll"], without("view.browser"))
    ).toBeUndefined()
  })

  it("lets an inner scope keep an Edit command from the scopes around it", () => {
    const keymap = resolveKeymap(actions, {}, false)
    const edits: Record<string, "delete" | "duplicate"> = {
      "channel.delete": "delete",
      "channel.duplicate": "duplicate",
    }
    const describe = (id: string) => ({ enabled: true, editCommand: edits[id] })
    const inspector = { scope: "rackInspector", keeps: ["delete"] } as const
    const rack = { scope: "channelRack", keeps: [] } as const
    const run = (chord: string, chain = [inspector, rack]) =>
      resolveInChain(keymap, chord, chain, describe)

    // Delete would have deleted the channel: it is used up instead.
    expect(run("Delete")).toEqual({ kept: true })
    // What the inspector does not keep goes on to the rack as before.
    expect(run("Ctrl+D")).toEqual({ action: "channel.duplicate" })
    expect(run("F2")).toEqual({ action: "channel.rename" })
    // Global shortcuts are no Edit commands, so they always get through.
    expect(run("Ctrl+B")).toEqual({ action: "view.browser" })
    // Without the inspector in the way the key deletes the channel.
    expect(run("Delete", [rack])).toEqual({ action: "channel.delete" })
    // A key nothing binds is nobody's.
    expect(run("F9")).toBeUndefined()
  })

  it("gives an inner scope's own action the key it keeps from the others", () => {
    const withEffect = [...actions, action("effect.remove", "Delete", "effect")]
    const keymap = resolveKeymap(withEffect, {}, false)
    const describe = (id: string) => ({
      enabled: id !== "disabled",
      editCommand:
        id.endsWith("delete") || id.endsWith("remove")
          ? ("delete" as const)
          : undefined,
    })
    expect(
      resolveInChain(
        keymap,
        "Delete",
        [
          { scope: "effect", keeps: [] },
          { scope: "effectInspector", keeps: ["delete"] },
          { scope: "channelRack", keeps: [] },
        ],
        describe
      )
    ).toEqual({ action: "effect.remove" })
  })

  it("applies a preset's keys inside the action's own scope", () => {
    const keymap = resolveKeymap(actions, fl, false)
    const run = (scope: NonNullable<Action["scope"]>, chord: string) =>
      resolveChord(keymap, chord, [scope], everything)
    // FL duplicates with Ctrl+B in its editors; elsewhere the key is free,
    // because FL shows the browser with Alt+F8.
    expect(run("pianoRoll", "Ctrl+B")).toBe("notes.duplicate")
    expect(run("playlist", "Ctrl+B")).toBe("clips.duplicate")
    expect(run("channelRack", "Ctrl+B")).toBeUndefined()
    expect(run("channelRack", "Alt+F8")).toBe("view.browser")
    // The same letter is a different tool in each editor's own preset.
    expect(run("pianoRoll", "P")).toBe("notes.draw")
    expect(run("playlist", "P")).toBe("clips.draw")
    expect(run("pianoRoll", "D")).toBeUndefined()
    // A panel keeps the Windfall key the preset uses elsewhere: F2 renames
    // the channel in the rack and the pattern everywhere else.
    expect(run("channelRack", "F2")).toBe("channel.rename")
    expect(run("mixer", "F2")).toBe("pattern.rename")
    expect(keymap.byAction.get("channel.duplicate")).toEqual(["Mod+D"])
  })
})

describe("what the focused element allows", () => {
  function inside(html: string): Element {
    document.body.innerHTML = html
    const target = document.body.querySelector("[data-target]")
    if (!target) throw new Error("no target in the markup")
    return target
  }

  it("allows everything on the page itself", () => {
    for (const chord of ["Space", "Ctrl+S", "L", "Enter", "Delete"]) {
      expect(shortcutAllowed(document.body, chord)).toBe(true)
      expect(shortcutAllowed(window, chord)).toBe(true)
    }
  })

  it("lets typing reach a text field", () => {
    for (const html of [
      "<input data-target />",
      "<textarea data-target></textarea>",
      "<div contenteditable='true' data-target></div>",
      "<div role='textbox' data-target></div>",
      "<button role='combobox' data-target></button>",
    ]) {
      const field = inside(html)
      expect(shortcutAllowed(field, "Space")).toBe(false)
      expect(shortcutAllowed(field, "L")).toBe(false)
      expect(shortcutAllowed(field, "Delete")).toBe(false)
      expect(shortcutAllowed(field, "Ctrl+Z")).toBe(false)
      expect(shortcutAllowed(field, "Ctrl+S")).toBe(true)
      expect(shortcutAllowed(field, "F9")).toBe(true)
    }
  })

  it("leaves plain keys to an open menu, list or dialog but not Ctrl shortcuts", () => {
    for (const role of ["menu", "listbox", "dialog", "alertdialog"]) {
      const item = inside(
        `<div role="${role}"><button data-target></button></div>`
      )
      expect(shortcutAllowed(item, "L")).toBe(false)
      expect(shortcutAllowed(item, "Space")).toBe(false)
      expect(shortcutAllowed(item, "Enter")).toBe(false)
      expect(shortcutAllowed(item, "Ctrl+Z")).toBe(true)
    }
  })

  it("takes Space from a focused button, slider, step or tab", () => {
    for (const html of [
      "<button data-target>Mute</button>",
      "<div role='slider' tabindex='0' data-target></div>",
      "<div role='tab' tabindex='0' data-target></div>",
      "<button aria-pressed='true' data-step='3' data-target></button>",
      "<a href='#' data-target>Link</a>",
    ]) {
      expect(shortcutAllowed(inside(html), "Space")).toBe(true)
    }
    expect(isTransportChord("Space")).toBe(true)
    expect(isTransportChord("Shift+Space")).toBe(false)
    expect(isTransportChord("Enter")).toBe(false)
  })

  it("leaves Enter to the focused control, which it presses", () => {
    expect(
      shortcutAllowed(inside("<button data-target></button>"), "Enter")
    ).toBe(false)
    expect(
      shortcutAllowed(inside("<div role='slider' data-target></div>"), "Enter")
    ).toBe(false)
    expect(
      shortcutAllowed(inside("<div tabindex='0' data-target></div>"), "Enter")
    ).toBe(true)
    // Only plain Enter presses a control.
    expect(
      shortcutAllowed(inside("<button data-target></button>"), "Ctrl+Enter")
    ).toBe(true)
    expect(shortcutAllowed(inside("<button data-target></button>"), "L")).toBe(
      true
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
