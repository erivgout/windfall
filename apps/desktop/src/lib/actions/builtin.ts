import { toast } from "sonner"

import {
  addChannel,
  addPattern,
  deletePattern,
  duplicatePattern,
  renamePattern,
  stepPattern,
} from "@/lib/flows/edit"
import {
  newProject,
  openProject,
  openProjectPath,
  saveProject,
  saveProjectAs,
} from "@/lib/flows/project"
import { redo, undo } from "@/lib/store/project"
import { selectedPatternId } from "@/lib/store/selectors"
import {
  play,
  setPlayMode,
  stop,
  togglePlayback,
  useTransportStore,
} from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"
import { fileName } from "@/lib/time"

import { registry, type Action, type AppState } from "./registry"

const ui = () => useUiStore.getState()

function patternIndex(state: AppState): number {
  const id = selectedPatternId(state.document.project, state.transport.pattern)
  return state.document.project.patterns.findIndex((item) => item.id === id)
}

/** The actions the shell itself provides. Panels register their own. */
export const BUILTIN_ACTIONS: Action[] = [
  {
    id: "file.new",
    title: "New project",
    section: "File",
    defaultShortcut: "Mod+N",
    run: newProject,
  },
  {
    id: "file.open",
    title: "Open project…",
    section: "File",
    defaultShortcut: "Mod+O",
    run: openProject,
  },
  {
    id: "file.save",
    title: "Save",
    section: "File",
    defaultShortcut: "Mod+S",
    keywords: "write project",
    run: async () => {
      await saveProject()
    },
  },
  {
    id: "file.saveAs",
    title: "Save as…",
    section: "File",
    defaultShortcut: "Mod+Shift+S",
    run: async () => {
      await saveProjectAs()
    },
  },
  {
    id: "file.export",
    title: "Export audio…",
    section: "File",
    defaultShortcut: "Mod+E",
    keywords: "render bounce wav",
    run: () => ui().openDialog("export"),
  },

  {
    id: "edit.undo",
    title: "Undo",
    section: "Edit",
    defaultShortcut: "Mod+Z",
    repeats: true,
    enabled: (state) => state.document.history.cursor > 0,
    run: undo,
  },
  {
    id: "edit.redo",
    title: "Redo",
    section: "Edit",
    defaultShortcut: ["Mod+Shift+Z", "Mod+Y"],
    repeats: true,
    enabled: (state) =>
      state.document.history.cursor < state.document.history.entries.length,
    run: redo,
  },

  {
    id: "transport.toggle",
    title: "Play or stop",
    section: "Transport",
    defaultShortcut: "Space",
    keywords: "start pause playback",
    run: togglePlayback,
  },
  {
    id: "transport.play",
    title: "Play",
    section: "Transport",
    enabled: (state) => !state.transport.playing,
    run: play,
  },
  {
    id: "transport.stop",
    title: "Stop",
    section: "Transport",
    run: stop,
  },
  {
    id: "transport.patternMode",
    title: "Play the pattern",
    section: "Transport",
    keywords: "pattern mode loop",
    checked: (state) => state.transport.mode === "pattern",
    run: () => setPlayMode("pattern"),
  },
  {
    id: "transport.songMode",
    title: "Play the song",
    section: "Transport",
    keywords: "song mode playlist",
    checked: (state) => state.transport.mode === "song",
    run: () => setPlayMode("song"),
  },
  {
    id: "transport.toggleMode",
    title: "Switch between pattern and song",
    section: "Transport",
    defaultShortcut: "L",
    run: () =>
      setPlayMode(
        useTransportStore.getState().mode === "pattern" ? "song" : "pattern"
      ),
  },

  {
    id: "channel.add",
    title: "Add channel",
    section: "Channels",
    defaultShortcut: "Alt+C",
    keywords: "new sampler instrument",
    run: addChannel,
  },

  {
    id: "pattern.add",
    title: "Add pattern",
    section: "Patterns",
    defaultShortcut: "Alt+P",
    keywords: "new",
    run: addPattern,
  },
  {
    id: "pattern.next",
    title: "Next pattern",
    section: "Patterns",
    defaultShortcut: "Mod+ArrowDown",
    enabled: (state) =>
      patternIndex(state) < state.document.project.patterns.length - 1,
    run: () => stepPattern(1),
  },
  {
    id: "pattern.previous",
    title: "Previous pattern",
    section: "Patterns",
    defaultShortcut: "Mod+ArrowUp",
    enabled: (state) => patternIndex(state) > 0,
    run: () => stepPattern(-1),
  },
  {
    id: "pattern.rename",
    title: "Rename pattern…",
    section: "Patterns",
    run: () => renamePattern(),
  },
  {
    id: "pattern.duplicate",
    title: "Duplicate pattern",
    section: "Patterns",
    keywords: "clone copy",
    run: () => duplicatePattern(),
  },
  {
    id: "pattern.delete",
    title: "Delete pattern",
    section: "Patterns",
    keywords: "remove",
    enabled: (state) => state.document.project.patterns.length > 1,
    run: () => deletePattern(),
  },

  {
    id: "view.commandPalette",
    title: "Command palette…",
    section: "View",
    defaultShortcut: "Mod+K",
    keywords: "search actions shortcuts",
    run: () => {
      if (ui().dialog === "palette") ui().closeDialog()
      else ui().openDialog("palette")
    },
  },
  {
    id: "view.browser",
    title: "Browser",
    section: "View",
    defaultShortcut: "Mod+B",
    keywords: "show hide toggle samples files panel",
    checked: (state) => state.ui.panels.browser,
    run: () => ui().togglePanel("browser"),
  },
  {
    id: "view.mixer",
    title: "Mixer",
    section: "View",
    defaultShortcut: "Mod+J",
    keywords: "show hide toggle panel",
    checked: (state) => state.ui.panels.mixer,
    run: () => ui().togglePanel("mixer"),
  },
  {
    id: "view.channelRack",
    title: "Channel rack",
    section: "View",
    defaultShortcut: "Alt+1",
    keywords: "show step sequencer",
    checked: (state) => state.ui.centerTab === "channelRack",
    run: () => ui().showCenterTab("channelRack"),
  },
  {
    id: "view.playlist",
    title: "Playlist",
    section: "View",
    defaultShortcut: "Alt+2",
    keywords: "show song timeline arrangement",
    checked: (state) => state.ui.centerTab === "playlist",
    run: () => ui().showCenterTab("playlist"),
  },
  {
    id: "view.pianoRoll",
    title: "Piano roll",
    section: "View",
    defaultShortcut: "Alt+3",
    keywords: "show notes",
    checked: (state) => state.ui.centerTab === "pianoRoll",
    run: () => ui().showCenterTab("pianoRoll"),
  },
  {
    id: "view.toggleTheme",
    title: "Switch between light and dark",
    section: "View",
    defaultShortcut: "Mod+Shift+L",
    keywords: "theme appearance",
    run: () => ui().toggleTheme(),
  },
  {
    id: "view.resetLayout",
    title: "Reset layout",
    section: "View",
    keywords: "panels default restore",
    run: () => ui().resetLayout(),
  },

  {
    id: "options.settings",
    title: "Settings…",
    section: "Options",
    defaultShortcut: "Mod+Comma",
    keywords: "preferences audio device theme keymap",
    run: () => ui().openDialog("settings"),
  },
  {
    id: "options.keymapWindfall",
    title: "Use Windfall shortcuts",
    section: "Options",
    keywords: "keymap keyboard preset",
    checked: (state) => state.ui.keymap === "windfall",
    run: () => ui().setKeymap("windfall"),
  },
  {
    id: "options.keymapFl",
    title: "Use FL Studio shortcuts",
    section: "Options",
    keywords: "keymap keyboard preset",
    checked: (state) => state.ui.keymap === "fl",
    run: () => ui().setKeymap("fl"),
  },

  {
    id: "help.shortcuts",
    title: "Keyboard shortcuts",
    section: "Help",
    keywords: "keys keymap",
    run: () => ui().openDialog("palette"),
  },
  {
    id: "help.about",
    title: "About Windfall",
    section: "Help",
    keywords: "version license",
    run: () => {
      toast("Windfall 0.1.0", {
        description: "A free digital audio workstation. Open source, GPL-3.0.",
      })
    },
  },
]

export const RECENT_SECTION = "Recent projects"

function recentActions(paths: string[]): Action[] {
  return paths.map((path, index) => ({
    id: `file.openRecent.${index}`,
    title: fileName(path),
    section: RECENT_SECTION,
    keywords: `open ${path}`,
    run: () => openProjectPath(path),
  }))
}

let removeRecent: (() => void) | null = null

/** Keeps one "open recent" action per recent project file. */
export function syncRecentActions(paths: string[]) {
  removeRecent?.()
  removeRecent =
    paths.length > 0 ? registry.register(recentActions(paths)) : null
}

export function registerBuiltinActions(): () => void {
  const remove = registry.register(BUILTIN_ACTIONS)
  return () => {
    remove()
    removeRecent?.()
    removeRecent = null
  }
}
