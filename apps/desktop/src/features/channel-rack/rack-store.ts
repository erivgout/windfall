import { create } from "zustand"
import { persist } from "zustand/middleware"

import type { ChannelId } from "@/bindings"

type RackState = {
  /** Whether the channel settings are showing beside the rack. */
  inspectorOpen: boolean
  /** The channel whose color swatches are open, if any. */
  colorPickerFor: ChannelId | null

  setInspectorOpen(open: boolean): void
  toggleInspector(): void
  openColorPicker(channel: ChannelId | null): void
}

/**
 * View state of the channel rack that is not part of the project. Whether
 * the settings are open survives a restart.
 */
export const useRackStore = create<RackState>()(
  persist(
    (set) => ({
      inspectorOpen: false,
      colorPickerFor: null,

      setInspectorOpen: (inspectorOpen) => set({ inspectorOpen }),
      toggleInspector: () =>
        set((state) => ({ inspectorOpen: !state.inspectorOpen })),
      openColorPicker: (colorPickerFor) => set({ colorPickerFor }),
    }),
    {
      name: "windfall.rack",
      version: 1,
      partialize: (state) => ({ inspectorOpen: state.inspectorOpen }),
    }
  )
)
