import {
  invalidateActionsOn,
  registry,
  type Action,
  type AppState,
  type PresetShortcuts,
} from "@/lib/actions"

import { LANE_KINDS } from "./lane-math"
import { NOTE_TOOLS, openNoteTools } from "./note-tools"
import { currentSession } from "./session"
import { openNoteProperties, setSelectedArticulation, setSelectedColorGroup } from "./note-properties"
import { openNoteCurves } from "./note-curves"
import { NOTE_COLOR_GROUPS, noteColorGroupLabel } from "@/lib/note-colors"
import { NOTE_ARTICULATIONS } from "@/lib/note-expression"
import { openNoteLfo } from "@/features/automation/lfo-dialog"
import { SNAP_OPTIONS } from "./snap"
import { usePianoRollStore, type Tool } from "./store"
import { openWaveformHelper } from "./waveform-helper"
import { openPatternTimeline } from "./pattern-timeline"

const SECTION = "Piano roll"

const roll = () => usePianoRollStore.getState()
const editor = () => currentSession()?.editor

/*
 * These act on the piano roll in view. What is selected is not part of the
 * state actions are handed; it is read from the piano roll's own store,
 * which `registerPianoRollActions` makes the registry follow.
 */
const inRoll = (state: AppState) => state.ui.centerTab === "pianoRoll"
const hasSelection = (state: AppState) =>
  inRoll(state) && roll().selectionCount > 0
const hasNotes = (state: AppState) =>
  inRoll(state) && (editor()?.notes.length ?? 0) > 0

const TOOL_ACTIONS: {
  tool: Tool
  title: string
  key: string
  words: string
}[] = [
  { tool: "draw", title: "Draw tool", key: "D", words: "pencil add notes" },
  { tool: "paint", title: "Paint tool", key: "B", words: "brush repeat" },
  { tool: "select", title: "Select tool", key: "S", words: "marquee box" },
  { tool: "erase", title: "Erase tool", key: "E", words: "delete rubber" },
]

function capital(word: string): string {
  return word.charAt(0).toUpperCase() + word.slice(1)
}

