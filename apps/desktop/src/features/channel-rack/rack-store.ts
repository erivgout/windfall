import { create } from "zustand"
import { persist } from "zustand/middleware"

import type { ChannelId } from "@/bindings"
import { useProjectStore } from "@/lib/store/project"
import { onProjectReplaced } from "@/lib/store/replaced"

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
  /** Per-lane display choices, never saved into a project or preferences. */
  noteViews: Record<string, RackNoteView>
  setNoteView(lane: string, view: RackNoteView): void

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
      noteViews: {},
      setNoteView: (lane, view) =>
        set((state) => {
          const noteViews = { ...state.noteViews }
          if (view === "auto") delete noteViews[lane]
          else noteViews[lane] = view
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

// The channel the swatches were open for is not a channel of the next
// project, though one there may have its id.
onProjectReplaced(() =>
  useRackStore.setState({ colorPickerFor: null, noteViews: {} })
)

// Deleting a target forgets its choice even if undo later restores its id.
useProjectStore.subscribe((state, previous) => {
  if (
    state.project.channels === previous.project.channels &&
    state.project.patterns === previous.project.patterns
  )
    return
  const views = useRackStore.getState().noteViews
  const noteViews = Object.fromEntries(
    Object.entries(views).filter(([key]) => {
      const [pattern, channel] = key.split(":").map(Number)
      return (
        state.project.patterns.some((item) => item.id === pattern) &&
        state.project.channels.some((item) => item.id === channel)
      )
    })
  )
  if (Object.keys(noteViews).length !== Object.keys(views).length) {
    useRackStore.setState({ noteViews })
  }
})
