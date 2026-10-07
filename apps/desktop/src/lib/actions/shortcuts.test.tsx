import { act, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"
import { settle, startTestApp } from "@/test/harness"

import {
  getAppState,
  INSPECTOR_KEEPS,
  installKeymap,
  invalidateActionsOn,
  isEnabled,
  registry,
  shortcutLabel,
  useActionEnabled,
  useShortcutLabel,
  useShortcutScope,
  type Action,
} from "."
import { scopeChain } from "./scope"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

function Mixer() {
  const scope = useShortcutScope("mixer")
  const roll = useShortcutScope("pianoRoll")
  const inspector = useShortcutScope("effectInspector", {
    keeps: INSPECTOR_KEEPS,
  })
  const effect = useShortcutScope("effect")
  return (
    <div aria-label="Mixer" {...scope}>
      <button>Mixer button</button>
      <div aria-label="Effects" {...inspector}>
        <div {...effect}>
          <button>Effect header</button>
        </div>
        <button>Effect setting</button>
        <p>Effect text</p>
      </div>
      <div aria-label="Roll" {...roll}>
        <button>Roll button</button>
      </div>
    </div>
  )
}

/** A window in small: chrome, three panels (one inside another), overlays. */
function Panels({ mixer = true }: { mixer?: boolean }) {
  const rack = useShortcutScope("channelRack")
  return (
    <div>
      <button>Chrome</button>
      <div aria-label="Rack" {...rack}>
        <button onClick={() => clicked("rack button")}>Rack button</button>
        <input aria-label="Rack field" />
        <div role="slider" tabIndex={0} aria-label="Rack knob" />
        <div role="tab" tabIndex={0} aria-label="Rack tab" />
        <div
          tabIndex={0}
          aria-label="Rack list"
          onKeyDown={(event) => {
            if (event.key === "Delete") event.preventDefault()
          }}
        />
      </div>
      {mixer && <Mixer />}
      <div role="dialog">
        <button onClick={() => clicked("dialog button")}>Dialog button</button>
      </div>
      <div role="menu">
        <button role="menuitem">Menu item</button>
      </div>
    </div>
  )
}

const ran: string[] = []
const clicks: string[] = []
const clicked = (name: string) => void clicks.push(name)
let disabled = new Set<string>()

function action(id: string, extra: Partial<Action> = {}): Action {
  return {
    id,
    title: id,
    section: "Test",
    enabled: () => !disabled.has(id),
    run: () => void ran.push(id),
    ...extra,
  }
}

const TEST_ACTIONS: Action[] = [
  action("rack.delete", { scope: "channelRack", defaultShortcut: "Delete" }),
  action("rack.open", { scope: "channelRack", defaultShortcut: "Enter" }),
  action("rack.tool", { scope: "channelRack", defaultShortcut: "D" }),
  action("mixer.delete", {
    scope: "mixer",
    editCommand: "delete",
    defaultShortcut: "Delete",
  }),
  action("mixer.duplicate", {
    scope: "mixer",
    editCommand: "duplicate",
    defaultShortcut: "Mod+D",
  }),
  action("effect.remove", {
    scope: "effect",
    editCommand: "delete",
    defaultShortcut: ["Delete", "Backspace"],
  }),
  action("mixer.rename", { scope: "mixer", defaultShortcut: "F8" }),
  action("roll.delete", { scope: "pianoRoll", defaultShortcut: "Delete" }),
  action("roll.tool", { scope: "pianoRoll", defaultShortcut: "D" }),
  action("roll.nudge", {
    scope: "pianoRoll",
    defaultShortcut: "ArrowRight",
    repeats: true,
  }),
  action("global.find", { defaultShortcut: "Mod+F" }),
  action("global.rename", {}),
]

const TEST_FL = {
  "roll.tool": ["P"],
  "global.rename": ["F8"],
}

let stop: () => void
let unregister: () => void

beforeEach(async () => {
  ran.length = 0
  clicks.length = 0
  disabled = new Set()
  ;({ stop } = await startTestApp())
  unregister = registry.register(TEST_ACTIONS, { presets: { fl: TEST_FL } })
})
afterEach(() => {
  unregister()
  stop()
})

const button = (name: string) => screen.getByRole("button", { name })
const press = (target: Element, init: KeyboardEventInit) =>
  fireEvent.keyDown(target, init)
const DELETE = { key: "Delete", code: "Delete" }
const SPACE = { key: " ", code: "Space" }
const playing = () => useTransportStore.getState().playing

describe("which panel a key goes to", () => {
  it("is the panel the focus is in, innermost first", () => {
    render(<Panels />)
    expect(scopeChain(button("Rack button"))).toEqual(["channelRack"])
    expect(scopeChain(button("Mixer button"))).toEqual(["mixer"])
    expect(scopeChain(button("Roll button"))).toEqual(["pianoRoll", "mixer"])

    press(button("Rack button"), DELETE)
    press(button("Mixer button"), DELETE)
    press(button("Roll button"), DELETE)
    expect(ran).toEqual(["rack.delete", "mixer.delete", "roll.delete"])
  })

  it("falls outward past a disabled action, then leaves the key alone", () => {
    render(<Panels />)
    disabled.add("roll.delete")
    expect(press(button("Roll button"), DELETE)).toBe(false)
    expect(ran).toEqual(["mixer.delete"])

    disabled.add("mixer.delete")
    // Nothing can run, so the event is not cancelled.
    expect(press(button("Roll button"), DELETE)).toBe(true)
    expect(ran).toEqual(["mixer.delete"])
  })

  it("leaves a key no action is bound to alone", () => {
    render(<Panels />)
    expect(press(button("Rack button"), { key: "q", code: "KeyQ" })).toBe(true)
    expect(press(document.body, { key: "Escape", code: "Escape" })).toBe(true)
    expect(ran).toEqual([])
  })

  it("is the center tab in view before any panel was used", () => {
    render(<Panels />)
    expect(scopeChain(document.body)).toEqual(["channelRack"])
    press(document.body, DELETE)
    expect(ran).toEqual(["rack.delete"])

    act(() => useUiStore.getState().showCenterTab("pianoRoll"))
    expect(scopeChain(document.body)).toEqual(["pianoRoll", "mixer"])
    press(document.body, DELETE)
    expect(ran).toEqual(["rack.delete", "roll.delete"])
  })

  it("stays with the panel that was clicked last while the focus is nowhere", () => {
    render(<Panels />)
    fireEvent.pointerDown(screen.getByLabelText("Mixer"))
    expect(useUiStore.getState().activeScope).toBe("mixer")
    press(document.body, DELETE)
    expect(ran).toEqual(["mixer.delete"])

    fireEvent.pointerDown(screen.getByLabelText("Rack"))
    press(document.body, DELETE)
    expect(ran).toEqual(["mixer.delete", "rack.delete"])
  })

  it("follows the focus into a panel, and stays there on a control outside any panel", () => {
    render(<Panels />)
    act(() => button("Mixer button").focus())
    expect(useUiStore.getState().activeScope).toBe("mixer")

    // A transport button is not a panel: the mixer keeps the keyboard.
    act(() => button("Chrome").focus())
    press(button("Chrome"), DELETE)
    press(button("Chrome"), { key: "F8", code: "F8" })
    expect(ran).toEqual(["mixer.delete", "mixer.rename"])
  })

  it("gives a dialog only the global shortcuts", () => {
    render(<Panels />)
    fireEvent.pointerDown(screen.getByLabelText("Mixer"))
    const inDialog = button("Dialog button")
    expect(scopeChain(inDialog)).toEqual([])
    press(inDialog, { key: "F8", code: "F8" })
    expect(ran).toEqual([])
    press(inDialog, { key: "f", code: "KeyF", ctrlKey: true })
    expect(ran).toEqual(["global.find"])
  })

  it("goes back to the center tab when the active panel is taken away", () => {
    const view = render(<Panels />)
    fireEvent.pointerDown(screen.getByLabelText("Mixer"))
    view.rerender(<Panels mixer={false} />)
    expect(useUiStore.getState().activeScope).toBeNull()
    press(document.body, DELETE)
    expect(ran).toEqual(["rack.delete"])
  })
})

describe("an inspector inside a panel", () => {
  const CTRL_D = { key: "d", code: "KeyD", ctrlKey: true }

  it("keeps Delete and Ctrl+D from the panel around it, and uses them up", () => {
    render(<Panels />)
    const setting = button("Effect setting")
    // Not handled by anything, and still not left to the webview.
    expect(press(setting, DELETE)).toBe(false)
    expect(press(setting, CTRL_D)).toBe(false)
    expect(ran).toEqual([])
    // The same keys on the panel itself still mean the panel's own thing.
    press(button("Mixer button"), DELETE)
    press(button("Mixer button"), CTRL_D)
    expect(ran).toEqual(["mixer.delete", "mixer.duplicate"])
  })

  it("lets every other key of the panel through", () => {
    render(<Panels />)
    press(button("Effect setting"), { key: "F8", code: "F8" })
    press(button("Effect setting"), { key: "f", code: "KeyF", ctrlKey: true })
    expect(ran).toEqual(["mixer.rename", "global.find"])
  })

  it("gives the key to an action of its own, innermost first", () => {
    render(<Panels />)
    press(button("Effect header"), DELETE)
    press(button("Effect header"), { key: "Backspace", code: "Backspace" })
    expect(ran).toEqual(["effect.remove", "effect.remove"])
    // With that action off, the key stops at the inspector all the same.
    disabled = new Set(["effect.remove"])
    expect(press(button("Effect header"), DELETE)).toBe(false)
    expect(ran).toEqual(["effect.remove", "effect.remove"])
  })

  it("keeps the keys after a click on a part of it that takes no focus", () => {
    render(<Panels />)
    fireEvent.pointerDown(screen.getByText("Effect text"))
    expect(useUiStore.getState().activeScope).toBe("effectInspector")
    expect(scopeChain(document.body)).toEqual(["effectInspector", "mixer"])
    press(document.body, DELETE)
    expect(ran).toEqual([])
  })

  it("plays and stops with Space, like anywhere else", async () => {
    render(<Panels />)
    press(button("Effect setting"), SPACE)
    await settle()
    expect(playing()).toBe(true)
  })

  it("turns Edit > Delete off instead of handing it to the panel", () => {
    render(<Panels />)
    const edit = registry.get("edit.delete")
    if (!edit) throw new Error("no Edit > Delete")
    fireEvent.pointerDown(button("Mixer button"))
    expect(edit.standsFor?.(getAppState())).toBe("mixer.delete")
    fireEvent.pointerDown(screen.getByText("Effect text"))
    expect(edit.standsFor?.(getAppState())).toBeUndefined()
    expect(isEnabled(edit, getAppState())).toBe(false)
    // On the effect itself it means the effect.
    fireEvent.pointerDown(button("Effect header"))
    expect(edit.standsFor?.(getAppState())).toBe("effect.remove")
  })
})

describe("what the focused control keeps", () => {
  it("lets a control that uses the key have it", () => {
    render(<Panels />)
    const list = screen.getByLabelText("Rack list")
    expect(press(list, DELETE)).toBe(false)
    expect(ran).toEqual([])
    // A key the control does not use still runs the panel's action.
    press(list, { key: "d", code: "KeyD" })
    expect(ran).toEqual(["rack.tool"])
  })

  it("leaves typing alone, but not Ctrl shortcuts", () => {
    render(<Panels />)
    const field = screen.getByLabelText("Rack field")
    expect(press(field, DELETE)).toBe(true)
    expect(press(field, { key: "d", code: "KeyD" })).toBe(true)
    expect(press(field, SPACE)).toBe(true)
    expect(ran).toEqual([])
    expect(playing()).toBe(false)
    expect(press(field, { key: "f", code: "KeyF", ctrlKey: true })).toBe(false)
    expect(ran).toEqual(["global.find"])
  })

  it("presses the focused control with Enter instead of running an Enter shortcut", () => {
    render(<Panels />)
    expect(press(button("Rack button"), { key: "Enter", code: "Enter" })).toBe(
      true
    )
    expect(
      press(screen.getByLabelText("Rack knob"), { key: "Enter", code: "Enter" })
    ).toBe(true)
    expect(ran).toEqual([])
    // With the focus on the panel itself Enter is the panel's.
    press(screen.getByLabelText("Rack"), { key: "Enter", code: "Enter" })
    expect(ran).toEqual(["rack.open"])
  })

  it("leaves plain keys to an open menu", () => {
    render(<Panels />)
    const item = screen.getByRole("menuitem", { name: "Menu item" })
    expect(press(item, DELETE)).toBe(true)
    expect(press(item, SPACE)).toBe(true)
    expect(ran).toEqual([])
    expect(playing()).toBe(false)
  })

  it("runs a held key again only for actions that repeat", () => {
    render(<Panels />)
    const roll = button("Roll button")
    press(roll, { key: "ArrowRight", code: "ArrowRight" })
    press(roll, { key: "ArrowRight", code: "ArrowRight", repeat: true })
    expect(ran).toEqual(["roll.nudge", "roll.nudge"])
    // The repeat is still cancelled, so the page does not act on it.
    expect(press(roll, { ...DELETE, repeat: true })).toBe(false)
    expect(ran).toEqual(["roll.nudge", "roll.nudge"])
  })
})

describe("Space", () => {
  it("plays from a focused button without pressing it", async () => {
    render(<Panels />)
    const target = button("Rack button")
    act(() => target.focus())
    // Both halves are cancelled, and neither reaches the button.
    expect(press(target, SPACE)).toBe(false)
    expect(fireEvent.keyUp(target, SPACE)).toBe(false)
    await settle()
    expect(playing()).toBe(true)
    expect(clicks).toEqual([])

    press(target, SPACE)
    await settle()
    expect(playing()).toBe(false)
  })

  it("plays from a slider, a tab, the page and a control outside any panel", async () => {
    render(<Panels />)
    const targets = [
      screen.getByLabelText("Rack knob"),
      screen.getByLabelText("Rack tab"),
      document.body,
      button("Chrome"),
    ]
    let expected = false
    for (const target of targets) {
      expect(press(target, SPACE)).toBe(false)
      await settle()
      expected = !expected
      expect(playing()).toBe(expected)
    }
  })

  it("is kept from React handlers on the way", async () => {
    const onKeyDown = vi.fn()
    const onKeyUp = vi.fn()
    render(
      <div onKeyDown={onKeyDown} onKeyUp={onKeyUp}>
        <button>Step</button>
      </div>
    )
    press(button("Step"), SPACE)
    fireEvent.keyUp(button("Step"), SPACE)
    press(button("Step"), { key: "Enter", code: "Enter" })
    await settle()
    expect(onKeyDown).toHaveBeenCalledTimes(1)
    expect(onKeyDown.mock.calls[0][0].key).toBe("Enter")
    expect(onKeyUp).not.toHaveBeenCalled()
  })

  it("presses the button in a dialog, which needs it", async () => {
    render(<Panels />)
    const target = button("Dialog button")
    expect(press(target, SPACE)).toBe(true)
    expect(fireEvent.keyUp(target, SPACE)).toBe(true)
    await settle()
    expect(playing()).toBe(false)
  })

  it("does not start again while the key is held", async () => {
    render(<Panels />)
    press(document.body, SPACE)
    expect(press(document.body, { ...SPACE, repeat: true })).toBe(false)
    await settle()
    expect(playing()).toBe(true)
  })

  it("only takes the plain key", async () => {
    render(<Panels />)
    expect(press(button("Rack button"), { ...SPACE, shiftKey: true })).toBe(
      true
    )
    await settle()
    expect(playing()).toBe(false)
  })
})

describe("presets and labels", () => {
  it("shows an action the key it has in its own scope", () => {
    expect(shortcutLabel("rack.delete")).toBe("Del")
    expect(shortcutLabel("mixer.delete")).toBe("Del")
    expect(shortcutLabel("roll.delete")).toBe("Del")
    expect(shortcutLabel("rack.tool")).toBe("D")
    expect(shortcutLabel("roll.tool")).toBe("D")
    expect(shortcutLabel("global.rename")).toBeUndefined()

    act(() => useUiStore.getState().setKeymap("fl"))
    expect(shortcutLabel("roll.tool")).toBe("P")
    expect(shortcutLabel("rack.tool")).toBe("D")
    expect(shortcutLabel("global.rename")).toBe("F8")
    expect(shortcutLabel("mixer.rename")).toBe("F8")
  })

  it("runs a preset's key inside the scope, and the outer one elsewhere", () => {
    render(<Panels />)
    act(() => useUiStore.getState().setKeymap("fl"))
    press(button("Roll button"), { key: "p", code: "KeyP" })
    press(button("Roll button"), { key: "d", code: "KeyD" })
    // F8 renames the mixer's thing in the mixer and the global one elsewhere.
    press(button("Mixer button"), { key: "F8", code: "F8" })
    press(button("Rack button"), { key: "F8", code: "F8" })
    expect(ran).toEqual(["roll.tool", "mixer.rename", "global.rename"])
  })

  it("follows the preset in a mounted label", () => {
    function Label() {
      return <span>{useShortcutLabel("roll.tool") ?? "none"}</span>
    }
    render(<Label />)
    expect(screen.getByText("D")).toBeVisible()
    act(() => useUiStore.getState().setKeymap("fl"))
    expect(screen.getByText("P")).toBeVisible()
  })
})

describe("state actions read from a panel's own store", () => {
  function Enabled({ id }: { id: string }) {
    return <span>{useActionEnabled(id) ? "enabled" : "disabled"}</span>
  }

  it("is read again when the panel invalidates", () => {
    render(<Enabled id="rack.delete" />)
    expect(screen.getByText("enabled")).toBeVisible()
    disabled.add("rack.delete")
    // Nothing told the registry yet.
    expect(screen.getByText("enabled")).toBeVisible()
    act(() => registry.invalidate())
    expect(screen.getByText("disabled")).toBeVisible()
  })

  it("follows the values a panel picks from its store", () => {
    type Listener = (state: Panel, previous: Panel) => void
    type Panel = { selection: number; scroll: number }
    const listeners = new Set<Listener>()
    let state: Panel = { selection: 0, scroll: 0 }
    const store = {
      subscribe(listener: Listener) {
        listeners.add(listener)
        return () => void listeners.delete(listener)
      },
      set(next: Panel) {
        const previous = state
        state = next
        for (const listener of listeners) listener(state, previous)
      },
    }
    const stopFollowing = invalidateActionsOn(store, (panel) => [
      panel.selection,
    ])
    const before = registry.stateVersion()

    store.set({ selection: 0, scroll: 40 })
    expect(registry.stateVersion()).toBe(before)
    store.set({ selection: 2, scroll: 40 })
    expect(registry.stateVersion()).toBe(before + 1)

    stopFollowing()
    store.set({ selection: 3, scroll: 40 })
    expect(registry.stateVersion()).toBe(before + 1)
  })

  it("follows the selection, which is part of the state actions get", () => {
    const remove = registry.register([
      {
        id: "test.needsChannel",
        title: "Needs a channel",
        section: "Test",
        enabled: (state) => state.ui.selectedChannel !== null,
        run() {},
      },
    ])
    render(<Enabled id="test.needsChannel" />)
    expect(screen.getByText("disabled")).toBeVisible()
    act(() => useUiStore.getState().selectChannel(1))
    expect(screen.getByText("enabled")).toBeVisible()
    remove()
  })
})

describe("installing the keymap", () => {
  it("listens once however often it is installed", () => {
    const uninstall = installKeymap()
    render(<Panels />)
    press(button("Rack button"), DELETE)
    expect(ran).toEqual(["rack.delete"])

    // The app's own listener is still there.
    uninstall()
    uninstall()
    press(button("Rack button"), DELETE)
    expect(ran).toEqual(["rack.delete", "rack.delete"])
  })
})
