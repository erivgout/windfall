import {
  contextSeparator,
  type ContextItem,
} from "@/components/context-actions"

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
  {
    submenu: "Selected-note tools",
    items: [
      "pianoRoll.legato",
      "pianoRoll.staccato",
      "pianoRoll.chop",
      "pianoRoll.glue",
      "pianoRoll.strum",
      "pianoRoll.flipTime",
      "pianoRoll.flipPitch",
      "pianoRoll.keyRange",
      "pianoRoll.scaleVelocity",
    ],
  },
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

/** The tools, for the parts of the panel around the grid. */
const TOOLS: ContextItem = {
  submenu: "Tool",
  items: [
    "pianoRoll.toolDraw",
    "pianoRoll.toolPaint",
    "pianoRoll.toolSelect",
    "pianoRoll.toolErase",
  ],
}

/**
 * Everywhere in the piano roll that has no menu of its own: the value
 * lane, its header, the scrollbars and the corners.
 */
export const PANEL_MENU: ContextItem[] = [
  TOOLS,
  contextSeparator,
  "pianoRoll.paste",
  "pianoRoll.selectAll",
  contextSeparator,
  ...VIEW_MENU,
]

/** The piano roll of a project that has no channel to write notes for. */
export const NO_CHANNEL_MENU: ContextItem[] = [
  "channel.add",
  "channel.addFromFile",
  contextSeparator,
  "view.channelRack",
]
