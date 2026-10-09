/** Match track names without changing their order or the original tracks. */
export function matchingTrackIds<Id>(
  tracks: readonly { id: Id; name: string }[],
  query: string
): Id[] {
  const needle = query.trim().toLowerCase()
  return tracks
    .filter((track) => !needle || track.name.toLowerCase().includes(needle))
    .map((track) => track.id)
}
