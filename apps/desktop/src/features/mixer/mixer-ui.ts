import { create } from "zustand"

import type { TrackId } from "@/bindings"

type MixerUiState = {
  /** The track whose name is being edited in its strip. */
  renaming: TrackId | null
  /** The track whose color swatches are open. */
  coloring: TrackId | null
}

/**
 * What the mixer is in the middle of. Actions set it and the strip of the
 * track responds, so "Rename" works the same from the context menu, the
 * command palette and a key.
 */
export const useMixerUi = create<MixerUiState>(() => ({
  renaming: null,
  coloring: null,
}))
