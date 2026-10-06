import { useRef, useState, type KeyboardEvent, type PointerEvent } from "react"

import { cn } from "@/lib/utils"
import { useGesture } from "@/lib/store/gesture"
import { useHint } from "@/lib/store/hint"
import { dispatch } from "@/lib/store/project"
import { useTempo } from "@/lib/store/selectors"
import { formatTempo, normalizeTempo, parseTempo } from "@/lib/time"
import { MAX_TEMPO_BPM, MIN_TEMPO_BPM } from "@/lib/units"

/** How far the pointer must move before a press counts as a drag. */
const DRAG_THRESHOLD_PX = 3
const PIXELS_PER_BPM = 3

type Drag = {
  startY: number
  startTempo: number
  moved: boolean
  sent: number
  pending: Promise<unknown>
}

/**
 * The tempo. Drag it up or down, nudge it with the arrow keys, or click it
 * and type. A drag shows its own value while it lasts and sends every change
 * under one gesture, so the whole drag is a single undo step.
 */
export function TempoField() {
  const tempo = useTempo()
  const gesture = useGesture()
  const drag = useRef<Drag | null>(null)
  const [dragValue, setDragValue] = useState<number | null>(null)
  const [draft, setDraft] = useState<string | null>(null)
  const hint = useHint(
    "Tempo: drag up or down, hold Shift for fine steps, or click to type"
  )

  const shown = dragValue ?? tempo

  function setTempo(value: number) {
    const tempoBpm = normalizeTempo(value)
    if (tempoBpm !== tempo) {
      void dispatch({ type: "updateSettings", patch: { tempoBpm } })
    }
  }

  function onPointerDown(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0) return
    event.currentTarget.setPointerCapture(event.pointerId)
    drag.current = {
      startY: event.clientY,
      startTempo: tempo,
      moved: false,
      sent: tempo,
      pending: Promise.resolve(),
    }
  }

  function onPointerMove(event: PointerEvent<HTMLDivElement>) {
    const current = drag.current
    if (!current) return
    const distance = current.startY - event.clientY
    if (!current.moved) {
      if (Math.abs(distance) < DRAG_THRESHOLD_PX) return
      current.moved = true
      gesture.begin()
    }
    const next = event.shiftKey
      ? normalizeTempo(current.startTempo + distance * 0.01)
      : normalizeTempo(
          Math.round(current.startTempo + distance / PIXELS_PER_BPM)
        )
    setDragValue(next)
    if (next === current.sent) return
    current.sent = next
    current.pending = gesture.dispatch({
      type: "updateSettings",
      patch: { tempoBpm: next },
    })
  }

  function onPointerUp(event: PointerEvent<HTMLDivElement>) {
    const current = drag.current
    drag.current = null
    if (!current) return
    event.currentTarget.releasePointerCapture(event.pointerId)
    if (!current.moved) {
      setDraft(formatTempo(tempo))
      return
    }
    gesture.end()
    // Hold the dragged value until the store has caught up, so the number
    // does not jump back for a frame.
    void current.pending.finally(() => setDragValue(null))
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const step = event.shiftKey ? 0.1 : 1
    if (event.key === "ArrowUp") setTempo(tempo + step)
    else if (event.key === "ArrowDown") setTempo(tempo - step)
    else if (event.key === "PageUp") setTempo(tempo + 10)
    else if (event.key === "PageDown") setTempo(tempo - 10)
    else if (event.key === "Enter") setDraft(formatTempo(tempo))
    else if (/^[0-9]$/.test(event.key)) setDraft(event.key)
    else return
    event.preventDefault()
    event.stopPropagation()
  }

  function commit(text: string) {
    setDraft(null)
    const value = parseTempo(text)
    if (value !== null) setTempo(value)
  }

  if (draft !== null) {
    return (
      <input
        autoFocus
        aria-label="Tempo in beats per minute"
        inputMode="decimal"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onFocus={(event) => {
          if (event.target.value.length > 1) event.target.select()
        }}
        onBlur={(event) => commit(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") commit(event.currentTarget.value)
          else if (event.key === "Escape") setDraft(null)
          else return
          event.preventDefault()
        }}
        className="h-6 w-[4.25rem] rounded-sm bg-display-foreground/10 px-1 text-right font-readout text-[0.8125rem] font-medium text-display-foreground ring-1 ring-brand outline-none"
      />
    )
  }

  return (
    <div
      role="spinbutton"
      tabIndex={0}
      aria-label="Tempo in beats per minute"
      aria-valuenow={shown}
      aria-valuemin={MIN_TEMPO_BPM}
      aria-valuemax={MAX_TEMPO_BPM}
      aria-valuetext={`${formatTempo(shown)} beats per minute`}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onKeyDown={onKeyDown}
      {...hint}
      className={cn(
        "flex h-6 w-[4.25rem] cursor-ns-resize touch-none items-center justify-end rounded-sm px-1 font-readout text-[0.8125rem] font-medium text-display-foreground outline-none hover:bg-display-foreground/10 focus-visible:ring-1 focus-visible:ring-brand",
        dragValue !== null && "bg-display-foreground/10"
      )}
    >
      {formatTempo(shown)}
    </div>
  )
}
