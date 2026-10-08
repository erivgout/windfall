import type { PatternId, PlaylistTrack } from "@/bindings"
import {
  invalidateActionsOn,
  registry,
  type Action,
  type AppState,
  type PresetShortcuts,
} from "@/lib/actions"

import { activeMetrics, activeSession, zoomBy, zoomToFit } from "./active"
import type { Tool } from "./intents"
import { patchSelectedAudioClips, selectedAudioClips } from "./audio/ops"
import {
  addTrack,
  copySelection,
  cutSelection,
  deleteSelection,
  deleteTrack,
  duplicateSelection,
  insertTrack,
  moveTrackBy,
  nudgeSelection,
  openPattern,
  paste,
  playSong,
  renameTrack,
  selectAll,
  toggleLoopSong,
  toggleMuteSelection,
  toggleTrackMute,
} from "./ops"
import { playlist, selectedClips } from "./selectors"
import { SNAP_MODES } from "./snap"
import { usePlaylistStore } from "./store"
import { TIMELINE_ACTIONS } from "./timeline-actions"
import { useTimelineStore } from "./timeline-store"

const SECTION = "Playlist"

const ui = () => usePlaylistStore.getState()

/*
 * These act on the playlist in view. The selection and the target track
 * are not part of the state actions are handed; they are read from the
 * playlist's own store, which `registerPlaylistActions` makes the registry
 * follow.
 */
const inPlaylist = (state: AppState) => state.ui.centerTab === "playlist"
const hasSelection = (state: AppState) =>
  inPlaylist(state) && ui().selection.size > 0
const hasClips = (state: AppState) =>
  inPlaylist(state) && state.document.project.playlist.clips.length > 0

function targetTrack(): PlaylistTrack | undefined {
  const id = ui().targetTrack
  return playlist().tracks.find((track) => track.id === id)
}

const hasTarget = () => targetTrack() !== undefined

/** The pattern of the one selected clip, when that clip plays a pattern. */
function selectedPatternClip(): PatternId | null {
  if (ui().selection.size !== 1) return null
  const [clip] = selectedClips()
  return clip?.content.type === "pattern" ? clip.content.pattern : null
}

function targetIndex(): number {
  const id = ui().targetTrack
  return playlist().tracks.findIndex((track) => track.id === id)
}

/** Runs `work` on the track whose header was last pressed, if there is one. */
function withTarget(work: (track: PlaylistTrack) => void | Promise<void>) {
  return () => {
    const track = targetTrack()
    if (track) return work(track)
  }
}

const TOOL_ACTIONS: {
  tool: Tool
  title: string
  key: string
  flKey: string
  words: string
}[] = [
  {
    tool: "draw",
    title: "Draw tool",
    key: "D",
    flKey: "P",
    words: "pencil place clips",
  },
  {
    tool: "paint",
    title: "Paint tool",
    key: "B",
    flKey: "B",
    words: "brush repeat fill",
  },
  {
    tool: "select",
    title: "Select tool",
    key: "S",
    flKey: "E",
    words: "marquee box",
  },
  {
    tool: "erase",
    title: "Erase tool",
    key: "E",
    flKey: "D",
    words: "delete rubber",
  },
  {
    tool: "mute",
    title: "Mute tool",
    key: "M",
    flKey: "T",
    words: "silence clips",
  },
]

function capital(word: string): string {
  return word.charAt(0).toUpperCase() + word.slice(1)
}

export const toolActionId = (tool: Tool) => `playlist.tool${capital(tool)}`

