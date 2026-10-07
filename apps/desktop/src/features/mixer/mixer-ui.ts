import { create } from "zustand"

import type { TrackId } from "@/bindings"
import { onProjectReplaced } from "@/lib/store/replaced"

type MixerUiState = {
  /** The track whose name is being edited in its strip. */
  renaming: TrackId | null
  /** The track whose color swatches are open. */
  coloring: TrackId | null
  /** The strip a key moved the selection to, which takes the focus next. */
  focusing: TrackId | null
}

/**
 * What the mixer is in the middle of. Actions set it and the strip of the
 * track responds, so "Rename" works the same from the context menu, the
 * command palette and a key.
 */
export const useMixerUi = create<MixerUiState>(() => ({
  renaming: null,
  coloring: null,
  focusing: null,
}))

// A rename or a color choice that was under way was for a track of the
// project that is gone, whose ids the next one hands out again.
onProjectReplaced(() =>
  useMixerUi.setState({ renaming: null, coloring: null, focusing: null })
)
