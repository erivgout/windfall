import type { Channel } from "@/bindings"
import { registry, type Action, type AppState } from "@/lib/actions"
import { selectedPatternId } from "@/lib/store/selectors"
import { useUiStore } from "@/lib/store/ui"

import {
  clearSteps,
  deleteChannel,
  duplicateChannel,
  fillEvery,
  moveChannelBy,
  renameChannel,
  routeToNewTrack,
  selectedChannel,
  setPatternLength,
  shiftSteps,
  showInMixer,
  toggleMute,
  toggleSolo,
} from "./channel-ops"
import { useRackStore } from "./rack-store"

export const LENGTH_PRESETS = [16, 32, 48, 64]
export const FILL_INTERVALS = [2, 4, 8]

/*
 * The selection is not part of the state actions are handed, so it is read
 * from the store. The hooks that follow action state listen to that store
 * too, so buttons and menus still update when the selection changes.
 */
function selected(state: AppState): Channel | undefined {
  const id = useUiStore.getState().selectedChannel
  return state.document.project.channels.find((channel) => channel.id === id)
}

function selectedIndex(state: AppState): number {
  const id = useUiStore.getState().selectedChannel
  return state.document.project.channels.findIndex(
    (channel) => channel.id === id
  )
}

function selectedNoteCount(state: AppState): number {
  const channel = selected(state)
  if (!channel) return 0
  const { project } = state.document
  const patternId = selectedPatternId(project, state.transport.pattern)
  const pattern = project.patterns.find((item) => item.id === patternId)
  return (
    pattern?.lanes.find((lane) => lane.channel === channel.id)?.notes.length ??
    0
  )
}

function patternLength(state: AppState): number | undefined {
  const { project } = state.document
  const patternId = selectedPatternId(project, state.transport.pattern)
  return project.patterns.find((item) => item.id === patternId)?.lengthSteps
}

/** Runs `work` on the selected channel, if there is one. */
function withSelected(work: (channel: Channel) => void | Promise<void>) {
  return () => {
    const channel = selectedChannel()
    if (channel) return work(channel)
  }
}

const hasSelection = (state: AppState) => selected(state) !== undefined

/**
 * What can be done to the selected channel and to the pattern's steps. A
 * row's right-click menu selects the row and then lists these.
 */
export const CHANNEL_RACK_ACTIONS: Action[] = [
  {
    id: "channel.rename",
    title: "Rename channel…",
    section: "Channels",
    defaultShortcut: "F2",
    enabled: hasSelection,
    run: withSelected((channel) => renameChannel(channel.id)),
  },
  {
    id: "channel.color",
    title: "Change channel color…",
    section: "Channels",
    keywords: "colour swatch",
    enabled: hasSelection,
    run: withSelected((channel) => {
      useUiStore.getState().showCenterTab("channelRack")
      useRackStore.getState().openColorPicker(channel.id)
    }),
  },
  {
    id: "channel.duplicate",
    title: "Duplicate channel",
    section: "Channels",
    defaultShortcut: "Mod+D",
    keywords: "clone copy",
    enabled: hasSelection,
    run: withSelected((channel) => duplicateChannel(channel.id)),
  },
  {
    id: "channel.delete",
    title: "Delete channel…",
    section: "Channels",
    defaultShortcut: "Delete",
    keywords: "remove",
    // Delete is a plain key, so it only acts while the rack is in view.
    enabled: (state) =>
      hasSelection(state) && state.ui.centerTab === "channelRack",
    run: withSelected((channel) => deleteChannel(channel.id)),
  },
  {
    id: "channel.moveUp",
    title: "Move channel up",
    section: "Channels",
    defaultShortcut: "Alt+ArrowUp",
    repeats: true,
    keywords: "reorder",
    enabled: (state) => selectedIndex(state) > 0,
    run: withSelected((channel) => moveChannelBy(channel.id, -1)),
  },
  {
    id: "channel.moveDown",
    title: "Move channel down",
    section: "Channels",
    defaultShortcut: "Alt+ArrowDown",
    repeats: true,
    keywords: "reorder",
    enabled: (state) => {
      const index = selectedIndex(state)
      return index >= 0 && index < state.document.project.channels.length - 1
    },
    run: withSelected((channel) => moveChannelBy(channel.id, 1)),
  },
  {
    id: "channel.mute",
    title: "Mute channel",
    section: "Channels",
    keywords: "silence lamp",
    enabled: hasSelection,
    checked: (state) => selected(state)?.muted ?? false,
    run: withSelected((channel) => toggleMute(channel.id)),
  },
  {
    id: "channel.solo",
    title: "Solo channel",
    section: "Channels",
    keywords: "only hear",
    enabled: hasSelection,
    checked: (state) => selected(state)?.solo ?? false,
    run: withSelected((channel) => toggleSolo(channel.id)),
  },
  {
    id: "channel.clearSteps",
    title: "Clear steps",
    section: "Channels",
    keywords: "erase notes empty row",
    enabled: (state) => selectedNoteCount(state) > 0,
    run: withSelected((channel) => clearSteps(channel.id)),
  },
  ...FILL_INTERVALS.map((every): Action => ({
    id: `channel.fill${every}`,
    title: `Fill every ${every} steps`,
    section: "Channels",
    keywords: "pattern hits repeat",
    enabled: hasSelection,
    run: withSelected((channel) => fillEvery(channel.id, every)),
  })),
  {
    id: "channel.shiftLeft",
    title: "Shift steps left",
    section: "Channels",
    defaultShortcut: "Mod+Shift+ArrowLeft",
    repeats: true,
    keywords: "rotate nudge earlier",
    enabled: (state) => selectedNoteCount(state) > 0,
    run: withSelected((channel) => shiftSteps(channel.id, -1)),
  },
  {
    id: "channel.shiftRight",
    title: "Shift steps right",
    section: "Channels",
    defaultShortcut: "Mod+Shift+ArrowRight",
    repeats: true,
    keywords: "rotate nudge later",
    enabled: (state) => selectedNoteCount(state) > 0,
    run: withSelected((channel) => shiftSteps(channel.id, 1)),
  },
  {
    id: "channel.routeToNewTrack",
    title: "Route to a new mixer track",
    section: "Channels",
    keywords: "insert output send",
    enabled: hasSelection,
    run: withSelected((channel) => routeToNewTrack(channel.id)),
  },
  {
    id: "channel.showInMixer",
    title: "Show the channel's mixer track",
    section: "Channels",
    keywords: "routing jump go to",
    enabled: hasSelection,
    run: withSelected((channel) => showInMixer(channel.id)),
  },
  {
    id: "channelRack.settings",
    title: "Channel settings",
    section: "View",
    keywords: "show hide toggle inspector sampler panel",
    checked: () => useRackStore.getState().inspectorOpen,
    run: () => {
      useUiStore.getState().showCenterTab("channelRack")
      useRackStore.getState().toggleInspector()
    },
  },
  ...LENGTH_PRESETS.map((steps): Action => ({
    id: `pattern.length${steps}`,
    title: `Set the pattern length to ${steps} steps`,
    section: "Patterns",
    keywords: "bars steps long",
    checked: (state) => patternLength(state) === steps,
    run: () => setPatternLength(steps),
  })),
]

/** Adds the rack's actions to the registry. Returns a function that removes them. */
export function registerChannelRackActions(): () => void {
  return registry.register(CHANNEL_RACK_ACTIONS)
}
