import type { CSSProperties } from "react"
import { useShallow } from "zustand/react/shallow"

import type { Channel, TrackId } from "@/bindings"
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover"
import { useHint, useProjectStore, useUiStore } from "@/lib/store"
import { colorToCss } from "@/lib/units"
import { cn } from "@/lib/utils"

import { channelsOfTrack, trackInputCount } from "./routing"

/** The channels that play into a track, in rack order. */
export function useTrackChannels(id: TrackId): Channel[] {
  return useProjectStore(
    useShallow((state) => channelsOfTrack(state.project.channels, id))
  )
}

/**
 * One channel that plays into the track, in the channel's color. Clicking
 * it selects the channel, which the channel rack highlights.
 */
export function ChannelChip({
  channel,
  className,
}: {
  channel: Channel
  className?: string
}) {
  const selected = useUiStore((state) => state.selectedChannel === channel.id)
  const hint = useHint(
    `The channel "${channel.name}" plays into this track. Click to select it in the channel rack`
  )
  return (
    <button
      type="button"
      data-slot="channel-chip"
      aria-pressed={selected}
      aria-label={`Channel ${channel.name}`}
      title={channel.name}
      style={{ "--chip": colorToCss(channel.color) } as CSSProperties}
      className={cn(
        "flex h-4 min-w-0 items-center gap-1 rounded-[3px] bg-[color-mix(in_oklch,var(--chip)_16%,transparent)] px-1 text-[10px] leading-none text-[color-mix(in_oklch,var(--chip)_45%,var(--foreground))] outline-none hover:bg-[color-mix(in_oklch,var(--chip)_28%,transparent)] focus-visible:ring-2 focus-visible:ring-ring aria-pressed:bg-[color-mix(in_oklch,var(--chip)_38%,transparent)] aria-pressed:text-foreground aria-pressed:ring-1 aria-pressed:ring-(--chip)",
        className
      )}
      onClick={() => useUiStore.getState().selectChannel(channel.id)}
      {...hint}
    >
      <span
        aria-hidden
        className="size-1.5 shrink-0 rounded-full bg-(--chip)"
      />
      <span className="truncate">{channel.name}</span>
    </button>
  )
}

function MoreChannels({ channels }: { channels: Channel[] }) {
  const hint = useHint(
    `${channels.length} channels play into this track. Click to list them`
  )
  return (
    <Popover>
      <PopoverTrigger
        render={
          <button
            type="button"
            aria-label={`All ${channels.length} channels of this track`}
            className="h-4 shrink-0 rounded-[3px] bg-muted px-1 font-readout text-[9px] text-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring aria-expanded:text-foreground"
            {...hint}
          />
        }
      >
        +{channels.length - 1}
      </PopoverTrigger>
      <PopoverContent side="bottom" align="start" className="w-44 gap-1 p-1.5">
        <ChannelList channels={channels} />
      </PopoverContent>
    </Popover>
  )
}

/** Every channel of a track, one per line. */
export function ChannelList({ channels }: { channels: Channel[] }) {
  return (
    <ul className="flex flex-col gap-1">
      {channels.map((channel) => (
        <li key={channel.id} className="flex">
          <ChannelChip channel={channel} className="h-5 flex-1" />
        </li>
      ))}
    </ul>
  )
}

function Unused() {
  const hint = useHint(
    "Nothing plays into this track. Route a channel to it in the channel rack, or another track's output or a send"
  )
  return (
    <span
      data-slot="track-unused"
      className="px-1 text-[10px] leading-none text-muted-foreground/70 italic"
      {...hint}
    >
      unused
    </span>
  )
}

function FedByTracks({ count }: { count: number }) {
  const label = count === 1 ? "1 track in" : `${count} tracks in`
  const hint = useHint(
    count === 1
      ? "Another track plays into this one through its output or a send"
      : `${count} other tracks play into this one through their outputs or sends`
  )
  return (
    <span
      data-slot="track-inputs"
      className="truncate px-1 text-[10px] leading-none text-muted-foreground"
      {...hint}
    >
      {label}
    </span>
  )
}

/**
 * The row under the name that shows what plays into the track: its channels
 * as chips, or how many tracks feed it, or that nothing does.
 */
export function ChannelChips({ id, master }: { id: TrackId; master: boolean }) {
  const channels = useTrackChannels(id)
  const inputs = useProjectStore((state) =>
    trackInputCount(state.project.mixer.tracks, id)
  )

  let content = null
  if (channels.length > 0) {
    content = (
      <>
        <ChannelChip channel={channels[0]} className="flex-1" />
        {channels.length > 1 && <MoreChannels channels={channels} />}
      </>
    )
  } else if (!master) {
    content = inputs > 0 ? <FedByTracks count={inputs} /> : <Unused />
  }

  return (
    <div
      data-slot="track-channels"
      className="flex h-4 shrink-0 items-center gap-0.5 px-1"
    >
      {content}
    </div>
  )
}
