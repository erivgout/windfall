import { Add01Icon, Cancel01Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"

import type { Send, TrackId } from "@/bindings"
import { faderTaper, gainUnit, Knob } from "@/components/audio"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { useHint, useProjectStore } from "@/lib/store"
import { MAX_GAIN } from "@/lib/units"
import { cn } from "@/lib/utils"

import { addSend, clampGain, removeSend } from "./operations"
import { LoopNote, TrackLabel } from "./output-select"
import { sendChoices } from "./routing"
import { useGestureValue } from "./use-gesture-value"

function SendRow({ from, send }: { from: TrackId; send: Send }) {
  const target = useProjectStore(
    (state) =>
      state.project.mixer.tracks.find((track) => track.id === send.target)
        ?.name ?? "a removed track"
  )
  const level = useGestureValue(
    send.gain,
    (gain) => ({ type: "setSend", from, to: send.target, gain }),
    clampGain
  )
  const hint = useHint(
    `How much of this track goes to "${target}", after its fader. Double-click for 0 dB`
  )

  return (
    <li
      data-slot="track-send"
      className="group/send relative flex h-[26px] shrink-0 items-center gap-1"
    >
      <Knob
        size="sm"
        min={0}
        max={MAX_GAIN}
        scale={faderTaper}
        defaultValue={1}
        aria-label={`Send to ${target}`}
        {...gainUnit}
        {...level}
        {...hint}
      />
      <span
        title={target}
        className="min-w-0 flex-1 truncate text-[10px] leading-none"
      >
        {target}
      </span>
      {/* A strip is too narrow for the name and the button side by side,
          so the button lies over the end of the name while the row is
          pointed at or focused. */}
      <button
        type="button"
        aria-label={`Remove the send to ${target}`}
        className="absolute top-1/2 right-0 flex size-4 -translate-y-1/2 items-center justify-center rounded-sm bg-muted text-muted-foreground opacity-0 outline-none group-hover/send:opacity-100 hover:text-foreground focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-ring pointer-coarse:opacity-100"
        onClick={() => void removeSend(from, send.target)}
      >
        <HugeiconsIcon
          icon={Cancel01Icon}
          strokeWidth={2}
          className="size-2.5"
        />
      </button>
    </li>
  )
}

/** The sends of a track: a level knob, the target and a remove button each. */
export function SendList({
  id,
  sends,
  className,
}: {
  id: TrackId
  sends: Send[]
  className?: string
}) {
  if (sends.length === 0) return null
  return (
    <ul
      aria-label="Sends"
      className={cn(
        "flex flex-col overflow-x-hidden overflow-y-auto",
        className
      )}
    >
      {sends.map((send) => (
        <SendRow key={send.target} from={id} send={send} />
      ))}
    </ul>
  )
}

// Mounted only while the menu is open, so it can follow every track.
function SendChoices({ id }: { id: TrackId }) {
  const tracks = useProjectStore((state) => state.project.mixer.tracks)
  const { tracks: open, looping } = sendChoices(tracks, id)
  return (
    <>
      {open.length === 0 && (
        <DropdownMenuItem disabled>No track left to send to</DropdownMenuItem>
      )}
      {open.map((track) => (
        <DropdownMenuItem
          key={track.id}
          onClick={() => void addSend(id, track.id)}
        >
          <TrackLabel track={track} number={tracks.indexOf(track)} />
        </DropdownMenuItem>
      ))}
      <LoopNote count={looping} />
    </>
  )
}

/** Opens the list of tracks this one can still send to. */
export function AddSendMenu({
  id,
  className,
}: {
  id: TrackId
  className?: string
}) {
  const hint = useHint(
    "Send a copy of this track to another one, such as a shared reverb"
  )
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <button
            type="button"
            data-slot="track-add-send"
            aria-label="Add send"
            className={cn(
              "flex h-[18px] min-w-0 items-center gap-0.5 rounded-[3px] pr-1 pl-0.5 text-[10px] leading-none text-muted-foreground outline-none hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring aria-expanded:bg-muted aria-expanded:text-foreground",
              className
            )}
            {...hint}
          />
        }
      >
        <HugeiconsIcon
          icon={Add01Icon}
          strokeWidth={2}
          className="size-3 shrink-0"
        />
        <span className="truncate">Send</span>
      </DropdownMenuTrigger>
      <DropdownMenuContent className="max-h-80 w-auto min-w-44">
        <SendChoices id={id} />
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
