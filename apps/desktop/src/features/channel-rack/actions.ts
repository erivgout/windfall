import type { Channel } from "@/bindings"
import { INSTRUMENT_KINDS, instrumentDescriptor } from "@/features/params"
import {
  invalidateActionsOn,
  registry,
  type Action,
  type AppState,
} from "@/lib/actions"
import { instrumentParams } from "@/lib/channel-source"
import { selectedPatternId } from "@/lib/store/selectors"
import { useUiStore } from "@/lib/store/ui"

import {
  addChannelFromPickedFile,
  addInstrumentChannel,
  clearSteps,
  deleteChannel,
  duplicateChannel,
  fillEvery,
  initInstrument,
  loadInstrumentSound,
  moveChannelBy,
  renameChannel,
  replaceSampleFromPickedFile,
  routeToNewTrack,
  selectedChannel,
  setPatternLength,
  shiftSteps,
  showInMixer,
  toggleMute,
  toggleSolo,
} from "./channel-ops"
import { useRackStore } from "./rack-store"
import { matchingPreset, SYNTH_PRESETS } from "./synth/presets"

/** The palette section, and menu, of the built-in instrument sounds. */
export const SOUNDS_SECTION = "Sounds"

export const LENGTH_PRESETS = [16, 32, 48, 64]
export const FILL_INTERVALS = [2, 4, 8]

function selected(state: AppState): Channel | undefined {
  return state.document.project.channels.find(
    (channel) => channel.id === state.ui.selectedChannel
  )
}

function selectedIndex(state: AppState): number {
  return state.document.project.channels.findIndex(
    (channel) => channel.id === state.ui.selectedChannel
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

const isSampler = (state: AppState) =>
  selected(state)?.source.type === "sampler"
const isInstrument = (state: AppState) =>
  selected(state)?.source.type === "instrument" &&
  !state.document.project.plugins?.some(
    (plugin) =>
      plugin.target.type === "instrument" &&
      plugin.target.channel === state.ui.selectedChannel
  )

/** The selected channel's synth settings, when it is a subtractive synth. */
function selectedSynth(state: AppState) {
  if (!isInstrument(state)) return null
  const params = instrumentParams(selected(state))
  return params?.type === "subtractiveSynth" ? params : null
}

/** Says which kind of channel an action is for, when the other is selected. */
const onlyFor =
  (wanted: "sampler" | "instrument", reason: string) =>
  (state: AppState): string | undefined => {
    const channel = selected(state)
    return channel && channel.source.type !== wanted ? reason : undefined
  }

const ADD_INSTRUMENT_ACTIONS = INSTRUMENT_KINDS.map((kind): Action => ({
  id: `channel.addInstrument.${kind}`,
  title: `Add ${instrumentDescriptor(kind).name.toLowerCase()}`,
  section: "Channels",
  keywords: "new channel instrument synthesizer synth keys bass lead pad",
  run: () => addInstrumentChannel(kind),
}))

/** The id of the action that loads a built-in synth sound. */
export const synthSoundActionId = (preset: string) =>
  `channel.synthSound.${preset}`

/** Every built-in synth sound but the init sound, which has its own action. */
const SYNTH_SOUND_ACTIONS = SYNTH_PRESETS.filter(
  (preset) => preset.id !== "init"
).map((preset): Action => ({
  id: synthSoundActionId(preset.id),
  title: `Load synth sound: ${preset.name}`,
  section: SOUNDS_SECTION,
  keywords: `preset patch ${preset.description}`,
  enabled: (state) => selectedSynth(state) !== null,
  whyDisabled: () => "Synth channels only",
  checked: (state) => {
    const params = selectedSynth(state)
    return params !== null && matchingPreset(params)?.id === preset.id
  },
  run: withSelected((channel) =>
    loadInstrumentSound(channel.id, preset.params, `Load sound: ${preset.name}`)
  ),
}))

/**
 * What can be done to the selected channel and to the pattern's steps. A
 * row's right-click menu selects the row and then lists these. The keys
 * belong to the rack: they work while it has the keyboard.
 */
export const CHANNEL_RACK_ACTIONS: Action[] = [
  {
    id: "channel.addFromFile",
    title: "Add channel from an audio file…",
    section: "Channels",
    keywords: "new sampler sample load open wav",
    run: addChannelFromPickedFile,
  },
  ...ADD_INSTRUMENT_ACTIONS,
  {
    id: "channel.replaceSample",
    title: "Replace sample from an audio file…",
    section: "Channels",
    keywords: "swap sound load sampler",
    enabled: isSampler,
    whyDisabled: onlyFor("sampler", "Samplers only"),
    run: withSelected((channel) => replaceSampleFromPickedFile(channel.id)),
  },
  {
    id: "channel.initInstrument",
    title: "Init instrument",
    section: "Channels",
    keywords: "reset default sound preset patch clear synth",
    enabled: isInstrument,
    whyDisabled: onlyFor("instrument", "Instruments only"),
    run: withSelected((channel) => initInstrument(channel.id)),
  },
  ...SYNTH_SOUND_ACTIONS,
  {
    id: "channel.rename",
    title: "Rename channel…",
    section: "Channels",
    scope: "channelRack",
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
    scope: "channelRack",
    editCommand: "duplicate",
    defaultShortcut: "Mod+D",
    keywords: "clone copy",
    enabled: hasSelection,
    run: withSelected((channel) => duplicateChannel(channel.id)),
  },
  {
    id: "channel.delete",
    title: "Delete channel…",
    section: "Channels",
    scope: "channelRack",
    editCommand: "delete",
    defaultShortcut: "Delete",
    keywords: "remove",
    enabled: hasSelection,
    run: withSelected((channel) => deleteChannel(channel.id)),
  },
  {
    id: "channel.moveUp",
    title: "Move channel up",
    section: "Channels",
    scope: "channelRack",
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
    scope: "channelRack",
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
    scope: "channelRack",
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
    scope: "channelRack",
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
  const stops = [
    registry.register(CHANNEL_RACK_ACTIONS),
    invalidateActionsOn(useRackStore, (state) => [state.inspectorOpen]),
  ]
  return () => {
    for (const stop of stops) stop()
  }
}
