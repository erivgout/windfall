/** Channel ids whose names contain the trimmed query, in their original order. */
export function matchingChannelIds<Id>(
  channels: readonly { id: Id; name: string }[],
  query: string
): Id[] {
  const filter = query.trim().toLowerCase()
  if (filter === "") return channels.map((channel) => channel.id)
  return channels
    .filter((channel) => channel.name.toLowerCase().includes(filter))
    .map((channel) => channel.id)
}
