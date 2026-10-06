// SPDX-License-Identifier: MIT
import * as React from "react"

import { cn } from "@/lib/utils"

import {
  observeCanvas,
  resolveColors,
  subscribeTheme,
  type CanvasSize,
} from "./canvas"
import { formatPercent } from "./units"
import { useDragValue } from "./use-drag-value"

export type WaveformPeaks = Float32Array | readonly number[]

export type WaveformHandle = {
  /** Moves the playhead to a 0 to 1 position, or hides it with null. */
  setPlayhead: (position: number | null) => void
}

/**
 * Draws min and max pairs (min, max, min, max...) as one vertical line per
 * device pixel column.
 */
export function drawWaveform(
  context: CanvasRenderingContext2D,
  peaks: WaveformPeaks,
  width: number,
  height: number,
  color: string,
  amplitude = 1
): void {
  const buckets = Math.floor(peaks.length / 2)
  if (buckets === 0 || width <= 0) {
    return
  }
  const middle = height / 2
  const reach = (height / 2) * amplitude
  context.fillStyle = color
  for (let x = 0; x < width; x += 1) {
    const from = Math.floor((x / width) * buckets)
    const to = Math.max(from + 1, Math.floor(((x + 1) / width) * buckets))
    let low = Infinity
    let high = -Infinity
    for (let bucket = from; bucket < to && bucket < buckets; bucket += 1) {
      low = Math.min(low, peaks[bucket * 2])
      high = Math.max(high, peaks[bucket * 2 + 1])
    }
    if (low > high) {
      continue
    }
    const top = Math.max(0, Math.floor(middle - high * reach))
    const bottom = Math.min(height, Math.ceil(middle - low * reach))
    context.fillRect(x, top, 1, Math.max(1, bottom - top))
  }
}

function peakAmplitude(peaks: WaveformPeaks): number {
  let loudest = 0
  for (let index = 0; index < peaks.length; index += 1) {
    loudest = Math.max(loudest, Math.abs(peaks[index]))
  }
  return loudest
}

type RegionHandleProps = {
  edge: "start" | "end"
  value: number
  min: number
  max: number
  label: string
  disabled: boolean
  format: (position: number) => string
  onValueChange?: (position: number) => void
  onGestureStart?: () => void
  onGestureEnd?: () => void
}

function RegionHandle({
  edge,
  value,
  min,
  max,
  label,
  disabled,
  format,
  onValueChange,
  onGestureStart,
  onGestureEnd,
}: RegionHandleProps) {
  const control = useDragValue({
    value,
    min,
    max,
    onValueChange,
    onGestureStart,
    onGestureEnd,
    defaultValue: edge === "start" ? 0 : 1,
    format,
    disabled,
    orientation: "horizontal",
    trackOvershoot: true,
    wheel: false,
    editable: false,
    // The handle follows the pointer: its full range covers this share of
    // the waveform's width.
    dragRange: (element) =>
      (element.parentElement?.clientWidth ?? 0) * (max - min),
  })

  return (
    <div
      {...control.sliderProps}
      data-slot="waveform-handle"
      data-edge={edge}
      aria-label={label}
      className="group/handle absolute inset-y-0 z-10 w-3 -translate-x-1/2 cursor-ew-resize touch-none outline-none data-disabled:cursor-default"
      style={{ left: `${(control.value * 100).toFixed(4)}%` }}
    >
      <div className="absolute inset-y-0 left-1/2 w-px -translate-x-1/2 bg-(--wf-brand) group-focus-visible/handle:w-0.5" />
      <div
        className={cn(
          "absolute size-2 bg-(--wf-brand) group-hover/handle:scale-125 group-focus-visible/handle:ring-2 group-focus-visible/handle:ring-ring group-data-dragging/handle:scale-125",
          edge === "start"
            ? "top-0 left-1/2 origin-top-left rounded-br-[3px]"
            : "right-1/2 bottom-0 origin-bottom-right rounded-tl-[3px]"
        )}
      />
    </div>
  )
}

type WaveformProps = Omit<React.ComponentProps<"div">, "ref" | "children"> & {
  ref?: React.Ref<WaveformHandle>
  /** Min and max pairs, one pair per bucket: min, max, min, max. */
  peaks: WaveformPeaks
  /** Region start and end, 0 to 1. Set them to show draggable handles. */
  start?: number
  end?: number
  onStartChange?: (start: number) => void
  onEndChange?: (end: number) => void
  /** A handle drag or key hold is one gesture. */
  onGestureStart?: () => void
  onGestureEnd?: () => void
  /** The smallest region the handles can leave, 0 to 1. */
  minRegion?: number
  /** Formats a 0 to 1 position for screen readers, such as a time. */
  formatPosition?: (position: number) => string
  /** Scale the drawing so the loudest peak fills the height. */
  normalize?: boolean
  /** Waveform color as any CSS color. Defaults to `--wf-waveform`. */
  color?: string
  disabled?: boolean
}