const ACTIONS: Action[] = [
  {
    id: "pianoRoll.patternTimeline",
    title: "Pattern markers and time signatures…",
    section: SECTION,
    keywords: "local meter change timeline pattern ruler",
    enabled: inRoll,
    run: () => openPatternTimeline(),
  },
  {
    id: "pianoRoll.waveformHelper",
    title: "Waveform helper…",
    section: SECTION,
    keywords: "audio waveform background timing reference",
    enabled: inRoll,
    run: openWaveformHelper,
  },
  {
    id: "pianoRoll.lfo",
    title: "Write note-event LFO…",
    section: SECTION,
    keywords: "sine triangle pulse random sample hold modulation event lane",
    enabled: hasNotes,
    run: openNoteLfo,
  },
  ...[null, ...NOTE_COLOR_GROUPS.map((_, group) => group)].flatMap((group): Action[] => [
    {
      id: `pianoRoll.color.${group ?? "auto"}`,
      title: `Set selected notes: ${noteColorGroupLabel(group)}`,
      section: SECTION,
      keywords: "note color group MIDI channel routing",
      enabled: hasSelection,
      run: () => setSelectedColorGroup(group),
    },
    {
      id: `pianoRoll.selectColor.${group ?? "auto"}`,
      title: `Select notes: ${noteColorGroupLabel(group)}`,
      section: SECTION,
      keywords: "note color group MIDI channel selection",
      enabled: hasNotes,
      run: () => {
        const current = editor()
        if (!current || current.busy) return
        current.setSelection(current.notes.filter((note) => (note.expression?.colorGroup ?? null) === group).map((note) => note.id))
        currentSession()?.focusGrid()
      },
    },
  ]),
  ...NOTE_ARTICULATIONS.map((item): Action => ({
    id: `pianoRoll.articulation.${item.value}`,
    title: `Set selected notes: ${item.label.toLowerCase()}`,
    section: SECTION,
    keywords: "slide portamento glide articulation pitch bend",
    enabled: hasSelection,
    run: () => setSelectedArticulation(item.value),
  })),
  {
    id: "pianoRoll.noteProperties",
    title: "Note properties…",
    section: SECTION,
    defaultShortcut: "Alt+Enter",
    keywords: "velocity pan pitch release modulation start length dialog",
    enabled: hasSelection,
    whyDisabled: () => "Select notes in the piano roll",
    run: openNoteProperties,
  },
  { id: "pianoRoll.noteCurves", title: "Note expression curves…", section: SECTION, keywords: "continuous pan pitch release modulation envelope per note", enabled: hasSelection, whyDisabled: () => "Select source notes", run: openNoteCurves },
  ...TOOL_ACTIONS.map(({ tool, title, key, words }): Action => ({
    id: `pianoRoll.tool${capital(tool)}`,
    title,
    section: SECTION,
    defaultShortcut: key,
    keywords: words,
    enabled: inRoll,
    checked: () => roll().tool === tool,
    run: () => {
      editor()?.cancel()
      roll().setTool(tool)
      currentSession()?.focusGrid()
    },
  })),

  {
    id: "pianoRoll.selectAll",
    title: "Select all notes",
    section: SECTION,
    editCommand: "selectAll",
    defaultShortcut: "Mod+A",
    enabled: hasNotes,
    run: () => editor()?.selectAll(),
  },
  {
    id: "pianoRoll.deselect",
    title: "Deselect all notes",
    section: SECTION,
    defaultShortcut: "Escape",
    keywords: "clear selection cancel",
    enabled: (state) =>
      inRoll(state) && (roll().selectionCount > 0 || (editor()?.busy ?? false)),
    run: () => {
      const current = editor()
      if (!current) return
      // Escape in the middle of a drag gives the drag up instead.
      if (current.busy) current.cancel()
      else current.setSelection([])
    },
  },
  {
    id: "pianoRoll.delete",
    title: "Delete selected notes",
    section: SECTION,
    editCommand: "delete",
    defaultShortcut: ["Delete", "Backspace"],
    keywords: "remove erase",
    enabled: hasSelection,
    run: () => editor()?.deleteSelection(),
  },
  {
    id: "pianoRoll.cut",
    title: "Cut notes",
    section: SECTION,
    editCommand: "cut",
    defaultShortcut: "Mod+X",
    enabled: hasSelection,
    run: () => editor()?.cut(),
  },
  {
    id: "pianoRoll.copy",
    title: "Copy notes",
    section: SECTION,
    editCommand: "copy",
    defaultShortcut: "Mod+C",
    enabled: hasSelection,
    run: () => {
      editor()?.copy()
    },
  },
  {
    id: "pianoRoll.paste",
    title: "Paste notes",
    section: SECTION,
    editCommand: "paste",
    defaultShortcut: "Mod+V",
    keywords: "clipboard insert",
    enabled: (state) => inRoll(state) && roll().clipboardCount > 0,
    run: () => {
      const session = currentSession()
      return session?.editor.paste(session.pasteTarget())
    },
  },
  {
    id: "pianoRoll.duplicate",
    title: "Duplicate notes to the right",
    section: SECTION,
    editCommand: "duplicate",
    defaultShortcut: "Mod+D",
    keywords: "repeat clone copy",
    enabled: hasSelection,
    run: () => editor()?.duplicate(),
  },

  {
    id: "pianoRoll.quantize",
    title: "Quantize selected note starts…",
    section: SECTION,
    defaultShortcut: "Mod+Q",
    keywords: "align grid timing",
    enabled: hasSelection,
    whyDisabled: () => "Select notes first",
    run: () => openNoteTools("quantize", "start"),
  },
  {
    id: "pianoRoll.quantizeEnds",
    title: "Quantize selected note ends…",
    section: SECTION,
    defaultShortcut: "Alt+Q",
    keywords: "align grid length",
    enabled: hasSelection,
    whyDisabled: () => "Select notes first",
    run: () => openNoteTools("quantize", "end"),
  },
  ...NOTE_TOOLS.filter((tool) => tool.value !== "quantize").map(
    (tool): Action => ({
      id: `pianoRoll.${tool.value}`,
      title: `${tool.label} selected notes…`,
      section: SECTION,
      keywords: tool.description,
      enabled: hasSelection,
      whyDisabled: () => "Select notes first",
      run: () => openNoteTools(tool.value),
    })
  ),

  {
    id: "pianoRoll.nudgeLeft",
    title: "Move notes left by one snap step",
    section: SECTION,
    defaultShortcut: "ArrowLeft",
    repeats: true,
    keywords: "nudge earlier",
    enabled: hasSelection,
    run: () => editor()?.nudge(-1, 0),
  },
  {
    id: "pianoRoll.nudgeRight",
    title: "Move notes right by one snap step",
    section: SECTION,
    defaultShortcut: "ArrowRight",
    repeats: true,
    keywords: "nudge later",
    enabled: hasSelection,
    run: () => editor()?.nudge(1, 0),
  },
  {
    id: "pianoRoll.transposeUp",
    title: "Move notes up a semitone",
    section: SECTION,
    defaultShortcut: "ArrowUp",
    repeats: true,
    keywords: "transpose pitch",
    enabled: hasSelection,
    run: () => editor()?.nudge(0, 1),
  },
  {
    id: "pianoRoll.transposeDown",
    title: "Move notes down a semitone",
    section: SECTION,
    defaultShortcut: "ArrowDown",
    repeats: true,
    keywords: "transpose pitch",
    enabled: hasSelection,
    run: () => editor()?.nudge(0, -1),
  },
  {
    id: "pianoRoll.octaveUp",
    title: "Transpose up an octave",
    section: SECTION,
    defaultShortcut: "Shift+ArrowUp",
    repeats: true,
    keywords: "pitch 12 semitones",
    enabled: hasSelection,
    whyDisabled: () => "Select notes first",
    run: () => editor()?.transpose(12),
  },
  {
    id: "pianoRoll.octaveDown",
    title: "Transpose down an octave",
    section: SECTION,
    defaultShortcut: "Shift+ArrowDown",
    repeats: true,
    keywords: "pitch 12 semitones",
    enabled: hasSelection,
    whyDisabled: () => "Select notes first",
    run: () => editor()?.transpose(-12),
  },

  {
    id: "pianoRoll.zoomFit",
    title: "Zoom to fit all notes",
    section: SECTION,
    defaultShortcut: "Mod+0",
    keywords: "view whole pattern",
    enabled: inRoll,
    run: () => currentSession()?.zoomToFit(),
  },
  {
    id: "pianoRoll.zoomSelection",
    title: "Zoom to the selection",
    section: SECTION,
    defaultShortcut: "Mod+Shift+0",
    keywords: "view fit",
    enabled: hasSelection,
    run: () => currentSession()?.zoomToSelection(),
  },
  {
    id: "pianoRoll.ghosts",
    title: "Ghost notes",
    section: SECTION,
    defaultShortcut: "Alt+G",
    keywords: "show hide other channels",
    checked: () => roll().ghosts,
    run: () => roll().setGhosts(!roll().ghosts),
  },
  {
    id: "pianoRoll.follow",
    title: "Follow the playhead",
    section: SECTION,
    keywords: "scroll auto playback",
    checked: () => roll().follow,
    run: () => roll().setFollow(!roll().follow),
  },

  {
    id: "pianoRoll.editGhosts",
    title: "Edit ghost notes",
    section: SECTION,
    defaultShortcut: "Alt+Shift+G",
    keywords: "other channels source lane ghost editing",
    enabled: inRoll,
    checked: () => roll().editGhosts,
    run: () => roll().setEditGhosts(!roll().editGhosts),
  },

  ...SNAP_OPTIONS.map((option): Action => ({
    id: `pianoRoll.snap.${option.id}`,
    title:
      option.id === "none"
        ? "Turn snap off"
        : `Snap to ${option.label.toLowerCase()}`,
    section: SECTION,
    keywords: "grid magnet",
    checked: () => roll().snap === option.id,
    run: () => roll().setSnap(option.id),
  })),
  ...LANE_KINDS.map((kind): Action => ({
    id: `pianoRoll.lane.${kind.id}`,
    title: `Show ${kind.label.toLowerCase()} under the notes`,
    section: SECTION,
    keywords: "lane bars",
    checked: () => roll().laneKind === kind.id,
    run: () => roll().setLaneKind(kind.id),
  })),
]

