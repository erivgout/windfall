import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { settle, startTestApp } from "@/test/harness"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { usePromptStore } from "@/lib/store/prompts"
import { useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"

import {
  getAppState,
  installKeymap,
  registry,
  runAction,
  shortcutLabel,
} from "."
import { BUILTIN_ACTIONS, syncRecentActions } from "./builtin"
import { MENUS } from "./menus"
import { createRegistry, isEnabled, type Action } from "./registry"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

const { toast } = await import("sonner")

const usePromptState = () => usePromptStore.getState()

function action(id: string, extra: Partial<Action> = {}): Action {
  return { id, title: id, section: "Test", run() {}, ...extra }
}

describe("registry", () => {
  it("lists actions in the order registered and removes them again", () => {
    const local = createRegistry()
    const remove = local.register([action("a"), action("b")])
    local.register([action("c")])
    expect(local.list().map((item) => item.id)).toEqual(["a", "b", "c"])
    expect(local.get("b")?.id).toBe("b")

    remove()
    expect(local.list().map((item) => item.id)).toEqual(["c"])
    expect(local.get("a")).toBeUndefined()
  })

  it("refuses to register an id twice and adds none of the batch", () => {
    const local = createRegistry()
    local.register([action("a")])
    expect(() => local.register([action("b"), action("a")])).toThrow(
      'Action "a" is registered twice'
    )
    expect(local.get("b")).toBeUndefined()
  })

  it("tells subscribers when the set changes, with a new list each time", () => {
    const local = createRegistry()
    const listener = vi.fn()
    const off = local.subscribe(listener)
    const before = local.list()
    const remove = local.register([action("a")])
    expect(local.list()).not.toBe(before)
    remove()
    off()
    local.register([action("b")])
    expect(listener).toHaveBeenCalledTimes(2)
  })
})

describe("built-in actions", () => {
  let stop: () => void

  beforeEach(async () => {
    vi.mocked(toast.error).mockClear()
    ;({ stop } = await startTestApp())
  })
  afterEach(() => stop())

  it("covers what the shell promises", () => {
    const ids = registry.list().map((item) => item.id)
    for (const id of [
      "file.new",
      "file.open",
      "file.save",
      "file.saveAs",
      "file.export",
      "edit.undo",
      "edit.redo",
      "transport.play",
      "transport.stop",
      "transport.toggle",
      "transport.patternMode",
      "transport.songMode",
      "channel.add",
      "pattern.add",
      "pattern.next",
      "pattern.previous",
      "view.browser",
      "view.mixer",
      "view.channelRack",
      "view.playlist",
      "view.pianoRoll",
      "view.toggleTheme",
      "view.resetLayout",
      "view.commandPalette",
      "options.settings",
    ]) {
      expect(ids).toContain(id)
    }
  })

  it("only puts registered actions in the menus", () => {
    const ids = new Set(BUILTIN_ACTIONS.map((item) => item.id))
    for (const menu of MENUS) {
      for (const entry of menu.entries) {
        if (typeof entry === "string") expect(ids).toContain(entry)
      }
    }
  })

  it("enables undo and redo from the history", async () => {
    const undo = registry.get("edit.undo")
    const redo = registry.get("edit.redo")
    if (!undo || !redo) throw new Error("undo and redo are not registered")

    expect(isEnabled(undo, getAppState())).toBe(false)
    await dispatch({ type: "addPattern" })
    expect(isEnabled(undo, getAppState())).toBe(true)
    expect(isEnabled(redo, getAppState())).toBe(false)

    await runAction("edit.undo")
    expect(isEnabled(redo, getAppState())).toBe(true)
    expect(useProjectStore.getState().project.patterns).toHaveLength(1)
  })

  it("does not run a disabled action", async () => {
    await runAction("edit.undo")
    await runAction("pattern.delete")
    expect(useProjectStore.getState().revision).toBe(0)
    expect(toast.error).not.toHaveBeenCalled()
  })

  it("reports an unknown action and an action that throws", async () => {
    await runAction("nope.nothing")
    expect(toast.error).toHaveBeenCalledWith(
      'There is no action called "nope.nothing".'
    )
    const remove = registry.register([
      action("test.boom", {
        title: "Blow up",
        run() {
          throw new Error("It broke.")
        },
      }),
    ])
    await runAction("test.boom")
    expect(toast.error).toHaveBeenCalledWith("Blow up", {
      description: "It broke.",
    })
    remove()
  })

  it("adds a channel, selects it and adds patterns that become current", async () => {
    await runAction("channel.add")
    const { project } = useProjectStore.getState()
    expect(project.channels).toHaveLength(5)
    expect(useUiStore.getState().selectedChannel).toBe(project.channels[4].id)

    await runAction("pattern.add")
    const patterns = useProjectStore.getState().project.patterns
    expect(useTransportStore.getState().pattern).toBe(patterns[1].id)
    await runAction("pattern.previous")
    expect(useTransportStore.getState().pattern).toBe(patterns[0].id)
    await runAction("pattern.next")
    expect(useTransportStore.getState().pattern).toBe(patterns[1].id)
  })

  it("toggles panels, tabs, theme and layout through the UI store", async () => {
    await runAction("view.mixer")
    await runAction("view.playlist")
    await runAction("view.toggleTheme")
    expect(useUiStore.getState()).toMatchObject({
      panels: { browser: true, mixer: false },
      centerTab: "playlist",
      theme: "light",
    })
    await runAction("view.resetLayout")
    expect(useUiStore.getState()).toMatchObject({
      panels: { browser: true, mixer: true },
      centerTab: "channelRack",
      layoutGeneration: 1,
    })
  })

  it("saves through Save as the first time and asks before discarding edits", async () => {
    await dispatch({ type: "updateSettings", patch: { name: "Song" } })
    await runAction("file.save")
    expect(useProjectStore.getState()).toMatchObject({
      dirty: false,
      path: "/projects/Song.windfall",
    })

    await dispatch({ type: "addPattern" })
    const pending = runAction("file.new")
    await settle()
    expect(usePromptState().confirm?.title).toBe('Save changes to "Song"?')
    usePromptState().confirm?.resolve(null)
    await pending
    expect(useProjectStore.getState().project.settings.name).toBe("Song")

    const again = runAction("file.new")
    await settle()
    usePromptState().confirm?.resolve("discard")
    await again
    expect(useProjectStore.getState().project.channels).toHaveLength(0)
    expect(useProjectStore.getState().path).toBeNull()
  })

  it("registers an action for each recent project", () => {
    syncRecentActions(["/projects/a.windfall", "/projects/b.windfall"])
    expect(registry.get("file.openRecent.1")).toMatchObject({
      title: "b.windfall",
      section: "Recent projects",
    })
    syncRecentActions([])
    expect(registry.get("file.openRecent.0")).toBeUndefined()
  })

  it("runs actions from the keyboard and shows the preset's shortcut", async () => {
    const uninstall = installKeymap()
    expect(shortcutLabel("view.mixer")).toBe("Ctrl+J")

    window.dispatchEvent(
      new KeyboardEvent("keydown", { key: "j", code: "KeyJ", ctrlKey: true })
    )
    await settle()
    expect(useUiStore.getState().panels.mixer).toBe(false)

    useUiStore.getState().setKeymap("fl")
    expect(shortcutLabel("view.mixer")).toBe("F9")
    expect(shortcutLabel("file.new")).toBeUndefined()
    window.dispatchEvent(
      new KeyboardEvent("keydown", { key: "F9", code: "F9" })
    )
    await settle()
    expect(useUiStore.getState().panels.mixer).toBe(true)
    uninstall()
  })
})
