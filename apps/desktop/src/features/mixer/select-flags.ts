import type { MixerTrack, TrackId } from "@/bindings"

/** Muted tracks, in the given order. */
export function mutedTrackIds(tracks: readonly MixerTrack[]): TrackId[] {
  return tracks.filter((track) => track.muted).map((track) => track.id)
}

/** Solo tracks, in the given order. */
export function soloTrackIds(tracks: readonly MixerTrack[]): TrackId[] {
  return tracks.filter((track) => track.solo).map((track) => track.id)
}
