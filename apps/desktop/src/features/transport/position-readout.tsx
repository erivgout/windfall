import { useRef } from "react"

import { songTempoMap } from "@/lib/automation/tempo-map"
import { useProjectStore } from "@/lib/store/project"
import { useRealtime } from "@/lib/store/realtime"
import { useTransportStore } from "@/lib/store/transport"
import { formatMusicalPosition, meterSegments } from "@/lib/timeline"
import {
  formatClock,
  formatPosition,
  ticksPerBeat,
  ticksToSeconds,
} from "@/lib/time"

function setText(element: HTMLElement | null, text: string) {
  if (element && element.textContent !== text) element.textContent = text
}

/**
 * Where the playhead is, as bar:beat:step and as time. It changes every
 * frame, so the text is written straight to the page instead of going
 * through React state.
 */
export function PositionReadout() {
  const position = useRef<HTMLSpanElement>(null)
  const clock = useRef<HTMLSpanElement>(null)
  const pulse = useRef<HTMLSpanElement>(null)

  useRealtime((frame) => {
    const project = useProjectStore.getState().project
    const { timeSignature, tempoBpm } = project.settings
    const inSong = useTransportStore.getState().mode === "song"
    const meters = project.playlist.timeline?.meters ?? []
    setText(
      position.current,
      inSong
        ? formatMusicalPosition(frame.tick, timeSignature, meters)
        : formatPosition(frame.tick, timeSignature)
    )
    // The song's clock follows the tempo automation, so the time it took
    // to get to a tick is summed along the curve. A pattern loops at the
    // stored tempo.
    const seconds = inSong
      ? songTempoMap(project).secondsAt(frame.tick)
      : ticksToSeconds(frame.tick, tempoBpm)
    setText(clock.current, formatClock(seconds))

    if (pulse.current) {
      const segment = inSong
        ? meterSegments(timeSignature, meters).findLast(
            (s) => s.start <= frame.tick
          )
        : undefined
      const beat = ticksPerBeat(segment?.signature ?? timeSignature)
      const intoBeat = ((frame.tick - (segment?.start ?? 0)) % beat) / beat
      const lit = frame.playing && intoBeat < 0.3
      pulse.current.style.opacity = lit ? String(1 - intoBeat * 2) : "0.18"
    }
  })

  return (
    <div
      className="flex items-center gap-2.5"
      role="group"
      aria-label="Song position"
    >
      <span
        ref={pulse}
        aria-hidden
        className="size-1.5 rounded-full bg-brand opacity-[0.18]"
      />
      <span
        ref={position}
        className="font-readout text-[0.8125rem] font-medium text-display-foreground"
        title="Bar, beat and step"
      >
        001:01:1
      </span>
      <span
        ref={clock}
        className="font-readout text-[0.6875rem] text-display-dim"
        title="Minutes and seconds"
      >
        0:00.00
      </span>
    </div>
  )
}