const ACTIONS: Action[] = [
  ...TIMELINE_ACTIONS,
  ...TOOL_ACTIONS.map(({ tool, title, key, words }): Action => ({
    id: toolActionId(tool),
    title,
    section: SECTION,
    defaultShortcut: key,
    keywords: words,
    enabled: inPlaylist,
    checked: () => ui().tool === tool,
    run: () => ui().setTool(tool),
  })),
  ...SNAP_MODES.map(({ mode, label }): Action => ({
    id: `playlist.snap${capital(mode)}`,
    title: mode === "none" ? "Snap off" : `Snap to ${label.toLowerCase()}s`,
    section: SECTION,
    keywords: "grid quantize magnet",
    checked: () => ui().snap === mode,
    run: () => ui().setSnap(mode),
  })),

  {
    id: "playlist.selectAll",
    title: "Select all clips",
    section: SECTION,
    editCommand: "selectAll",
    defaultShortcut: "Mod+A",
    enabled: hasClips,
    run: selectAll,
  },
  {
    id: "playlist.deselect",
    title: "Deselect clips",
    section: SECTION,
    defaultShortcut: "Escape",
    keywords: "clear selection cancel",
    enabled: (state) =>
      inPlaylist(state) &&
      (ui().selection.size > 0 || (activeSession()?.busy ?? false)),
    run: () => {
      const session = activeSession()
      if (session?.busy) session.cancel()
      else ui().clearSelection()
    },
  },
  {
    id: "playlist.deleteClips",
    title: "Delete clips",
    section: SECTION,
    editCommand: "delete",
    defaultShortcut: ["Delete", "Backspace"],
    keywords: "remove",
    enabled: hasSelection,
    run: deleteSelection,
  },
  {
    id: "playlist.cut",
    title: "Cut clips",
    section: SECTION,
    editCommand: "cut",
    defaultShortcut: "Mod+X",
    enabled: hasSelection,
    run: cutSelection,
  },
  {
    id: "playlist.copy",
    title: "Copy clips",
    section: SECTION,
    editCommand: "copy",
    defaultShortcut: "Mod+C",
    enabled: hasSelection,
    run: () => {
      copySelection()
    },
  },
  {
    id: "playlist.paste",
    title: "Paste clips at the song position",
    section: SECTION,
    editCommand: "paste",
    defaultShortcut: "Mod+V",
    enabled: (state) => inPlaylist(state) && ui().clipboard.length > 0,
    run: paste,
  },
  {
    id: "playlist.duplicate",
    title: "Duplicate clips to the right",
    section: SECTION,
    editCommand: "duplicate",
    defaultShortcut: "Mod+D",
    keywords: "clone copy repeat",
    enabled: hasSelection,
    run: duplicateSelection,
  },
  {
    id: "playlist.muteClips",
    title: "Mute clips",
    section: SECTION,
    defaultShortcut: "Shift+M",
    keywords: "silence unmute",
    enabled: hasSelection,
    checked: () => {
      const clips = selectedClips()
      return clips.length > 0 && clips.every((clip) => clip.muted)
    },
    run: toggleMuteSelection,
  },
  {
    id: "playlist.nudgeLeft",
    title: "Move clips left",
    section: SECTION,
    defaultShortcut: "ArrowLeft",
    repeats: true,
    keywords: "nudge earlier",
    enabled: hasSelection,
    run: () => nudgeSelection(-1, 0),
  },
  {
    id: "playlist.nudgeRight",
    title: "Move clips right",
    section: SECTION,
    defaultShortcut: "ArrowRight",
    repeats: true,
    keywords: "nudge later",
    enabled: hasSelection,
    run: () => nudgeSelection(1, 0),
  },
  {
    id: "playlist.moveUp",
    title: "Move clips up a track",
    section: SECTION,
    defaultShortcut: "ArrowUp",
    repeats: true,
    enabled: hasSelection,
    run: () => nudgeSelection(0, -1),
  },
  {
    id: "playlist.moveDown",
    title: "Move clips down a track",
    section: SECTION,
    defaultShortcut: "ArrowDown",
    repeats: true,
    enabled: hasSelection,
    run: () => nudgeSelection(0, 1),
  },
  {
    id: "playlist.editPattern",
    title: "Edit the clip's pattern",
    section: SECTION,
    defaultShortcut: "Enter",
    keywords: "open channel rack",
    enabled: (state) => inPlaylist(state) && selectedPatternClip() !== null,
    whyDisabled: (state) =>
      inPlaylist(state) &&
      ui().selection.size === 1 &&
      selectedPatternClip() === null
        ? "Pattern clips only"
        : undefined,
    run: () => {
      const pattern = selectedPatternClip()
      if (pattern !== null) return openPattern(pattern)
    },
  },

  {
    id: "playlist.reverseClips",
    title: "Reverse audio clips",
    section: SECTION,
    keywords: "backwards flip",
    enabled: (state) => inPlaylist(state) && selectedAudioClips().length > 0,
    whyDisabled: (state) =>
      inPlaylist(state) && ui().selection.size > 0
        ? "Audio clips only"
        : undefined,
    checked: () => {
      const clips = selectedAudioClips()
      return (
        clips.length > 0 &&
        clips.every(
          (clip) => clip.content.type === "audio" && clip.content.reverse
        )
      )
    },
    run: async () => {
      const clips = selectedAudioClips()
      const all = clips.every(
        (clip) => clip.content.type === "audio" && clip.content.reverse
      )
      await patchSelectedAudioClips({ reverse: !all })
    },
  },
  {
    id: "playlist.clipInspector",
    title: "Audio clip settings",
    section: SECTION,
    keywords: "inspector gain pan pitch fade route show hide",
    checked: () => ui().inspectorOpen,
    run: () => ui().toggleInspector(),
  },
  {
    id: "playlist.tallTracks",
    title: "Tall tracks",
    section: SECTION,
    keywords: "row height automation curve zoom lanes",
    enabled: (state) => inPlaylist(state) && activeMetrics() !== null,
    checked: () => activeMetrics()?.tall ?? false,
    run: () => {
      activeMetrics()?.toggleTall()
      registry.invalidate()
    },
  },

  {
    id: "playlist.zoomToFit",
    title: "Zoom to fit the song",
    section: SECTION,
    defaultShortcut: "Mod+0",
    keywords: "show all whole",
    enabled: (state) => inPlaylist(state) && activeMetrics() !== null,
    run: zoomToFit,
  },
  {
    id: "playlist.zoomIn",
    title: "Zoom in on the timeline",
    section: SECTION,
    defaultShortcut: "Mod+=",
    repeats: true,
    enabled: (state) => inPlaylist(state) && activeMetrics() !== null,
    run: () => zoomBy(1.25),
  },
  {
    id: "playlist.zoomOut",
    title: "Zoom out of the timeline",
    section: SECTION,
    defaultShortcut: "Mod+-",
    repeats: true,
    enabled: (state) => inPlaylist(state) && activeMetrics() !== null,
    run: () => zoomBy(0.8),
  },
  {
    id: "playlist.follow",
    title: "Follow the playhead",
    section: SECTION,
    keywords: "scroll auto",
    checked: () => ui().follow,
    run: () => ui().toggleFollow(),
  },
  {
    id: "playlist.patterns",
    title: "Clip list",
    section: SECTION,
    keywords: "show hide toggle picker brush patterns audio automation",
    checked: () => ui().pickerOpen,
    run: () => ui().togglePicker(),
  },
  {
    id: "playlist.loopSong",
    title: "Loop the song",
    section: SECTION,
    keywords: "repeat end restart",
    checked: (state) => state.transport.loopSong,
    run: toggleLoopSong,
  },
  {
    id: "playlist.playSong",
    title: "Play the song now",
    section: SECTION,
    keywords: "song mode start playlist",
    enabled: (state) =>
      !(state.transport.mode === "song" && state.transport.playing),
    run: playSong,
  },

  {
    id: "playlist.addTrack",
    title: "Add playlist track",
    section: SECTION,
    keywords: "new lane row",
    run: addTrack,
  },
  {
    id: "playlist.insertTrack",
    title: "Insert a track above",
    section: SECTION,
    keywords: "new lane row",
    enabled: hasTarget,
    run: withTarget((track) =>
      insertTrack(playlist().tracks.findIndex((item) => item.id === track.id))
    ),
  },
  {
    id: "playlist.moveTrackUp",
    title: "Move playlist track up",
    section: SECTION,
    defaultShortcut: "Alt+ArrowUp",
    keywords: "reorder lane row",
    enabled: () => targetIndex() > 0,
    run: withTarget((track) => moveTrackBy(track.id, -1)),
  },
  {
    id: "playlist.moveTrackDown",
    title: "Move playlist track down",
    section: SECTION,
    defaultShortcut: "Alt+ArrowDown",
    keywords: "reorder lane row",
    enabled: () => {
      const index = targetIndex()
      return index >= 0 && index < playlist().tracks.length - 1
    },
    run: withTarget((track) => moveTrackBy(track.id, 1)),
  },
  {
    id: "playlist.renameTrack",
    title: "Rename playlist track…",
    section: SECTION,
    defaultShortcut: "F2",
    enabled: hasTarget,
    run: withTarget((track) => renameTrack(track.id)),
  },
  {
    id: "playlist.muteTrack",
    title: "Mute playlist track",
    section: SECTION,
    keywords: "silence lamp",
    enabled: hasTarget,
    checked: () => targetTrack()?.muted ?? false,
    run: withTarget((track) => toggleTrackMute(track.id)),
  },
  {
    id: "playlist.deleteTrack",
    title: "Delete playlist track…",
    section: SECTION,
    keywords: "remove lane row",
    enabled: hasTarget,
    run: withTarget((track) => deleteTrack(track.id)),
  },
]

