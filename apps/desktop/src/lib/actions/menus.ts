import { RECENT_SECTION } from "./builtin"

export type MenuEntry =
  /** An action id. */
  | string
  | { separator: true }
  /** A submenu holding every action of a section, such as recent projects. */
  | { submenu: string; section: string; empty: string }

export type MenuSpec = { title: string; entries: MenuEntry[] }

const separator = { separator: true } as const

/** The menu bar. Titles, shortcuts and state come from the registry. */
export const MENUS: MenuSpec[] = [
  {
    title: "File",
    entries: [
      "file.new",
      "file.open",
      {
        submenu: "Open recent",
        section: RECENT_SECTION,
        empty: "No recent projects",
      },
      separator,
      "file.save",
      "file.saveAs",
      separator,
      "file.export",
    ],
  },
  {
    title: "Edit",
    entries: [
      "edit.undo",
      "edit.redo",
      separator,
      "channel.add",
      separator,
      "pattern.add",
      "pattern.duplicate",
      "pattern.rename",
      "pattern.delete",
    ],
  },
  {
    title: "View",
    entries: [
      "view.commandPalette",
      separator,
      "view.browser",
      "view.mixer",
      separator,
      "view.channelRack",
      "view.playlist",
      "view.pianoRoll",
      separator,
      "view.toggleTheme",
      "view.resetLayout",
    ],
  },
  {
    title: "Options",
    entries: [
      "options.settings",
      separator,
      "options.keymapWindfall",
      "options.keymapFl",
    ],
  },
  {
    title: "Help",
    entries: ["help.shortcuts", "help.about"],
  },
]
