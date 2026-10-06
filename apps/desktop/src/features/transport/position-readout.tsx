import { useRef } from "react"

import { useProjectStore } from "@/lib/store/project"
import { useRealtime } from "@/lib/store/realtime"
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
    const { timeSignature, tempoBpm } =
      useProjectStore.getState().project.settings
    setText(position.current, formatPosition(frame.tick, timeSignature))
    setText(clock.current, formatClock(ticksToSeconds(frame.tick, tempoBpm)))

    if (pulse.current) {
      const beat = ticksPerBeat(timeSignature)
      const intoBeat = (frame.tick % beat) / beat
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
