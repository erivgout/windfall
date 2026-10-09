import type { Channel, ChannelId } from "@/bindings"

/** Finds the next matching channel in rack order, excluding the current one. */
export function nextFlaggedChannelId(
  channels: readonly Pick<Channel, "id" | "muted" | "solo">[],
  currentId: ChannelId | null,
  flag: "muted" | "solo"
): ChannelId | null {
  const currentIndex = channels.findIndex((channel) => channel.id === currentId)
  for (let offset = 1; offset <= channels.length; offset++) {
    const channel = channels[(currentIndex + offset) % channels.length]
    if (channel.id !== currentId && channel[flag]) return channel.id
  }
  return null
}