/**
 * Every action here belongs to the piano roll: its keys work while the
 * piano roll has the keyboard, and other panels may use the same keys.
 */
export const PIANO_ROLL_ACTIONS: Action[] = ACTIONS.map((action) => ({
  ...action,
  scope: "pianoRoll",
}))

/**
 * FL Studio's piano roll shortcuts, from the shortcut list in its manual.
 * FL moves the selection with Shift and the arrows; the plain arrows are
 * kept as well. Actions not listed keep their Windfall shortcut. The
 * preset changes keys only: the mouse modifiers are those of
 * `lib/edit-modifiers` in both presets (FL equivalent: Shift+drag clones a
 * note, which here is Ctrl+drag, the same as on the playlist).
 */
export const PIANO_ROLL_FL_KEYMAP: PresetShortcuts = {
  "pianoRoll.toolDraw": ["P"],
  "pianoRoll.toolPaint": ["B"],
  "pianoRoll.toolErase": ["D"],
  "pianoRoll.toolSelect": ["E"],
  "pianoRoll.deselect": ["Mod+D", "Escape"],
  "pianoRoll.duplicate": ["Mod+B"],
  "pianoRoll.nudgeLeft": ["Shift+ArrowLeft", "ArrowLeft"],
  "pianoRoll.nudgeRight": ["Shift+ArrowRight", "ArrowRight"],
  "pianoRoll.transposeUp": ["Shift+ArrowUp", "ArrowUp"],
  "pianoRoll.transposeDown": ["Shift+ArrowDown", "ArrowDown"],
  "pianoRoll.octaveUp": ["Mod+ArrowUp"],
  "pianoRoll.octaveDown": ["Mod+ArrowDown"],
  "pianoRoll.ghosts": ["Alt+V"],
}

/** Adds the piano roll's actions to the registry. Returns a function that removes them. */
export function registerPianoRollActions(): () => void {
  const stops = [
    registry.register(PIANO_ROLL_ACTIONS, {
      presets: { fl: PIANO_ROLL_FL_KEYMAP },
    }),
    invalidateActionsOn(usePianoRollStore, (state) => [
      state.tool,
      state.snap,
      state.ghosts,
      state.follow,
      state.laneKind,
      state.selectionCount,
      state.clipboardCount,
    ]),
  ]
  return () => {
    for (const stop of stops) stop()
  }
}
