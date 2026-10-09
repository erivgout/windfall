import type { MixerTrack, TrackId } from "@/bindings"

/** Tracks whose main output routes to the target, in the given order. */
export function routedTrackIds(
  tracks: readonly MixerTrack[],
  target: TrackId
): TrackId[] {
  return tracks
    .filter((track) => track.id !== target && track.output === target)
    .map((track) => track.id)
}