/**
 * Every action here belongs to the playlist: its keys work while the
 * playlist has the keyboard, and other panels may use the same keys.
 */
export const PLAYLIST_ACTIONS: Action[] = ACTIONS.map((action) => ({
  ...action,
  scope: "playlist",
}))

/**
 * Where FL Studio's playlist uses another key than Windfall. The preset
 * changes keys only: the mouse modifiers are those of `lib/edit-modifiers`
 * in both presets (FL equivalent: Shift+drag clones a clip, which here is
 * Ctrl+drag, the same as in the piano roll).
 */
export const PLAYLIST_FL_KEYMAP: PresetShortcuts = {
  ...Object.fromEntries(
    TOOL_ACTIONS.map(({ tool, flKey }) => [toolActionId(tool), [flKey]])
  ),
  "playlist.duplicate": ["Mod+B"],
  "playlist.muteClips": ["Alt+M"],
}

/** Adds the playlist's actions to the registry. Returns a function that removes them. */
export function registerPlaylistActions(): () => void {
  const stops = [
    registry.register(PLAYLIST_ACTIONS, {
      presets: { fl: PLAYLIST_FL_KEYMAP },
    }),
    invalidateActionsOn(usePlaylistStore, (state) => [
      state.tool,
      state.snap,
      state.follow,
      state.pickerOpen,
      state.inspectorOpen,
      state.selection,
      state.clipboard,
      state.targetTrack,
    ]),
    invalidateActionsOn(useTimelineStore, (state) => [
      state.tool,
      state.selection,
      state.active,
    ]),
  ]
  return () => {
    for (const stop of stops) stop()
  }
}
