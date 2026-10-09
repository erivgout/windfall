import type { Clip, ClipId, PlaylistTrackId } from "@/bindings"

/** Returns clips on the given track in their original order. */
export function clipIdsOnTrack(
  clips: readonly Clip[],
  trackId: PlaylistTrackId
): ClipId[] {
  return clips.filter((clip) => clip.track === trackId).map((clip) => clip.id)
}
