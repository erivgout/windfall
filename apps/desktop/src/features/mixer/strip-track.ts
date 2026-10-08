import { useShallow } from "zustand/react/shallow"

import type { EffectId, EffectKind, MixerTrack, TrackId } from "@/bindings"
import { useProjectStore } from "@/lib/store"

/*
 * What a strip reads from its track. A track changes with every setting of
 * every effect on it, so a strip that took the whole track would render on
 * each step of a knob in an effect's editor. These pick the parts a strip
 * shows, and each renders only when its own part changes.
 */

/** A mixer track without its effects. */
export type StripTrack = Omit<MixerTrack, "effects">

const find = (tracks: MixerTrack[], id: TrackId | null) =>
  tracks.find((track) => track.id === id)

export function useStripTrack(id: TrackId): StripTrack | undefined {
  return useProjectStore(
    useShallow((state): StripTrack | undefined => {
      const track = find(state.project.mixer.tracks, id)
      if (!track) return undefined
      const { id: trackId, name, color, volume, pan } = track
      const { muted, solo, output, sends } = track
      return {
        id: trackId,
        name,
        color,
        volume,
        pan,
        muted,
        solo,
        output,
        sends,
        sidechains: track.sidechains,
        recording: track.recording,
        current: track.current,
        externalOutput: track.externalOutput,
        latencyOffsetMs: track.latencyOffsetMs,
      }
    })
  )
}

/** The ids of a track's effects, in chain order. */
export function useEffectIds(track: TrackId | null): EffectId[] {
  return useProjectStore(
    useShallow(
      (state) =>
        find(state.project.mixer.tracks, track)?.effects.map(
          (slot) => slot.id
        ) ?? []
    )
  )
}

export function useEffectCount(track: TrackId | null): number {
  return useProjectStore(
    (state) => find(state.project.mixer.tracks, track)?.effects.length ?? 0
  )
}

/** What a slot shows of its effect: which kind it is and whether it is on. */
export function useSlotView(
  track: TrackId,
  effect: EffectId
): { kind: EffectKind; enabled: boolean } | undefined {
  return useProjectStore(
    useShallow((state) => {
      const slot = find(state.project.mixer.tracks, track)?.effects.find(
        (item) => item.id === effect
      )
      return slot && { kind: slot.params.type, enabled: slot.enabled }
    })
  )
}

/** The longest effect chain on any track, which sizes every strip's rack. */
export function maxEffectCount(tracks: readonly MixerTrack[]): number {
  return tracks.reduce((most, track) => Math.max(most, track.effects.length), 0)
}
