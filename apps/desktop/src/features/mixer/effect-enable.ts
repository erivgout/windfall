import type { EffectId, MixerTrack, TrackId } from "@/bindings"

/** Slots that need their enabled state changed, in track and slot order. */
export function effectEnableUpdates(
  tracks: readonly MixerTrack[],
  enabled: boolean
): { track: TrackId; effect: EffectId }[] {
  return tracks.flatMap((track) =>
    track.effects
      .filter((slot) => slot.enabled !== enabled)
      .map((slot) => ({ track: track.id, effect: slot.id }))
  )
}
