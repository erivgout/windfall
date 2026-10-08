import { create } from "zustand"
import type { MixerTrack, TrackId } from "@/bindings"
import { useUiStore } from "@/lib/store/ui"
import { getProjectGeneration, onProjectReplaced } from "@/lib/store/replaced"

type MixerRenderRequest = {
  tracks: TrackId[]
  includeMix: boolean
  generation: number
}

/** A one-shot export draft, scoped to the document in which it was opened. */
export const useMixerRender = create<{ request: MixerRenderRequest | null }>(() => ({ request: null }))
onProjectReplaced(() => useMixerRender.setState({ request: null }))

export function renderableMixerTracks(tracks: readonly MixerTrack[]) {
  return tracks.filter((track) => !track.current)
}

/** Opens the existing offline renderer with captured mixer ids, including Master as the mix. */
export function openMixerRender(tracks: readonly MixerTrack[]) {
  const eligible = renderableMixerTracks(tracks)
  if (!eligible.length) return
  useMixerRender.setState({ request: {
    tracks: eligible.filter((track) => track.id !== 0).map((track) => track.id),
    includeMix: eligible.some((track) => track.id === 0),
    generation: getProjectGeneration(),
  } })
  useUiStore.getState().openDialog("export")
}
