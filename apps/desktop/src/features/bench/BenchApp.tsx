import { useCallback, useMemo, useRef } from "react"

import type { TimeGridView, TimeGridViewOptions } from "@/lib/canvas"
import { TimeGridCanvas } from "@/lib/canvas/react"

import { SONG_TICKS, TICKS_PER_BAR } from "./generate-notes"
import { ROW_COUNT } from "./model"
import type { BenchParams } from "./params"
import { startSession } from "./session"

/** The benchmark page: a full-window piano-roll canvas and a small readout. */
export function BenchApp({ params }: { params: BenchParams }) {
  const hudRef = useRef<HTMLPreElement>(null)

  const options = useMemo<TimeGridViewOptions>(
    () => ({
      renderer: params.renderer,
      // The automated run draws inside its own frame loop so it can time it.
      autoRender: !params.auto,
      limits: {
        rowCount: ROW_COUNT,
        contentTicks: SONG_TICKS,
        minPxPerTick: 0.0005,
        maxPxPerTick: 1,
      },
      initial: {
        pxPerTick: window.innerWidth / (8 * TICKS_PER_BAR),
        rowHeight: 16,
        scrollRow: 30,
      },
    }),
    [params]
  )

  const setHud = useCallback((text: string) => {
    if (hudRef.current) hudRef.current.textContent = text
  }, [])

  const onReady = useCallback(
    (view: TimeGridView) => startSession(view, params, setHud),
    [params, setHud]
  )

  const onError = useCallback(
    (error: unknown) => {
      const message = error instanceof Error ? error.message : String(error)
      window.__benchError = message
      window.__benchDone = true
      setHud(message)
    },
    [setHud]
  )

  return (
    <div className="fixed inset-0 overflow-hidden bg-background select-none">
      <TimeGridCanvas
        className="absolute inset-0"
        options={options}
        onReady={onReady}
        onError={onError}
      />
      <pre
        ref={hudRef}
        className="pointer-events-none absolute right-2 bottom-2 rounded-md border border-border bg-popover/90 px-2 py-1.5 font-mono text-[11px] leading-4 text-popover-foreground"
      />
    </div>
  )
}
