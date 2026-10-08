import { create } from "zustand"

import type { MixerTrack, TrackId } from "@/bindings"
import { onProjectReplaced } from "@/lib/store/replaced"
import { useUiStore } from "@/lib/store/ui"
import { useProjectStore } from "@/lib/store/project"

type MixerUiState = {
  selected: TrackId[]
  anchor: TrackId | null
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
  selected: [], anchor: null,
  renaming: null,
  coloring: null,
  focusing: null,
}))

// A rename or a color choice that was under way was for a track of the
// project that is gone, whose ids the next one hands out again.
onProjectReplaced(() =>
  useMixerUi.setState({ renaming: null, coloring: null, focusing: null, selected: [], anchor: null })
)

export function visualMixerOrder(tracks: readonly MixerTrack[]): MixerTrack[] {
  return [...tracks.filter((track) => track.id === 0), ...tracks.filter((track) => track.current), ...["left", "middle", "right"].flatMap((dock) => tracks.filter((track) => track.id !== 0 && !track.current && (track.dock ?? "middle") === dock))]
}

export function selectMixerTrack(id: TrackId, toggle = false, range = false) {
  const state = useMixerUi.getState()
  const anchor = state.anchor ?? useUiStore.getState().selectedTrack ?? id
  let selected: TrackId[]
  if (range) {
    const order = visualMixerOrder(useProjectStore.getState().project.mixer.tracks).map((track) => track.id)
    const from = order.indexOf(anchor)
    const to = order.indexOf(id)
    selected = from < 0 || to < 0 ? [id] : order.slice(Math.min(from, to), Math.max(from, to) + 1)
    if (toggle) selected = [...new Set([...state.selected, ...selected])]
  } else if (toggle) selected = state.selected.includes(id) ? state.selected.filter((item) => item !== id) : [...state.selected, id]
  else selected = state.selected.includes(id) ? state.selected : [id]
  useMixerUi.setState({ selected, anchor: range ? anchor : id })
  useUiStore.getState().selectTrack(toggle && !selected.includes(id) ? selected.at(-1) ?? null : id)
}

export function selectedMixerTracks(forTrack?: TrackId): MixerTrack[] {
  const selected = useMixerUi.getState().selected
  const ids = forTrack !== undefined && !selected.includes(forTrack) ? [forTrack] : selected.length ? selected : [useUiStore.getState().selectedTrack]
  return useProjectStore.getState().project.mixer.tracks.filter((track) => ids.includes(track.id))
}

useUiStore.subscribe((state, before) => {
  if (state.selectedTrack === before.selectedTrack) return
  const id = state.selectedTrack
  if (id === null) useMixerUi.setState({ selected: [], anchor: null })
  else if (!useMixerUi.getState().selected.includes(id)) useMixerUi.setState({ selected: [id], anchor: id })
})
useProjectStore.subscribe((state) => {
  const ids = new Set(state.project.mixer.tracks.map((track) => track.id))
  const selection = useMixerUi.getState()
  const selected = selection.selected.filter((id) => ids.has(id))
  if (selected.length !== selection.selected.length) useMixerUi.setState({ selected, anchor: selection.anchor !== null && ids.has(selection.anchor) ? selection.anchor : selected[0] ?? null })
})
