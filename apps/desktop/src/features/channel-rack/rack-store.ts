import { create } from "zustand"
import { persist } from "zustand/middleware"

import type { ChannelId } from "@/bindings"
import { onProjectReplaced } from "@/lib/store/replaced"

/**
 * How many keys the keyboard in the channel settings shows: around the
 * root key, six octaves, or every key there is.
 */
export type KeyboardRange = "auto" | "wide" | "full"

type RackState = {
  /** Whether the channel settings are showing beside the rack. */
  inspectorOpen: boolean
  /** Print note names on the keyboard in the channel settings. */
  keyboardLabels: boolean
  keyboardRange: KeyboardRange
  /** The channel whose color swatches are open, if any. */
  colorPickerFor: ChannelId | null

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
onProjectReplaced(() => useRackStore.setState({ colorPickerFor: null }))