const defaultFormatPosition = (position: number) => formatPercent(position, 1)

/** A waveform overview with optional region handles and a playhead. */
function Waveform({
  ref,
  className,
  peaks,
  start,
  end,
  onStartChange,
  onEndChange,
  onGestureStart,
  onGestureEnd,
  minRegion = 0.001,
  formatPosition = defaultFormatPosition,
  normalize = false,
  color = "var(--wf-waveform)",
  disabled = false,
  ...props
}: WaveformProps) {
  const rootRef = React.useRef<HTMLDivElement>(null)
  const canvasRef = React.useRef<HTMLCanvasElement>(null)
  const playheadRef = React.useRef<HTMLDivElement>(null)
  const redraw = React.useRef<(() => void) | null>(null)
  const input = React.useRef({ peaks, normalize, color })

  React.useLayoutEffect(() => {
    input.current = { peaks, normalize, color }
    redraw.current?.()
  }, [peaks, normalize, color])

  React.useEffect(() => {
    const root = rootRef.current
    const canvas = canvasRef.current
    const context = canvas?.getContext("2d")
    if (!root || !canvas || !context) {
      return undefined
    }
    let size: CanvasSize | null = null
    const draw = () => {
      if (!size) {
        return
      }
      const { peaks, normalize, color } = input.current
      const colors = resolveColors(root, {
        wave: color,
        line: "var(--wf-grid-line-strong)",
      })
      context.clearRect(0, 0, size.pixelWidth, size.pixelHeight)
      context.fillStyle = colors.line
      const line = Math.max(1, Math.round(size.dpr))
      context.fillRect(
        0,
        Math.floor((size.pixelHeight - line) / 2),
        size.pixelWidth,
        line
      )
      const loudest = normalize ? peakAmplitude(peaks) : 1
      drawWaveform(
        context,
        peaks,
        size.pixelWidth,
        size.pixelHeight,
        colors.wave,
        loudest > 0 ? 1 / loudest : 1
      )
    }
    redraw.current = draw
    const stopCanvas = observeCanvas(canvas, (next) => {
      size = next
      draw()
    })
    const stopTheme = subscribeTheme(draw)
    return () => {
      redraw.current = null
      stopCanvas()
      stopTheme()
    }
  }, [])

  React.useImperativeHandle(
    ref,
    () => ({
      setPlayhead(position) {
        const playhead = playheadRef.current
        if (!playhead) {
          return
        }
        if (position === null) {
          playhead.style.display = "none"
          return
        }
        const clamped = Math.min(1, Math.max(0, position))
        playhead.style.display = "block"
        playhead.style.left = `${(clamped * 100).toFixed(4)}%`
      },
    }),
    []
  )

  const hasRegion = start !== undefined || end !== undefined
  const from = Math.min(1, Math.max(0, start ?? 0))
  const to = Math.min(1, Math.max(from, end ?? 1))

  return (
    <div
      ref={rootRef}
      data-slot="waveform"
      data-disabled={disabled ? "" : undefined}
      className={cn(
        "relative h-20 w-full overflow-hidden rounded-sm bg-(--wf-meter-bg)/40 select-none data-disabled:opacity-50",
        className
      )}
      {...props}
    >
      <canvas
        ref={canvasRef}
        aria-hidden="true"
        className="absolute inset-0 block size-full"
      />
      {hasRegion ? (
        <>
          <div
            data-slot="waveform-shade"
            className="pointer-events-none absolute inset-y-0 left-0 bg-background/70"
            style={{ width: `${(from * 100).toFixed(4)}%` }}
          />
          <div
            data-slot="waveform-shade"
            className="pointer-events-none absolute inset-y-0 right-0 bg-background/70"
            style={{ width: `${((1 - to) * 100).toFixed(4)}%` }}
          />
        </>
      ) : null}
      <div
        ref={playheadRef}
        data-slot="waveform-playhead"
        aria-hidden="true"
        className="pointer-events-none absolute inset-y-0 hidden w-px bg-(--wf-playhead)"
      />
      {start !== undefined ? (
        <RegionHandle
          edge="start"
          value={from}
          min={0}
          max={Math.max(0, to - minRegion)}
          label="Region start"
          disabled={disabled}
          format={formatPosition}
          onValueChange={onStartChange}
          onGestureStart={onGestureStart}
          onGestureEnd={onGestureEnd}
        />
      ) : null}
      {end !== undefined ? (
        <RegionHandle
          edge="end"
          value={to}
          min={Math.min(1, from + minRegion)}
          max={1}
          label="Region end"
          disabled={disabled}
          format={formatPosition}
          onValueChange={onEndChange}
          onGestureStart={onGestureStart}
          onGestureEnd={onGestureEnd}
        />
      ) : null}
    </div>
  )
}

export { Waveform }
export type { WaveformProps }
