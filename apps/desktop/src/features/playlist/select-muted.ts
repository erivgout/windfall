import type { Clip, ClipId, PlaylistTrack } from "@/bindings"

/** Returns individually muted clips in their original order. */
export function mutedClipIds(clips: readonly Clip[]): ClipId[] {
  return clips.filter((clip) => clip.muted).map((clip) => clip.id)
}

/** Returns clips on muted tracks in clip order, regardless of their own mute. */
export function clipsOnMutedTracks(
  clips: readonly Clip[],
  tracks: readonly PlaylistTrack[]
): ClipId[] {
  const mutedTracks = new Set(
    tracks.filter((track) => track.muted).map((track) => track.id)
  )
  return clips
    .filter((clip) => mutedTracks.has(clip.track))
    .map((clip) => clip.id)
}
