import { create } from "zustand"
import { persist } from "zustand/middleware"

import type { ChannelId } from "@/bindings"
import { useProjectStore } from "@/lib/store/project"
import { onProjectReplaced } from "@/lib/store/replaced"
import { useUiStore } from "@/lib/store/ui"

import type { NotePreviewLane } from "./note-preview-target"
import type { GraphProperty } from "./graph-values"

import type { NotePreviewLane } from "./note-preview-target"

/**
 * How many keys the keyboard in the channel settings shows: around the
 * root key, six octaves, or every key there is.
 */
export type KeyboardRange = "auto" | "wide" | "full"
export type RackNoteView = "auto" | "steps" | "notes"

type RackState = {
  /** Whether the channel settings are showing beside the rack. */
  inspectorOpen: boolean
  /** Print note names on the keyboard in the channel settings. */
  keyboardLabels: boolean
  keyboardRange: KeyboardRange
  /** The channel whose color swatches are open, if any. */
  colorPickerFor: ChannelId | null
  /** Null shows all channels; an empty string shows ungrouped channels. */
  groupFilter: string | null
  graphOpen: boolean
  graphProperty: GraphProperty
  setGraphOpen(open: boolean): void
  setGraphProperty(property: GraphProperty): void
  setGroupFilter(group: string | null): void
  /** Per-lane display choices, never saved into a project or preferences. */
  noteViews: Record<string, { lane: NotePreviewLane; view: RackNoteView }>
  setNoteView(lane: NotePreviewLane, view: RackNoteView): void

  setInspectorOpen(open: boolean): void
  toggleInspector(): void
  openColorPicker(channel: ChannelId | null): void
  setKeyboardLabels(labels: boolean): void
  setKeyboardRange(range: KeyboardRange): void
}

/**
 * View state of the channel rack that is not part of the project. Whether
 * the settings are open survives a restart.
 */
export const useRackStore = create<RackState>()(
  persist(
    (set) => ({
      inspectorOpen: false,
      keyboardLabels: true,
      keyboardRange: "auto",
      colorPickerFor: null,
      groupFilter: null,
      graphOpen: false,
      graphProperty: "velocity",
      setGraphOpen: (graphOpen) => set({ graphOpen }),
      setGraphProperty: (graphProperty) => set({ graphProperty }),
      setGroupFilter: (groupFilter) => {
        const channels = useProjectStore.getState().project.channels
        if (groupFilter && !channels.some((channel) => channel.group === groupFilter)) return
        set({ groupFilter })
        const visible = channels.filter((channel) => groupFilter === null || (channel.group ?? "") === groupFilter)
        const selected = useUiStore.getState().selectedChannel
        if (!visible.some((channel) => channel.id === selected)) {
          useUiStore.getState().selectChannel(visible[0]?.id ?? null)
        }
      },
      noteViews: {},
      setNoteView: (lane, view) =>
        set((state) => {
          const noteViews = { ...state.noteViews }
          const key = laneKey(lane)
          if (view === "auto") delete noteViews[key]
          else noteViews[key] = { lane, view }
          return { noteViews }
        }),

      setInspectorOpen: (inspectorOpen) => set({ inspectorOpen }),
      toggleInspector: () =>
        set((state) => ({ inspectorOpen: !state.inspectorOpen })),
      openColorPicker: (colorPickerFor) => set({ colorPickerFor }),
      setKeyboardLabels: (keyboardLabels) => set({ keyboardLabels }),
      setKeyboardRange: (keyboardRange) => set({ keyboardRange }),
    }),
    {
      name: "windfall.rack",
      version: 1,
      partialize: (state) => ({
        inspectorOpen: state.inspectorOpen,
        keyboardLabels: state.keyboardLabels,
        keyboardRange: state.keyboardRange,
      }),
    }
  )
)

// Encoding is private to this transient store; callers pass typed lane identity.
function laneKey(lane: NotePreviewLane): string {
  return `${lane.pattern}:${lane.channel}`
}

export function rackNoteView(
  state: RackState,
  lane: NotePreviewLane
): RackNoteView {
  return state.noteViews[laneKey(lane)]?.view ?? "auto"
}

// The channel the swatches were open for is not a channel of the next
// project, though one there may have its id.
onProjectReplaced(() =>
  useRackStore.setState({ colorPickerFor: null, noteViews: {}, groupFilter: null })
)

// Deleting a target forgets its choice even if undo later restores its id.
useProjectStore.subscribe((state, previous) => {
  const filter = useRackStore.getState().groupFilter
  if (filter !== null && state.project.channels !== previous.project.channels) {
    if (filter && !state.project.channels.some((channel) => channel.group === filter)) {
      useRackStore.getState().setGroupFilter(null)
    } else {
      const visible = state.project.channels.filter((channel) => (channel.group ?? "") === filter)
      const selected = useUiStore.getState().selectedChannel
      if (!visible.some((channel) => channel.id === selected)) {
        // A new or externally selected channel should remain discoverable.
        if (state.project.channels.length > previous.project.channels.length) {
          useRackStore.getState().setGroupFilter(null)
        } else {
          useUiStore.getState().selectChannel(visible[0]?.id ?? null)
        }
      }
    }
  }
  if (
    state.project.channels === previous.project.channels &&
    state.project.patterns === previous.project.patterns
  )
    return
  const views = useRackStore.getState().noteViews
  const noteViews = Object.fromEntries(
    Object.entries(views).filter(([, { lane }]) => {
      return (
        state.project.patterns.some((item) => item.id === lane.pattern) &&
        state.project.channels.some((item) => item.id === lane.channel)
      )
    })
  )
  if (Object.keys(noteViews).length !== Object.keys(views).length) {
    useRackStore.setState({ noteViews })
  }
})

// Navigation from another editor may select a hidden channel. Reveal it
// instead of leaving its inspector disconnected from the visible rack.
useUiStore.subscribe((state, previous) => {
  if (state.selectedChannel === previous.selectedChannel || state.selectedChannel === null) return
  const filter = useRackStore.getState().groupFilter
  if (filter === null) return
  const selected = useProjectStore.getState().project.channels.find((channel) => channel.id === state.selectedChannel)
  if (selected && (selected.group ?? "") !== filter) useRackStore.getState().setGroupFilter(null)
})
